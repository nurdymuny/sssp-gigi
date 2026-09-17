# Measurement protocol, 2026-09-16

This file was saved before the revision campaign. This is an engineering plan,
not a claim of preregistration. Raw CSV files are the evidence for the manuscript.

The original harness is archived under archive/pre_revision_20260916. The author
implementation remains in src/lib.rs and ../sssp_rust/src/main.rs. Revised methods
live in src/revision.rs; campaign.rs is the measurement entry point.

## Numerical and measurement contract

Directed graphs, finite nonnegative binary64 weights, strict label improvements.
Parallel arcs and isolated vertices are retained. Small solver fixtures are checked
against independent Bellman-Ford; every measured arm is checked, with and without
counters, against the author Dijkstra before its timed samples. Equality tolerance
is 1e-10 relative (with absolute floor 1e-10); unreachable status must match exactly.
Integer radix runs only on genuine integer-weight inputs with path bounds below
2^53. No rounding of the floating input to fit an integer baseline.

Solve times include distance/queue allocation and destruction of internal queues,
but exclude destruction of the returned distance vector, graph generation/loading,
and width preparation. The author original_mean computes its mean inside the solve.
Width-specific preparation and total feature extraction are recorded separately.
Returned distances pass through std::hint::black_box. Counter builds are separate
const-generic instantiations; timing runs have counters compiled out.

Each validation round measures every arm. Consecutive rounds use a seeded random
permutation and its reverse, balancing position exactly. Checks provide warmups.
No concurrent campaign processes; do not compile, download, or render while a timed
campaign is active. No claim of an otherwise idle operating system, fixed frequency,
or hardware-counter isolation. Save environment, timestamps, commands and hashes.

Training: source 0, 12 multipliers of mean including 1, six calibration rounds each.
Choose a width by training median. Validate selected width and named rules on other
sources with the same repetition count and interleaving. This is a selected grid
width, not an oracle or a continuous optimum. Calibration costs are retained.

## Planned panels

Core: seeds 42,1729,2026; original sparse, clustered, dense, exact-size rectangular
grid, and six-neighbor road-like generators. Sizes 1000,2000,5000,10000 except dense
and road-like stop at 5000. Two held-out sources per graph, 12 timed rounds per arm.
The clustered generator links every pair of clusters; its degree grows with n.

Density: expected directed degree 2,4,8,16,32,64,128,256; n=1000,5000; same three
seeds, two sources and eight rounds. Laws: same topology, uniform, near constant,
skewed, bimodal, zero-containing, integer, permuted, and positive-rescaled weights;
sparse/clustered n=2000, three seeds, two sources, eight rounds.

Scale: rejection-sampled sparse generator, same degree/weight distribution but
different random stream from the original shuffle-based generator. Sizes 10000,
100000,1000000; three seeds, two sources, six rounds. Label it sparse_fast.

Real graphs: DIMACS Challenge 9 distance NY,BAY,COL, unmodified directed arcs;
six held-out sources, eight rounds; integer radix included. Download provenance
and checksums recorded. No assertion that the road data are error-free maps.

Layout: weight, destination, random adjacency order, BFS/RCM relabeling on sparse
graphs, n=5000,100000, three seeds, two corresponding sources, eight rounds.

Gate controls: NEVER, ALWAYS, degree4, high/high (AND), logical complement (OR),
tail-only, head-only, adaptive, five random masks with exact graph-edge cardinality,
and the same masks capped at degree4's executed scout-arc work. Count all heap pushes,
pops, comparisons, peak queue, main/scout scans, scout improvements, stale pops,
bucket phases, unchanged-label scans, and heavy relaxations. A cap may underspend
because a scout is indivisible. Graph admission is not executed work matching.

Statistics: retain per-round paired differences and ratios, per-query medians/IQRs,
and intervals clustered by independent graph seed. Do not treat sources or timed
repetitions as independent graph samples. Three graph seeds support descriptive
uncertainty, not broad population guarantees. Report signs without turning failure
to reject into equivalence. Timing claims need simultaneous/balanced comparisons;
no ratios across sessions.

Policy analysis, if performed: train on seed42, validate on1729/2026, choose among
fixed measured solvers using only recorded graph features; compare against the best
fixed training solver and held-out per-case best. Charge features/tuning for Q=1,
10,100 explicitly. Repeated-source projections are not measured multiquery services.

GIGI: preserve edge identity with an edge_id base key and use a vertex registry for
isolates. Verify every edge and all planted distances after snapshot/mmap reload.
K means population variance/range^2. A matched pipeline null permutes edge weights,
then recomputes the identical two-hop statistic; vertex smoothing is not that null.
Rank edge scores with deterministic tie breaking, recording actual admission.
Circulation can order vertices, never replace tentative distance in bucket assignment.

## Reproduction

    cargo test --no-default-features --all-targets
    cargo build --release --no-default-features --bin campaign
    target/release/campaign core results/revision/core
    target/release/campaign density results/revision/density
    target/release/campaign laws results/revision/laws
    target/release/campaign scale results/revision/scale
    target/release/campaign roads results/revision/roads data
    target/release/campaign layout results/revision/layout

All panels are separate invocations. Never silently substitute a subset for a
completed panel: the run manifest and manuscript must state actual completed counts.
