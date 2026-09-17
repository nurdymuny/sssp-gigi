//! Two-hop normalized variance with exact edge admission and a weight-permutation null.
use gigi::{
    bundle::BundleStore,
    types::{BundleSchema, FieldDef, Record, Value},
};
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;
use sssp_rust::{revision::*, *};
use std::{
    fs::File,
    io::{BufWriter, Write},
    time::Instant,
};
fn weights(g: &Graph, u: usize) -> Vec<f64> {
    let mut tails = vec![u];
    tails.extend(g.adj[u].iter().map(|x| x.0));
    tails.sort_unstable();
    tails.dedup();
    tails
        .into_iter()
        .flat_map(|v| g.adj[v].iter().map(|x| x.1))
        .collect()
}
fn shape(w: &[f64]) -> Option<f64> {
    if w.len() < 2 {
        return None;
    }
    let lo = w.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = w.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if hi == lo {
        return None;
    }
    let mut mu = 0.;
    let mut m2 = 0.;
    for (i, &x) in w.iter().enumerate() {
        let d = x - mu;
        mu += d / (i + 1) as f64;
        m2 += d * (x - mu);
    }
    Some(m2 / w.len() as f64 / (hi - lo).powi(2))
}
fn engine_shape(w: &[f64]) -> f64 {
    let mut b = BundleStore::new(
        BundleSchema::new("k")
            .base(FieldDef::numeric("edge_id"))
            .fiber(FieldDef::numeric("weight")),
    );
    for (i, &x) in w.iter().enumerate() {
        let mut r = Record::new();
        r.insert("edge_id".into(), Value::Integer(i as i64));
        r.insert("weight".into(), Value::Float(x));
        b.insert(&r);
    }
    gigi::curvature::scalar_curvature(&b)
}
fn field(g: &Graph, engine_all: bool, check_vertex: usize) -> Vec<Option<f64>> {
    (0..g.n)
        .map(|u| {
            let w = weights(g, u);
            let k = shape(&w);
            if let Some(x) = k {
                if engine_all || u == check_vertex {
                    let y = engine_shape(&w);
                    assert!((x - y).abs() < 1e-12, "engine formula mismatch");
                    return Some(y);
                }
            }
            k
        })
        .collect()
}
fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or("results/revision/gigi_gate.csv".into());
    let mut log = BufWriter::new(File::create(out).unwrap());
    writeln!(log,"family,seed,source,rate,arm,null_seed,admitted,potential_work,pushes,pops,comparisons,scouts,scout_scans,scout_yield,field_ns").unwrap();
    for w in [
        vec![1., 100.],
        vec![49., 51.],
        vec![1., 50.5, 100.],
        vec![49., 50., 51.],
    ] {
        assert!((engine_shape(&w) - shape(&w).unwrap()).abs() < 1e-12);
    }
    for seed in [42, 1729, 2026] {
        for family in ["sparse", "clustered", "roadlike"] {
            let g = match family {
                "sparse" => random_sparse_graph(1000, 4, seed),
                "clustered" => clustered_graph(20, 50, 0.5, 2, seed),
                _ => road_network_like(1000, seed),
            };
            let t = Instant::now();
            let actual = field(&g, true, 0);
            let field_ns = t.elapsed().as_nanos();
            let refs = [dijkstra(&g, 333), dijkstra(&g, 666)];
            let mut emit = |name: &str, ns: i32, k: &[Option<f64>], high: bool, rate: f64| {
                let mask = if name == "never" {
                    Mask::predicate(&g, |_, _| false)
                } else {
                    Mask::ranked(&g, k, (rate * g.m as f64).floor() as usize, high)
                };
                for (j, s) in [333, 666].into_iter().enumerate() {
                    let (d, c) = heap::<true, 2>(&g, s, Some(&mask), u64::MAX, None);
                    check(&refs[j], &d);
                    writeln!(
                        log,
                        "{family},{seed},{s},{rate},{name},{ns},{},{},{},{},{},{},{},{},{field_ns}",
                        mask.admitted,
                        mask.potential_work,
                        c.pushes,
                        c.pops,
                        c.comparisons,
                        c.scouts,
                        c.scout_scans,
                        c.scout_yield
                    )
                    .unwrap();
                }
            };
            for rate in [0.05, 0.1, 0.25, 0.5] {
                emit("never", -1, &actual, false, rate);
                emit("low", -1, &actual, false, rate);
                emit("high", -1, &actual, true, rate);
            }
            for ns in 0..100 {
                let mut rng = ChaCha8Rng::seed_from_u64(seed * 1000 + ns);
                let mut perm = actual.clone();
                perm.shuffle(&mut rng);
                let mut h = g.clone();
                let mut ws: Vec<_> = g.adj.iter().flatten().map(|x| x.1).collect();
                ws.shuffle(&mut rng);
                for (e, w) in h.adj.iter_mut().flatten().zip(ws) {
                    e.1 = w;
                }
                let null = field(&h, false, (ns as usize * 997) % g.n);
                for rate in [0.05, 0.1, 0.25, 0.5] {
                    emit("vertex_permutation", ns as i32, &perm, false, rate);
                    emit("weight_permutation", ns as i32, &null, false, rate);
                }
            }
            log.flush().unwrap();
            println!("K controls complete: {family} seed={seed}");
        }
    }
}
