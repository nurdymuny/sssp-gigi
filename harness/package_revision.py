"""Validate and package the manuscript and reproducible evidence, excluding caches."""
from pathlib import Path
import hashlib,json,re,subprocess,zipfile
from datetime import datetime,timezone
from pypdf import PdfReader
R=Path(__file__).resolve().parent; BASE=R.parent; O=R/'results/revision'
pdf=BASE/'davis_manifold_sssp_v3.pdf'; reader=PdfReader(pdf)
text='\n'.join(p.extract_text() for p in reader.pages)
assert not re.search(r'\b(first|novel|patented|retract\w*)\b',text,re.I)
assert not any(x in text for x in ['??','PLACEHOLDER','TODO'])
log=(BASE/'davis_manifold_sssp_v3.log').read_text(errors='replace')
assert not re.search(r'Overfull|Underfull|undefined|LaTeX Warning|Package .* Warning',log)
assert len(reader.pages)==15
for name in ['tests_all_targets.txt','tests_portable.txt']:
    t=(O/name).read_text(encoding='utf-8-sig')
    assert '6 passed; 0 failed' in t and 'test result: FAILED' not in t
mut=json.loads((O/'mutation_results.json').read_text());assert [x['exit_code']==0 for x in mut]==[True,False,False]
assert (O/'engine_controls_completed.txt').exists()
env=json.loads((O/'environment.json').read_text(encoding='utf-8-sig'))
assert hashlib.sha256((O/'primary_source/src/revision.rs').read_bytes()).hexdigest().lower()==env['source'].lower()
assert hashlib.sha256((O/'primary_source/src/bin/campaign.rs').read_bytes()).hexdigest().lower()==env['harness'].lower()
engine=Path('C:/Users/nurdm/OneDrive/Documents/gigi')
head=subprocess.check_output(['git','-C',str(engine),'rev-parse','HEAD'],text=True).strip()
tracked=subprocess.check_output(['git','-C',str(engine),'status','--short'],text=True)
manifest=dict(completed_utc=datetime.now(timezone.utc).isoformat(),pages=len(reader.pages),words=len(text.split()),
    primary=json.loads((O/'analysis_manifest.json').read_text()),gigi_commit=head,gigi_status=tracked,
    tests=dict(all_targets='passed, six solver tests',portable='passed, six solver tests',mutation='control passes; each removed mechanism fails',substrate='complete edge/vertex/all-source distance roundtrip passed'),
    engine_control_rows=dict(k_gate=sum(1 for _ in (O/'gigi_gate.csv').open())-1,potential_timings=sum(1 for _ in (O/'potential_order/samples.csv').open())-1),
    remote_artifact='not modified; benchmark_v3.html is a local generated replacement',
    source_note='Primary measured sources are frozen with matching hashes. Current solver changes after timing consist of additional tests and formatting.',
    files={})
manifest['primary']['measured_source_sha256']={'src/revision.rs':env['source'].lower(),'src/bin/campaign.rs':env['harness'].lower(),'REVISION_PROTOCOL.md':env['protocol'].lower()}
manifest['primary']['source_sha256_note']='Current working-source hashes; measured_source_sha256 identifies the archived source used for primary timings.'
manifest['layout_summary_exclusion']=dict(case=29,sources='both',reason='Review-reported background synchronization interference',raw_samples_preserved=True,retained_configurations=29)
manifest['driver_note']='run_campaign.ps1 is a documented reconstruction for future runs, not an archived original launch script.'
selected=[pdf,BASE/'davis_manifold_sssp_v3.tex',BASE/'benchmark_v3.html',BASE/'ADVISOR_NOTE.md',BASE/'V3_FIX_REPORT.md']
selected+=list((BASE/'figs_v3').glob('*'))
selected+=[R/n for n in ['Cargo.toml','Cargo.lock','REVISION_PROTOCOL.md','README_REVISION.md','run_campaign.ps1','download_roads.py','analyze_revision.py','render_results.py','mutation_check.py','freeze_primary_source.py','package_revision.py']]
selected+=list((R/'src').rglob('*.rs'))
selected+=list((R/'portable').glob('Cargo.*'))
selected+=list((R/'data').glob('*.gz'))+[R/'data/manifest.json']
for p in (R/'results/verify_repro').rglob('*'):
    if p.is_file() and 'target' not in p.relative_to(R/'results/verify_repro').parts and p.suffix in ['.csv','.json','.txt','.py']:
        selected.append(p)
for directory in ['revision_followup_checks','revision_driver_smoke']:
    for p in (R/'results'/directory).rglob('*'):
        if p.is_file() and 'mutation_crate' not in p.relative_to(R/'results'/directory).parts and p.suffix in ['.csv','.json','.txt']:
            selected.append(p)
for p in O.rglob('*'):
    if not p.is_file():continue
    parts=p.relative_to(O).parts
    if 'target' in parts or 'mutation_crate' in parts or 'substrate_store' in parts:continue
    if parts[0]=='potential_order' and len(parts)>2:continue
    if p.name=='final_manifest.json':continue
    if p.suffix in ['.csv','.json','.txt','.md','.rs','.toml','.lock'] or p.name=='Cargo.lock':selected.append(p)
selected=sorted(set(p for p in selected if p.is_file()))
for p in selected:manifest['files'][str(p.relative_to(BASE)).replace('\\','/')]=dict(bytes=p.stat().st_size,sha256=hashlib.sha256(p.read_bytes()).hexdigest())
(O/'final_manifest.json').write_text(json.dumps(manifest,indent=2),encoding='utf-8')
selected.append(O/'final_manifest.json')
target=BASE/'sssp_review_artifact_v3.zip'
with zipfile.ZipFile(target,'w',zipfile.ZIP_DEFLATED,compresslevel=6) as z:
    for p in selected:z.write(p,str(p.relative_to(BASE)))
with zipfile.ZipFile(target) as z:assert z.testzip() is None
print(json.dumps(dict(pdf_pages=len(reader.pages),files=len(selected),zip_bytes=target.stat().st_size,pdf_sha256=hashlib.sha256(pdf.read_bytes()).hexdigest(),validation_samples=manifest['primary']['validation_samples'],calibration_samples=manifest['primary']['calibration_samples']),indent=2))
