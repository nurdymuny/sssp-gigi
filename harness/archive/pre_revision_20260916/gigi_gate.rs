//! The GIGI curvature gate, redone so that every step can be checked.
//!
//! The earlier result -- "GIGI's curvature gates worse than degree" -- was
//! produced by an instrument I had already shown to be degenerate: on a
//! two-record sub-bundle var/range^2 is identically 0.25 by Popoviciu, and the
//! generators have mean out-degree 4. So the number never measured geometry.
//! This file does not re-run that test. It rebuilds it in seven steps, each of
//! which prints something a reader can verify without trusting the others.
//!
//!   STEP 1  Prove the degeneracy on the engine itself: two-record bundles with
//!           very different weights must both give K = 0.25 exactly.
//!   STEP 2  Audit the old instrument on the real graph: how many vertices had
//!           <= 2 samples, and how many K values sit at exactly 0.25.
//!   STEP 3  Fix the instrument: 2-hop neighbourhoods, so the statistic has
//!           ~20 samples instead of ~4. Report the sample-size distribution.
//!   STEP 4  Gate the corridor scout on the fixed K. Her corridor, verbatim,
//!           with only the predicate swapped. Rate-matched at the EDGE level.
//!           Scored on priority-queue insertions, which are deterministic, so
//!           CPU load cannot touch the result.
//!   STEP 5  Mechanism removal: permute the real K values across vertices and
//!           gate on the permuted copy. If the permuted gate does as well as
//!           the real one, K carries no information about where to scout.
//!   STEP 6  Falsifier: gate on HIGH K instead of low. If that does better, the
//!           "scout where flat" reading is wrong even though K is informative.
//!
//! Every curvature number comes from gigi::curvature::scalar_curvature on a
//! real gigi::bundle::BundleStore. Nothing is reimplemented.

use std::collections::{BinaryHeap, HashSet};

use gigi::bundle::BundleStore;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

use sssp_rust::{
    clustered_graph, dijkstra, grid_graph, random_dense_graph, random_sparse_graph,
    road_network_like, verify_distances, Graph, State,
};

// ── GIGI ────────────────────────────────────────────────────────────────────

fn edge_schema() -> BundleSchema {
    BundleSchema::new("nbhd")
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .fiber(FieldDef::numeric("weight"))
}

fn k_of_edges(edges: &[(usize, usize, f64)]) -> Option<f64> {
    if edges.len() < 2 {
        return None;
    }
    let mut s = BundleStore::new(edge_schema());
    for &(a, b, w) in edges {
        let mut r = Record::new();
        r.insert("vertex_a".into(), Value::Integer(a as i64));
        r.insert("vertex_b".into(), Value::Integer(b as i64));
        r.insert("weight".into(), Value::Float(w));
        s.insert(&r);
    }
    Some(gigi::curvature::scalar_curvature(&s))
}

/// 1-hop: the edges leaving u. This is the old, degenerate instrument.
fn k_1hop(g: &Graph) -> (Vec<Option<f64>>, Vec<usize>) {
    let mut ks = Vec::with_capacity(g.n);
    let mut ns = Vec::with_capacity(g.n);
    for u in 0..g.n {
        let e: Vec<(usize, usize, f64)> = g.adj[u].iter().map(|&(v, w)| (u, v, w)).collect();
        ns.push(e.len());
        ks.push(k_of_edges(&e));
    }
    (ks, ns)
}

/// 2-hop: the edges leaving u, plus the edges leaving each out-neighbour of u.
/// Deduplicated on (a,b) so a parallel edge is one sample, matching GIGI's own
/// key semantics on an edge bundle.
fn k_2hop(g: &Graph) -> (Vec<Option<f64>>, Vec<usize>) {
    let mut ks = Vec::with_capacity(g.n);
    let mut ns = Vec::with_capacity(g.n);
    for u in 0..g.n {
        let mut seen: HashSet<(usize, usize)> = HashSet::new();
        let mut e: Vec<(usize, usize, f64)> = Vec::new();
        for &(v, w) in &g.adj[u] {
            if seen.insert((u, v)) {
                e.push((u, v, w));
            }
            for &(x, w2) in &g.adj[v] {
                if seen.insert((v, x)) {
                    e.push((v, x, w2));
                }
            }
        }
        ns.push(e.len());
        ks.push(k_of_edges(&e));
    }
    (ks, ns)
}

