//! Is the delta rule a contribution, or is the win just Delta-stepping?
//!
//! The paper's fastest variant sets delta = mean edge weight. Delta-stepping
//! itself (Meyer & Sanders 2003) is known to beat a binary-heap Dijkstra on
//! sparse graphs, so without a plain Delta-stepping baseline the measured 2x
//! has nowhere to be attributed. A referee will ask this first.
//!
//! Four delta rules, one implementation, so the only thing that varies is the
//! bucket width:
//!
//!   MEAN     delta = mean edge weight                    (the paper's rule)
//!   MEDIAN   delta = median edge weight
//!   MS       delta = max_weight / max_degree             (classic heuristic)
//!   ORACLE   best delta found by sweeping a grid         (tuned, not practical)
//!
//! ORACLE is the one that settles it. A parameter-free rule that lands within
//! a few percent of a tuned delta is a real result: it removes the tuning step
//! that makes Delta-stepping awkward in practice. A rule that trails the
//! oracle badly is just one arbitrary choice among many.
//!
//! The bucket loop below is the paper's own `davis_delta` with delta lifted
//! out as a parameter and nothing else changed.

use std::collections::BTreeMap;
use std::time::Instant;

use sssp_rust::{
    clustered_graph, dijkstra, grid_graph, random_dense_graph, random_sparse_graph,
    road_network_like, verify_distances, Graph,
};

/// The paper's bucket algorithm, delta supplied by the caller.
fn delta_stepping(graph: &Graph, source: usize, delta: f64) -> Vec<f64> {
    let n = graph.n;
    if n == 0 {
        return Vec::new();
    }
    let delta = if delta > 0.0 { delta } else { 1.0 };
    let mut dist = vec![f64::INFINITY; n];
    let adj = &graph.adj;
    dist[source] = 0.0;

    let mut buckets: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    buckets.entry(0).or_default().push(source);
    let mut current = 0usize;

    while !buckets.is_empty() {
        let next = match buckets.range(current..).next() {
            Some((&i, _)) => i,
            None => break,
        };
        current = next;
        let mut heavy = Vec::new();

        while let Some(nodes) = buckets.remove(&current) {
            if nodes.is_empty() {
                break;
            }
            for u in nodes {
                let d = dist[u];
                if (d / delta) as usize != current {
                    continue;
                }
                for &(v, w) in &adj[u] {
                    if w <= delta {
                        let nd = d + w;
                        if nd < dist[v] {
                            dist[v] = nd;
                            buckets.entry((nd / delta) as usize).or_default().push(v);
                        }
                    } else {
                        heavy.push((u, v, w));
                    }
                }
            }
        }
        for (u, v, w) in heavy {
            let nd = dist[u] + w;
            if nd < dist[v] {
                dist[v] = nd;
                buckets.entry((nd / delta) as usize).or_default().push(v);
            }
        }
        current += 1;
    }
    dist
}

fn weights(g: &Graph) -> Vec<f64> {
    let mut w = Vec::new();
    for u in 0..g.n {
        for &(_, x) in &g.adj[u] {
            w.push(x);
        }
    }
    w
}

fn mean_delta(g: &Graph) -> f64 {
    let w = weights(g);
    if w.is_empty() { 1.0 } else { w.iter().sum::<f64>() / w.len() as f64 }
}
fn median_delta(g: &Graph) -> f64 {
    let mut w = weights(g);
    if w.is_empty() {
        return 1.0;
    }
    w.sort_by(|a, b| a.partial_cmp(b).unwrap());
    w[w.len() / 2]
}
/// Density-corrected: the mean rule scaled down by the branching factor.
/// A wide bucket on a high-degree graph puts many vertices in one phase and
/// each gets re-improved repeatedly, which is where the mean rule collapses.
fn deg_delta(g: &Graph) -> f64 {
    let n = g.n.max(1);
    let avg = g.degree.iter().sum::<usize>() as f64 / n as f64;
    mean_delta(g) / avg.max(1.0)
}
/// Half-corrected: divide by the square root of the branching factor.
fn sqrt_delta(g: &Graph) -> f64 {
    let n = g.n.max(1);
    let avg = g.degree.iter().sum::<usize>() as f64 / n as f64;
    mean_delta(g) / avg.max(1.0).sqrt()
}

