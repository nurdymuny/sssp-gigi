"""Generate manuscript inputs and an offline benchmark report from saved analyses."""
from pathlib import Path
import json, html, re
import pandas as pd
import numpy as np
R=Path(__file__).resolve().parent; O=R/'results/revision'; F=R.parent/'paper'/'figs_v3'; F.mkdir(exist_ok=True)
manifest=json.loads((O/'analysis_manifest.json').read_text()); S=pd.read_csv(O/'summary.csv'); I=pd.read_csv(O/'seed_intervals.csv'); P=pd.read_csv(O/'paired.csv'); M=pd.read_csv(O/'query_medians.csv'); policy=json.loads((O/'policy.json').read_text())
def write(name,text): (F/name).write_text(text,encoding='utf-8')
def val(panel,family,n,arm):return float(S[(S.panel==panel)&(S.family==family)&(S.n==n)&(S.arm==arm)].ms.iloc[0])
def ratio(panel,family,n,arm):return float(I[(I.panel==panel)&(I.family==family)&(I.n==n)&(I.arm==arm)].ratio.iloc[0])
def ms_cell(panel,family,n,arm,digits=3):
    q=S[(S.panel==panel)&(S.family==family)&(S.n==n)&(S.arm==arm)]
    return '--' if q.empty else f'{q.ms.iloc[0]:.{digits}f}'
def esc(s):return str(s).replace('_',r'\_')
def table(name,caption,headers,rows,fmt=None):
    fmt=fmt or ('l'+'r'*(len(headers)-1))
    body='\n'.join(' & '.join(map(str,r))+r' \\' for r in rows)
    style=r'\scriptsize\setlength{\tabcolsep}{2.5pt}' if len(headers)>9 else r'\small\setlength{\tabcolsep}{3.5pt}'
    write(name,r'\begin{table}[tb]\centering'+style+'\n'+r'\caption{'+caption+'}\n'+r'\begin{tabular}{'+fmt+'}\n'+r'\toprule'+'\n'+' & '.join(headers)+r' \\ \midrule'+'\n'+body+'\n'+r'\bottomrule\end{tabular}\end{table}'+'\n')
road_ratios=[ratio('roads',fam,n,'mean_ring') for fam,n in [('NY',264346),('BAY',321270),('COL',435666)]]
counts=pd.read_csv(O/'core/counts.csv');cases=pd.read_csv(O/'core/cases.csv');co=counts.merge(cases[['case','family','n','m']],on='case');w=co.pivot(index=['case','source'],columns='arm',values='pushes');ad=co[co.arm=='degree4'].set_index(['case','source']);live=ad[(ad.admitted>0)&(ad.admitted<ad.m)].index;z=w.loc[live];wins=int((z.degree4<z.binary).sum());losses=int((z.degree4>z.binary).sum());rand=z[[f'random_{i}' for i in range(5)]].median(axis=1)
assert losses==len(z)
abstract=(f"At $n=5000$, mean-width stepping has a sparse-graph speed ratio of {ratio('core','sparse',5000,'original_mean'):.2f} with append-only lists and {ratio('core','sparse',5000,'mean_map'):.2f} with duplicate suppression, relative to the author's standard-library-heap Dijkstra. The append-only solver reaches {ratio('core','sparse',10000,'original_mean'):.2f} at $n=10000$. Suppression changes the dense ratio at $n=5000$ from {ratio('core','dense',5000,'original_mean'):.2f} to {ratio('core','dense',5000,'mean_map'):.2f}. A cyclic-container implementation gives {min(road_ratios):.2f}--{max(road_ratios):.2f} on the road inputs; the maximum-weight/maximum-degree rule is competitive. Degree-gated lookahead increases heap insertions in all {len(z)} live core validation queries.")
sel_old=pd.read_csv(O/'laws/selection.csv');sel_new=pd.read_csv(R/'results/verify_repro/laws/selection.csv')
sel_old=sel_old[sel_old.selected].set_index('case').multiplier
sel_new=sel_new[sel_new.selected].set_index('case').multiplier.reindex(sel_old.index)
assert sel_new.notna().all()
changed=int((sel_old!=sel_new).sum())
write('numbers.tex',f"\\newcommand{{\\GraphCount}}{{{manifest['graphs']:,}}}\n\\newcommand{{\\ValidationCount}}{{{manifest['validation_samples']:,}}}\n\\newcommand{{\\SelectionChanges}}{{{changed}}}\n\\newcommand{{\\SelectionCases}}{{{len(sel_old)}}}\n\\newcommand{{\\AbstractFinding}}{{{abstract}}}\n")
rows=[['Core synthetic','54','3','2','12'],['Density','48','3','2','8'],['Weight laws','48','3','2','8'],['Sparse scale','9','3','2','6'],['DIMACS roads','3','--','6','8'],['Layout','30','3','2','8']]
table('instances.tex','Completed primary panels. Configurations include paired transformations of the same topology, so they are not all independent graph draws. Sources shown are for validation; calibration uses source 0.',['Panel','Configs.','Seeds','Sources','Rounds'],rows,'lrrrr')
rows=[]
for family in ['sparse','clustered','dense','grid','roadlike']:
    for n in sorted(S[(S.panel=='core')&(S.family==family)].n.unique()):
        rows.append([family.capitalize(),f'{n:,}']+[ms_cell('core',family,n,a) for a in ['std_binary','fourary','original_mean','mean_map','mean_ring','mean_degree','selected_grid']])
