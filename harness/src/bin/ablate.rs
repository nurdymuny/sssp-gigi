//! Runs the four-arm corridor gate ablation across the paper's graph families.

use sssp_rust::ablation::{corridor_ablated, dijkstra_counted, Gate};
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

fn median_time<F: FnMut() -> u64>(mut f: F, runs: usize) -> f64 {
    let mut ts: Vec<f64> = Vec::new();
    for _ in 0..runs {
        let t0 = std::time::Instant::now();
        let _ = f();
        ts.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    ts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ts[ts.len() / 2]
}

fn main() {
    const T: usize = 4; // the paper's FLAT_THRESHOLD
    let sizes = [1000usize, 2000, 5000, 10000];
    let arms = [
        ("GATED", Gate::Gated),
        ("ALWAYS", Gate::Always),
        ("INVERTED", Gate::Inverted),
        ("RANDOM", Gate::Random),
        ("ADAPTIVE", Gate::Adaptive),
    ];

    println!("Davis-Corridor gate ablation  (threshold = {T}, 5 runs, median)");
    println!("pushes = priority-queue insertions, the paper's stated mechanism\n");
    println!(
        "{:<11} {:>6} | {:>9} {:>9} {:>9} {:>9} | {:>8} {:>8}",
        "family/n", "arm", "pushes", "vs dij", "scouts", "ms", "correct", ""
    );
    println!("{}", "-".repeat(84));

    // aggregate: does GATED beat ALWAYS on pushes?
    let mut gated_better = 0usize;
    let mut always_better = 0usize;
    let mut ties = 0usize;
    let mut gated_vs_random = (0usize, 0usize, 0usize);
    let mut adaptive_wins = 0usize; let mut fixed_wins = 0usize; let mut adaptive_ties = 0usize;

    for &n in &sizes {
        for (fam, g) in families(n, 42) {
            let reference = dijkstra_counted(&g, 0);
            let dij_pushes = reference.pushes;
            let mut row: Vec<(String, u64, u64, f64, bool)> = Vec::new();
            for (name, gate) in arms {
                let out = corridor_ablated(&g, 0, gate, T);
                let ok = verify_distances(&out.dist, &reference.dist);
                let ms = median_time(|| corridor_ablated(&g, 0, gate, T).pushes, 5);
                row.push((name.to_string(), out.pushes, out.scouts, ms, ok));
            }
            for (name, pushes, scouts, ms, ok) in &row {
                println!(
                    "{:<11} {:>6} | {:>9} {:>8.2}x {:>9} {:>9.2} | {:>8}",
                    format!("{fam}/{n}"),
                    name,
                    pushes,
                    *pushes as f64 / dij_pushes as f64,
                    scouts,
                    ms,
                    if *ok { "yes" } else { "NO" }
                );
            }
            let gp = row[0].1;
            let ap = row[1].1;
            let rp = row[3].1;
            let adp = row[4].1;
            if adp < gp { adaptive_wins += 1 } else if gp < adp { fixed_wins += 1 } else { adaptive_ties += 1 }
            if gp < ap {
                gated_better += 1
            } else if ap < gp {
                always_better += 1
            } else {
                ties += 1
            }
            if gp < rp {
                gated_vs_random.0 += 1
            } else if rp < gp {
                gated_vs_random.1 += 1
            } else {
                gated_vs_random.2 += 1
            }
            println!();
        }
    }

    println!("{}", "=".repeat(84));
    println!("VERDICT on the curvature gate, by priority-queue insertions");
    println!(
        "  GATED fewer pushes than ALWAYS   : {gated_better} of {} configs",
        gated_better + always_better + ties
    );
    println!("  ALWAYS fewer pushes than GATED   : {always_better}");
    println!("  tie                              : {ties}");
    println!(
        "  GATED fewer pushes than RANDOM   : {} of {} (rate-matched)",
        gated_vs_random.0,
        gated_vs_random.0 + gated_vs_random.1 + gated_vs_random.2
    );
    println!();
    println!("  GATED > ALWAYS  => scouting only pays in flat regions: the gate earns its place.");
    println!("  GATED ~ ALWAYS  => the win is scouting itself; the degree test is decoration.");
    println!("  GATED > RANDOM  => DEGREE is the right signal, not just 'scout sometimes'.");
    println!();
    println!("ADAPTIVE threshold (Davis-Deep's tau) applied to Corridor:");
    println!("  ADAPTIVE fewer pushes than fixed-4 : {adaptive_wins}");
    println!("  fixed-4 fewer pushes than ADAPTIVE : {fixed_wins}");
    println!("  tie                                : {adaptive_ties}");
}
