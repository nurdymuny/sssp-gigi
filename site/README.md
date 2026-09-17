# Sequential shortest paths: research companion

A static, interactive companion to Bee Rosa Davis's sequential SSSP study.
The research layout uses compact document typography, numbered sections, a
data-derived summary table, technical captions, and restrained color within
illustrations. It does not imply SSRN publication or affiliation.

The page has two explicitly distinct kinds of animation:

- Recorded timing rounds: 396 original validation rounds across seven selected
  configurations, including all their measured seeds and sources. Durations are
  slowed for replay; progress is elapsed time, not an internal execution trace.
- Teaching graphs: actual JavaScript Dijkstra and mean-width stepping on a small
  graph. Display events are synchronized for explanation, not for benchmarking.

export_data.py reads the adjacent study's saved CSVs. It exports original round
durations plus published aggregate ratios and copies the paper and evidence ZIP.
No performance measurements are generated. Data hashes are included in data.mjs.
The paper and source experiment remain unchanged.

Static entry point: dist/index.html. Serve dist with any static HTTP server.
No package installation or build step is needed. System serif, sans-serif, and
monospace fonts are used without external font requests. All interactive code
and study data are local assets. The adjacent shortest-paths-offline.html embeds
all presentation and interactive code for opening directly without a server.

Run node verify.mjs for independent Bellman-Ford checks of both teaching solvers
and validation of local assets, exported timing populations, and document IDs.
Use node --check dist/app.mjs and node --check dist/engine.mjs for syntax checks.

The site adapts to narrow screens, provides keyboard-native controls, respects
reduced-motion preferences, and pauses animations when the document is hidden.

The timing section also renders 42 source-specific network views, covering every
available input, seed, and source. Timing bars use distinct method colors;
algorithm diagrams use blue for finite labels, amber for active vertices, and
teal for successful relaxations. Sensitivity and work-count charts also use
restrained color, while document surfaces and prose remain neutral.
The complete 100 × 100 grid is drawn; roads use sampled actual arcs and official
DIMACS coordinates; sparse and dense inputs use bounded source-rooted samples
with schematic positions. All methods share geometry and distance ordering.
The color wave uses recomputed final distances, rescaled to each saved finish
time. It is not an internal execution trace or a work-completion estimate.

To reproduce geometry from the adjacent study:

1. Run `python download_coordinates.py` to fetch the official coordinate files.
2. Run `cargo run --release --manifest-path geometry_export/Cargo.toml -- ../sssp_ablation geometry_export/output`.
3. Run `python pack_geometry.py`.

The exporter checks each complete graph against its saved campaign fingerprint,
recomputes distances without timing, and validates every displayed arc against
the graph. The provenance download records 15 input fingerprints and coordinate
file SHA-256 hashes. Original benchmark measurements are unchanged.