table('core_table.tex',"Median solve time in milliseconds, taking the median of query medians across three seeds and two held-out sources. List is the author's append-only mean-width solver, retained unchanged; Map and Ring use both duplicate controls. Selected uses a width calibrated on source 0. Preparation is excluded except for the mean computation inside List.",['Family','$n$','Std heap','4-ary','List','Map','Ring',r'$\meanw/\meand$','Selected'],rows,'lrrrrrrrr')
write('core_findings.tex',f"On sparse graphs, the seed-aggregated mean/map speed ratios are {', '.join(f'{ratio("core","sparse",n,"mean_map"):.2f}' for n in [1000,2000,5000,10000])} at $n=1000,2000,5000,10000$, respectively. The corresponding ratios of the list implementation are {', '.join(f'{ratio("core","sparse",n,"original_mean"):.2f}' for n in [1000,2000,5000,10000])}. These are a measured sequence, not an asserted monotone scaling law. On clustered graphs at $n=5000$, the mean/map ratio is {ratio('core','clustered',5000,'mean_map'):.2f}; the 4-ary heap ratio is {ratio('core','clustered',5000,'fourary'):.2f}. On dense graphs at that size, the list and mean/map ratios are {ratio('core','dense',5000,'original_mean'):.3f} and {ratio('core','dense',5000,'mean_map'):.3f}. Removing duplicate work changes the magnitude of the slowdown; its residual must still be measured.\n")
with (F/'core_findings.tex').open('a',encoding='utf-8') as f:
    for fam in ['sparse','clustered']:
        q=I[(I.panel=='core')&(I.family==fam)&(I.n==5000)&(I.arm=='mean_map')].iloc[0]
        a=P[(P.panel=='core')&(P.family==fam)&(P.n==5000)&(P.arm=='mean_map')&(P.seed==42)].sort_values('source').iloc[0]
        f.write(f"For {fam} at $n=5000$, the descriptive seed interval for that ratio is [{q.lo:.2f}, {q.hi:.2f}]. As a within-query example, seed 42, source {int(a.source)} has paired median margin {a.median_margin_ms:.3f} ms with middle-half interval [{a.margin_q25_ms:.3f}, {a.margin_q75_ms:.3f}] ms. ")
    f.write('\n')
    for fam in ['grid','roadlike']:
        q=I[(I.panel=='core')&(I.family==fam)&I.arm.isin(['original_mean','mean_map','mean_ring'])]
        f.write(f"Across the List, Map, and Ring mean-width implementations, {fam} ratios range from {q.ratio.min():.2f} to {q.ratio.max():.2f}. ")
    f.write('The road-network gains below refer to DIMACS inputs, not the six-neighbor synthetic generator. List reaches its largest reported sparse ratio at $n=10000$; Ring has larger ratios at the three smaller sparse sizes.\n')
