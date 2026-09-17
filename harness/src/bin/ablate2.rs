//! Deterministic source-zero gate reproduction; expanded controls are in campaign.rs.
use sssp_rust::{revision::*, *};
fn main() {
    println!("family,n,m,class,arm,admitted,pushes,pops,scouts,scout_scans,comparisons");
    for n in [1000, 2000, 5000, 10000] {
        for family in ["sparse", "dense", "grid", "clustered", "roadlike"] {
            let g = match family {
                "sparse" => random_sparse_graph(n, 4, 42),
                "dense" => random_dense_graph(n, 0.1, 42),
                "grid" => paper_grid(n, 42),
                "clustered" => clustered_graph(n / 50, 50, 0.5, 2, 42),
                _ => road_network_like(n, 42),
            };
            let reference = dijkstra(&g, 0);
            let gated = Mask::predicate(&g, |u, v| g.degree[u] <= 4 && g.degree[v] <= 4);
            let class = if gated.admitted == 0 {
                "dead"
            } else if gated.admitted == g.m {
                "saturated"
            } else {
                "live"
            };
            let tau = 3usize.max(g.m / g.n / 2);
            let controls = [
                ("never", Mask::predicate(&g, |_, _| false)),
                ("always", Mask::predicate(&g, |_, _| true)),
                ("degree4", gated.clone()),
                (
                    "high_high",
                    Mask::predicate(&g, |u, v| g.degree[u] > 4 && g.degree[v] > 4),
                ),
                (
                    "complement",
                    Mask::predicate(&g, |u, v| g.degree[u] > 4 || g.degree[v] > 4),
                ),
                ("random_exact", Mask::random(&g, gated.admitted, 42)),
                (
                    "adaptive",
                    Mask::predicate(&g, |u, v| g.degree[u] <= tau && g.degree[v] <= tau),
                ),
            ];
            for (name, m) in controls {
                let (d, c) = heap::<true, 2>(&g, 0, Some(&m), u64::MAX, None);
                check(&reference, &d);
                println!(
                    "{family},{n},{},{class},{name},{},{},{},{},{},{}",
                    g.m, m.admitted, c.pushes, c.pops, c.scouts, c.scout_scans, c.comparisons
                );
            }
        }
    }
}
