//! Edge identity, isolated vertices, snapshot/mmap reconstruction, and circulation.
use gigi::{
    engine::Engine,
    types::{BundleSchema, FieldDef, Record, Value},
};
use sssp_rust::{dijkstra, revision::*, Graph};
use std::{path::PathBuf, time::Instant};
fn row(id: usize, u: usize, v: usize, w: f64) -> Record {
    let mut r = Record::new();
    for (k, x) in [("edge_id", id), ("vertex_a", u), ("vertex_b", v)] {
        r.insert(k.into(), Value::Integer(x as i64));
    }
    r.insert("weight".into(), Value::Float(w));
    r
}
fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("fresh output directory"));
    assert!(!root.exists(), "use a fresh directory");
    std::fs::create_dir_all(&root).unwrap();
    let edges = [
        (0, 1, 7.),
        (0, 1, 2.),
        (1, 2, 3.),
        (0, 2, 9.),
        (2, 3, 0.),
        (3, 1, 1.),
    ];
    let mut g = Graph::new(5);
    for (u, v, w) in edges {
        g.add_edge(u, v, w)
    }
    let reference = dijkstra(&g, 0);
    check(&[0., 2., 5., 5., f64::INFINITY], &reference);
    let t = Instant::now();
    {
        let mut e = Engine::open(&root).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(BundleSchema::new("vertices").base(FieldDef::numeric("vertex_id")))
            .unwrap();
        e.create_bundle(
            BundleSchema::new("edges")
                .base(FieldDef::numeric("edge_id"))
                .base(FieldDef::numeric("vertex_a"))
                .base(FieldDef::numeric("vertex_b"))
                .fiber(FieldDef::numeric("weight")),
        )
        .unwrap();
        for u in 0..5 {
            let mut r = Record::new();
            r.insert("vertex_id".into(), Value::Integer(u));
            e.insert("vertices", &r).unwrap();
        }
        for (i, &(u, v, w)) in edges.iter().enumerate() {
            e.insert("edges", &row(i, u, v, w)).unwrap();
        }
        e.add_index("edges", "vertex_a").unwrap();
        {
            let b = e.bundle("edges").unwrap();
            let h = b.as_heap().unwrap();
            assert_eq!(
                h.field_bitmap("vertex_a", &Value::Integer(0))
                    .unwrap()
                    .len(),
                3
            );
        }
        let flow = gigi::ml::circulation::circulation_flow(
            &e,
            "edges",
            "vertex_a",
            "vertex_b",
            Some("weight".into()),
            1.,
            10,
        )
        .unwrap();
        println!(
            "circulation measured_vertices={} potential={:?} notes={:?}",
            flow.n_nodes, flow.potential, flow.notes
        );
        assert_eq!(flow.n_nodes, 4);
        e.snapshot().unwrap();
    }
    let stored = t.elapsed();
    let t = Instant::now();
    {
        let e = Engine::open_mmap(&root).unwrap();
        let vertices = e.bundle("vertices").unwrap();
        assert_eq!(vertices.len(), 5);
        let mut ids: Vec<_> = vertices
            .records()
            .map(|r| r["vertex_id"].as_f64().unwrap() as usize)
            .collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![0, 1, 2, 3, 4]);
        let b = e.bundle("edges").unwrap();
        assert_eq!(b.len(), edges.len());
        let mut restored = vec![None; edges.len()];
        let mut h = Graph::new(5);
        for r in b.records() {
            let id = r["edge_id"].as_f64().unwrap() as usize;
            let u = r["vertex_a"].as_f64().unwrap() as usize;
            let v = r["vertex_b"].as_f64().unwrap() as usize;
            let w = r["weight"].as_f64().unwrap();
            assert!(restored[id].is_none());
            restored[id] = Some((u, v, w));
            h.add_edge(u, v, w);
        }
        for (i, x) in restored.iter().enumerate() {
            assert_eq!(*x, Some(edges[i]));
        }
        for s in 0..5 {
            check(&dijkstra(&g, s), &dijkstra(&h, s));
        }
        println!("PASS all edge identities/weights, parallel arcs, isolated vertex, and all-source distances after mmap reload");
    }
    println!(
        "ingest_index_circulation_snapshot_ns={} reopen_extract_verify_ns={}",
        stored.as_nanos(),
        t.elapsed().as_nanos()
    );
}
