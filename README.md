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
   useful low-cost default, once redundant bucket work has been removed?
2. Does degree-gated one-hop lookahead improve a heap solver, or does it only
   reduce the overhead of unrestricted lookahead?
3. Can a graph database supply the representation and features these decisions
   need without changing the graph being solved?

## What the measurements show

Speed ratios are against a standard-library binary-heap Dijkstra, measured in the
same interleaved validation rounds, with every solver checked against reference
distances before it is timed.

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

- **Dense graphs favour the heap.** Duplicate suppression turns a catastrophic
  slowdown into a moderate one; it does not turn it into a win.
- **The gain has a degree envelope.** At expected out-degree 2 the mean rule is
  0.85×; it peaks around degree 4–32 and is back to 1.01× by degree 64.
- **A competing cheap rule does as well on roads.** The Boost-documented
  `w_max/d_max` width reaches 1.55–1.74× on the same road inputs. The
  contribution is the controlled comparison and the measured operating range,
  not a claim that one width statistic dominates.

The lookahead result is negative and reported as such: the degree gate is a real
selector — negating its predicate is worse, and it beats random masks admitting
exactly the same number of arcs — but no scouting arm beat the identical
never-scout baseline on insertions or on the clock.

## Layout

| Path | Contents |
|---|---|
| `paper/` | The manuscript, its LaTeX source, and the generated tables and figures. Every number in the paper is emitted from the raw results by `harness/render_results.py`. |
| `harness/` | The measurement study: solver sources, the campaign driver, the analysis scripts, and `results/` with per-round samples, operation counters, and run manifests. |
| `site/` | The interactive companion: a static page with recorded-timing replay and live teaching solvers. `site/dist` is what is deployed. |

Start with [`harness/REVISION_PROTOCOL.md`](harness/REVISION_PROTOCOL.md) for the
measurement contract and [`harness/results/revision/SCHEMA.md`](harness/results/revision/SCHEMA.md)
before interpreting any CSV — it records which columns are comparable across arms
and which are not.

## Reproducing

The solvers and the campaign build without the database dependency:

```bash
cargo test  --manifest-path harness/portable/Cargo.toml --all-targets
cargo build --manifest-path harness/portable/Cargo.toml --release --bin campaign
harness/portable/target/release/campaign core results/my_run/core
```

Panels are `core`, `density`, `laws`, `scale`, `roads`, and `layout`; each needs a
fresh output directory, and `roads` takes the data directory as a third argument.
Run `python harness/download_roads.py` to fetch the official DIMACS inputs and
record their digests; the compressed inputs used here are in `harness/data`.
Then `python harness/analyze_revision.py` and `python harness/render_results.py`
regenerate every table in the paper.

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

An arbitrary weighted digraph stores as one record per arc with base keys
`edge_id`, `vertex_a`, `vertex_b` and a numeric `weight` fibre, with a separate
bundle retaining isolated vertices. The study verifies every arc identity,
endpoint, weight, the vertex registry, and all-source distances after snapshot
and memory-mapped reload. The engine's normalised weight-variance statistic is
assessed as an ordinary feature, with its elementary properties stated and with
matched permutation nulls; it is not treated as a graph metric, and it did not
produce a useful gate. Appendix B of the paper records which database interfaces
apply to a vertex digraph and what each would require.
