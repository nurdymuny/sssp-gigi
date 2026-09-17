from pathlib import Path
import urllib.request, gzip, hashlib, json
root = Path(__file__).resolve().parent / 'data'
root.mkdir(exist_ok=True)
manifest = []
for name in ['NY', 'BAY', 'COL']:
    filename = f'USA-road-d.{name}.gr.gz'
    url = f'https://www.diag.uniroma1.it/challenge9/data/USA-road-d/{filename}'
    target = root / filename
    if not target.exists():
        urllib.request.urlretrieve(url, target)
    packed = target.read_bytes()
    unpacked = gzip.decompress(packed)
    (root / filename[:-3]).write_bytes(unpacked)
    row = dict(name=name, url=url, compressed_bytes=len(packed), sha256_gz=hashlib.sha256(packed).hexdigest(), sha256_gr=hashlib.sha256(unpacked).hexdigest())
    manifest.append(row)
    print(row, flush=True)
(root/'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
