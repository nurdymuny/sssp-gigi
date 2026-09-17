# Raw-data conventions

The primary panel directories are core, density, laws, scale, roads, and layout.
Case numbers restart per panel. Join using (panel,case), not case alone.
Only a completed.txt marker confirms a complete panel. started.txt/completed.txt
are local timestamps. environment.json identifies the measured release executable.

samples.csv records nanoseconds, an arm's actual width, preparation nanoseconds,
round and within-round position. Rows marked calibration belong to source-zero
training, not validation. Selected-grid preparation includes calibration once;
do not sum it across every query when amortizing one stored graph. Preparation
estimates are separated from solve durations. The sd preparation path currently
uses the generic feature routine, including its median sort. The all-feature cost
in cases.csv is one observed duration, not a repeated microbenchmark.

counts.csv is generated outside timed runs. Heap counters apply to binary,
fourary, caliber, and scout arms. Bucket counters apply to explicit delta arms.
Uninstrumented author std_binary, original_mean, and radix rows contain zero
counter fields meaning unavailable, not measured absence. Likewise peak occupancy
is a heap-only measurement; comparisons is a heap comparison count. Do not use
zero fields from inapplicable algorithms in a cross-arm counter comparison.
For buckets, scans counts adjacency inspections; heavy counts evaluated heavy
relaxations, which are a different operation. max_phase_scans is a maximum over
the phase's executed vertex scans. unchanged counts encounters of unchanged
labels, including entries rejected by the suppression arm.

admitted counts mask bits across all graph arcs. potential_work sums the head
out-degrees over those mask bits. Neither is executed scouting work. scout_scans
is executed scout-arc work, and scout_yield counts successful scout relaxations.
budget is an upper bound, not a target that must be exhausted.

The primary timings were completed before engine-dependent controls. GIGI K null
replicates use the same Welford statistic and neighborhood extraction as the
observed fields; each replicate includes an engine cross-check. potential_order
is a separate timing session; never divide its times by primary-panel times.

Derived summaries preserve raw CSVs. Summary medians and geometric paired ratios
are different aggregations and need not equal the quotient of displayed medians.
Three-seed bootstrap intervals are descriptive. Repeated sources, weight-law
transformations, and layouts do not create independent graph samples.

The minimizing training multiplier and selected_grid rows are session-specific
by construction: a fresh six-round calibration can select another candidate.
The independent laws rerun selects different multipliers on 21/48 configurations;
this can change both timing rows and operation counts for selected_grid.

All-case derived tables preserve the original primary population. The manuscript
layout summary additionally excludes both sources of layout case 29, identified
in the verification report as affected by synchronization interference.
layout_retained_query_medians.csv records the exact included population.
Raw sample counts refer to data collected, including excluded layout rows.
