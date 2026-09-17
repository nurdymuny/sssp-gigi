"""Pack verified input geometry and recomputed distances for display, not timing."""
from pathlib import Path
import csv,json,hashlib
root=Path(__file__).resolve().parent
out=root/'geometry_export/output'
def rows(p):
    with p.open(newline='') as f:return list(csv.DictReader(f))
verified={(x['scenario'],int(x['seed'])):x for x in rows(out/'fingerprints.csv')}
views={}
for file in sorted(out.glob('*.nodes.csv')):
    key=file.name.removesuffix('.nodes.csv');scenario,seed,source=key.split('-');nodes=rows(file)
    rawx=[float(x['x']) for x in nodes];rawy=[float(x['y']) for x in nodes]
    left,right=min(rawx),max(rawx);top,bottom=min(rawy),max(rawy)
    span=max(right-left,bottom-top,1e-9)
    packed=[];source_index=None
    for i,n in enumerate(nodes):
        x=round(((float(n['x'])-(left+right)/2)/span+.5)*65535)
        y=round(((float(n['y'])-(top+bottom)/2)/span+.5)*65535)
        packed.extend([x,y,int(n['phase'])])
        if n['source']=='true':source_index=i
    assert source_index is not None
    edges=[]
    for e in rows(out/f'{key}.edges.csv'):
        u,v=int(e['u']),int(e['v']);assert 0<=u<len(nodes) and 0<=v<len(nodes);edges.extend([u,v])
    evidence=verified[(scenario,int(seed))]
    views[key]=dict(nodes=packed,edges=edges,sourceIndex=source_index,fullN=int(evidence['n']),fullM=int(evidence['m']),fingerprint=evidence['fingerprint'],
        layout='geographic' if scenario in ['ny','bay','col'] else 'grid' if scenario=='grid' else 'topological',
        sampling='Uniform arc sample; endpoints and source retained' if scenario in ['ny','bay','col'] else 'Complete 100 × 100 grid' if scenario=='grid' else 'Source-rooted breadth traversal; bounded vertex and arc sample')
assert len(views)==42,len(views)
(root/'dist/geometry.mjs').write_text('export default '+json.dumps(views,separators=(',',':'))+';\n',encoding='utf-8')
meta=dict(views=len(views),graphs=len(verified),method='Graph fingerprints match primary inputs; distances recomputed by original Dijkstra. Reveal order uses quantized final distance, not measured execution events.',coordinate_sources=json.loads((out/'coordinate_sources.json').read_text()),fingerprints=list(verified.values()))
(root/'dist/evidence/geometry-provenance.json').write_text(json.dumps(meta,indent=2),encoding='utf-8')
print(f'Packed {len(views)} source-specific views from {len(verified)} fingerprint-verified inputs.')
