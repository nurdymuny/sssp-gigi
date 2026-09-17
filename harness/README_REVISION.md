# Sequential shortest-path study

The paper is ../davis_manifold_sssp_v3.pdf; its editable LaTeX source is adjacent.
../benchmark_v3.html is the generated local benchmark report. The remote Claude
artifact was not modified by this workflow.

## Algorithms without GIGI

From this directory:

    cargo test --manifest-path portable/Cargo.toml --all-targets
    cargo build --manifest-path portable/Cargo.toml --release --bin campaign
    portable/target/release/campaign core results/my_run/core

On Windows the executable has an .exe suffix. The portable manifest uses the
same source and release settings without requiring a local GIGI checkout. Cargo
must resolve the main manifest's optional database path even with that feature off.

Panels are core, density, laws, scale, roads, and layout. Each needs a fresh output
directory. For roads, pass the data directory as the third argument. Run
download_roads.py to fetch the official inputs and record digests. Read
REVISION_PROTOCOL.md and results/revision/SCHEMA.md before interpreting the CSVs.

## GIGI controls

Update the main manifest's gigi path to a checkout of the measured engine.
Engine provenance is recorded in results/revision/final_manifest.json.

    cargo test --all-targets
    cargo run --release --bin substrate -- results/new_substrate_store
    cargo run --release --bin gigi_gate -- results/new_gigi_gate.csv
    cargo run --release --bin potential_order -- results/new_potential_order

substrate and potential_order require directories that do not exist yet. The
substrate checks every edge identity, endpoint, weight, vertex, and all-source
distance after snapshot/mmap reload. Potential ordering includes BFS/RCM controls
and retains the engine's complete refusal messages.

## Analysis and manuscript

Install numpy, pandas, matplotlib, and pillow in a Python environment. The local
.venv_analysis is excluded from the portable artifact. Exact packages are recorded
in results/revision/python_requirements.txt.

    python analyze_revision.py
    python render_results.py
    cd ..
    pdflatex -interaction=nonstopmode -halt-on-error davis_manifold_sssp_v3.tex
    pdflatex -interaction=nonstopmode -halt-on-error davis_manifold_sssp_v3.tex

Analysis intentionally reads the completed results/revision tree. Adapt its input
root for another reproduction while preserving the schema. Missing completed.txt
means an incomplete panel. Run timed panels sequentially without concurrent builds
or rendering. The paper's Q=1,10,100 figures are amortization projections.

## Preservation and tests

The author implementation and older papers remain intact. Earlier instruments
are in archive/pre_revision_20260916. Exact measured primary sources are frozen in
results/revision/primary_source and match environment.json hashes. Subsequent
solver-file changes add tests and formatting, without changing measured methods.

Run python mutation_check.py --output-dir results/my_mutation_run to check the mechanism-removal gate. The output directory must not exist. The control must
pass; removing either duplicate mechanism must fail. Isolated mutations never edit
the real source. Complete all-target test and mutation logs are saved with results.

Distribution-gap-derived nonuniform distance buckets, a weighted lattice SSSP
consumer, and coordinate-based A* remain algorithm-design questions. The paper
does not claim implementations or speedups for them.

## Reporting decisions and provenance

The disclosure note about a different paper is outside this manuscript and its
fix report. Do not add it during subsequent revisions.

The layout summary excludes both sources of layout case 29: BFS, n=100000,
seed 2026. The verification report identified synchronization interference during
that case and attributed it to OneDrive. This revision uses that reported reason
for exclusion; it does not independently establish the process-level cause.
All raw rows and original all-case summaries remain intact. The retained query
medians are in results/revision/layout_retained_query_medians.csv. BFS at this
size has two seeds and four queries; other layouts have three seeds and six.
The 192 configurations and 43,956 validation samples describe data collected,
including the excluded case. Pause sync or use a nonsynced working directory
for future timed runs; run no builds or rendering during a panel.

run_campaign.ps1 is a documented reconstruction of the sequential launch
protocol for future runs, not a recovered copy of the historical driver. Build
the portable campaign from the current source, then invoke it with a fresh
OutputRoot. ValidateOnly checks paths without launching or creating files.
Each completed panel receives started.txt, completed.txt, and argument/log files.
The original environment.json is preserved; final_manifest.json distinguishes
current source hashes from measured_source_sha256.

freeze_primary_source.py now checks both archived hashes and compares production
code after rustfmt. It fails if a measured snapshot is missing instead of trying
to recreate historical bytes from current tests.

The saved tests_all_targets.txt records the harness command cargo test
--all-targets with CARGO_TARGET_DIR=C:/Users/nurdm/OneDrive/Documents/gigi/target.
That shared target directory is visible in its test executable paths. It is a
harness test run, not a claim about the entire GIGI repository test suite.
The portable log records cargo test --manifest-path portable/Cargo.toml
--all-targets. Neither log establishes live HTTP behavior.

results/revision/smoke is a pre-panel functional smoke run, outside the six
primary timing panels and their sample totals. It lacks original launch and
completion metadata; no stronger historical provenance is inferred. The later
independent smoke comparison and laws rerun are under results/verify_repro.
Their detailed output records individual query sign changes as well as stable
aggregated trends. Selected-width choices differ on 21/48 laws configurations.

The 24 MiB cache figure is from Windows Win32_Processor.L3CacheSize=24576 KiB,
queried on September 16, 2026 and recorded in results/revision/machine_cache.json.
