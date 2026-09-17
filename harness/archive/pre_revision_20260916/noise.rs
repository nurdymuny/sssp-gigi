//! Is Davis-Corridor's margin over Dijkstra larger than the noise floor?
//!
//! The faithful run showed Corridor firing ZERO scouts on dense, clustered and
//! road_like -- the three families the paper reports it winning. With no
//! scouts, Corridor is Dijkstra plus two degree lookups per relaxed edge, so
//! any margin there has to be measurement noise unless it clears the spread.
//!
//! Method: 31 runs each, interleaved A/B/A/B so drift and thermal effects hit
//! both arms equally, discarding a warmup. Report the median margin against
//! the run-to-run spread of the baseline itself. A margin inside the spread is
//! not a win.

use std::time::Instant;

use sssp_rust::{
    clustered_graph, davis_corridor, davis_delta, dijkstra, grid_graph, random_dense_graph,
    random_sparse_graph, road_network_like, Graph,
};

fn pct(v: &mut Vec<f64>, p: f64) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((p * (v.len() - 1) as f64).round() as usize).min(v.len() - 1)]
}

/// Interleaved A/B timing. Returns (median_a, median_b, iqr_a).
fn ab(g: &Graph, runs: usize, a: fn(&Graph, usize) -> Vec<f64>, b: fn(&Graph, usize) -> Vec<f64>) -> (f64, f64, f64) {
    let mut ta = Vec::new();
    let mut tb = Vec::new();
    // warmup
    a(g, 0);
    b(g, 0);
    for _ in 0..runs {
        let t = Instant::now();
        a(g, 0);
        ta.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        b(g, 0);
        tb.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    let ma = pct(&mut ta.clone(), 0.5);
    let mb = pct(&mut tb.clone(), 0.5);
    let q1 = pct(&mut ta.clone(), 0.25);
    let q3 = pct(&mut ta.clone(), 0.75);
    (ma, mb, q3 - q1)
}

/// Scouts Davis-Corridor actually fires on this graph (threshold 4, as shipped).
fn corridor_fires(g: &Graph) -> usize {
    let mut c = 0;
    for u in 0..g.n {
        for &(v, _) in &g.adj[u] {
            if g.degree[u] <= 4 && g.degree[v] <= 4 {
                c += 1;
            }
        }
    }
    c
}

fn main() {
    let runs = 31;
    println!("IS THE MARGIN BIGGER THAN THE NOISE?");
    println!("{runs} interleaved runs per arm, warmup discarded\n");
    println!(
        "{:<18} {:>9} {:>9} {:>9} {:>9} {:>8} {:>9}",
        "graph", "dij med", "corr med", "margin", "dij IQR", "scouts", "verdict"
    );
    println!("{}", "-".repeat(78));

    let mut real = 0;
    let mut noise = 0;

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
            let (md, mc, iqr) = ab(g, runs, dijkstra, davis_corridor);
            let margin = md - mc; // positive = corridor faster
            let fires = corridor_fires(g);
            let verdict = if margin.abs() < iqr {
                noise += 1;
                "noise"
            } else if margin > 0.0 {
                real += 1;
                "corridor"
            } else {
                real += 1;
                "dijkstra"
            };
            println!(
                "{:<18} {:>9.3} {:>9.3} {:>+9.3} {:>9.3} {:>8} {:>9}",
                format!("{fam}/{n}"),
                md,
                mc,
                margin,
                iqr,
                fires,
                verdict
            );
        }
        println!();
    }

    println!("{}", "=".repeat(78));
    println!("margin inside the baseline's own IQR (indistinguishable) : {noise}");
    println!("margin outside it                                       : {real}");
    println!();
    println!("Davis-Delta is a separate matter -- it is textbook delta-stepping and");
    println!("its sparse-graph advantage is a real algorithmic difference, not this.");

    // Davis-Delta, the variant that carries the headline speedups
    println!("\nDAVIS-DELTA vs DIJKSTRA (the variant with the real margin)");
    println!("{:<18} {:>9} {:>9} {:>9} {:>9}", "graph", "dij med", "delta med", "speedup", "dij IQR");
    println!("{}", "-".repeat(60));
    for &n in &[1000usize, 5000] {
        for (fam, g) in [
            ("sparse", random_sparse_graph(n, 4, 42)),
            ("clustered", clustered_graph(n / 50, 50, 0.5, 2, 42)),
        ] {
            let (md, mx, iqr) = ab(&g, runs, dijkstra, davis_delta);
            println!(
                "{:<18} {:>9.3} {:>9.3} {:>8.2}x {:>9.3}",
                format!("{fam}/{n}"),
                md,
                mx,
                md / mx,
                iqr
            );
        }
    }
}