mech=pd.read_csv(O/'mechanisms.csv');rows=[]
for family in ['sparse','clustered','dense']:
    for arm,label in [('list_both','List/list'),('skip_only','Skip only'),('heavy_only','Heavy only'),('mean_map','Both')]:
        z=mech[(mech.family==family)&(mech.n==5000)&(mech.arm==arm)].iloc[0]
        rows.append([family.capitalize(),label]+[f'{z[k]:,.0f}' for k in ['vertex_scans','heavy','phases']])
table('mechanism_table.tex','Median operation counts at $n=5000$, same mean width and map container. Vertex scans omit skipped entries; heavy relaxations count evaluated heavy candidates.',['Family','Controls','Vertex scans','Heavy relaxations','Phases'],rows,'llrrr')
z=I[(I.panel=='density')&(I.n==5000)&I.arm.isin(['mean_map','mean_degree'])].pivot(index='family',columns='arm',values='ratio');z['degree']=z.index.str.replace('er_d','').astype(int);z=z.sort_values('degree')
write('density_findings.tex',f"At $n=5000$, the mean/map ratios span {z.mean_map.min():.2f}--{z.mean_map.max():.2f} across the degree sweep, while mean/degree/map spans {z.mean_degree.min():.2f}--{z.mean_degree.max():.2f}. The mean/map ratio is {float(z[z.degree==2].mean_map.iloc[0]):.2f} at expected degree 2, with a descriptive seed interval spanning 1, and {float(z[z.degree==64].mean_map.iloc[0]):.2f} at degree 64. At expected degree 256 the two ratios are {float(z[z.degree==256].mean_map.iloc[0]):.2f} and {float(z[z.degree==256].mean_degree.iloc[0]):.2f}. Figure~\\ref{{fig:density}} shows the full transition rather than identifying a universal cutoff from one dense endpoint.\n")
z=I[(I.panel=='laws')&(I.arm=='mean_map')]
write('law_findings.tex',f"Across these law/topology combinations, the mean/map ratio ranges from {z.ratio.min():.2f} ({esc(z.loc[z.ratio.idxmin(),'family'])}) to {z.ratio.max():.2f} ({esc(z.loc[z.ratio.idxmax(),'family'])}). The full paired margins and named-rule timings are retained in the artifact.\n")
rows=[]
for panel,family,n in [('scale','sparse_fast',10000),('scale','sparse_fast',100000),('scale','sparse_fast',1000000),('roads','NY',264346),('roads','BAY',321270),('roads','COL',435666)]:
    row=[esc(family),f'{n:,}']
    for arm in ['std_binary','fourary','mean_map','mean_ring','mean_degree','max_maxdegree','selected_grid','caliber','radix']:
        z=S[(S.panel==panel)&(S.family==family)&(S.n==n)&(S.arm==arm)];row.append('--'if z.empty else f'{z.ms.iloc[0]:.2f}')
    rows.append(row)
table('large_table.tex','Median solve milliseconds at scale and on DIMACS roads. Scale has three seeds and two sources; each road graph has six sources. Max/degree uses the Boost-documented width in our map solver; Selected calibrates that solver on source 0. Radix uses unchanged integer weights.',['Input','$n$','Std heap','4-ary','Map','Ring',r'$\meanw/\meand$','Max/degree','Selected','Caliber','Radix'],rows,'lrrrrrrrrrr')
write('large_findings.tex',f"At one million vertices, mean/map and mean/ring have ratios {ratio('scale','sparse_fast',1000000,'mean_map'):.2f} and {ratio('scale','sparse_fast',1000000,'mean_ring'):.2f} against the binary reference; the 4-ary ratio is {ratio('scale','sparse_fast',1000000,'fourary'):.2f}. On NY, BAY, and COL, respectively, the mean/ring ratios are {', '.join(f'{ratio("roads",fam,n,"mean_ring"):.2f}'for fam,n in [('NY',264346),('BAY',321270),('COL',435666)])}; the radix ratios are {', '.join(f'{ratio("roads",fam,n,"radix"):.2f}'for fam,n in [('NY',264346),('BAY',321270),('COL',435666)])}. These integer-road comparisons are necessary context for any advantage over one binary-heap implementation.\n")
boost_ratios=[ratio('roads',fam,n,'max_maxdegree') for fam,n in [('NY',264346),('BAY',321270),('COL',435666)]]
selected_ratios=[ratio('roads',fam,n,'selected_grid') for fam,n in [('NY',264346),('BAY',321270),('COL',435666)]]
spread=max(max(a,b,c)/min(a,b,c)-1 for a,b,c in zip(road_ratios,boost_ratios,selected_ratios))
with (F/'large_findings.tex').open('a',encoding='utf-8') as f:
    f.write(f"The maximum-weight/maximum-degree ratios on NY, BAY, and COL are {', '.join(f'{x:.2f}' for x in boost_ratios)}, and the selected-width ratios are {', '.join(f'{x:.2f}' for x in selected_ratios)}. The largest within-road spread among these two search-free implementations and calibration is {100*spread:.1f}\\%. This comparison includes a container difference: the mean rule uses Ring here, whereas the other two use Map. It does not establish a resolved winner for small differences.\n")
