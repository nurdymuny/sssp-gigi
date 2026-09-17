//! Measured circulation relabeling, using exact distance buckets throughout.
use gigi::{
    engine::Engine,
    types::{BundleSchema, FieldDef, Record, Value},
};
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;
use sssp_rust::{revision::*, *};
use std::{
    fs::File,
    hint::black_box,
    io::{BufWriter, Write},
    path::PathBuf,
    time::Instant,
};
fn permute(g: &Graph, order: &[usize]) -> (Graph, Vec<usize>) {
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
fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("fresh directory"));
    assert!(!root.exists());
    std::fs::create_dir_all(&root).unwrap();
    let mut out = BufWriter::new(File::create(root.join("samples.csv")).unwrap());
    writeln!(
        out,
        "family,seed,source,layout,arm,round,position,ns,prep_ns,ingest_ns"
    )
    .unwrap();
    let mut refusals = BufWriter::new(File::create(root.join("responses.txt")).unwrap());
    for seed in [42, 1729, 2026] {
        for family in ["sparse", "clustered", "grid"] {
            let g = match family {
                "sparse" => random_sparse_graph(1000, 4, seed),
                "clustered" => clustered_graph(20, 50, 0.5, 2, seed),
                _ => paper_grid(1000, seed),
            };
            let f = features(&g);
            let t = Instant::now();
            let mut engine = Engine::open(&root.join(format!("{family}_{seed}"))).unwrap();
            engine.compaction_policy_mut().disabled = true;
            engine
                .create_bundle(
                    BundleSchema::new("edges")
                        .base(FieldDef::numeric("edge_id"))
                        .base(FieldDef::numeric("vertex_a"))
                        .base(FieldDef::numeric("vertex_b"))
                        .fiber(FieldDef::numeric("weight")),
                )
                .unwrap();
            let mut id = 0;
            for (u, a) in g.adj.iter().enumerate() {
                for &(v, w) in a {
                    let mut r = Record::new();
                    for (k, x) in [("edge_id", id), ("vertex_a", u), ("vertex_b", v)] {
                        r.insert(k.into(), Value::Integer(x as i64));
                    }
                    r.insert("weight".into(), Value::Float(w));
                    engine.insert("edges", &r).unwrap();
                    id += 1;
                }
            }
            let ingest = t.elapsed().as_nanos();
            let mut layouts = vec![("identity", g.clone(), (0..g.n).collect::<Vec<_>>(), 0u128)];
            for (name, rev) in [("bfs", false), ("rcm", true)] {
                let t = Instant::now();
                let (h, map) = reorder_with_map(&g, rev);
                layouts.push((name, h, map, t.elapsed().as_nanos()));
            }
            let t = Instant::now();
            let result = gigi::ml::circulation::circulation_flow(
                &engine,
                "edges",
                "vertex_a",
                "vertex_b",
                Some("weight".into()),
                1.,
                10,
            );
            match result {
                Ok(flow) => {
                    writeln!(
                        refusals,
                        "{family} {seed}: nodes={} edges={} notes={:?}",
                        flow.n_nodes, flow.n_edges, flow.notes
                    )
                    .unwrap();
                    let mut phi = vec![0.; g.n];
                    for (label, x) in flow.potential {
                        phi[label.parse::<usize>().unwrap()] = x;
                    }
                    let mut order: Vec<_> = (0..g.n).collect();
                    order.sort_by(|&a, &b| phi[a].total_cmp(&phi[b]).then(a.cmp(&b)));
                    let (h, map) = permute(&g, &order);
                    layouts.push(("potential", h, map, t.elapsed().as_nanos()));
                }
                Err((status, message)) => {
                    writeln!(
                        refusals,
                        "{family} {seed}: status={status} message={message}"
                    )
                    .unwrap();
                    if family == "grid" {
                        assert_eq!(status.as_u16(), 422)
                    } else {
                        panic!("unexpected refusal: {message}")
                    }
                }
            }
            for s in [333, 666] {
                let expected = dijkstra(&g, s);
                for (_, h, map, _) in &layouts {
                    let d = delta::<false>(h, map[s], f.mean, f.max, true, true, true).0;
                    for v in 0..g.n {
                        check(&[expected[v]], &[d[map[v]]]);
                    }
                    check(&d, &heap::<false, 4>(h, map[s], None, u64::MAX, None).0);
                }
                let mut order: Vec<_> = (0..layouts.len() * 2).collect();
                let mut rng = ChaCha8Rng::seed_from_u64(seed + s as u64);
                for round in 0..12 {
                    if round % 2 == 0 {
                        order.shuffle(&mut rng)
                    } else {
                        order.reverse()
                    };
                    for (pos, &j) in order.iter().enumerate() {
                        let (name, h, map, prep) = &layouts[j / 2];
                        let t = Instant::now();
                        let d = if j % 2 == 0 {
                            delta::<false>(black_box(h), map[s], f.mean, f.max, true, true, true).0
                        } else {
                            heap::<false, 4>(black_box(h), map[s], None, u64::MAX, None).0
                        };
                        black_box(&d);
                        let ns = t.elapsed().as_nanos();
                        writeln!(
                            out,
                            "{family},{seed},{s},{name},{},{round},{pos},{ns},{prep},{ingest}",
                            if j % 2 == 0 { "mean_ring" } else { "fourary" }
                        )
                        .unwrap();
                    }
                }
            }
            out.flush().unwrap();
            refusals.flush().unwrap();
            println!("potential control complete {family} {seed}");
        }
    }
}
