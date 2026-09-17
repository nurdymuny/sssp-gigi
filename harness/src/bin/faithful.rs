//! The curvature gate, tested FAITHFULLY.
//!
//! Three substitutions invalidated my last attempt, and this removes all of
//! them:
//!
//!   1. I paraphrased `davis_corridor` -- dropped its stale check, relaxed
//!      from `dist[u]` instead of the popped `d`, and quantised the priority
//!      key to u64, which reorders near-ties. Here her `davis_corridor` and
//!      `dijkstra` are called VERBATIM from the library built out of her own
//!      `main.rs`; the gated arm is a copy of her corridor with ONLY the
//!      predicate swapped.
//!   2. My own xorshift graphs stood in for hers. Here her five generators
//!      are called directly, seeded the same way.
//!   3. I measured priority-queue pushes. Her paper's claim is WALL TIME
//!      against her Dijkstra, so time is the headline column here and pushes
//!      ride alongside as the mechanism.
//!
//! And the rate-match is now at the EDGE level. Last time I matched the
//! fraction of VERTICES admitted while the gate fires on PAIRS, so the
//! curvature arm scouted 249 edges against degree's 145 and the whole gap was
//! firing more, not firing better.
//!
//! The curvature itself comes from GIGI: each vertex's incident edges are
//! stored as a real `BundleStore` and handed to
//! `gigi::curvature::scalar_curvature`.

use std::collections::BinaryHeap;
use std::time::Instant;

use gigi::bundle::BundleStore;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

use sssp_rust::{
    clustered_graph, davis_corridor, dijkstra, grid_graph, random_dense_graph,
    random_sparse_graph, road_network_like, verify_distances, Graph, State,
};

// ── GIGI: per-vertex curvature of the incident-edge sub-bundle ─────────────

fn edge_schema() -> BundleSchema {
    BundleSchema::new("nbhd")
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .fiber(FieldDef::numeric("weight"))
}

/// K(u) from GIGI, plus the sample size it was computed from.
///
/// `scalar_curvature` is `mean_f(variance/range^2)`. On two records that is
/// identically 0.25 (Popoviciu is tight for a two-point mass at the
/// extremes), so a degree-2 vertex reads maximally curved no matter what its
/// weights are. The sample size is returned so that degeneracy is visible
/// rather than silently gating.
fn gigi_k(graph: &Graph) -> (Vec<f64>, Vec<usize>) {
    let n = graph.n;
    let mut ks = vec![f64::NAN; n];
    let mut ns = vec![0usize; n];
    for u in 0..n {
        let inc = &graph.adj[u];
        ns[u] = inc.len();
        if inc.len() < 2 {
            continue;
        }
        let mut sub = BundleStore::new(edge_schema());
        for &(v, w) in inc {
            let mut r = Record::new();
            r.insert("vertex_a".into(), Value::Integer(u as i64));
            r.insert("vertex_b".into(), Value::Integer(v as i64));
            r.insert("weight".into(), Value::Float(w));
            sub.insert(&r);
        }
        ks[u] = gigi::curvature::scalar_curvature(&sub);
    }
    (ks, ns)
}

// ── her corridor, predicate swapped, nothing else ──────────────────────────

#[derive(Clone, Copy)]
enum Gate<'a> {
    Degree(usize),
    Curvature(&'a [f64], f64),
}

/// Byte-for-byte her `davis_corridor` except for the `fire` line. Kept next to
/// the original so any drift is visible in review.
fn corridor_gated(graph: &Graph, source: usize, gate: Gate) -> (Vec<f64>, u64) {
    let n = graph.n;
    if n == 0 {
        return (Vec::new(), 0);
    }
    let mut dist = vec![f64::INFINITY; n];
    let mut visited = vec![false; n];
    let adj = &graph.adj;
    let degree = &graph.degree;
    let mut fired = 0u64;

    dist[source] = 0.0;
    let mut pq = BinaryHeap::with_capacity(n);
    pq.push(State { dist: 0.0, node: source });

    while let Some(State { dist: d, node: u }) = pq.pop() {
        if d > dist[u] {
            continue;
        }
        if visited[u] {
            continue;
        }
        visited[u] = true;

        for &(v, w) in &adj[u] {
            let new_d = d + w;
            if new_d < dist[v] {
                dist[v] = new_d;

                let fire = match gate {
                    Gate::Degree(t) => degree[u] <= t && degree[v] <= t,
                    Gate::Curvature(k, t) => {
                        k[u].is_finite() && k[v].is_finite() && k[u] <= t && k[v] <= t
                    }
                };

                if fire && !visited[v] {
                    fired += 1;
                    for &(v2, w2) in &adj[v] {
                        let new_d2 = new_d + w2;
                        if new_d2 < dist[v2] {
                            dist[v2] = new_d2;
                            if !visited[v2] {
                                pq.push(State { dist: new_d2, node: v2 });
                            }
                        }
                    }
                }

                pq.push(State { dist: new_d, node: v });
            }
        }
    }
    (dist, fired)
}

