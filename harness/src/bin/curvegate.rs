//! Curvature gate experiment: does real discrete curvature beat the degree
//! proxy at deciding where corridor scouting pays?

use sssp_rust::curvature_gate::{corridor_curved, edge_scores, pearson, CurveGate};
use sssp_rust::{
    clustered_graph, grid_graph, random_dense_graph, random_sparse_graph, road_network_like,
    verify_distances, Graph,
};

fn families(n: usize, seed: u64) -> Vec<(&'static str, Graph)> {
    let side = (n as f64).sqrt() as usize;
    vec![
        ("sparse", random_sparse_graph(n, 4, seed)),
        ("dense", random_dense_graph(n, 0.1, seed)),
        ("grid", grid_graph(side, side, seed)),
        ("clustered", clustered_graph(n / 100, 100, 0.5, n / 50, seed)),
        ("road_like", road_network_like(n, seed)),
    ]
}

fn dijkstra_pushes(g: &Graph, s: usize) -> (u64, Vec<f64>) {
    use std::collections::BinaryHeap;
    use sssp_rust::State;
    let n = g.n;
    let mut dist = vec![f64::INFINITY; n];
    let mut vis = vec![false; n];
    let mut p = 0u64;
    dist[s] = 0.0;
    let mut pq = BinaryHeap::new();
    pq.push(State { dist: 0.0, node: s });
    p += 1;
    while let Some(State { dist: d, node: u }) = pq.pop() {
        if d > dist[u] || vis[u] {
            continue;
        }
        vis[u] = true;
        for &(v, w) in &g.adj[u] {
            if d + w < dist[v] {
                dist[v] = d + w;
                pq.push(State { dist: dist[v], node: v });
                p += 1;
            }
        }
    }
    (p, dist)
}

fn main() {
    const T: usize = 4;
    let sizes = [1000usize, 2000, 5000, 10000];
    let arms = [
        ("DEGREE", CurveGate::Degree),
        ("DEG_ADAPT", CurveGate::DegreeAdaptive),
        ("FORMAN", CurveGate::Forman),
        ("TRIANGLE", CurveGate::Triangle),
    ];

    println!("CURVATURE GATE EXPERIMENT");
    println!("all arms rate-matched to the published gate's firing rate on each graph");
    println!("pushes = priority-queue insertions (the paper's stated mechanism)\n");
    println!(
        "{:<16} {:>10} {:>9} {:>8} {:>8} {:>7}",
        "family/n  arm", "pushes", "vs dij", "scouts", "correct", "corr"
    );
    println!("{}", "-".repeat(66));

    let mut best_count = std::collections::BTreeMap::<&str, usize>::new();
    let mut rows_json: Vec<String> = Vec::new();

    for &n in &sizes {
        for (fam, g) in families(n, 42) {
            let (dij, reference) = dijkstra_pushes(&g, 0);
            let sc = edge_scores(&g, T);
            // Does the cheap proxy track the real curvature? Negated because
            // LOW degree-sum and HIGH Forman both mean "flat".
            let corr = -pearson(&sc.degree_sum, &sc.forman);

            let mut best = ("", u64::MAX);
            for (name, gate) in arms {
                let out = corridor_curved(&g, 0, gate, T);
                let ok = verify_distances(&out.dist, &reference);
                if out.pushes < best.1 {
                    best = (name, out.pushes);
                }
                println!(
                    "{:<16} {:>10} {:>8.2}x {:>8} {:>8} {:>7.2}",
                    format!("{fam}/{n} {name}"),
                    out.pushes,
                    out.pushes as f64 / dij as f64,
                    out.scouts,
                    if ok { "yes" } else { "NO" },
                    corr
                );
                rows_json.push(format!(
                    "{{\"family\":\"{fam}\",\"n\":{n},\"arm\":\"{name}\",\"pushes\":{},\
                     \"dijkstra\":{dij},\"scouts\":{},\"correct\":{ok},\"corr\":{:.4}}}",
                    out.pushes, out.scouts, corr
                ));
            }
            *best_count.entry(best.0).or_insert(0) += 1;
            println!();
        }
    }

    println!("{}", "=".repeat(66));
    println!("BEST ARM BY FEWEST PUSHES, over {} configurations", sizes.len() * 5);
    for (arm, c) in &best_count {
        println!("  {arm:<12} {c}");
    }
    println!();
    println!("Reading: DEGREE winning means the O(1) proxy is not costing anything.");
    println!("FORMAN/TRIANGLE winning means real curvature sees something degree misses.");

    std::fs::write(
        "curvegate_results.json",
        format!("[{}]", rows_json.join(",\n")),
    )
    .ok();
    println!("\nwrote curvegate_results.json");
}