layout=M[(M.panel=='layout')&(M.key!='layout:29')].copy()
layout.to_csv(O/'layout_retained_query_medians.csv',index=False)
lay=layout[(layout.n==100000)&(layout.arm=='mean_ring')].groupby('family').ns.median()/1e6
write('layout_findings.tex',f"For $n=100000$, median ring solve times are {lay['weight']:.3f} ms in weight order, {lay['destination']:.3f} ms in destination order, {lay['random']:.3f} ms in random adjacency order, {lay['bfs']:.3f} ms after BFS relabeling, and {lay['rcm']:.3f} ms after reversal. Layout preparation is recorded separately; these figures describe solve time only.\n")
with (F/'layout_findings.tex').open('a',encoding='utf-8') as f:
    f.write('The layout summary excludes both sources of case 29 (BFS, $n=100000$, seed 2026), flagged during review for background synchronization interference. BFS at that size consequently has two seeds and four queries; other layouts retain three seeds and six queries. Raw samples, including the excluded case, remain available.\n')
gate=pd.read_csv(O/'ablate2.csv');rows=[]
for n in [1000,2000,5000,10000]:
    z=gate[(gate.family=='sparse')&(gate.n==n)].set_index('arm');rows.append([f'{n:,}']+[f'{z.loc[a,"pushes"]:,}'for a in ['never','degree4','random_exact','high_high','complement','always','adaptive']])
table('gate_table.tex','Source-zero sparse gate reproduction, seed 42. Random admits exactly the same number of graph arcs as Degree; it is a distinct control from an expected-rate coin. High/high and Complement are different predicates.',['$n$','NEVER','Degree','Random','High/high','Complement','ALWAYS','Adaptive'],rows,'rrrrrrrr')
z=w.loc[live]
write('gate_findings.tex',f"Among {len(z)} live core graph/source cases, degree gating uses fewer insertions than NEVER in {wins}, more in {losses}, and ties in {len(z)-wins-losses}. It uses fewer insertions than the median of five exact-admission random masks in {int((z.degree4<rand).sum())} cases. These are paired operation counts, not independent significance tests. For source zero on the sparse configurations, the gate reproduces 1445, 2796, 7210, and 14195 insertions, while NEVER uses 1397, 2697, 6879, and 13603. The saturated grids and dead high-degree cases are classified separately in the saved table.\n")
clock=pd.read_csv(O/'gate_clock.csv');clock=clock[clock.panel=='core'];ratios=clock.groupby(['family','n','seed','source']).ratio.median();live_clock=ratios.loc['sparse'];all_geo=float(np.exp(np.log(live_clock).mean()))
with (F/'gate_findings.tex').open('a',encoding='utf-8') as f:
    f.write(f"On the clock, the geometric NEVER/GSR-1 ratio on the {len(live_clock)} live sparse queries is {all_geo:.3f}; {int((live_clock>1).sum())} query medians favor scouting. This compares the shared heap implementation in the same timing rounds. Small median differences alone are not called resolved wins; paired round margins are available in \\code{{gate\\_clock.csv}}.\n")