// ── her corridor, predicate swapped, nothing else ─────────────────────────

#[derive(Clone, Copy)]
enum Gate<'a> {
    /// never scouts: her corridor reduces to Dijkstra with the same code path
    Never,
    /// always scouts
    Always,
    Degree(usize),
    /// k[u] <= t and k[v] <= t   (scout where FLAT)
    Low(&'a [Option<f64>], f64),
    /// k[u] >= t and k[v] >= t   (scout where CURVED -- the falsifier)
    High(&'a [Option<f64>], f64),
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
                    Gate::Low(k, t) => match (k[u], k[v]) {
                        (Some(a), Some(b)) => a <= t && b <= t,
                        _ => false,
                    },
                    Gate::High(k, t) => match (k[u], k[v]) {
                        (Some(a), Some(b)) => a >= t && b >= t,
                        _ => false,
                    },
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

/// Edges on which the degree gate fires, counted over the whole graph.
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

/// Threshold admitting `target` edges under "both endpoints pass".
/// For LOW gating the pair statistic is max(k_u, k_v) and we take the k-th
/// smallest; for HIGH it is min(k_u, k_v) and we take the k-th largest.
fn edge_rate_matched(g: &Graph, k: &[Option<f64>], target: usize, high: bool) -> f64 {
    let mut stat: Vec<f64> = Vec::new();
    for u in 0..g.n {
        for &(v, _) in &g.adj[u] {
            if let (Some(a), Some(b)) = (k[u], k[v]) {
                stat.push(if high { a.min(b) } else { a.max(b) });
            }
        }
    }
    // target 0 must admit nothing: a threshold at the minimum stat would still
    // admit the minimum itself (this produced 1-scout rows in the first run)
    if stat.is_empty() || target == 0 {
        return if high { f64::INFINITY } else { f64::NEG_INFINITY };
    }
    stat.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let idx = target.min(stat.len().saturating_sub(1));
    if high {
        stat[stat.len() - 1 - idx]
    } else {
        stat[idx]
    }
}

/// Spatially coherent noise: one random value per vertex, averaged over the
/// same 2-hop vertex set K was computed on. A permutation destroys K's value
/// AND its spatial coherence at once; this control keeps the coherence and
/// destroys only the value, so "K beats its shuffle" can be separated from
/// "clustered admitted edges beat scattered ones".
fn smooth_random(g: &Graph, seed: u64) -> Vec<Option<f64>> {
    let mut s = seed | 1;
    let raw: Vec<f64> = (0..g.n)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            (s % 1_000_000) as f64 / 1e6
        })
        .collect();
    let mut out = Vec::with_capacity(g.n);
    for u in 0..g.n {
        let mut seen: HashSet<usize> = HashSet::new();
        seen.insert(u);
        let mut sum = raw[u];
        let mut cnt = 1.0;
        for &(v, _) in &g.adj[u] {
            if seen.insert(v) {
                sum += raw[v];
                cnt += 1.0;
            }
            for &(x, _) in &g.adj[v] {
                if seen.insert(x) {
                    sum += raw[x];
                    cnt += 1.0;
                }
            }
        }
        out.push(Some(sum / cnt));
    }
    out
}

/// Deterministic Fisher-Yates so the mechanism-removal arm is reproducible.
fn permuted(k: &[Option<f64>], seed: u64) -> Vec<Option<f64>> {
    let mut out = k.to_vec();
    let mut s = seed | 1;
    for i in (1..out.len()).rev() {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        let j = (s % (i as u64 + 1)) as usize;
        out.swap(i, j);
    }
    out
}

