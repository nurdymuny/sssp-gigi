"""Read-only analysis of completed panels; all tables derive from saved samples."""
from pathlib import Path
import json, hashlib, math
import numpy as np
import pandas as pd
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.ticker import NullLocator
ROOT=Path(__file__).resolve().parent
OUT=ROOT/'results/revision'
FIG=ROOT.parent/'figs_v3'; FIG.mkdir(exist_ok=True)
panels=['core','density','laws','scale','roads','layout']
samples=[]; cases=[]; counts=[]
for panel in panels:
    assert (OUT/panel/'completed.txt').exists(), f'incomplete {panel}'
    for name, sink in [('samples',samples),('cases',cases),('counts',counts)]:
        d=pd.read_csv(OUT/panel/f'{name}.csv'); d['panel']=panel; d['key']=panel+':'+d['case'].astype(str);sink.append(d)
s=pd.concat(samples,ignore_index=True); c=pd.concat(cases,ignore_index=True); counts=pd.concat(counts,ignore_index=True)
v=s[s.stage!='calibration'].copy()
assert not v.duplicated(['key','source','arm','round']).any()
blocks={index:block.sort_values('position').arm.tolist() for index,block in v.groupby(['key','source','round'])}
for (key,source,rnd), arms in blocks.items():
    if rnd%2==0:assert arms==list(reversed(blocks[key,source,rnd+1]))
med=v.groupby(['key','panel','family','n','m','seed','source','arm'],as_index=False).agg(ns=('ns','median'),prep_ns=('prep_ns','median'))
med.to_csv(OUT/'query_medians.csv',index=False)
p=v.pivot(index=['key','panel','family','n','seed','source','round'],columns='arm',values='ns')
paired=[]
for arm in p.columns:
    z=p[['std_binary',arm]].dropna() if arm!='std_binary' else None
    if z is None: continue
    for idx, x in z.groupby(level=['key','panel','family','n','seed','source']):
        diff=(x.std_binary-x[arm])/1e6; ratio=x.std_binary/x[arm]
        paired.append(dict(zip(['key','panel','family','n','seed','source'],idx),arm=arm,median_margin_ms=diff.median(),margin_q25_ms=diff.quantile(.25),margin_q75_ms=diff.quantile(.75),median_ratio=ratio.median(),positive_rounds=int((diff>0).sum()),rounds=len(x)))
paired=pd.DataFrame(paired);paired.to_csv(OUT/'paired.csv',index=False)
gateclock=p[['binary','degree4']].dropna().copy()
gateclock['ratio']=gateclock.binary/gateclock.degree4
gateclock['margin_ms']=(gateclock.binary-gateclock.degree4)/1e6
gateclock.reset_index().to_csv(OUT/'gate_clock.csv',index=False)
summary=med.groupby(['panel','family','n','arm']).agg(ms=('ns',lambda x:np.median(x)/1e6),min_ms=('ns',lambda x:np.min(x)/1e6),max_ms=('ns',lambda x:np.max(x)/1e6),queries=('ns','size')).reset_index()
summary.to_csv(OUT/'summary.csv',index=False)
print('MANIFEST',len(c),'graphs',len(med),'query arms',len(v),'validation samples',len(s)-len(v),'calibration samples')
print(summary[(summary.panel=='core')&summary.arm.isin(['std_binary','fourary','original_mean','mean_map','mean_ring','mean_degree','selected_grid'])].pivot(index=['family','n'],columns='arm',values='ms').round(4).to_string())
print('SCALE/ROADS\n',summary[summary.panel.isin(['scale','roads'])&summary.arm.isin(['std_binary','fourary','mean_map','mean_ring','mean_degree','caliber','radix','selected_grid'])].pivot(index=['panel','family','n'],columns='arm',values='ms').round(3).to_string())
# Seed-level ratios and a descriptive bootstrap interval; sources are averaged
# inside each seed. With only three seeds the interval is deliberately qualified.
rng=np.random.default_rng(20260916); intervals=[]
for (panel,fam,n,arm), z in paired.groupby(['panel','family','n','arm']):
    values=z.groupby('seed').median_ratio.apply(lambda x:np.log(x).mean()).to_numpy()
    point=float(np.exp(values.mean()))
    if len(values)>=3:
        boots=np.exp(rng.choice(values,(4000,len(values)),replace=True).mean(axis=1));lo,hi=np.quantile(boots,[.025,.975])
    else:lo=hi=float('nan')
    intervals.append(dict(panel=panel,family=fam,n=n,arm=arm,ratio=point,lo=lo,hi=hi,seeds=len(values)))
