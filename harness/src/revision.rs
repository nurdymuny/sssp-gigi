//! Reproducible SSSP instruments. Counters compile out of timed solvers.
use crate::{Graph, State};
use rand::{seq::SliceRandom, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Default, Debug)]
pub struct Counts {
    pub pushes: u64,
    pub pops: u64,
    pub comparisons: u64,
    pub peak: u64,
    pub scans: u64,
    pub scouts: u64,
    pub scout_scans: u64,
    pub scout_yield: u64,
    pub improvements: u64,
    pub superseded: u64,
    pub phases: u64,
    pub vertex_scans: u64,
    pub unchanged: u64,
    pub heavy: u64,
    pub max_phase_scans: u64,
}
macro_rules! count {
    ($c:ident, $x:expr) => {
        if $c {
            $x;
        }
    };
}
struct Heap<const D: usize> {
    a: Vec<State>,
}
impl<const D: usize> Heap<D> {
    fn new() -> Self {
        Self { a: Vec::new() }
    }
    fn less<const C: bool>(a: State, b: State, c: &mut Counts) -> bool {
        count!(C, c.comparisons += 1);
        a.dist < b.dist || (a.dist == b.dist && a.node < b.node)
    }
    fn push<const C: bool>(&mut self, x: State, c: &mut Counts) {
        count!(C, c.pushes += 1);
        self.a.push(x);
        let mut i = self.a.len() - 1;
        while i > 0 {
            let p = (i - 1) / D;
            if !Self::less::<C>(self.a[i], self.a[p], c) {
                break;
            }
            self.a.swap(i, p);
            i = p;
        }
        count!(C, c.peak = c.peak.max(self.a.len() as u64));
    }
    fn pop<const C: bool>(&mut self, c: &mut Counts) -> Option<State> {
        if self.a.is_empty() {
            return None;
        }
        count!(C, c.pops += 1);
        let x = self.a.swap_remove(0);
        let mut i = 0;
        loop {
            let start = D * i + 1;
            if start >= self.a.len() {
                break;
            }
            let mut j = start;
            for k in start + 1..(start + D).min(self.a.len()) {
                if Self::less::<C>(self.a[k], self.a[j], c) {
                    j = k
                }
            }
            if !Self::less::<C>(self.a[j], self.a[i], c) {
                break;
            }
            self.a.swap(i, j);
            i = j;
        }
        Some(x)
    }
}

#[derive(Clone)]
pub struct Mask {
    pub bits: Vec<Vec<bool>>,
    pub admitted: usize,
    pub potential_work: usize,
}
impl Mask {
    pub fn predicate(g: &Graph, f: impl Fn(usize, usize) -> bool) -> Self {
        let mut admitted = 0;
        let mut potential_work = 0;
        let bits = g
            .adj
            .iter()
            .enumerate()
            .map(|(u, a)| {
                a.iter()
                    .map(|&(v, _)| {
                        let x = f(u, v);
                        if x {
                            admitted += 1;
                            potential_work += g.degree[v];
                        }
                        x
                    })
                    .collect()
            })
            .collect();
        Self {
            bits,
            admitted,
            potential_work,
        }
    }
    /// Exact graph-edge admission, including deterministic tie breaking.
    pub fn ranked(g: &Graph, scores: &[Option<f64>], target: usize, high: bool) -> Self {
        let mut ranked = Vec::new();
        for (u, a) in g.adj.iter().enumerate() {
            for (j, &(v, _)) in a.iter().enumerate() {
                if let (Some(x), Some(y)) = (scores[u], scores[v]) {
                    let s = if high { -x.min(y) } else { x.max(y) };
                    ranked.push((s, u, j, v));
                }
            }
        }
        ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        let mut m = Self::predicate(g, |_, _| false);
        for &(_, u, j, v) in ranked.iter().take(target) {
            m.bits[u][j] = true;
            m.admitted += 1;
            m.potential_work += g.degree[v];
        }
        m
    }
    pub fn random(g: &Graph, target: usize, seed: u64) -> Self {
        let mut e = Vec::with_capacity(g.m);
        for (u, a) in g.adj.iter().enumerate() {
            for (j, &(v, _)) in a.iter().enumerate() {
                e.push((u, j, v));
            }
        }
        e.shuffle(&mut ChaCha8Rng::seed_from_u64(seed));
        let mut m = Self::predicate(g, |_, _| false);
        for &(u, j, v) in e.iter().take(target) {
            m.bits[u][j] = true;
            m.admitted += 1;
            m.potential_work += g.degree[v];
        }
        m
    }
}