am=pd.read_csv(O/'tuning_amortization.csv')
write('amortization.tex',f"Across calibrated queries, charging calibration gives geometric selected-width/mean-map cost ratios of {', '.join(f'{x:.2f}'for x in am.geomean_selected_over_mean)} for $Q=1,10,100$. These are projections using repeated representative query times, not measured multiquery services; common graph-loading cost is omitted from the ratio.\n")
write('policy_findings.tex',f"The fitted threshold is $\\meand={policy['threshold']:.3f}$, choosing \\code{{{esc(policy['left'])}}} below it and \\code{{{esc(policy['right'])}}} above it. The best fixed training arm is \\code{{{esc(policy['best_fixed'])}}}. On {policy['test_graphs']} held-out configurations, the switch has geometric regret {policy['geometric_regret']:.3f}, versus {policy['fixed_regret']:.3f} for that fixed arm; worst per-query regret is {policy['worst_regret']:.2f}. Charging the full feature routine gives switch/fixed cost ratios {', '.join(f'{x["geomean_policy_over_fixed"]:.2f}'for x in policy['feature_charged'])} at $Q=1,10,100$.\n")
loo=pd.read_csv(O/'policy_leave_family_out.csv')
with (F/'policy_findings.tex').open('a',encoding='utf-8') as f:
    er=loo[loo.family=='independent_edge'].iloc[0]
    f.write(f"Holding all independent-edge topologies out of training raises regret on that held-out topology to {er.policy_regret:.3f}. This exposes a distributional dependence that a seed-only split does not test.\n")
substrate=(O/'substrate_stdout.txt').read_text(encoding='utf-8-sig')
assert 'PASS all edge identities/weights' in substrate
incident=int(re.search(r'circulation measured_vertices=(\d+)',substrate).group(1))
write('substrate.tex',f'The saved fixture output reports PASS for edge identities and weights, parallel arcs, the isolated vertex, and all-source distances after mmap reload. Its circulation call reports {incident} incident vertices.\n')
k=pd.read_csv(O/'gigi_gate.csv');keys=['family','seed','source','rate'];kw=k[k.arm.isin(['low','high','never'])].pivot(index=keys,columns='arm',values='pushes');null=k[k.arm=='weight_permutation'].groupby(keys).pushes.median();vp=k[k.arm=='vertex_permutation'].groupby(keys).pushes.median()
write('k_findings.tex',f"Across {len(kw)} family/seed/source/rate cases, low-$K$ uses fewer insertions than NEVER in {int((kw.low<kw.never).sum())}. It is below the median weight-permutation null in {int((kw.low<null).sum())} cases, and below the median vertex-permutation null in {int((kw.low<vp).sum())}. There are 100 null replicates per field transformation. These counts summarize related cases; they are not a binomial test of a common chance probability.\n")
with (F/'k_findings.tex').open('a',encoding='utf-8') as f:
    f.write(f"High-$K$ also reduces insertions in {int((kw.high<kw.never).sum())} of {len(kw)} cases; its geometric insertion ratio to NEVER is {float(np.exp(np.log(kw.high/kw.never).mean())):.3f}. Neither gate demonstrates an insertion benefit over NEVER. Low-$K$ appears more favorable against vertex-score permutations than against weight permutations; these descriptive comparisons do not establish statistical equivalence to either null.\n")
pot=pd.read_csv(O/'potential_order/samples.csv')
pm=pot.groupby(['family','seed','source','arm','layout'],as_index=False).agg(ns=('ns','median'),prep_ns=('prep_ns','median'))
pw=pm.pivot(index=['family','seed','source','arm'],columns='layout',values='ns')
pc=pm.pivot(index=['family','seed','source','arm'],columns='layout',values='prep_ns')
stat=[]
for fam in ['sparse','clustered']:
    z=pw.loc[fam].dropna();cost=pc.loc[fam].loc[z.index,'potential']
    rat=float(np.exp(np.log(z.identity/z.potential).mean()))
    paid=float(np.exp(np.log((cost+100*z.potential)/(100*z.identity)).mean()))
    bfs=float(np.exp(np.log(z.bfs/z.potential).mean()));rcm=float(np.exp(np.log(z.rcm/z.potential).mean()))
    stat.append(f"For {fam}, the geometric identity/potential solve ratio across the two solvers is {rat:.3f}; the corresponding BFS/potential and RCM/potential ratios are {bfs:.3f} and {rcm:.3f}. After charging preprocessing, potential/identity cost at $Q=100$ is {paid:.2f}.")
