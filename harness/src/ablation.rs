//! Mechanism-removal test for the Davis-Corridor curvature gate.
//!
//! Davis-Corridor scouts one hop ahead when BOTH endpoints are "flat"
//! (degree <= 4). The paper's thesis is that this degree gate -- the
//! connectivity curvature proxy -- is what makes the scouting pay off.
//!
//! That has never been tested against the obvious alternatives. Four arms,
//! identical in every other respect:
//!
//!   GATED     scout iff deg(u) <= T and deg(v) <= T        (as published)
//!   ALWAYS    scout unconditionally                        (gate removed)
//!   INVERTED  scout iff deg(u) > T and deg(v) > T          (gate reversed)
//!   RANDOM    scout on a hash of (u,v), rate-matched to GATED's firing rate
//!
//! If GATED beats ALWAYS, the gate is doing real work: scouting is only
//! profitable in flat regions, which is exactly the paper's claim.
//! If GATED ties ALWAYS, the win comes from scouting per se and the curvature
//! proxy is decoration.
//! If GATED beats RANDOM at the same firing rate, then DEGREE specifically is
//! the right signal, not merely "scout sometimes".
//!
//! Heap pushes are counted alongside wall time. The paper's stated mechanism
//! is "reducing priority queue operations", so pushes are the direct
//! measurement and time is the noisy proxy for it.

use std::collections::BinaryHeap;

use crate::{Graph, State};

#[derive(Clone, Copy, PartialEq)]
pub enum Gate {
    Gated,
    Always,
    Inverted,
    Random,
    /// Davis-Deep's adaptive threshold, applied to Corridor.
    Adaptive,
}

pub struct Outcome {
    pub dist: Vec<f64>,
    pub pushes: u64,
    pub scouts: u64,
}

/// Rate-matched pseudo-random gate: cheap deterministic hash of the edge,
/// thresholded so it fires at approximately `rate`.
#[inline]
fn coin(u: usize, v: usize, rate: f64) -> bool {
    let mut h = (u as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ (v as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    h ^= h >> 31;
    h = h.wrapping_mul(0x94D049BB133111EB);
    h ^= h >> 29;
    ((h >> 11) as f64 / (1u64 << 53) as f64) < rate
}

pub fn corridor_ablated(graph: &Graph, source: usize, gate: Gate, threshold: usize) -> Outcome {
    let n = graph.n;
    if n == 0 {
        return Outcome { dist: Vec::new(), pushes: 0, scouts: 0 };
    }

    // Firing rate of the published gate on THIS graph, so RANDOM scouts as
    // often as GATED does and the comparison isolates *which* edges, not
    // *how many*.
    let flat_frac = {
        let f = graph.degree.iter().filter(|&&d| d <= threshold).count() as f64 / n as f64;
        f * f
    };

    // Davis-Deep computes tau = max(3, avg_degree/2). Corridor hardcodes 4.
    let avg_deg = graph.degree.iter().sum::<usize>() as f64 / n.max(1) as f64;
    let adaptive_t = std::cmp::max(3, (avg_deg / 2.0) as usize);

    let mut dist = vec![f64::INFINITY; n];
    let mut visited = vec![false; n];
    let adj = &graph.adj;
    let degree = &graph.degree;
    let mut pushes: u64 = 0;
    let mut scouts: u64 = 0;

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

        let u_flat = degree[u] <= threshold;

        for &(v, w) in &adj[u] {
            let new_d = d + w;
            if new_d < dist[v] {
                dist[v] = new_d;

                let v_flat = degree[v] <= threshold;
                let fire = match gate {
                    Gate::Gated => u_flat && v_flat,
                    Gate::Always => true,
                    Gate::Inverted => !u_flat && !v_flat,
                    Gate::Random => coin(u, v, flat_frac),
                    Gate::Adaptive => degree[u] <= adaptive_t && degree[v] <= adaptive_t,
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

    Outcome { dist, pushes, scouts }
}

/// Dijkstra with the same push counter, so "pushes saved" has a baseline.
pub fn dijkstra_counted(graph: &Graph, source: usize) -> Outcome {
    let n = graph.n;
    let mut dist = vec![f64::INFINITY; n];
    let mut visited = vec![false; n];
    let mut pushes: u64 = 0;
    dist[source] = 0.0;
    let mut pq = BinaryHeap::with_capacity(n);
    pq.push(State { dist: 0.0, node: source });
    pushes += 1;
    while let Some(State { dist: d, node: u }) = pq.pop() {
        if d > dist[u] || visited[u] {
            continue;
        }
        visited[u] = true;
        for &(v, w) in &graph.adj[u] {
            let nd = d + w;
            if nd < dist[v] {
                dist[v] = nd;
                pq.push(State { dist: nd, node: v });
                pushes += 1;
            }
        }
    }
    Outcome { dist, pushes, scouts: 0 }
}