/// GSR-1, a conventional D-ary Dijkstra when mask is absent. Optional budget
/// caps executed scout arc scans; it does not pretend to equalize opportunities.
pub fn heap<const C: bool, const D: usize>(
    g: &Graph,
    s: usize,
    mask: Option<&Mask>,
    budget: u64,
    incoming: Option<&[f64]>,
) -> (Vec<f64>, Counts) {
    assert!(s < g.n);
    let mut c = Counts::default();
    let mut d = vec![f64::INFINITY; g.n];
    let mut settled = vec![false; g.n];
    let mut q = Heap::<D>::new();
    let mut free = Vec::new();
    let mut spent = 0;
    d[s] = 0.;
    q.push::<C>(State { dist: 0., node: s }, &mut c);
    while let Some(root) = q.pop::<C>(&mut c) {
        if settled[root.node] || root.dist != d[root.node] {
            count!(C, c.superseded += 1);
            continue;
        }
        let lower = root.dist;
        free.push(root.node);
        while let Some(u) = free.pop() {
            if settled[u] {
                continue;
            }
            settled[u] = true;
            count!(C, c.vertex_scans += 1);
            for (j, &(v, w)) in g.adj[u].iter().enumerate() {
                count!(C, c.scans += 1);
                let nd = d[u] + w;
                if nd < d[v] {
                    d[v] = nd;
                    count!(C, c.improvements += 1);
                    if let Some(m) = mask {
                        if m.bits[u][j] && !settled[v] && spent + (g.degree[v] as u64) <= budget {
                            spent += g.degree[v] as u64;
                            count!(C, c.scouts += 1);
                            for &(z, wz) in &g.adj[v] {
                                count!(C, c.scout_scans += 1);
                                let nz = nd + wz;
                                if nz < d[z] {
                                    d[z] = nz;
                                    count!(C, c.scout_yield += 1);
                                    count!(C, c.improvements += 1);
                                    if !settled[z] {
                                        q.push::<C>(State { dist: nz, node: z }, &mut c);
                                    }
                                }
                            }
                        }
                    }
                    if incoming.map_or(false, |inc| nd <= lower + inc[v]) {
                        free.push(v)
                    } else {
                        q.push::<C>(State { dist: nd, node: v }, &mut c)
                    }
                }
            }
        }
    }
    (d, c)
}
pub fn incoming(g: &Graph) -> Vec<f64> {
    let mut a = vec![f64::INFINITY; g.n];
    for e in &g.adj {
        for &(v, w) in e {
            a[v] = a[v].min(w)
        }
    }
    a
}

