use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;
use sssp_rust::{revision::*, *};
use std::{
    fs::{create_dir_all, File},
    hint::black_box,
    io::{BufRead, BufReader, BufWriter, Write},
    time::Instant,
};

#[derive(Clone)]
enum Method {
    AuthorHeap,
    Heap2,
    Heap4,
    Original,
    Delta(f64, bool, bool, bool),
    Scout(Mask, u64),
    Caliber(Vec<f64>),
    Radix,
}
#[derive(Clone)]
struct Arm {
    name: String,
    method: Method,
    prep: u128,
}
fn solve<const C: bool>(g: &Graph, s: usize, f: &Features, a: &Arm) -> (Vec<f64>, Counts) {
    match &a.method {
        Method::AuthorHeap => (dijkstra(g, s), Counts::default()),
        Method::Heap2 => heap::<C, 2>(g, s, None, u64::MAX, None),
        Method::Heap4 => heap::<C, 4>(g, s, None, u64::MAX, None),
        Method::Original => (davis_delta(g, s), Counts::default()),
        Method::Delta(w, skip, dd, ring) => delta::<C>(g, s, *w, f.max, *skip, *dd, *ring),
        Method::Scout(m, b) => heap::<C, 2>(g, s, Some(m), *b, None),
        Method::Caliber(inc) => heap::<C, 2>(g, s, None, u64::MAX, Some(inc)),
        Method::Radix => (radix(g, s), Counts::default()),
    }
}
fn arm(name: &str, method: Method, prep: u128) -> Arm {
    Arm {
        name: name.into(),
        method,
        prep,
    }
}
struct Log {
    samples: BufWriter<File>,
    counts: BufWriter<File>,
    cases: BufWriter<File>,
    selection: BufWriter<File>,
    case: usize,
}
impl Log {
    fn new(path: &str) -> Self {
        create_dir_all(path).unwrap();
        let mut l = Self {
            samples: BufWriter::new(File::create(format!("{path}/samples.csv")).unwrap()),
            counts: BufWriter::new(File::create(format!("{path}/counts.csv")).unwrap()),
            cases: BufWriter::new(File::create(format!("{path}/cases.csv")).unwrap()),
            selection: BufWriter::new(File::create(format!("{path}/selection.csv")).unwrap()),
            case: 0,
        };
        writeln!(
            l.samples,
            "case,stage,family,n,m,seed,source,arm,round,position,ns,width,prep_ns"
        )
        .unwrap();
        writeln!(l.counts,"case,source,arm,pushes,pops,comparisons,peak,scans,scouts,scout_scans,scout_yield,improvements,superseded,phases,vertex_scans,unchanged,heavy,max_phase_scans,admitted,potential_work,budget").unwrap();
        writeln!(l.cases,"case,stage,family,n,m,seed,mean,median,max,sd,degree,max_degree,generation_ns,feature_ns,fingerprint").unwrap();
        writeln!(
            l.selection,
            "case,training_source,multiplier,median_ns,total_calibration_ns,selected"
        )
        .unwrap();
        l
    }
    fn run(
        &mut self,
        stage: &str,
        family: &str,
        g: &Graph,
        seed: u64,
        generation: u128,
        sources: &[usize],
        rounds: usize,
        tune: bool,
        extra: bool,
    ) {
        self.case += 1;
        let id = self.case;
        let t = Instant::now();
        let f = features(g);
        let feature = t.elapsed().as_nanos();
        let mut hash = 0xcbf29ce484222325u64;
        for (u, a) in g.adj.iter().enumerate() {
            for &(v, w) in a {
                for x in [u as u64, v as u64, w.to_bits()] {
                    hash = (hash ^ x).wrapping_mul(0x100000001b3)
                }
            }
        }
        writeln!(self.cases,"{id},{stage},{family},{},{},{seed},{:.15},{:.15},{:.15},{:.15},{:.9},{},{generation},{feature},{hash:016x}",g.n,g.m,f.mean,f.median,f.max,f.sd,f.degree,f.max_degree).unwrap();
        self.cases.flush().unwrap();
        let mean = f.mean.max(f64::MIN_POSITIVE);
        let prep = |which: &str| -> u128 {
            let mut ns = Vec::new();
            for _ in 0..7 {
                let t = Instant::now();
                match which {
                    "mean" => {
                        black_box(g.adj.iter().flatten().map(|x| x.1).sum::<f64>() / g.m as f64);
                    }
                    "median" => {
                        let mut w: Vec<_> = g.adj.iter().flatten().map(|x| x.1).collect();
                        w.sort_by(f64::total_cmp);
                        black_box(w[w.len() / 2]);
                    }
                    "max" => {
                        black_box(g.adj.iter().flatten().map(|x| x.1).fold(0., f64::max));
                        black_box(g.degree.iter().max());
                    }
                    "sd" => {
                        black_box(features(g));
                    }
                    _ => {}
                }
                ns.push(t.elapsed().as_nanos());
            }
            ns.sort_unstable();
            ns[3]
        };
        let mp = prep("mean");
        let mut arms = vec![
            arm("std_binary", Method::AuthorHeap, 0),
            arm("binary", Method::Heap2, 0),
            arm("fourary", Method::Heap4, 0),
            arm("mean_map", Method::Delta(mean, true, true, false), mp),
            arm("mean_ring", Method::Delta(mean, true, true, true), mp),
            arm(
                "mean_degree",
                Method::Delta(mean / f.degree.max(1.), true, true, false),
                mp,
            ),
            arm(
                "max_maxdegree",
                Method::Delta(f.max / f.max_degree.max(1) as f64, true, true, false),
                prep("max"),
            ),
        ];
        if extra {
            arms.extend([
                arm("original_mean", Method::Original, 0),
                arm("list_both", Method::Delta(mean, false, false, false), mp),
                arm("skip_only", Method::Delta(mean, true, false, false), mp),
                arm("heavy_only", Method::Delta(mean, false, true, false), mp),
                arm(
                    "mean_sqrtdegree",
                    Method::Delta(mean / f.degree.max(1.).sqrt(), true, true, false),
                    mp,
                ),
                arm(
                    "median",
                    Method::Delta(f.median.max(mean * 1e-6), true, true, false),
                    prep("median"),
                ),
                arm(
                    "sd",
                    Method::Delta(f.sd.max(mean * 1e-6), true, true, false),
                    prep("sd"),
                ),
            ]);
        }
        let t = Instant::now();
        let inc = incoming(g);
        let ip = t.elapsed().as_nanos();
        arms.push(arm("caliber", Method::Caliber(inc), ip));
        let t = Instant::now();
        let mask = Mask::predicate(g, |u, v| g.degree[u] <= 4 && g.degree[v] <= 4);
        let gateprep = t.elapsed().as_nanos();
        arms.push(arm(
            "degree4",
            Method::Scout(mask.clone(), u64::MAX),
            gateprep,
        ));
        if g.adj.iter().flatten().all(|x| x.1.fract() == 0.) {
            assert!(f.max * (g.n as f64) < (1u64 << 53) as f64);
            arms.push(arm("radix", Method::Radix, 0));
        }
        if tune {
            let multipliers = [
                0.03125, 0.05, 0.1, 0.25, 0.5, 0.75, 1., 1.5, 2., 4., 8., 16.,
            ];
            let mut ns = vec![Vec::new(); multipliers.len()];
            let training = 0;
            let expected = dijkstra(g, training);
            for &x in &multipliers {
                check(
                    &expected,
                    &delta::<false>(g, training, mean * x, f.max, true, true, false).0,
                );
            }
            let t_all = Instant::now();
            let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0xabcd);
            let mut order: Vec<_> = (0..multipliers.len()).collect();
            for round in 0..6 {
                if round % 2 == 0 {
                    order.shuffle(&mut rng)
                } else {
                    order.reverse()
                }
                for (pos, &j) in order.iter().enumerate() {
                    let t = Instant::now();
                    let r = delta::<false>(
                        black_box(g),
                        training,
                        mean * multipliers[j],
                        f.max,
                        true,
                        true,
                        false,
                    )
                    .0;
                    black_box(&r);
                    let elapsed = t.elapsed().as_nanos();
                    ns[j].push(elapsed);
                    writeln!(self.samples,"{id},calibration,{family},{},{},{seed},{training},grid_{},{round},{pos},{elapsed},{},{}",g.n,g.m,multipliers[j],mean*multipliers[j],mp).unwrap();
                }
            }
            let calibration = t_all.elapsed().as_nanos();
            let med: Vec<_> = ns
                .iter_mut()
                .map(|x| {
                    x.sort_unstable();
                    (x[2] + x[3]) / 2
                })
                .collect();
            let best = (0..med.len()).min_by_key(|&i| med[i]).unwrap();
            for (j, &x) in multipliers.iter().enumerate() {
                writeln!(
                    self.selection,
                    "{id},{training},{x},{},{calibration},{}",
                    med[j],
                    j == best
                )
                .unwrap();
            }
            arms.push(arm(
                "selected_grid",
                Method::Delta(mean * multipliers[best], true, true, false),
                mp + calibration,
            ));
        }
        for &s in sources {
            let expected = dijkstra(g, s);
            for a in &arms {
                let (d, c) = solve::<true>(g, s, &f, a);
                check(&expected, &d);
                self.record(id, s, a, &c);
                check(&expected, &solve::<false>(g, s, &f, a).0);
            }
            // Mechanism arms are counted separately; exact graph admission is not an
            // assertion about equal executed opportunities or equal work.
            if stage == "core" || stage == "gate" {
                let gated = heap::<true, 2>(g, s, Some(&mask), u64::MAX, None).1;
                let tau = 3usize.max((f.degree / 2.) as usize);
                let mut controls = vec![
                    arm(
                        "always",
                        Method::Scout(Mask::predicate(g, |_, _| true), u64::MAX),
                        0,
                    ),
                    arm(
                        "high_high",
                        Method::Scout(
                            Mask::predicate(g, |u, v| g.degree[u] > 4 && g.degree[v] > 4),
                            u64::MAX,
                        ),
                        0,
                    ),
                    arm(
                        "complement",
                        Method::Scout(
                            Mask::predicate(g, |u, v| g.degree[u] > 4 || g.degree[v] > 4),
                            u64::MAX,
                        ),
                        0,
                    ),
                    arm(
                        "tail_only",
                        Method::Scout(Mask::predicate(g, |u, _| g.degree[u] <= 4), u64::MAX),
                        0,
                    ),
                    arm(
                        "head_only",
                        Method::Scout(Mask::predicate(g, |_, v| g.degree[v] <= 4), u64::MAX),
                        0,
                    ),
                    arm(
                        "adaptive",
                        Method::Scout(
                            Mask::predicate(g, |u, v| g.degree[u] <= tau && g.degree[v] <= tau),
                            u64::MAX,
                        ),
                        0,
                    ),
                ];
                for rs in 0..5 {
                    let m = Mask::random(g, mask.admitted, seed * 100 + rs);
                    controls.push(arm(
                        &format!("random_{rs}"),
                        Method::Scout(m.clone(), u64::MAX),
                        0,
                    ));
                    controls.push(arm(
                        &format!("budget_random_{rs}"),
                        Method::Scout(m, gated.scout_scans),
                        0,
                    ));
                }
                for a in &controls {
                    let (d, c) = solve::<true>(g, s, &f, a);
                    check(&expected, &d);
                    self.record(id, s, a, &c);
                }
            }
            // Each pair of rounds uses the same permutation and its reverse.
            // Thus every arm occupies complementary positions in adjacent rounds.
            let mut rng = ChaCha8Rng::seed_from_u64(seed ^ s as u64 ^ 0xf00d);
            let mut order: Vec<_> = (0..arms.len()).collect();
            for round in 0..rounds {
                if round % 2 == 0 {
                    order.shuffle(&mut rng)
                } else {
                    order.reverse()
                }
                for (pos, &j) in order.iter().enumerate() {
                    let a = &arms[j];
                    let t = Instant::now();
                    let r = solve::<false>(black_box(g), black_box(s), &f, a).0;
                    black_box(&r);
                    let ns = t.elapsed().as_nanos();
                    let width = match a.method {
                        Method::Delta(w, _, _, _) => w,
                        Method::Original => mean,
                        _ => 0.,
                    };
                    writeln!(
                        self.samples,
                        "{id},{stage},{family},{},{},{seed},{s},{},{round},{pos},{ns},{width},{}",
                        g.n, g.m, a.name, a.prep
                    )
                    .unwrap();
                }
            }
        }
        self.samples.flush().unwrap();
        self.counts.flush().unwrap();
        self.selection.flush().unwrap();
        println!(
            "done case {id}: {stage} {family} n={} m={} seed={seed}",
            g.n, g.m
        );
    }
    fn record(&mut self, id: usize, s: usize, a: &Arm, c: &Counts) {
        let (admitted, work, budget) = match &a.method {
            Method::Scout(m, b) => (m.admitted, m.potential_work, *b),
            _ => (0, 0, 0),
        };
        writeln!(
            self.counts,
            "{id},{s},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{admitted},{work},{budget}",
            a.name,
            c.pushes,
            c.pops,
            c.comparisons,
            c.peak,
            c.scans,
            c.scouts,
            c.scout_scans,
            c.scout_yield,
            c.improvements,
            c.superseded,
            c.phases,
            c.vertex_scans,
            c.unchanged,
            c.heavy,
            c.max_phase_scans
        )
        .unwrap();
    }
}
fn dimacs(path: &str) -> Graph {
    let file = BufReader::new(File::open(path).unwrap());
    let mut g = None;
    let mut expected = 0;
    for line in file.lines() {
        let line = line.unwrap();
        let p: Vec<_> = line.split_whitespace().collect();
        if p.is_empty() {
            continue;
        }
        match p[0] {
            "p" => {
                g = Some(Graph::new(p[2].parse().unwrap()));
                expected = p[3].parse().unwrap()
            }
            "a" => {
                let h = g.as_mut().unwrap();
                h.add_edge(
                    p[1].parse::<usize>().unwrap() - 1,
                    p[2].parse::<usize>().unwrap() - 1,
                    p[3].parse().unwrap(),
                );
            }
            _ => {}
        }
    }
    let mut g = g.unwrap();
    assert_eq!(g.m, expected);
    finish(&mut g);
    g
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let stage = args.get(1).map(String::as_str).unwrap_or("core");
    let out = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("results/revision/core");
    let mut log = Log::new(out);
    match stage {
        "smoke" => {
            let g = fast_sparse(100, 4, 42);
            log.run(stage, "sparse_fast", &g, 42, 0, &[0, 33], 2, true, true);
        }
        "core" => {
            for seed in [42, 1729, 2026] {
                for family in ["sparse", "clustered", "dense", "grid", "roadlike"] {
                    for n in [1000, 2000, 5000, 10000] {
                        if (family == "dense" || family == "roadlike") && n > 5000 {
                            continue;
                        }
                        let t = Instant::now();
                        let g = match family {
                            "sparse" => random_sparse_graph(n, 4, seed),
                            "clustered" => clustered_graph(n / 50, 50, 0.5, 2, seed),
                            "dense" => random_dense_graph(n, 0.1, seed),
                            "grid" => paper_grid(n, seed),
                            _ => road_network_like(n, seed),
                        };
                        let gen = t.elapsed().as_nanos();
                        log.run(
                            stage,
                            family,
                            &g,
                            seed,
                            gen,
                            &[n / 3, 2 * n / 3],
                            12,
                            family == "sparse" || family == "clustered" || family == "dense",
                            true,
                        );
                    }
                }
            }
        }
        "density" => {
            for seed in [42, 1729, 2026] {
                for n in [1000, 5000] {
                    for degree in [2, 4, 8, 16, 32, 64, 128, 256] {
                        let t = Instant::now();
                        let g = random_dense_graph(n, degree as f64 / (n - 1) as f64, seed);
                        log.run(
                            stage,
                            &format!("er_d{degree}"),
                            &g,
                            seed,
                            t.elapsed().as_nanos(),
                            &[n / 3, 2 * n / 3],
                            8,
                            true,
                            false,
                        );
                    }
                }
            }
        }
        "laws" => {
            for seed in [42, 1729, 2026] {
                for family in ["sparse", "clustered"] {
                    let base = if family == "sparse" {
                        random_sparse_graph(2000, 4, seed)
                    } else {
                        clustered_graph(40, 50, 0.5, 2, seed)
                    };
                    for name in [
                        "uniform", "near", "skew", "bimodal", "zero", "integer", "shuffle", "scale",
                    ] {
                        let t = Instant::now();
                        let g = law(&base, name, seed);
                        log.run(
                            stage,
                            &format!("{family}_{name}"),
                            &g,
                            seed,
                            t.elapsed().as_nanos(),
                            &[667, 1333],
                            8,
                            true,
                            false,
                        );
                    }
                }
            }
        }
        "scale" => {
            for seed in [42, 1729, 2026] {
                for n in [10000, 100000, 1000000] {
                    let t = Instant::now();
                    let g = fast_sparse(n, 4, seed);
                    log.run(
                        stage,
                        "sparse_fast",
                        &g,
                        seed,
                        t.elapsed().as_nanos(),
                        &[n / 3, 2 * n / 3],
                        6,
                        false,
                        false,
                    );
                }
            }
        }
        "roads" => {
            let data = args.get(3).expect("data directory");
            for name in ["NY", "BAY", "COL"] {
                let t = Instant::now();
                let g = dimacs(&format!("{data}/USA-road-d.{name}.gr"));
                log.run(
                    stage,
                    name,
                    &g,
                    0,
                    t.elapsed().as_nanos(),
                    &[
                        g.n / 7,
                        2 * g.n / 7,
                        3 * g.n / 7,
                        4 * g.n / 7,
                        5 * g.n / 7,
                        6 * g.n / 7,
                    ],
                    8,
                    true,
                    false,
                );
            }
        }
        "layout" => {
            for seed in [42, 1729, 2026] {
                for n in [5000, 100000] {
                    let base = fast_sparse(n, 4, seed);
                    for name in ["weight", "destination", "random", "bfs", "rcm"] {
                        let t = Instant::now();
                        let (mut g, map) = match name {
                            "bfs" => reorder_with_map(&base, false),
                            "rcm" => reorder_with_map(&base, true),
                            _ => (base.clone(), (0..n).collect()),
                        };
                        if name == "destination" {
                            for a in &mut g.adj {
                                a.sort_by_key(|x| x.0)
                            }
                        }
                        if name == "random" {
                            let mut r = ChaCha8Rng::seed_from_u64(seed);
                            for a in &mut g.adj {
                                a.shuffle(&mut r)
                            }
                        }
                        let elapsed = t.elapsed().as_nanos();
                        for s in [n / 3, 2 * n / 3] {
                            let old = dijkstra(&base, s);
                            let new = dijkstra(&g, map[s]);
                            for v in 0..n {
                                check(&[old[v]], &[new[map[v]]]);
                            }
                        }
                        log.run(
                            stage,
                            name,
                            &g,
                            seed,
                            elapsed,
                            &[map[n / 3], map[2 * n / 3]],
                            8,
                            false,
                            false,
                        );
                    }
                }
            }
        }
        _ => panic!("unknown stage"),
    }
}
