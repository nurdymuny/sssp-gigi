# Bucket Width, Duplicate Work, and Local Lookahead in Sequential Shortest Paths

Bee Rosa Davis · September 2026

An experimental study of three implementation decisions in sequential
single-source shortest paths, with the complete measurement artifact: raw
per-round timings, deterministic operation counters, the solver sources that
produced them, and the checks that constrain what they mean.

**[Read the paper](paper/davis_manifold_sssp_v3.pdf)** ·
**[Interactive companion](https://davisgeometric.com/sssp)** ·
**[Benchmark report](paper/benchmark_v3.html)**

## The questions

1. Does setting the Δ-stepping bucket width to the mean stored arc weight give a
   useful low-cost default, with and without duplicate suppression?
2. Does degree-gated one-hop lookahead improve a heap solver, or does it only
   reduce the overhead of unrestricted lookahead?
3. Can a graph database supply the representation and features these decisions
   need without changing the graph being solved?

## What the measurements show

Speed ratio = baseline time / method time; values above 1 mean the method ran
faster. Width comparisons use the author's standard-library-heap Dijkstra as the
baseline. Lookahead comparisons use a different baseline: the same custom heap
with scouting disabled. Arms are measured in the same interleaved validation
rounds, and every solver is checked against reference distances before it is
timed.

| Input | Result |
|---|---|
| Sparse, *n* = 5,000 | 1.76× with append-only lists, 1.76× with duplicate suppression |
| Sparse, *n* = 10,000 | 1.97× (append-only), 1.89× (cyclic container) |
| Sparse, *n* = 10⁶ | 1.81–1.82× |
| DIMACS roads (NY, BAY, COL) | 1.51–1.64× with a cyclic bucket array |
| Dense (p = 0.1), *n* = 5,000 | 0.56× with suppression, 0.06× without |
| Degree-gated lookahead | more heap insertions than no lookahead in all 24 live core queries |

Three qualifications travel with those numbers, and the paper states them at the
same volume:

- **Dense graphs favour the heap.** Without suppression the mean rule takes about
  16× the baseline's duration at *n* = 5,000; with it, about 1.8×. Suppression
  removes most of the penalty, and the heap is still faster.
- **The gain has a degree envelope.** At expected out-degree 2 the mean rule is
  0.85×; it peaks around degree 4–32 and is back to 1.01× by degree 64.
- **A competing cheap rule does as well on roads.** The Boost-documented
  `w_max/d_max` width reaches 1.55–1.74× on the same road inputs. The
  contribution is the controlled comparison and the measured operating range,
  not a claim that one width statistic dominates.

The lookahead result is negative and reported as such. The degree gate is a real
selector — negating its predicate is worse, and it beats random masks admitting
exactly the same number of arcs. But on the 24 live core queries, degree-gated
lookahead increased heap insertions in every case, and none of its query-median
solve times improved on the identical never-scout baseline.

## Layout

| Path | Contents |
|---|---|
| `paper/` | The manuscript, its LaTeX source, and the generated tables and figures. Reported experimental results are derived from the archived measurements through the analysis and rendering scripts. |
| `harness/` | The measurement study: solver sources, the campaign driver, the analysis scripts, and `results/` with per-round samples, operation counters, and run manifests. |
| `site/` | The interactive companion: a static page with recorded-timing replay and live teaching solvers. `site/dist` is what is deployed. |

Start with [`harness/REVISION_PROTOCOL.md`](harness/REVISION_PROTOCOL.md) for the
measurement contract and [`harness/results/revision/SCHEMA.md`](harness/results/revision/SCHEMA.md)
before interpreting any CSV — it records which columns are comparable across arms
and which are not.

## Reproducing

These are two different tasks with different entry points.

**Regenerate the published tables, figures, and manuscript** from the archived
measurements. This runs no solvers: it reads `harness/results/revision` and
writes into `paper/figs_v3` and `paper/benchmark_v3.html`.

```bash
pip install numpy pandas matplotlib pillow
python harness/analyze_revision.py
python harness/render_results.py
cd paper && pdflatex -interaction=nonstopmode davis_manifold_sssp_v3.tex
```

**Run a new experiment.** The solvers and the campaign build without the
database dependency:

```bash
cargo test  --manifest-path harness/portable/Cargo.toml --all-targets
cargo build --manifest-path harness/portable/Cargo.toml --release --bin campaign
harness/portable/target/release/campaign core results/my_run/core
```

Panels are `core`, `density`, `laws`, `scale`, `roads`, and `layout`; each needs a
fresh output directory, and `roads` takes the data directory as a third argument.
Run `python harness/download_roads.py` to fetch the official DIMACS inputs and
record their digests; the compressed inputs used here are in `harness/data`.

The analysis scripts read the archived `results/revision` tree by default, so a
new campaign is not analysed simply by running them — point their input root at
the new output directory, keeping the schema described in `SCHEMA.md`.

Run timed panels sequentially, with no concurrent compilation, rendering, or file
synchronisation. One case in the recorded layout panel was disturbed by
background synchronisation and is excluded from the manuscript summary; its raw
rows are retained.

The mechanism claim has a removal test that does not depend on timing:

```bash
python harness/mutation_check.py --output-dir results/mutation_check
```

The control must pass and removing either duplicate-suppression mechanism must
fail. Mutations are applied to an isolated copy, never to the real sources.

## What is not claimed

No asymptotic bound, no multicore or GPU result, no externally maintained solver
was timed, and no priority claim for a weight statistic. Evidence is
single-machine and sequential, with three seeds per synthetic setting; repeated
timings measure runtime variation conditional on a graph, not additional graph
instances. The interactive page replays recorded durations — it is not a live
benchmark, and its teaching solvers are instrumented for explanation rather than
for timing.

## The database side

GIGI (Geometric Intrinsic Global Index) is the author's Rust database engine
([source and documentation](https://github.com/nurdymuny/gigi), PolyForm
Noncommercial License 1.0.0). It organises keyed records into *bundles*,
separating identifying and indexed base fields from the fibre values associated
with them, and provides statistical and geometric queries over those records.

Here an arbitrary weighted digraph stores as one record per arc with base keys
`edge_id`, `vertex_a`, `vertex_b` and a numeric `weight` fibre, with a separate
bundle retaining isolated vertices. The study verifies every arc identity,
endpoint, weight, the vertex registry, and all-source distances after snapshot
and memory-mapped reload. The timed solvers run on extracted adjacency lists.

The engine's normalised weight-variance statistic is assessed as an ordinary
feature, with its elementary properties stated and with matched permutation
nulls; it is not treated as a graph metric, and it did not produce a useful gate.
Appendix B of the paper records which database interfaces apply to a vertex
digraph and what each would require.