intervals=pd.DataFrame(intervals);intervals.to_csv(OUT/'seed_intervals.csv',index=False)
# Policy: a single threshold in observed average degree, optimized on seed42.
# Both leaves choose from fixed implementations, without source or family inputs.
policy_arms=['fourary','mean_map','mean_ring','mean_degree']
query=med[med.arm.isin(policy_arms)].pivot(index=['key','panel','family','n','seed','source'],columns='arm',values='ns').dropna().reset_index().merge(c[['key','degree','feature_ns']],on='key')
train=query[(query.seed==42)&query.panel.isin(['core','density','laws'])]
test=query[(query.seed.isin([1729,2026]))&query.panel.isin(['core','density','laws','scale'])]
def fit(x):
    costs=x[policy_arms].to_numpy();mins=costs.min(axis=1)
    fixed=min(policy_arms,key=lambda a:np.log(x[a]/mins).mean())
    best=(float('inf'),0,fixed,fixed)
    degrees=np.sort(x.degree.unique());thresholds=np.r_[-np.inf,(degrees[:-1]+degrees[1:])/2,np.inf]
    for t in thresholds:
        m=x.degree<=t
        for a in policy_arms:
            for b in policy_arms:
                chosen=np.where(m,x[a],x[b]);loss=np.log(chosen/mins).mean()
                if loss<best[0]:best=(loss,float(t),a,b)
    return fixed,best
fixed,best=fit(train);_,threshold,left,right=best
query['chosen']=np.where(query.degree<=threshold,left,right)
query['policy_ns']=[r[r.chosen] for _,r in query.iterrows()]
query['best_ns']=query[policy_arms].min(axis=1)
query['fixed_ns']=query[fixed]
query['split']=np.where(query.seed==42,'train','holdout')
query.to_csv(OUT/'policy_queries.csv',index=False)
test=query.loc[test.index] if False else query[(query.seed.isin([1729,2026]))&query.panel.isin(['core','density','laws','scale'])]
policy=dict(train_graphs=int(train.key.nunique()),test_graphs=int(test.key.nunique()),threshold=threshold,left=left,right=right,best_fixed=fixed,
    geometric_regret=float(np.exp(np.log(test.policy_ns/test.best_ns).mean())),fixed_regret=float(np.exp(np.log(test.fixed_ns/test.best_ns).mean())),worst_regret=float((test.policy_ns/test.best_ns).max()))
print('POLICY',policy)
amort=[]
for q in [1,10,100]:
    # Feature vector includes median sorting; this is an upper-cost generic policy
    # interface, whereas a degree-only implementation can read n,m directly.
    policy_cost=test.feature_ns+q*test.policy_ns
    ratio=policy_cost/(q*test.fixed_ns)
    amort.append(dict(Q=q,geomean_policy_over_fixed=float(np.exp(np.log(ratio).mean()))))
policy['feature_charged']=amort
(OUT/'policy.json').write_text(json.dumps(policy,indent=2))
# Leave-one-family-out evaluation on held-out graph seeds; no tuning on that family.
family_rows=[]
def topology(fam):
    if fam=='dense' or fam.startswith('er_d'):return 'independent_edge'
    if fam.startswith('sparse'):return 'sparse'
    if fam.startswith('clustered'):return 'clustered'
    return fam
train=train.copy();test=test.copy();train['topology']=train.family.map(topology);test['topology']=test.family.map(topology)
for fam in sorted(train.topology.unique()):
    tr=train[train.topology!=fam];te=test[test.topology==fam]
    if te.empty:continue
    fx,(_,th,l,r)=fit(tr);pred=np.where(te.degree<=th,te[l],te[r]);bm=te[policy_arms].min(axis=1)
    family_rows.append(dict(family=fam,threshold=th,left=l,right=r,policy_regret=float(np.exp(np.log(pred/bm).mean())),fixed_regret=float(np.exp(np.log(te[fx]/bm).mean()))))
pd.DataFrame(family_rows).to_csv(OUT/'policy_leave_family_out.csv',index=False)
# Selection amortization, charging recorded calibration to the selected width.
am=[]
wide=med.pivot(index=['key','panel','family','n','seed','source'],columns='arm',values='ns')
prep=med.pivot(index=['key','panel','family','n','seed','source'],columns='arm',values='prep_ns')
for q in [1,10,100]:
    z=wide[['selected_grid','mean_map']].dropna();rat=(prep.loc[z.index,'selected_grid']+q*z.selected_grid)/(prep.loc[z.index,'mean_map']+q*z.mean_map)
    am.append(dict(Q=q,geomean_selected_over_mean=float(np.exp(np.log(rat).mean())),queries=len(z)))
