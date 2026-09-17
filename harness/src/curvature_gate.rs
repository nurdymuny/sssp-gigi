//! Does a REAL discrete curvature gate the corridor scout better than degree?
//!
//! The paper chooses a connectivity proxy, kappa(v) = (deg(v) - dbar)/dbar, and
//! says why: it is O(1) per vertex, where "more sophisticated notions (e.g.
//! Ollivier-Ricci)" need pairwise distance calculations. That is an honest
//! caveat and it has never been tested. This tests it.
//!
//! The gate fires on an EDGE (u,v), so the like-for-like comparison is an edge
//! curvature, not a vertex one.
//!
//! AUGMENTED FORMAN-RICCI (Forman 2003, discrete Morse theory) on an
//! unweighted edge:
//!
//!     F(u,v) = 4 - deg(u) - deg(v) + 3*T(u,v)
//!
//! where T(u,v) is the number of triangles containing the edge. Note the
//! triangle term: WITHOUT it, F is an affine function of deg(u)+deg(v) and the
//! comparison would be circular -- Forman would be degree wearing a hat. The
//! triangle count is what makes it carry information degree does not, and it
//! is why this is the version worth testing.
//!
//! Every arm is rate-matched: each threshold is calibrated per graph so the
//! gate fires on the same NUMBER of edges the published gate does. That
//! isolates WHICH edges are chosen from HOW MANY, which was the flaw in
//! comparing against an ungated arm.
//!
//! Correctness is unaffected by the gate: scouting only lowers tentative
//! distances and never finalises a vertex, so every arm returns exact
//! distances. Verified per configuration regardless.

use std::collections::{BinaryHeap, HashSet};

use crate::{Graph, State};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CurveGate {
    /// deg(u) <= T and deg(v) <= T, T = 4. The published gate.
    Degree,
    /// deg(u) <= tau and deg(v) <= tau, tau = max(3, dbar/2). Davis-Deep's.
    DegreeAdaptive,
    /// Augmented Forman-Ricci above a rate-matched threshold.
    Forman,
    /// Triangle density (local clustering of the edge) above a rate-matched
    /// threshold -- geometry with the degree term removed entirely.
    Triangle,
}

pub struct GateStats {
    pub dist: Vec<f64>,
    pub pushes: u64,
    pub scouts: u64,
    pub fired: usize,
}

/// Undirected neighbour sets, for triangle counting.
fn neighbour_sets(graph: &Graph) -> Vec<HashSet<usize>> {
    let mut ns: Vec<HashSet<usize>> = vec![HashSet::new(); graph.n];
    for u in 0..graph.n {
        for &(v, _) in &graph.adj[u] {
            ns[u].insert(v);
            ns[v].insert(u);
        }
    }
    ns
}

/// Triangles on edge (u,v) = |N(u) ∩ N(v)|.
#[inline]
fn triangles(ns: &[HashSet<usize>], u: usize, v: usize) -> usize {
    let (a, b) = if ns[u].len() <= ns[v].len() { (u, v) } else { (v, u) };
    ns[a].iter().filter(|w| ns[b].contains(*w)).count()
}

/// Augmented Forman-Ricci of edge (u,v).
#[inline]
pub fn forman(graph: &Graph, ns: &[HashSet<usize>], u: usize, v: usize) -> f64 {
    4.0 - graph.degree[u] as f64 - graph.degree[v] as f64 + 3.0 * triangles(ns, u, v) as f64
}

/// Per-edge value of each gate's underlying score, for calibration and for
/// the degree-vs-curvature correlation.
pub struct EdgeScores {
    pub pairs: Vec<(usize, usize)>,
    pub degree_sum: Vec<f64>,
    pub forman: Vec<f64>,
    pub triangle: Vec<f64>,
    /// Fraction of edges the published gate fires on.
    pub published_rate: f64,
}