enum Buckets {
    Map(BTreeMap<usize, Vec<usize>>),
    Ring {
        slots: Vec<(usize, Vec<usize>)>,
        cursor: usize,
        len: usize,
    },
}
impl Buckets {
    fn new(ring: bool, width: f64, max: f64) -> Self {
        if ring {
            let n = (max / width).ceil() as usize + 2;
            assert!(n <= 2_000_000, "ring window too large");
            Self::Ring {
                slots: (0..n).map(|_| (usize::MAX, Vec::new())).collect(),
                cursor: 0,
                len: 0,
            }
        } else {
            Self::Map(BTreeMap::new())
        }
    }
    fn push(&mut self, i: usize, u: usize) {
        match self {
            Self::Map(b) => b.entry(i).or_default().push(u),
            Self::Ring { slots, len, .. } => {
                let n = slots.len();
                let slot = &mut slots[i % n];
                assert!(slot.1.is_empty() || slot.0 == i, "generation collision");
                slot.0 = i;
                slot.1.push(u);
                *len += 1;
            }
        }
    }
    fn pop(&mut self) -> Option<(usize, Vec<usize>)> {
        match self {
            Self::Map(b) => b.pop_first(),
            Self::Ring { slots, cursor, len } => {
                if *len == 0 {
                    return None;
                }
                loop {
                    let n = slots.len();
                    let x = &mut slots[*cursor % n];
                    if x.0 == *cursor && !x.1.is_empty() {
                        let v = std::mem::take(&mut x.1);
                        *len -= v.len();
                        return Some((*cursor, v));
                    }
                    *cursor += 1;
                }
            }
        }
    }
    fn take(&mut self, i: usize) -> Vec<usize> {
        match self {
            Self::Map(b) => b.remove(&i).unwrap_or_default(),
            Self::Ring { slots, len, .. } => {
                let n = slots.len();
                let x = &mut slots[i % n];
                if x.0 != i {
                    return Vec::new();
                }
                let v = std::mem::take(&mut x.1);
                *len -= v.len();
                v
            }
        }
    }
}
/// Four factorial arms: unchanged-label suppression and heavy-edge deduplication.
pub fn delta<const C: bool>(
    g: &Graph,
    s: usize,
    width: f64,
    max: f64,
    skip: bool,
    dedup: bool,
    ring: bool,
) -> (Vec<f64>, Counts) {
    assert!(s < g.n && width.is_finite() && width > 0.);
    let mut c = Counts::default();
    let mut d = vec![f64::INFINITY; g.n];
    let mut last = vec![f64::INFINITY; g.n];
    let mut mark = vec![usize::MAX; g.n];
    let mut b = Buckets::new(ring, width, max);
    d[s] = 0.;
    b.push(0, s);
    count!(C, c.pushes += 1);
    while let Some((i, mut pending)) = b.pop() {
        count!(C, c.phases += 1);
        let mut touched = Vec::new();
        let mut heavy = Vec::new();
        let phase_start = c.vertex_scans;
        while !pending.is_empty() {
            for u in pending {
                count!(C, c.pops += 1);
                if (d[u] / width) as usize != i {
                    count!(C, c.superseded += 1);
                    continue;
                }
                if last[u] == d[u] {
                    count!(C, c.unchanged += 1);
                    if skip {
                        continue;
                    }
                }
                last[u] = d[u];
                count!(C, c.vertex_scans += 1);
                if mark[u] != i {
                    mark[u] = i;
                    touched.push(u);
                }
                for &(v, w) in &g.adj[u] {
                    count!(C, c.scans += 1);
                    if w <= width {
                        let nd = d[u] + w;
                        if nd < d[v] {
                            d[v] = nd;
                            b.push((nd / width) as usize, v);
                            count!(C, c.pushes += 1);
                            count!(C, c.improvements += 1);
                        }
                    } else if !dedup {
                        heavy.push((u, v, w));
                    }
                }
            }
            pending = b.take(i);
        }
        count!(
            C,
            c.max_phase_scans = c.max_phase_scans.max(c.vertex_scans - phase_start)
        );
        if dedup {
            for u in touched {
                for &(v, w) in &g.adj[u] {
                    count!(C, c.scans += 1);
                    if w > width {
                        count!(C, c.heavy += 1);
                        let nd = d[u] + w;
                        if nd < d[v] {
                            d[v] = nd;
                            b.push((nd / width) as usize, v);
                            count!(C, c.pushes += 1);
                            count!(C, c.improvements += 1);
                        }
                    }
                }
            }
        } else {
            for (u, v, w) in heavy {
                count!(C, c.heavy += 1);
                let nd = d[u] + w;
                if nd < d[v] {
                    d[v] = nd;
                    b.push((nd / width) as usize, v);
                    count!(C, c.pushes += 1);
                    count!(C, c.improvements += 1);
                }
            }
        }
    }
    (d, c)
}

