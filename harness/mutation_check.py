"""Remove each duplicate mechanism in an isolated crate; require its gate to fail."""
from pathlib import Path
import shutil, subprocess, json, re, argparse
root=Path(__file__).resolve().parent
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output-dir',type=Path,required=True,help='Fresh directory for logs and isolated crate')
out=parser.parse_args().output_dir.resolve()
if out.exists(): raise SystemExit(f'Output directory already exists: {out}')
out.mkdir(parents=True)
dest=out/'mutation_crate'
(dest/'src').mkdir(parents=True,exist_ok=True)
(dest/'Cargo.toml').write_text('[package]\nname="sssp_mutation_gate"\nversion="0.1.0"\nedition="2021"\n[dependencies]\nrand="=0.8.5"\nrand_chacha="=0.3.1"\n')
for file in ['lib.rs','ablation.rs','curvature_gate.rs']:
    shutil.copyfile(root/'src'/file,dest/'src'/file)
source=(root/'src/revision.rs').read_text(encoding='utf-8')
results=[]
for name,modified in [('control',source),('remove_skip',re.sub(r'if\s+skip\s*\{\s*continue;?\s*\}', 'if false { continue; }',source)),('remove_heavy_dedup',re.sub(r'if\s+dedup\s*\{\s*for u in touched','if false { for u in touched',re.sub(r'if\s+!dedup\s*\{\s*heavy.push','if true { heavy.push',source)))]:
    assert name=='control' or modified!=source, 'mutation failed to match code'
    (dest/'src/revision.rs').write_text(modified,encoding='utf-8')
    proc=subprocess.run(['cargo','test','--manifest-path',str(dest/'Cargo.toml'),'--lib','duplicate_mechanisms','--','--nocapture'],capture_output=True,text=True)
    (out/f'mutation_{name}.txt').write_text(proc.stdout+'\n'+proc.stderr,encoding='utf-8')
    results.append(dict(mutation=name,exit_code=proc.returncode))
    assert (proc.returncode==0)==(name=='control'),f'gate failed to distinguish {name}'
(dest/'src/revision.rs').write_text(source,encoding='utf-8')
(out/'mutation_results.json').write_text(json.dumps(results,indent=2))
print(results)