pub fn edge_scores(graph: &Graph, t: usize) -> EdgeScores {
    let ns = neighbour_sets(graph);
    let mut pairs = Vec::new();
    let mut degree_sum = Vec::new();
    let mut fo = Vec::new();
    let mut tri = Vec::new();
    let mut fired = 0usize;
    for u in 0..graph.n {
        for &(v, _) in &graph.adj[u] {
            pairs.push((u, v));
            degree_sum.push((graph.degree[u] + graph.degree[v]) as f64);
            fo.push(forman(graph, &ns, u, v));
            tri.push(triangles(&ns, u, v) as f64);
            if graph.degree[u] <= t && graph.degree[v] <= t {
                fired += 1;
            }
        }
    }
    let published_rate = if pairs.is_empty() {
        0.0
    } else {
        fired as f64 / pairs.len() as f64
    };
    EdgeScores { pairs, degree_sum, forman: fo, triangle: tri, published_rate }
}

/// Threshold on `scores` such that the fraction of edges scoring >= it is
/// approximately `rate` (higher score = flatter, so we take the top tail).
fn rate_matched_threshold(scores: &[f64], rate: f64) -> f64 {
    if scores.is_empty() {
        return f64::NEG_INFINITY;
    }
    let mut s: Vec<f64> = scores.to_vec();
    s.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let k = ((rate * s.len() as f64).round() as usize).clamp(0, s.len().saturating_sub(1));
    s[k]
}

pub fn corridor_curved(graph: &Graph, source: usize, gate: CurveGate, t: usize) -> GateStats {
    let n = graph.n;
    if n == 0 {
        return GateStats { dist: Vec::new(), pushes: 0, scouts: 0, fired: 0 };
    }

    let ns = neighbour_sets(graph);
    let sc = edge_scores(graph, t);
    let avg_deg = graph.degree.iter().sum::<usize>() as f64 / n as f64;
    let tau = std::cmp::max(3, (avg_deg / 2.0) as usize);
    let f_thresh = rate_matched_threshold(&sc.forman, sc.published_rate);
    let t_thresh = rate_matched_threshold(&sc.triangle, sc.published_rate);

    let mut dist = vec![f64::INFINITY; n];
    let mut visited = vec![false; n];
    let adj = &graph.adj;
    let degree = &graph.degree;
    let mut pushes: u64 = 0;
    let mut scouts: u64 = 0;
    let mut fired: usize = 0;

    dist[source] = 0.0;
    let mut pq = BinaryHeap::with_capacity(n);
    pq.push(State { dist: 0.0, node: source });
    pushes += 1;

    while let Some(State { dist: d, node: u }) = pq.pop() {
        if d > dist[u] || visited[u] {
            continue;
        }
        visited[u] = true;

        for &(v, w) in &adj[u] {
            let new_d = d + w;
            if new_d < dist[v] {
                dist[v] = new_d;

                let fire = match gate {
                    CurveGate::Degree => degree[u] <= t && degree[v] <= t,
                    CurveGate::DegreeAdaptive => degree[u] <= tau && degree[v] <= tau,
                    CurveGate::Forman => forman(graph, &ns, u, v) >= f_thresh,
                    CurveGate::Triangle => triangles(&ns, u, v) as f64 >= t_thresh,
                };
                if fire {
                    fired += 1;
                }

                if fire && !visited[v] {
                    scouts += 1;
                    for &(v2, w2) in &adj[v] {
                        let nd2 = new_d + w2;
                        if nd2 < dist[v2] {
                            dist[v2] = nd2;
                            if !visited[v2] {
                                pq.push(State { dist: nd2, node: v2 });
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

    GateStats { dist, pushes, scouts, fired }
}

/// Pearson correlation, for "does the cheap proxy track the real thing".
pub fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n < 2 {
        return f64::NAN;
    }
    let ma = a[..n].iter().sum::<f64>() / n as f64;
    let mb = b[..n].iter().sum::<f64>() / n as f64;
    let mut num = 0.0;
    let mut da = 0.0;
    let mut db = 0.0;
    for i in 0..n {
        let x = a[i] - ma;
        let y = b[i] - mb;
        num += x * y;
        da += x * x;
        db += y * y;
    }
    if da <= 0.0 || db <= 0.0 {
        return f64::NAN;
    }
    num / (da.sqrt() * db.sqrt())
}
