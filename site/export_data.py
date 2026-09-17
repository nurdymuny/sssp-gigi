"""Export published summaries and original paired timing rounds; do not rerun solvers."""
from pathlib import Path
import csv, json, hashlib, shutil
R=Path(__file__).resolve().parent
O=R.parent/'sssp_ablation/results/revision'
def rows(path):
    with path.open(newline='',encoding='utf-8-sig') as f:return list(csv.DictReader(f))
I=rows(O/'seed_intervals.csv'); S=rows(O/'summary.csv')
arms=['std_binary','original_mean','mean_map','mean_ring','max_maxdegree','selected_grid']
spec=[('sparse','Sparse network','core','sparse',10000),('million','A million places','scale','sparse_fast',1000000),('ny','New York roads','roads','NY',264346),('bay','Bay Area roads','roads','BAY',321270),('col','Colorado roads','roads','COL',435666),('dense','A densely connected network','core','dense',5000),('grid','A grid network','core','grid',10000)]
scenarios=[]
cache={p:rows(O/p/'samples.csv') for p in ['core','scale','roads']}
for id,title,panel,family,n in spec:
    summary=[]
    for arm in arms:
        z=[x for x in S if x['panel']==panel and x['family']==family and int(x['n'])==n and x['arm']==arm]
        if not z:continue
        q=[x for x in I if x['panel']==panel and x['family']==family and int(x['n'])==n and x['arm']==arm]
        summary.append(dict(arm=arm,ms=float(z[0]['ms']),ratio=float(q[0]['ratio']) if q else 1))
    runs={}
    for x in cache[panel]:
        if x['stage']=='calibration' or x['family']!=family or int(x['n'])!=n or x['arm'] not in arms:continue
        key=tuple(int(x[k]) for k in ['seed','source','round'])
        if key not in runs:runs[key]=dict(seed=key[0],source=key[1],round=key[2],case=int(x['case']),times={},positions={})
        runs[key]['times'][x['arm']]=int(x['ns'])/1e6
        runs[key]['positions'][x['arm']]=int(x['position'])
    scenarios.append(dict(id=id,title=title,panel=panel,family=family,n=n,summary=summary,runs=list(runs.values())))
density=[]
for x in I:
    if x['panel']=='density' and int(x['n'])==5000 and x['arm']=='mean_map':
        density.append(dict(degree=int(x['family'][4:]),ratio=float(x['ratio']),lo=float(x['lo']),hi=float(x['hi'])))
density.sort(key=lambda x:x['degree'])
mech=[x for x in rows(O/'mechanisms.csv') if x['family']=='dense' and int(x['n'])==5000]
data=dict(scenarios=scenarios,density=density,mechanisms=mech,
    provenance=dict(configurations=192,validation_timings=43956,calibration_timings=9504,
        study='Bucket Width, Duplicate Work, and Local Lookahead in Sequential Shortest Paths',author='Bee Rosa Davis',
        date='September 2026',machine='Intel Core i7-13620H · 64 GB RAM · Windows 11 · one software thread',
        sources={str(p.relative_to(O)):hashlib.sha256(p.read_bytes()).hexdigest() for p in [O/'summary.csv',O/'seed_intervals.csv',O/'mechanisms.csv',*[O/p/'samples.csv' for p in cache]]}))
(R/'dist/data.mjs').write_text('export default '+json.dumps(data,separators=(',',':'))+';\n',encoding='utf-8')
evidence=R/'dist/evidence';evidence.mkdir(exist_ok=True)
for name in ['summary.csv','seed_intervals.csv','mechanisms.csv']:
    shutil.copyfile(O/name,evidence/name)
shutil.copyfile(R.parent/'davis_manifold_sssp_v3.pdf',evidence/'paper.pdf')
shutil.copyfile(R.parent/'sssp_review_artifact_v3.zip',evidence/'study-artifact.zip')
print(f'Exported {len(scenarios)} scenarios, {sum(len(x["runs"]) for x in scenarios)} recorded rounds; primary data unchanged.')