/// How many EDGES her degree gate fires on, counted over the graph.
fn degree_edge_fires(g: &Graph, t: usize) -> usize {
    let mut c = 0;
    for u in 0..g.n {
        for &(v, _) in &g.adj[u] {
            if g.degree[u] <= t && g.degree[v] <= t {
                c += 1;
            }
        }
    }
    c
}

/// Curvature threshold admitting the same number of EDGES as the degree gate.
fn edge_rate_matched(g: &Graph, k: &[f64], target_edges: usize) -> f64 {
    let mut pair_max: Vec<f64> = Vec::new();
    for u in 0..g.n {
        for &(v, _) in &g.adj[u] {
            if k[u].is_finite() && k[v].is_finite() {
                pair_max.push(k[u].max(k[v]));
            }
        }
    }
    if pair_max.is_empty() {
        return f64::NEG_INFINITY;
    }
    pair_max.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let idx = target_edges.min(pair_max.len().saturating_sub(1));
    pair_max[idx]
}

fn median_ms<F: FnMut()>(mut f: F, runs: usize) -> f64 {
    let mut v = Vec::new();
    for _ in 0..runs {
        let t = Instant::now();
        f();
        v.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() {
    const T: usize = 4;
    let sizes = [1000usize, 2000, 5000];
    let mut deg_wins = 0;
    let mut k_wins = 0;

    println!("FAITHFUL CURVATURE GATE TEST");
    println!("her generators, her dijkstra, her corridor (predicate swapped only)");
    println!("edge-level rate match; time is the headline, as in the paper\n");
    println!(
        "{:<18} {:>8} {:>8} {:>8} {:>7} {:>7} {:>9}",
        "graph", "dij ms", "corr ms", "K ms", "fireD", "fireK", "deg2 frac"
    );
    println!("{}", "-".repeat(72));

    for &n in &sizes {
        let side = (n as f64).sqrt() as usize;
        let cases: Vec<(&str, Graph)> = vec![
            ("sparse", random_sparse_graph(n, 4, 42)),
            ("dense", random_dense_graph(n, 0.1, 42)),
            ("grid", grid_graph(side, side, 42)),
            ("clustered", clustered_graph(n / 100, 100, 0.5, n / 50, 42)),
            ("road_like", road_network_like(n, 42)),
        ];
        for (fam, g) in &cases {
            let reference = dijkstra(g, 0);
            let (k, ns) = gigi_k(g);
            let target = degree_edge_fires(g, T);
            let kt = edge_rate_matched(g, &k, target);

            let (d1, f1) = corridor_gated(g, 0, Gate::Degree(T));
            let (d2, f2) = corridor_gated(g, 0, Gate::Curvature(&k, kt));
            assert!(verify_distances(&d1, &reference), "{fam}/{n} degree arm");
            assert!(verify_distances(&d2, &reference), "{fam}/{n} curvature arm");

            let t_dij = median_ms(|| { dijkstra(g, 0); }, 5);
            let t_cor = median_ms(|| { davis_corridor(g, 0); }, 5);
            let t_k = median_ms(|| { corridor_gated(g, 0, Gate::Curvature(&k, kt)); }, 5);

            // how much of the graph sits in the degenerate 2-sample regime
            let deg2 = ns.iter().filter(|&&s| s == 2).count() as f64 / g.n as f64;

            println!(
                "{:<18} {:>8.2} {:>8.2} {:>8.2} {:>7} {:>7} {:>9.2}",
                format!("{fam}/{n}"),
                t_dij,
                t_cor,
                t_k,
                f1,
                f2,
                deg2
            );
            if t_k < t_cor { k_wins += 1 } else { deg_wins += 1 }
        }
        println!();
    }

    println!("{}", "=".repeat(72));
    println!("GIGI curvature gate faster than her degree gate : {k_wins}");
    println!("her degree gate faster                          : {deg_wins}");
    println!();
    println!("`deg2 frac` is the share of vertices with exactly two incident edges,");
    println!("where var/range^2 is identically 0.25 regardless of the weights. Where");
    println!("that number is large the curvature gate is not measuring geometry.");
}