responses=(O/'potential_order/responses.txt').read_text()
assert responses.count('status=422')==3
write('potential.tex',' '.join(stat)+' All three symmetric grid inputs return the expected 422 refusal after reciprocal flow cancellation. The raw responses and disclosures are retained. These experiments support safe use as an ordering input, without establishing a general acceleration.\n')
pm.to_csv(O/'potential_order/medians.csv',index=False)
mut=json.loads((O/'mutation_results.json').read_text());assert [x['exit_code']==0 for x in mut]==[True,False,False]
write('checks.tex',f"The completed primary panels contain {manifest['calibration_samples']:,} calibration and {manifest['validation_samples']:,} validation timings. The duplicate-mechanism test passes in the control crate and fails when either suppression mechanism is removed in an isolated source copy. The complete harness target tests and the GIGI substrate checks are saved alongside the run outputs.\n")
# Offline artifact page: generated from the same data as the manuscript.
report=f'''<!doctype html><html lang="en"><meta charset="utf-8"><title>Sequential SSSP benchmark</title><style>body{{font:16px/1.5 system-ui;max-width:1300px;margin:40px auto;padding:0 24px;color:#182331}}h1,h2{{line-height:1.15}}table{{border-collapse:collapse;font-size:13px}}th,td{{padding:8px 12px;border-bottom:1px solid #dbe3eb;text-align:right}}th{{background:#eef3f8}}section{{overflow:auto;margin:32px 0}}.meta{{color:#526275}}img{{max-width:100%}}a{{color:#16558c}}</style><h1>Bucket width, duplicate work, and local lookahead</h1><p class="meta">Bee Rosa Davis · September 2026 · Local reproducible benchmark report</p><p>{manifest['graphs']} primary graph configurations; {manifest['validation_samples']:,} validation samples; {manifest['calibration_samples']:,} calibration samples. Three synthetic seeds, held-out sources, balanced timing order, complete distance checks.</p><p>Ratios compare arms measured in the same rounds. Selected widths are trained on source 0. They are not oracles. Graph admission and executed scouting work are separate quantities.</p><h2>Core solve times</h2><img src="figs_v3/core.png" alt="Paired speed ratios by graph family"><section>{S[(S.panel=='core')&S.arm.isin(['std_binary','fourary','original_mean','mean_map','mean_ring','mean_degree','selected_grid'])].pivot(index=['family','n'],columns='arm',values='ms').round(4).to_html()}</section><h2>Gate mechanisms</h2><p>The fixed gate is live on sparse graphs, saturated on grids, and dead on the dense, clustered, and road-like source-zero instances. NEVER is the relevant no-lookahead comparator. High/high uses AND; the logical complement uses OR.</p><section>{gate.to_html(index=False)}</section><h2>Real roads and scale</h2><section>{S[S.panel.isin(['scale','roads'])].pivot(index=['panel','family','n'],columns='arm',values='ms').round(3).to_html()}</section><h2>Reproduction</h2><p><a href="davis_manifold_sssp_v3.pdf">Paper</a> · <a href="../harness/REVISION_PROTOCOL.md">Protocol</a> · <a href="../harness/results/revision/analysis_manifest.json">Run manifest</a> · <a href="../harness/results/revision/paired.csv">Paired differences and IQRs</a></p><p>Timings exclude loading and most preparation. One machine, three synthetic seeds, and descriptive uncertainty do not establish universal performance. The report is generated by render_results.py.</p></html>'''
report=report.replace('<h2>Reproduction</h2>', '<h2>Layout exclusion</h2><p>Both sources of layout case 29 (BFS, n=100000, seed 2026) are excluded from the reported layout summary following reported synchronization interference. Raw measurements remain intact. BFS retains two seeds; other layouts retain three.</p><section>'+layout[layout.arm=='mean_ring'].groupby(['family','n']).ns.median().div(1e6).round(3).rename('median_ms').to_frame().to_html()+'</section><h2>Reproduction</h2>')
(R.parent/'paper'/'benchmark_v3.html').write_text(report,encoding='utf-8')
print('Manuscript inputs and benchmark_v3.html generated')
