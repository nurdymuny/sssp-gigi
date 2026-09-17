//! The degree-gate ablation, redone on the paper's own families.
//!
//! The numbers in the paper's ablation section came from ablate.rs, which
//! used a clustered family that is not the paper's (n/100 clusters of 100
//! with n/50 inter-edges per cluster pair, instead of n/50 clusters of 50
//! with 2) and a side*side grid instead of the paper's rows*cols. It also
//! rate-matched RANDOM at the vertex level, so RANDOM fired far more often
//! than GATED, and it never printed the never-scout baseline.
//!
//! Beyond the parameters, the count "GATED beats ALWAYS 16 of 20" needs
//! unpacking. The degree-4 gate is LIVE only where some edges pass and some
//! do not. On the grid every vertex has degree <= 4, so GATED == ALWAYS by
//! construction. On dense, clustered and road-like no vertex does, so GATED
//! == NEVER and "GATED beats ALWAYS" is "not scouting beats scouting". Only
//! the sparse family is a live comparison. This file prints all three
//! classes separately so a reader can see which rows test the selector.
//!
//! Arms, all inside her corridor with only the predicate swapped:
//!   NEVER     never scouts                  (Dijkstra on the same code path)
//!   ALWAYS    always scouts
//!   GATED     deg(u) <= 4 and deg(v) <= 4   (the paper's gate)
//!   INVERTED  deg(u) >  4 and deg(v) >  4
//!   RANDOM    deterministic per-edge coin at the EDGE admission rate of GATED
//!   ADAPTIVE  deg <= max(3, floor(mean_deg / 2))
//!
//! Scored on priority-queue insertions (deterministic) with every arm
//! verified exact against her dijkstra.

use std::collections::BinaryHeap;

use sssp_rust::{
    clustered_graph, dijkstra, grid_graph, random_dense_graph, random_sparse_graph,
    road_network_like, verify_distances, Graph, State,
};

#[derive(Clone, Copy)]
enum Gate {
    Never,
    Always,
    Degree(usize),
    Inverted(usize),
    /// admit edge (u,v) when hash(u,v) < p
    Random(f64),
}

fn coin(u: usize, v: usize) -> f64 {
    let mut z = (u as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (v as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

fn corridor_gated(graph: &Graph, source: usize, gate: Gate) -> (Vec<f64>, u64, u64) {
    let n = graph.n;
    let mut dist = vec![f64::INFINITY; n];
    let mut visited = vec![false; n];
    let adj = &graph.adj;
    let degree = &graph.degree;
    let mut pushes = 0u64;
    let mut scouts = 0u64;

    dist[source] = 0.0;
    let mut pq = BinaryHeap::with_capacity(n);
    pq.push(State { dist: 0.0, node: source });
    pushes += 1;

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
                    Gate::Never => false,
                    Gate::Always => true,
                    Gate::Degree(t) => degree[u] <= t && degree[v] <= t,
                    Gate::Inverted(t) => degree[u] > t && degree[v] > t,
                    Gate::Random(p) => coin(u, v) < p,
                };

                if fire && !visited[v] {
                    scouts += 1;
                    for &(v2, w2) in &adj[v] {
                        let new_d2 = new_d + w2;
                        if new_d2 < dist[v2] {
                            dist[v2] = new_d2;
                            if !visited[v2] {
                                pq.push(State { dist: new_d2, node: v2 });
                                pushes += 1;
                            }
                        }
                    }
                }

                pq.push(State { dist: new_d, node: v });
                pushes += 1;
            }
        }
    }
    (dist, pushes, scouts)
}

/// Fraction of edges whose both endpoints pass the degree gate.
fn admission(g: &Graph, t: usize) -> (usize, usize) {
    let mut pass = 0;
    let mut m = 0;
    for u in 0..g.n {
        for &(v, _) in &g.adj[u] {
            m += 1;
            if g.degree[u] <= t && g.degree[v] <= t {
                pass += 1;
            }
        }
    }
    (pass, m)
}

/// The paper's grid: rows = largest divisor of n that is <= sqrt(n).
fn paper_grid(n: usize, seed: u64) -> Graph {
    let s = (n as f64).sqrt() as usize;
    let mut rows = 1;
    for r in 1..=s {
        if n % r == 0 {
            rows = r;
        }
    }
    grid_graph(rows, n / rows, seed)
}

