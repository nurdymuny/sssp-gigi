from pathlib import Path
import urllib.request, gzip, hashlib, json
root=Path(__file__).resolve().parent
out=root/'geometry_export/output';out.mkdir(parents=True,exist_ok=True)
manifest=[]
for name in ['NY','BAY','COL']:
    url=f'https://www.diag.uniroma1.it/challenge9/data/USA-road-d/USA-road-d.{name}.co.gz'
    zipped=out/f'USA-road-d.{name}.co.gz'
    if not zipped.exists(): urllib.request.urlretrieve(url,zipped)
    raw=gzip.decompress(zipped.read_bytes());(out/f'USA-road-d.{name}.co').write_bytes(raw)
    manifest.append(dict(region=name,url=url,sha256=hashlib.sha256(zipped.read_bytes()).hexdigest(),uncompressed_sha256=hashlib.sha256(raw).hexdigest()))
(out/'coordinate_sources.json').write_text(json.dumps(manifest,indent=2))
print('Official NY, BAY, COL coordinate files downloaded and hashed.')