fn main() {
    const T: usize = 4;

    // ── STEP 1: the degeneracy, on the engine ──────────────────────────
    println!("STEP 1  Popoviciu degeneracy, computed by gigi::curvature::scalar_curvature");
    for (label, w1, w2) in [("weights 1 and 100", 1.0, 100.0), ("weights 50 and 51", 50.0, 51.0), ("weights 7 and 7.001", 7.0, 7.001)] {
        let k = k_of_edges(&[(0, 1, w1), (0, 2, w2)]).unwrap();
        println!("  two records, {label:<20}  K = {k:.6}");
    }
    let k3 = k_of_edges(&[(0, 1, 1.0), (0, 2, 50.0), (0, 3, 100.0)]).unwrap();
    let k3b = k_of_edges(&[(0, 1, 49.0), (0, 2, 50.0), (0, 3, 51.0)]).unwrap();
    println!("  three records, spread 1/50/100   K = {k3:.6}");
    println!("  three records, spread 49/50/51   K = {k3b:.6}");
    println!("  -> at n=2 the statistic is constant; it starts carrying weight information at n=3.\n");

    let sizes = [1000usize, 2000];
    let mut tally: Vec<(String, u64, u64, u64, u64, u64)> = Vec::new();
    // (graph, rate, ungated, K low, K perm, smooth-random, K high, ALWAYS)
    let mut sweep: Vec<(String, f64, u64, u64, u64, u64, u64, u64)> = Vec::new();

    for &n in &sizes {
        let side = (n as f64).sqrt() as usize;
        let cases: Vec<(&str, Graph)> = vec![
            ("sparse", random_sparse_graph(n, 4, 42)),
            ("grid", grid_graph(side, side, 42)),
            ("clustered", clustered_graph(n / 50, 50, 0.5, 2, 42)),
            ("road_like", road_network_like(n, 42)),
            ("dense", random_dense_graph(n, 0.1, 42)),
        ];
        for (fam, g) in &cases {
            println!("{}", "=".repeat(78));
            println!("{fam}/{n}");

            // ── STEP 2: audit the old instrument ──────────────────────────
            let (k1, n1) = k_1hop(g);
            let le2 = n1.iter().filter(|&&s| s <= 2).count();
            let at_quarter = k1.iter().filter(|k| matches!(k, Some(v) if (v - 0.25).abs() < 1e-12)).count();
            let none = k1.iter().filter(|k| k.is_none()).count();
            println!(
                "STEP 2  1-hop instrument: {le2} of {} vertices have <=2 samples; \
                 {at_quarter} have K == 0.25 exactly; {none} undefined (<2 edges)",
                g.n
            );

            // ── STEP 3: fix the instrument ────────────────────────────────
            let (k2, n2) = k_2hop(g);
            let mut ss: Vec<usize> = n2.clone();
            ss.sort_unstable();
            let med = ss[ss.len() / 2];
            let at_q2 = k2.iter().filter(|k| matches!(k, Some(v) if (v - 0.25).abs() < 1e-12)).count();
            let defined: Vec<f64> = k2.iter().filter_map(|k| *k).collect();
            let (kmin, kmax) = defined.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| (lo.min(v), hi.max(v)));
            println!(
                "STEP 3  2-hop instrument: median samples {med} (min {}, max {}); \
                 {at_q2} at K == 0.25; K ranges {kmin:.4} .. {kmax:.4}",
                ss[0], ss[ss.len() - 1]
            );

            // ── STEP 4-6: the gates, on deterministic pushes ──────────────
            let reference = dijkstra(g, 0);
            let target = degree_edge_fires(g, T);
            let t_low1 = edge_rate_matched(g, &k1, target, false);
            let t_low2 = edge_rate_matched(g, &k2, target, false);
            let t_high2 = edge_rate_matched(g, &k2, target, true);
            let k2_perm = permuted(&k2, 0xC0FFEE);
            let t_perm = edge_rate_matched(g, &k2_perm, target, false);

            let (dn, p_never, _) = corridor_gated(g, 0, Gate::Never);
            assert!(verify_distances(&dn, &reference), "{fam}/{n}: NEVER arm wrong");
            println!("        {:<14} {:>8} {:>8}   (never scouts: Dijkstra on this code path)", "UNGATED", p_never, 0);
            let (dd, p_deg, s_deg) = corridor_gated(g, 0, Gate::Degree(T));
            let (d1, p_k1, s_k1) = corridor_gated(g, 0, Gate::Low(&k1, t_low1));
            let (d2, p_k2, s_k2) = corridor_gated(g, 0, Gate::Low(&k2, t_low2));
            let (dp, p_perm, s_perm) = corridor_gated(g, 0, Gate::Low(&k2_perm, t_perm));
            let (dh, p_high, s_high) = corridor_gated(g, 0, Gate::High(&k2, t_high2));

            for (nm, d) in [("degree", &dd), ("K 1-hop", &d1), ("K 2-hop", &d2), ("K permuted", &dp), ("K high", &dh)] {
                assert!(verify_distances(d, &reference), "{fam}/{n}: {nm} arm returned wrong distances");
            }

            println!("        {:<14} {:>8} {:>8}", "arm", "pushes", "scouts");
            println!("        {:<14} {:>8} {:>8}   (her gate)", "DEGREE", p_deg, s_deg);
            println!("        {:<14} {:>8} {:>8}   (old instrument)", "K low, 1-hop", p_k1, s_k1);
            println!("        {:<14} {:>8} {:>8}   (fixed instrument)", "K low, 2-hop", p_k2, s_k2);
            println!("        {:<14} {:>8} {:>8}   STEP 5 mechanism removed", "K permuted", p_perm, s_perm);
            println!("        {:<14} {:>8} {:>8}   STEP 6 falsifier", "K high, 2-hop", p_high, s_high);
            println!(
                "        all five arms exact vs Dijkstra; gate rate-matched at {target} edges"
            );
            tally.push((format!("{fam}/{n}"), p_deg, p_k2, p_perm, p_high, target as u64));

            // ── STEP 7: where her gate is dead, sweep K's admission rate ──
            // Steps 4-6 rate-match to the degree gate, so on a family where
            // that gate admits nothing they cannot test K at all. Those are
            // the families with metric structure (road_like weights come from
            // Euclidean distance; clustered has two weight classes), so this
            // is the one place a curvature gate could have something to see.
            if target == 0 {
                let m: usize = g.adj.iter().map(|a| a.len()).sum();
                let (d0, p0, _) = corridor_gated(g, 0, Gate::Never);
                let (da, pa, sa) = corridor_gated(g, 0, Gate::Always);
                assert!(verify_distances(&d0, &reference), "{fam}/{n}: NEVER arm wrong");
                assert!(verify_distances(&da, &reference), "{fam}/{n}: ALWAYS arm wrong");
                println!("STEP 7  degree gate admits 0 of {m} edges here; sweeping K admission rate instead");
                println!("        ungated (same code, never scouts) pushes {p0}; ALWAYS-scout pushes {pa} ({sa} scouts)");
                let ksm = smooth_random(g, 0xA11CE);
                println!(
                    "        {:>6} {:>8} {:>8} {:>8} {:>8}   scouts(K low)",
                    "rate", "K low", "K perm", "smooth", "K high"
                );
                for rate in [0.05f64, 0.10, 0.25, 0.50] {
                    let tgt = (m as f64 * rate) as usize;
                    let tl = edge_rate_matched(g, &k2, tgt, false);
                    let tp = edge_rate_matched(g, &k2_perm, tgt, false);
                    let ts = edge_rate_matched(g, &ksm, tgt, false);
                    let th = edge_rate_matched(g, &k2, tgt, true);
                    let (dl, pl, sl) = corridor_gated(g, 0, Gate::Low(&k2, tl));
                    let (dp2, pp2, _) = corridor_gated(g, 0, Gate::Low(&k2_perm, tp));
                    let (ds2, ps2, _) = corridor_gated(g, 0, Gate::Low(&ksm, ts));
                    let (dh2, ph2, _) = corridor_gated(g, 0, Gate::High(&k2, th));
                    for (nm, d) in [("K low", &dl), ("K perm", &dp2), ("smooth", &ds2), ("K high", &dh2)] {
                        assert!(verify_distances(d, &reference), "{fam}/{n}: step 7 {nm} wrong at rate {rate}");
                    }
                    println!(
                        "        {:>5.0}% {:>8} {:>8} {:>8} {:>8}   {sl}",
                        rate * 100.0, pl, pp2, ps2, ph2
                    );
                    sweep.push((format!("{fam}/{n}"), rate, p0, pl, pp2, ps2, ph2, pa));
                }
            }
        }
    }

    println!("\n{}", "=".repeat(78));
    println!("VERDICT, on priority-queue insertions (deterministic, lower is better)");
    println!("{:<16} {:>8} {:>8} {:>8} {:>8}   {}", "graph", "degree", "K 2hop", "K perm", "K high", "reads");
    let mut k_beats_perm = 0;
    let mut k_beats_deg = 0;
    let mut informative = 0;
    for (name, pd, pk, pp, ph, tgt) in &tally {
        let read = if *tgt == 0 {
            "gate never fires here"
        } else if pk < pp && pk < pd {
            k_beats_perm += 1; k_beats_deg += 1; informative += 1;
            "K informative AND beats degree"
        } else if pk < pp {
            k_beats_perm += 1; informative += 1;
            "K informative, degree still better"
        } else if pk == pp {
            "K no better than its own shuffle"
        } else {
            "shuffled K does better -- K misleading here"
        };
        println!("{:<16} {:>8} {:>8} {:>8} {:>8}   {}", name, pd, pk, pp, ph, read);
    }
    println!();
    println!("  K beats its own permutation (carries information) : {informative}");
    println!("  K beats degree                                    : {k_beats_deg}");
    println!("  K beats permutation but not degree                : {}", k_beats_perm - k_beats_deg);
    println!();
    println!("  If K never beats its permutation, the curvature is not telling the gate");
    println!("  anything -- and that would be a result about the statistic, not about");
    println!("  the idea. If it beats the permutation but not degree, the curvature is");
    println!("  real but degree is the cheaper equivalent on these graphs.");

    println!("\n{}", "=".repeat(78));
    println!("STEP 7 VERDICT  K-gated scouting where the degree gate is dead (pushes; lower is better)");
    println!(
        "{:<16} {:>5} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}   {}",
        "graph", "rate", "ungated", "K low", "K perm", "smooth", "K high", "ALWAYS", "reads"
    );
    let mut low_beats_ungated = 0;
    let mut low_beats_perm = 0;
    let mut low_beats_smooth = 0;
    let mut rows = 0;
    for (name, rate, p0, pl, pp, ps, ph, pa) in &sweep {
        rows += 1;
        if pl < p0 {
            low_beats_ungated += 1;
        }
        if pl < pp {
            low_beats_perm += 1;
        }
        if pl < ps {
            low_beats_smooth += 1;
        }
        let read = if pl < p0 {
            "K low beats UNGATED"
        } else if pl < ps && pl < pp {
            "K value matters beyond coherence; scouting still costs"
        } else if pl < pp {
            "beats shuffle only -- coherence, not curvature"
        } else {
            "no signal"
        };
        println!(
            "{:<16} {:>4.0}% {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}   {}",
            name, rate * 100.0, p0, pl, pp, ps, ph, pa, read
        );
    }
    println!();
    println!("  rows {rows}: K low beats ungated {low_beats_ungated}; beats its shuffle {low_beats_perm}; beats coherent noise {low_beats_smooth}");
    println!("  The shuffle removes value AND spatial coherence. Coherent noise removes only the value.");
    println!("  Only 'beats coherent noise' is evidence that the curvature number itself is doing work.");
}