/// Integer-only monotone radix heap, no rounding of floating-weight instances.
pub fn radix(g: &Graph, s: usize) -> Vec<f64> {
    let mut buckets: Vec<Vec<(u64, usize)>> = (0..65).map(|_| Vec::new()).collect();
    let mut last = 0u64;
    let mut len = 1usize;
    let idx = |x: u64, l: u64| {
        if x == l {
            0
        } else {
            64 - (x ^ l).leading_zeros() as usize
        }
    };
    let mut d = vec![u64::MAX; g.n];
    d[s] = 0;
    buckets[0].push((0, s));
    while len > 0 {
        if buckets[0].is_empty() {
            let j = (1..65).find(|&j| !buckets[j].is_empty()).unwrap();
            last = buckets[j].iter().map(|x| x.0).min().unwrap();
            for (x, u) in std::mem::take(&mut buckets[j]) {
                let k = idx(x, last);
                buckets[k].push((x, u));
            }
        }
        let (x, u) = buckets[0].pop().unwrap();
        len -= 1;
        if x != d[u] {
            continue;
        }
        for &(v, w) in &g.adj[u] {
            debug_assert!(w >= 0. && w.fract() == 0.);
            let nd = x.checked_add(w as u64).expect("integer overflow");
            if nd < d[v] {
                d[v] = nd;
                let j = idx(nd, last);
                buckets[j].push((nd, v));
                len += 1;
            }
        }
    }
    d.into_iter()
        .map(|x| {
            if x == u64::MAX {
                f64::INFINITY
            } else {
                x as f64
            }
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct Features {
    pub mean: f64,
    pub median: f64,
    pub max: f64,
    pub sd: f64,
    pub degree: f64,
    pub max_degree: usize,
}
pub fn features(g: &Graph) -> Features {
    let mut w: Vec<f64> = g.adj.iter().flatten().map(|x| x.1).collect();
    assert!(!w.is_empty());
    assert!(w.iter().all(|x| x.is_finite() && *x >= 0.));
    let mean = w.iter().sum::<f64>() / w.len() as f64;
    let sd = (w.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / w.len() as f64).sqrt();
    w.sort_by(f64::total_cmp);
    Features {
        mean,
        median: w[w.len() / 2],
        max: *w.last().unwrap(),
        sd,
        degree: g.m as f64 / g.n as f64,
        max_degree: *g.degree.iter().max().unwrap(),
    }
}
pub fn check(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (i, (&x, &y)) in a.iter().zip(b).enumerate() {
        assert!(
            (x == y)
                || (x.is_finite() && y.is_finite() && (x - y).abs() <= 1e-10 * x.abs().max(1.)),
            "distance mismatch at {i}: {x} {y}"
        );
    }
}
pub fn finish(g: &mut Graph) {
    for a in &mut g.adj {
        a.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
    }
}
pub fn fast_sparse(n: usize, k: usize, seed: u64) -> Graph {
    let mut r = ChaCha8Rng::seed_from_u64(seed);
    let mut g = Graph::new(n);
    for u in 0..n {
        let d = r.gen_range(1..=(2 * k).min(n - 1));
        let mut targets = Vec::with_capacity(d);
        while targets.len() < d {
            let v = r.gen_range(0..n);
            if v != u && !targets.contains(&v) {
                targets.push(v);
            }
        }
        for v in targets {
            g.add_edge(u, v, r.gen_range(1.0..100.0));
        }
    }
    finish(&mut g);
    g
}
pub fn paper_grid(n: usize, seed: u64) -> Graph {
    let rows = (1..=(n as f64).sqrt() as usize)
        .filter(|r| n % r == 0)
        .max()
        .unwrap();
    crate::grid_graph(rows, n / rows, seed)
}
pub fn law(g: &Graph, name: &str, seed: u64) -> Graph {
    let mut h = g.clone();
    let mut r = ChaCha8Rng::seed_from_u64(seed);
    let mut ws: Vec<f64> = g.adj.iter().flatten().map(|x| x.1).collect();
    ws.shuffle(&mut r);
    let mut j = 0;
    for a in &mut h.adj {
        for (_, w) in a {
            *w = match name {
                "near" => 50. + r.gen_range(-0.01..0.01),
                "skew" => 1. + 99. * r.gen::<f64>().powi(8),
                "bimodal" => {
                    if r.gen_bool(0.8) {
                        1.
                    } else {
                        100.
                    }
                }
                "zero" => {
                    if r.gen_bool(0.25) {
                        0.
                    } else {
                        r.gen_range(1.0..100.)
                    }
                }
                "integer" => r.gen_range(1..=100) as f64,
                "shuffle" => ws[j],
                "scale" => *w * 1024.,
                _ => *w,
            };
            j += 1;
        }
    }
    finish(&mut h);
    h
}
/// Relabel by BFS on the undirected support; reversed order is the RCM-style
/// control (degree-sorted neighbor expansion, component roots of least degree).
pub fn reorder_with_map(g: &Graph, reverse: bool) -> (Graph, Vec<usize>) {
    let mut und = vec![Vec::new(); g.n];
    for (u, a) in g.adj.iter().enumerate() {
        for &(v, _) in a {
            und[u].push(v);
            und[v].push(u);
        }
    }
    for a in &mut und {
        a.sort_unstable();
        a.dedup();
    }
    let degrees: Vec<_> = und.iter().map(Vec::len).collect();
    for a in &mut und {
        a.sort_by_key(|&v| (degrees[v], v));
    }
    let mut roots: Vec<_> = (0..g.n).collect();
    roots.sort_by_key(|&v| (degrees[v], v));
    let mut seen = vec![false; g.n];
    let mut order = Vec::new();
    let mut q = VecDeque::new();
    for root in roots {
        if seen[root] {
            continue;
        }
        seen[root] = true;
        q.push_back(root);
        while let Some(u) = q.pop_front() {
            order.push(u);
            for &v in &und[u] {
                if !seen[v] {
                    seen[v] = true;
                    q.push_back(v)
                }
            }
        }
    }
    if reverse {
        order.reverse()
    }
    let mut map = vec![0; g.n];
    for (i, &u) in order.iter().enumerate() {
        map[u] = i
    }
    let mut h = Graph::new(g.n);
    for (u, a) in g.adj.iter().enumerate() {
        for &(v, w) in a {
            h.add_edge(map[u], map[v], w)
        }
    }
    finish(&mut h);
    (h, map)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn planted() -> Graph {
        let mut g = Graph::new(6);
        for (u, v, w) in [
            (0, 1, 1.),
            (0, 2, 2.),
            (0, 3, 9.),
            (1, 3, 2.),
            (2, 3, 3.),
            (3, 4, 100.),
        ] {
            g.add_edge(u, v, w)
        }
        g
    }
    #[test]
    fn duplicate_mechanisms() {
        let g = planted();
        let expected = [0., 1., 2., 3., 103., f64::INFINITY];
        let (a, ca) = delta::<true>(&g, 0, 10., 100., false, false, false);
        let (b, cb) = delta::<true>(&g, 0, 10., 100., true, true, true);
        check(&expected, &a);
        check(&expected, &b);
        assert_eq!(ca.unchanged, 1);
        assert_eq!(ca.heavy, 2);
        assert_eq!(cb.heavy, 1);
        assert!(ca.vertex_scans > cb.vertex_scans);
        let mut removed = g.clone();
        removed.adj[0].retain(|&(v, _)| v != 3);
        let (_, cc) = delta::<true>(&removed, 0, 10., 100., false, false, false);
        assert_eq!(cc.unchanged, 0);
        assert_eq!(cc.heavy, 1);
        let mut improving = g.clone();
        improving.adj[0].reverse();
        let (a, without) = delta::<true>(&improving, 0, 10., 100., true, false, false);
        let (b, with) = delta::<true>(&improving, 0, 10., 100., true, true, false);
        check(&expected, &a);
        check(&expected, &b);
        assert_eq!(without.heavy, 2);
        assert_eq!(with.heavy, 1);
    }
    #[test]
    fn independent_bellman_ford_all_arms() {
        for seed in 0..20 {
            let base = fast_sparse(31, 4, seed);
            for name in ["uniform", "zero", "integer", "bimodal"] {
                let g = law(&base, name, seed);
                let f = features(&g);
                for s in [0, 7, 30] {
                    let mut refd = vec![f64::INFINITY; g.n];
                    refd[s] = 0.;
                    for _ in 1..g.n {
                        for (u, a) in g.adj.iter().enumerate() {
                            for &(v, w) in a {
                                refd[v] = refd[v].min(refd[u] + w)
                            }
                        }
                    }
                    let mask = Mask::predicate(&g, |_, _| true);
                    for d in [
                        heap::<true, 2>(&g, s, None, u64::MAX, None).0,
                        heap::<false, 4>(&g, s, None, u64::MAX, None).0,
                        heap::<true, 2>(&g, s, Some(&mask), u64::MAX, None).0,
                        heap::<true, 2>(&g, s, None, u64::MAX, Some(&incoming(&g))).0,
                    ] {
                        check(&refd, &d)
                    }
                    for width in [f.mean, f.mean / f.degree, 0.1, 1000.] {
                        for skip in [false, true] {
                            for dedup in [false, true] {
                                for ring in [false, true] {
                                    check(
                                        &refd,
                                        &delta::<true>(&g, s, width, f.max, skip, dedup, ring).0,
                                    )
                                }
                            }
                        }
                    }
                    if name == "integer" || name == "bimodal" {
                        check(&refd, &radix(&g, s))
                    }
                }
            }
        }
    }
    #[test]
    fn exact_admission_ties_and_complement() {
        let g = fast_sparse(50, 4, 3);
        let a = Mask::predicate(&g, |u, v| g.degree[u] <= 4 && g.degree[v] <= 4);
        let b = Mask::predicate(&g, |u, v| g.degree[u] > 4 || g.degree[v] > 4);
        assert_eq!(a.admitted + b.admitted, g.m);
        let scores = vec![Some(0.25); g.n];
        for k in [0, 1, 13, g.m] {
            assert_eq!(Mask::ranked(&g, &scores, k, false).admitted, k);
            assert_eq!(Mask::random(&g, k, 7).admitted, k);
        }
    }
    #[test]
    fn zero_cycles_parallel_and_budget() {
        let mut g = planted();
        g.add_edge(3, 2, 0.);
        g.add_edge(2, 3, 0.);
        g.add_edge(0, 1, 2.);
        let r = crate::dijkstra(&g, 0);
        let m = Mask::predicate(&g, |_, _| true);
        let (d, c) = heap::<true, 2>(&g, 0, Some(&m), 2, None);
        check(&r, &d);
        assert!(c.scout_scans <= 2);
        for ring in [false, true] {
            check(&r, &delta::<false>(&g, 0, 0.1, 100., true, true, ring).0)
        }
    }
    #[test]
    fn irrelevant_component_and_scaling() {
        let g = fast_sparse(50, 4, 42);
        let f = features(&g);
        let mut h = Graph::new(52);
        for (u, a) in g.adj.iter().enumerate() {
            for &(v, w) in a {
                h.add_edge(u, v, w)
            }
        }
        h.add_edge(50, 51, 1e9);
        let hf = features(&h);
        assert!(hf.mean > 1000. * f.mean);
        let expected = crate::dijkstra(&g, 0);
        let result = delta::<false>(&h, 0, hf.mean, hf.max, true, true, false).0;
        check(&expected, &result[..50]);
        assert!(result[50].is_infinite() && result[51].is_infinite());
        let scaled = law(&g, "scale", 42);
        let sf = features(&scaled);
        let (a, ca) = delta::<true>(&g, 0, f.mean, f.max, true, true, false);
        let (b, cb) = delta::<true>(&scaled, 0, sf.mean, sf.max, true, true, false);
        check(&a.iter().map(|x| x * 1024.).collect::<Vec<_>>(), &b);
        assert_eq!(
            (ca.pushes, ca.phases, ca.heavy),
            (cb.pushes, cb.phases, cb.heavy)
        );
    }
    #[test]
    fn all_zero_graph() {
        let mut g = Graph::new(3);
        g.add_edge(0, 1, 0.);
        g.add_edge(1, 0, 0.);
        g.add_edge(1, 2, 0.);
        for ring in [false, true] {
            check(
                &[0., 0., 0.],
                &delta::<true>(&g, 0, 1., 0., true, true, ring).0,
            )
        }
    }
}