fn ms_delta(g: &Graph) -> f64 {
    let w = weights(g);
    let maxw = w.iter().cloned().fold(0.0f64, f64::max);
    let maxd = g.degree.iter().cloned().max().unwrap_or(1).max(1);
    if maxw > 0.0 { maxw / maxd as f64 } else { 1.0 }
}

fn med(v: &mut Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}
fn iqr(v: &mut Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |p: f64| v[((p * (v.len() - 1) as f64).round() as usize).min(v.len() - 1)];
    q(0.75) - q(0.25)
}

/// Interleaved timing of delta-stepping at a given delta against Dijkstra.
fn timed(g: &Graph, delta: f64, runs: usize) -> (f64, f64, f64) {
    let mut td = Vec::new();
    let mut ts = Vec::new();
    dijkstra(g, 0);
    delta_stepping(g, 0, delta);
    for _ in 0..runs {
        let t = Instant::now();
        dijkstra(g, 0);
        td.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        delta_stepping(g, 0, delta);
        ts.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    (med(&mut td.clone()), med(&mut ts.clone()), iqr(&mut td))
}

fn main() {
    let runs = 21;
    println!("DELTA RULE COMPARISON");
    println!("one bucket implementation, only the bucket width differs\n");
    println!(
        "{:<17} {:>8} {:>8} {:>8} {:>8} {:>8} {:>7} {:>8}",
        "graph", "dij ms", "MEAN", "m/deg", "m/sqrt", "ORACLE", "orac k", "best"
    );
    println!("{}", "-".repeat(80));

    let mut within5 = 0usize;
    let mut total = 0usize;

    for &n in &[1000usize, 2000, 5000] {
        let side = (n as f64).sqrt() as usize;
        let cases: Vec<(&str, Graph)> = vec![
            ("sparse", random_sparse_graph(n, 4, 42)),
            ("dense", random_dense_graph(n, 0.1, 42)),
            ("grid", grid_graph(side, side, 42)),
            ("clustered", clustered_graph(n / 50, 50, 0.5, 2, 42)),
            ("road_like", road_network_like(n, 42)),
        ];
        for (fam, g) in &cases {
            let reference = dijkstra(g, 0);
            let dm = mean_delta(g);

            // correctness once per rule
            for (nm, d) in [("mean", dm), ("median", median_delta(g)), ("ms", ms_delta(g))] {
                assert!(
                    verify_distances(&delta_stepping(g, 0, d), &reference),
                    "{fam}/{n} delta rule {nm} produced wrong distances"
                );
            }

            let (tdij, t_mean, _) = timed(g, dm, runs);
            let (_, t_deg, _) = timed(g, deg_delta(g), runs);
            let (_, t_sqrt, _) = timed(g, sqrt_delta(g), runs);

            // ORACLE: sweep multiples of the mean weight
            let mut best = f64::INFINITY;
            let mut best_d = dm;
            for k in [0.05, 0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 4.0, 8.0] {
                let d = dm * k;
                let (_, t, _) = timed(g, d, 7);
                if t < best {
                    best = t;
                    best_d = d;
                }
            }
            let orac_k = best_d / dm;

            let ratio = t_mean / best;
            total += 1;
            if ratio <= 1.05 {
                within5 += 1;
            }

            let mut names = vec![("MEAN", t_mean), ("m/deg", t_deg), ("m/sqrt", t_sqrt)];
            names.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            println!(
                "{:<17} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>7.2} {:>8}",
                format!("{fam}/{n}"),
                tdij, t_mean, t_deg, t_sqrt, best, orac_k, names[0].0
            );
        }
        println!();
    }

    println!("{}", "=".repeat(72));
    println!(
        "mean-weight rule within 5% of the tuned oracle : {within5} of {total} configurations"
    );
    println!();
    println!("If that count is high, the contribution is a PARAMETER-FREE delta rule:");
    println!("it removes the tuning step, which is the practical objection to");
    println!("Delta-stepping. If it is low, the mean rule is one arbitrary choice.");
}
