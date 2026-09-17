"""Verify measured bytes and rustfmt-normalized production code; never reconstruct history."""
from pathlib import Path
import hashlib, json, re, subprocess
root=Path(__file__).resolve().parent
env=json.loads((root/'results/revision/environment.json').read_text(encoding='utf-8-sig'))
frozen=root/'results/revision/primary_source'
def formatted_production(path):
    source=path.read_text(encoding='utf-8')
    production=re.split(r'#\[cfg\(test\)\]',source,maxsplit=1)[0]
    result=subprocess.run(['rustfmt','--edition','2021','--emit','stdout'],input=production,text=True,capture_output=True,check=True)
    return result.stdout.strip()
for relative,field in [('src/revision.rs','source'),('src/bin/campaign.rs','harness')]:
    snapshot=frozen/relative
    assert snapshot.exists(),f'Measured snapshot missing: {snapshot}'
    assert hashlib.sha256(snapshot.read_bytes()).hexdigest().lower()==env[field].lower(),relative
    assert formatted_production(snapshot)==formatted_production(root/relative),f'Production source differs after rustfmt: {relative}'
    print(f'Hash verified; production code agrees after rustfmt: {relative}')