pd.DataFrame(am).to_csv(OUT/'tuning_amortization.csv',index=False)
print('AMORT',am)
# Counter summaries compare the identical implemented heap path (binary == NEVER).
co=counts.merge(c[['key','family','n','seed','m']],on='key')
cw=co.pivot(index=['key','family','n','seed','source'],columns='arm',values='pushes')
live=co[(co.arm=='degree4')&(co.admitted>0)&(co.admitted<co.m)]
livekeys=set(zip(live.key,live.source));cl=cw[[ (i[0],i[-1]) in livekeys for i in cw.index]]
print('LIVE GATE',len(cl),'fewer pushes than NEVER',int((cl.degree4<cl.binary).sum()),'more',int((cl.degree4>cl.binary).sum()))
mechanism=co[(co.panel=='core')&co.arm.isin(['list_both','skip_only','heavy_only','mean_map'])].groupby(['family','n','arm'])[['vertex_scans','unchanged','heavy','phases','scans']].median()
mechanism.to_csv(OUT/'mechanisms.csv')
print('DENSE MECHANISM\n',mechanism.loc['dense'].to_string())
# Plots: geometric seed-aggregated solve ratios, never cross-session quotients.
plt.rcParams.update({'font.family':'DejaVu Sans','font.size':9,'axes.spines.top':False,'axes.spines.right':False,'pdf.fonttype':42})
fig,axs=plt.subplots(1,3,figsize=(10.2,3.3))
for ax,fam in zip(axs,['sparse','clustered','dense']):
    for arm,label in [('original_mean','List implementation'),('mean_map','Deduplicated map'),('mean_degree','Mean / degree'),('fourary','4-ary heap')]:
        z=intervals[(intervals.panel=='core')&(intervals.family==fam)&(intervals.arm==arm)].sort_values('n')
        ax.plot(z.n,z.ratio,marker='o',markersize=3,label=label);ax.fill_between(z.n.to_numpy(),z.lo.to_numpy(),z.hi.to_numpy(),alpha=.10)
    ax.axhline(1,color='black',lw=.7,ls='--');ax.set_xscale('log');ax.set_yscale('log');ax.set_title(fam.capitalize());ax.set_xlabel('Vertices');ax.set_ylabel('Binary baseline / solve time')
    ticks=[1000,2000,5000]+([10000] if fam!='dense' else []);ax.set_xticks(ticks,[f'{x//1000}k' for x in ticks]);ax.xaxis.set_minor_locator(NullLocator())
handles,labels=axs[0].get_legend_handles_labels();fig.legend(handles,labels,ncol=4,loc='upper center',fontsize=8,frameon=False);fig.tight_layout(rect=[0,0,1,.91]);fig.savefig(FIG/'core.pdf');fig.savefig(FIG/'core.png',dpi=180);plt.close(fig)
fig,axs=plt.subplots(1,2,figsize=(8.8,3))
for ax,n in zip(axs,[1000,5000]):
    for arm,label in [('mean_map','Mean'),('mean_degree','Mean / degree'),('max_maxdegree','Max / max degree'),('selected_grid','Selected grid')]:
        z=intervals[(intervals.panel=='density')&(intervals.n==n)&(intervals.arm==arm)].copy();z['deg']=z.family.str.replace('er_d','').astype(int);z=z.sort_values('deg');ax.plot(z.deg,z.ratio,marker='o',markersize=3,label=label)
    ax.axhline(1,color='black',lw=.7,ls='--');ax.set_xscale('log',base=2);ax.set_yscale('log');ax.set_title(f'n = {n:,}');ax.set_xlabel('Expected out-degree');ax.set_ylabel('Binary baseline / solve time')
axs[0].legend(fontsize=7);fig.tight_layout();fig.savefig(FIG/'density.pdf');fig.savefig(FIG/'density.png',dpi=180);plt.close(fig)
manifest=dict(graphs=len(c),query_arms=len(med),validation_samples=len(v),calibration_samples=len(s)-len(v),panels={str(k):int(x)for k,x in c.groupby('panel').size().items()},source_sha256={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()for p in [ROOT/'src/revision.rs',ROOT/'src/bin/campaign.rs',ROOT/'REVISION_PROTOCOL.md']})
env=json.loads((OUT/'environment.json').read_text(encoding='utf-8-sig'))
manifest['measured_source_sha256']={'src/revision.rs':env['source'].lower(),'src/bin/campaign.rs':env['harness'].lower(),'REVISION_PROTOCOL.md':env['protocol'].lower()}
manifest['source_sha256_note']='Current source hashes; measured_source_sha256 refers to the archived primary timing source.'
(OUT/'analysis_manifest.json').write_text(json.dumps(manifest,indent=2))