fn main() {
    const T: usize = 4;
    println!("DEGREE-GATE ABLATION on the paper's families, insertions, every arm exact");
    println!("gate class: LIVE = some edges pass, SATURATED = all pass (GATED==ALWAYS), DEAD = none pass (GATED==NEVER)\n");
    println!(
        "{:<16} {:>6} {:>5} {:<9} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7}",
        "graph", "admit%", "d_bar", "class", "NEVER", "ALWAYS", "GATED", "INVERT", "RANDOM", "ADAPT"
    );
    println!("{}", "-".repeat(100));

    let mut live: Vec<(String, u64, u64, u64, u64, u64, u64)> = Vec::new();
    let mut dead_or_sat = 0usize;
    let mut dead_never_beats_always = 0usize;

    for &n in &[1000usize, 2000, 5000, 10000] {
        let cases: Vec<(&str, Graph)> = vec![
            ("sparse", random_sparse_graph(n, 4, 42)),
            ("dense", random_dense_graph(n, 0.1, 42)),
            ("grid", paper_grid(n, 42)),
            ("clustered", clustered_graph(n / 50, 50, 0.5, 2, 42)),
            ("road_like", road_network_like(n, 42)),
        ];
        for (fam, g) in &cases {
            let reference = dijkstra(g, 0);
            let (pass, m) = admission(g, T);
            let p = pass as f64 / m.max(1) as f64;
            let d_bar = g.degree.iter().sum::<usize>() as f64 / g.n.max(1) as f64;
            let tau = 3usize.max((d_bar / 2.0).floor() as usize);
            let class = if pass == 0 { "DEAD" } else if pass == m { "SATURATED" } else { "LIVE" };

            let arms = [
                ("NEVER", Gate::Never),
                ("ALWAYS", Gate::Always),
                ("GATED", Gate::Degree(T)),
                ("INVERT", Gate::Inverted(T)),
                ("RANDOM", Gate::Random(p)),
                ("ADAPT", Gate::Degree(tau)),
            ];
            let mut pushes = [0u64; 6];
            let mut scouts = [0u64; 6];
            for (i, (nm, gate)) in arms.iter().enumerate() {
                let (d, pu, sc) = corridor_gated(g, 0, *gate);
                assert!(verify_distances(&d, &reference), "{fam}/{n}: {nm} wrong distances");
                pushes[i] = pu;
                scouts[i] = sc;
            }
            println!(
                "{:<16} {:>5.1}% {:>5.1} {:<9} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7}   scouts G/R {}/{}",
                format!("{fam}/{n}"), p * 100.0, d_bar, class,
                pushes[0], pushes[1], pushes[2], pushes[3], pushes[4], pushes[5],
                scouts[2], scouts[4]
            );
            if class == "LIVE" {
                live.push((format!("{fam}/{n}"), pushes[0], pushes[1], pushes[2], pushes[3], pushes[4], pushes[5]));
            } else {
                dead_or_sat += 1;
                if pushes[0] < pushes[1] {
                    dead_never_beats_always += 1;
                }
            }
        }
        println!();
    }

    println!("{}", "=".repeat(100));
    println!("LIVE configurations (the only rows that test the selector): {}", live.len());
    let mut g_lt_a = 0;
    let mut g_lt_r = 0;
    let mut g_lt_i = 0;
    let mut g_lt_n = 0;
    let mut ad_le_g = 0;
    for (name, nv, al, ga, inv, ra, ad) in &live {
        if ga < al { g_lt_a += 1; }
        if ga < ra { g_lt_r += 1; }
        if ga < inv { g_lt_i += 1; }
        if ga < nv { g_lt_n += 1; }
        if ad <= ga { ad_le_g += 1; }
        println!(
            "  {:<14} NEVER {:>6}  ALWAYS {:>6}  GATED {:>6}  INVERT {:>6}  RANDOM {:>6}  ADAPT {:>6}   order: {}",
            name, nv, al, ga, inv, ra, ad,
            {
                let mut v = vec![("never", *nv), ("gated", *ga), ("random", *ra), ("invert", *inv), ("always", *al)];
                v.sort_by_key(|x| x.1);
                v.iter().map(|x| x.0).collect::<Vec<_>>().join(" < ")
            }
        );
    }
    println!();
    println!("  on live rows: GATED < ALWAYS {g_lt_a}/{}, GATED < RANDOM(edge-matched) {g_lt_r}/{}, GATED < INVERTED {g_lt_i}/{}, GATED < NEVER {g_lt_n}/{}, ADAPTIVE <= GATED {ad_le_g}/{}",
        live.len(), live.len(), live.len(), live.len(), live.len());
    println!("  dead or saturated rows: {dead_or_sat}; of those, NEVER < ALWAYS in {dead_never_beats_always} (scouting costs insertions wherever it happens)");
    println!();
    println!("  GATED < RANDOM and GATED < INVERTED on live rows is the selection evidence.");
    println!("  GATED < NEVER on any row would be the first sign the lookahead pays on this metric.");
}
