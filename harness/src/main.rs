//! SSSP Benchmark in Rust
//! Comparing: Dijkstra vs Manifold (Davis) approach
//! 
//! This eliminates Python overhead to test pure algorithmic differences.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::time::Instant;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

// =============================================================================
// GRAPH REPRESENTATION
// =============================================================================

#[derive(Clone)]
struct Graph {
    n: usize,
    m: usize,
    adj: Vec<Vec<(usize, f64)>>,  // adj[u] = [(v, weight), ...]
    degree: Vec<usize>,           // Pre-computed out-degrees
}

impl Graph {
    fn new(n: usize) -> Self {
        Graph {
            n,
            m: 0,
            adj: vec![Vec::new(); n],
            degree: vec![0; n],
        }
    }
    
    fn add_edge(&mut self, u: usize, v: usize, w: f64) {
        self.adj[u].push((v, w));
        self.degree[u] += 1;
        self.m += 1;
    }
    
    /// Finalize graph - sort adjacency lists by weight for better cache locality
    fn finalize(&mut self) {
        for adj in &mut self.adj {
            adj.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        }
    }
}

// Priority queue entry (reversed for min-heap behavior)
#[derive(Clone, Copy)]
struct State {
    dist: f64,
    node: usize,
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        // Use bit representation for exact f64 comparison (handles NaN consistently)
        self.dist.to_bits() == other.dist.to_bits() && self.node == other.node
    }
}

impl Eq for State {}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reversed for min-heap
        // Handle NaN: treat NaN as greater than everything (push to back)
        match (self.dist.is_nan(), other.dist.is_nan()) {
            (true, true) => self.node.cmp(&other.node),
            (true, false) => Ordering::Less,  // self (NaN) goes to back
            (false, true) => Ordering::Greater, // other (NaN) goes to back
            (false, false) => other.dist.partial_cmp(&self.dist).unwrap()
                .then_with(|| other.node.cmp(&self.node))
        }
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// =============================================================================
// BUCKET QUEUE for Dial's Algorithm
// =============================================================================

/// Bucket queue for integer-weighted SSSP (Dial's Algorithm)
/// Time: O(m + nC) where C = max edge weight
struct BucketQueue {
    buckets: Vec<Vec<usize>>,
    current_bucket: usize,
    size: usize,
}

impl BucketQueue {
    fn new(num_buckets: usize) -> Self {
        BucketQueue {
            buckets: vec![Vec::new(); num_buckets],
            current_bucket: 0,
            size: 0,
        }
    }
    
    fn push(&mut self, node: usize, priority: usize) {
        let bucket_idx = priority % self.buckets.len();
        self.buckets[bucket_idx].push(node);
        self.size += 1;
    }
    
    fn pop(&mut self) -> Option<usize> {
        if self.size == 0 {
            return None;
        }
        
        // Find next non-empty bucket
        let num_buckets = self.buckets.len();
        for _ in 0..num_buckets {
            if !self.buckets[self.current_bucket].is_empty() {
                self.size -= 1;
                return self.buckets[self.current_bucket].pop();
            }
            self.current_bucket = (self.current_bucket + 1) % num_buckets;
        }
        None
    }
    
    fn is_empty(&self) -> bool {
        self.size == 0
    }
}

// =============================================================================
// GRAPH GENERATORS
// =============================================================================

fn random_sparse_graph(n: usize, avg_degree: usize, seed: u64) -> Graph {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut graph = Graph::new(n);
    
    for u in 0..n {
        // BUG FIX: use (1..=avg_degree*2) but cap at n-1
        let max_edges = (avg_degree * 2).min(n - 1);
        let num_edges = if max_edges > 0 { rng.gen_range(1..=max_edges) } else { 0 };
        
        // BUG FIX: Sample without self-loops
        let mut targets: Vec<usize> = (0..n).filter(|&v| v != u).collect();
        targets.shuffle(&mut rng);
        
        for &v in targets.iter().take(num_edges) {
            let w = rng.gen_range(1.0..100.0);
            graph.add_edge(u, v, w);
        }
    }
    graph.finalize();
    graph
}

fn random_dense_graph(n: usize, density: f64, seed: u64) -> Graph {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut graph = Graph::new(n);
    
    // BUG FIX: Clamp density to [0, 1]
    let density = density.clamp(0.0, 1.0);
    
    for u in 0..n {
        for v in 0..n {
            if u != v && rng.gen::<f64>() < density {
                let w = rng.gen_range(1.0..100.0);
                graph.add_edge(u, v, w);
            }
        }
    }
    graph.finalize();
    graph
}

fn grid_graph(rows: usize, cols: usize, seed: u64) -> Graph {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    
    // BUG FIX: Handle edge cases
    if rows == 0 || cols == 0 {
        return Graph::new(0);
    }
    
    let n = rows * cols;
    let mut graph = Graph::new(n);
    
    let idx = |r: usize, c: usize| r * cols + c;
    
    for r in 0..rows {
        for c in 0..cols {
            let u = idx(r, c);
            // Right
            if c + 1 < cols {
                let w = rng.gen_range(1.0..10.0);
                graph.add_edge(u, idx(r, c + 1), w);
                graph.add_edge(idx(r, c + 1), u, w);
            }
            // Down
            if r + 1 < rows {
                let w = rng.gen_range(1.0..10.0);
                graph.add_edge(u, idx(r + 1, c), w);
                graph.add_edge(idx(r + 1, c), u, w);
            }
        }
    }
    graph.finalize();
    graph
}

fn clustered_graph(n_clusters: usize, cluster_size: usize, intra_density: f64, inter_edges: usize, seed: u64) -> Graph {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    
    // BUG FIX: Handle edge cases
    if n_clusters == 0 || cluster_size == 0 {
        return Graph::new(0);
    }
    
    let n = n_clusters * cluster_size;
    let mut graph = Graph::new(n);
    let intra_density = intra_density.clamp(0.0, 1.0);
    
    // Intra-cluster edges (dense, short weights)
    for c in 0..n_clusters {
        let base = c * cluster_size;
        for i in 0..cluster_size {
            for j in 0..cluster_size {
                if i != j && rng.gen::<f64>() < intra_density {
                    let w = rng.gen_range(1.0..10.0);
                    graph.add_edge(base + i, base + j, w);
                }
            }
        }
    }
    
    // Inter-cluster edges (sparse, longer weights)
    for c1 in 0..n_clusters {
        for c2 in (c1 + 1)..n_clusters {
            for _ in 0..inter_edges {
                let u = c1 * cluster_size + rng.gen_range(0..cluster_size);
                let v = c2 * cluster_size + rng.gen_range(0..cluster_size);
                let w = rng.gen_range(50.0..100.0);
                graph.add_edge(u, v, w);
                graph.add_edge(v, u, w);
            }
        }
    }
    graph.finalize();
    graph
}

fn road_network_like(n: usize, seed: u64) -> Graph {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    
    // BUG FIX: Handle edge case
    if n == 0 {
        return Graph::new(0);
    }
    
    let mut graph = Graph::new(n);
    
    // Random points in unit square
    let points: Vec<(f64, f64)> = (0..n)
        .map(|_| (rng.gen::<f64>(), rng.gen::<f64>()))
        .collect();
    
    // Connect each point to ~6 nearest neighbors
    let k_neighbors = 6.min(n.saturating_sub(1));
    
    for i in 0..n {
        let mut dists: Vec<(usize, f64)> = (0..n)
            .filter(|&j| j != i)
            .map(|j| {
                let dx = points[i].0 - points[j].0;
                let dy = points[i].1 - points[j].1;
                (j, (dx * dx + dy * dy).sqrt())
            })
            .collect();
        
        dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        
        for (j, d) in dists.iter().take(k_neighbors) {
            // BUG FIX: Ensure weight is positive (d could be 0 if points coincide)
            let w = (d * rng.gen_range(0.8..1.2) * 100.0).max(0.001);
            graph.add_edge(i, *j, w);
        }
    }
    graph.finalize();
    graph
}

// =============================================================================
// ALGORITHM 1: DIJKSTRA (optimized)
// =============================================================================

fn dijkstra(graph: &Graph, source: usize) -> Vec<f64> {
    let n = graph.n;
    if n == 0 { return Vec::new(); }
    
    let mut dist = vec![f64::INFINITY; n];
    let mut visited = vec![false; n];
    let adj = &graph.adj;  // Local reference for faster access
    
    dist[source] = 0.0;
    let mut pq = BinaryHeap::with_capacity(n);
    pq.push(State { dist: 0.0, node: source });
    
    while let Some(State { dist: d, node: u }) = pq.pop() {
        // Skip stale entries FIRST (before marking visited)
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
                pq.push(State { dist: new_d, node: v });
            }
        }
    }
    
    dist
}

// =============================================================================
// ALGORITHM 2: DAVIS MANIFOLD Δ (Delta-stepping variant)
// Curvature-aware bucket phases based on weight threshold delta
// =============================================================================

fn davis_delta(graph: &Graph, source: usize) -> Vec<f64> {
    use std::collections::BTreeMap;
    
    let n = graph.n;
    if n == 0 { return Vec::new(); }
    
    // Choose delta based on graph characteristics
    // Delta = average edge weight works well empirically
    let mut total_weight = 0.0;
    let mut edge_count = 0;
    for adj_list in &graph.adj {
        for &(_, w) in adj_list {
            total_weight += w;
            edge_count += 1;
        }
    }
    let delta = if edge_count > 0 { total_weight / edge_count as f64 } else { 1.0 };
    
    let mut dist = vec![f64::INFINITY; n];
    let adj = &graph.adj;
    
    dist[source] = 0.0;
    
    // BUG FIX: Use BTreeMap for dynamic buckets (no hard cap)
    // BTreeMap keeps buckets sorted by index, so we can iterate in order
    let mut buckets: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    buckets.entry(0).or_insert_with(Vec::new).push(source);
    
    // Track which bucket we're processing
    let mut current_bucket = 0usize;
    
    // Classify edges as "light" (<= delta) or "heavy" (> delta)
    // Light edges are relaxed immediately, heavy edges are deferred
    
    while !buckets.is_empty() {
        // Find the smallest non-empty bucket >= current_bucket
        let next_bucket = match buckets.range(current_bucket..).next() {
            Some((&idx, _)) => idx,
            None => break,
        };
        current_bucket = next_bucket;
        
        // Process bucket: relax light edges first (may add back to same bucket)
        let mut relaxed = Vec::new();
        
        while let Some(nodes) = buckets.remove(&current_bucket) {
            if nodes.is_empty() {
                break;
            }
            
            for u in nodes {
                let d = dist[u];
                
                // Skip if this entry is stale (node already processed at lower distance)
                let expected_bucket = (d / delta) as usize;
                if expected_bucket != current_bucket {
                    continue;
                }
                
                // Relax light edges (weight <= delta)
                for &(v, w) in &adj[u] {
                    if w <= delta {
                        let new_d = d + w;
                        if new_d < dist[v] {
                            dist[v] = new_d;
                            let bucket_idx = (new_d / delta) as usize;
                            buckets.entry(bucket_idx).or_insert_with(Vec::new).push(v);
                        }
                    } else {
                        // Defer heavy edge
                        relaxed.push((u, v, w));
                    }
                }
            }
        }
        
        // Now relax heavy edges from this bucket
        for (u, v, w) in relaxed {
            let new_d = dist[u] + w;
            if new_d < dist[v] {
                dist[v] = new_d;
                let bucket_idx = (new_d / delta) as usize;
                buckets.entry(bucket_idx).or_insert_with(Vec::new).push(v);
            }
        }
        
        current_bucket += 1;
    }
    
    dist
}

// =============================================================================
// ALGORITHM 3: DAVIS MANIFOLD CORRIDOR - Flat region geodesic scouting
// =============================================================================

fn davis_corridor(graph: &Graph, source: usize) -> Vec<f64> {
    let n = graph.n;
    if n == 0 { return Vec::new(); }
    
    let mut dist = vec![f64::INFINITY; n];
    let mut visited = vec![false; n];
    let adj = &graph.adj;
    let degree = &graph.degree;
    
    // Pre-compute "flat" vertices (low degree = geodesic corridor)
    // Threshold: degree <= 4 means "flat" region
    const FLAT_THRESHOLD: usize = 4;
    
    dist[source] = 0.0;
    let mut pq = BinaryHeap::with_capacity(n);
    pq.push(State { dist: 0.0, node: source });
    
    while let Some(State { dist: d, node: u }) = pq.pop() {
        // Skip stale entries FIRST (before marking visited)
        if d > dist[u] {
            continue;
        }
        if visited[u] {
            continue;
        }
        visited[u] = true;
        
        let u_is_flat = degree[u] <= FLAT_THRESHOLD;
        
        for &(v, w) in &adj[u] {
            let new_d = d + w;
            if new_d < dist[v] {
                dist[v] = new_d;
                
                // GEODESIC CORRIDOR SCOUTING:
                // If u is flat and v is also flat, scout one hop ahead
                // This is SAFE because we only UPDATE distances, not mark visited
                if u_is_flat && degree[v] <= FLAT_THRESHOLD && !visited[v] {
                    for &(v2, w2) in &adj[v] {
                        let new_d2 = new_d + w2;
                        if new_d2 < dist[v2] {
                            dist[v2] = new_d2;
                            // Only push if not visited (reduces heap operations)
                            if !visited[v2] {
                                pq.push(State { dist: new_d2, node: v2 });
                            }
                        }
                    }
                }
                
                pq.push(State { dist: new_d, node: v });
            }
        }
    }
    
    dist
}

// =============================================================================
// ALGORITHM 4: DAVIS MANIFOLD DEEP - Multi-hop corridor scouting (2-3 hops)
// =============================================================================

fn davis_deep(graph: &Graph, source: usize) -> Vec<f64> {
    let n = graph.n;
    if n == 0 { return Vec::new(); }
    
    let mut dist = vec![f64::INFINITY; n];
    let mut visited = vec![false; n];
    let adj = &graph.adj;
    let degree = &graph.degree;
    
    // Compute average degree to set adaptive threshold
    let avg_degree: f64 = if n > 0 {
        degree.iter().sum::<usize>() as f64 / n as f64
    } else { 0.0 };
    
    // Flat threshold: vertices with degree below half average
    let flat_threshold = (avg_degree / 2.0).max(3.0) as usize;
    
    dist[source] = 0.0;
    let mut pq = BinaryHeap::with_capacity(n);
    pq.push(State { dist: 0.0, node: source });
    
    while let Some(State { dist: d, node: u }) = pq.pop() {
        // Skip stale entries FIRST (before marking visited)
        if d > dist[u] {
            continue;
        }
        if visited[u] {
            continue;
        }
        visited[u] = true;
        
        let u_is_flat = degree[u] <= flat_threshold;
        
        for &(v, w) in &adj[u] {
            let new_d = d + w;
            if new_d < dist[v] {
                dist[v] = new_d;
                
                if u_is_flat && degree[v] <= flat_threshold && !visited[v] {
                    // Scout depth 2
                    for &(v2, w2) in &adj[v] {
                        let new_d2 = new_d + w2;
                        if new_d2 < dist[v2] {
                            dist[v2] = new_d2;
                            
                            // Scout depth 3 if still flat
                            if degree[v2] <= flat_threshold && !visited[v2] {
                                for &(v3, w3) in &adj[v2] {
                                    let new_d3 = new_d2 + w3;
                                    if new_d3 < dist[v3] && !visited[v3] {
                                        dist[v3] = new_d3;
                                        pq.push(State { dist: new_d3, node: v3 });
                                    }
                                }
                            }
                            
                            if !visited[v2] {
                                pq.push(State { dist: new_d2, node: v2 });
                            }
                        }
                    }
                }
                
                pq.push(State { dist: new_d, node: v });
            }
        }
    }
    
    dist
}

// =============================================================================
// ALGORITHM 5: DAVIS MANIFOLD HUB - Batch relaxation for high-curvature hubs
// =============================================================================

fn davis_hub(graph: &Graph, source: usize) -> Vec<f64> {
    let n = graph.n;
    if n == 0 { return Vec::new(); }
    
    let mut dist = vec![f64::INFINITY; n];
    let mut visited = vec![false; n];
    let adj = &graph.adj;
    let degree = &graph.degree;
    
    // Identify "hub" vertices (high degree)
    let avg_degree: f64 = degree.iter().sum::<usize>() as f64 / n.max(1) as f64;
    let hub_threshold = (avg_degree * 2.0) as usize;
    
    dist[source] = 0.0;
    let mut pq = BinaryHeap::with_capacity(n);
    pq.push(State { dist: 0.0, node: source });
    
    // Buffer for batch processing
    let mut batch_buffer: Vec<(usize, f64)> = Vec::with_capacity(64);
    
    while let Some(State { dist: d, node: u }) = pq.pop() {
        // Skip stale entries FIRST (before marking visited)
        if d > dist[u] {
            continue;
        }
        if visited[u] {
            continue;
        }
        visited[u] = true;
        
        // If u is a hub, batch-relax all neighbors
        if degree[u] >= hub_threshold {
            batch_buffer.clear();
            
            for &(v, w) in &adj[u] {
                let new_d = d + w;
                if new_d < dist[v] {
                    dist[v] = new_d;
                    if !visited[v] {
                        batch_buffer.push((v, new_d));
                    }
                }
            }
            
            // Sort batch by distance and add to heap
            batch_buffer.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            for &(v, new_d) in &batch_buffer {
                pq.push(State { dist: new_d, node: v });
            }
        } else {
            // Standard processing with 1-hop scout for flat regions
            for &(v, w) in &adj[u] {
                let new_d = d + w;
                if new_d < dist[v] {
                    dist[v] = new_d;
                    
                    // Scout if v is also low-degree
                    if degree[v] <= 4 && !visited[v] {
                        for &(v2, w2) in &adj[v] {
                            let new_d2 = new_d + w2;
                            if new_d2 < dist[v2] && !visited[v2] {
                                dist[v2] = new_d2;
                                pq.push(State { dist: new_d2, node: v2 });
                            }
                        }
                    }
                    
                    pq.push(State { dist: new_d, node: v });
                }
            }
        }
    }
    
    dist
}

// =============================================================================
// BENCHMARKING
// =============================================================================

fn verify_distances(d1: &[f64], d2: &[f64]) -> bool {
    if d1.len() != d2.len() {
        return false;
    }
    d1.iter().zip(d2.iter()).all(|(a, b)| {
        // BUG FIX: Handle all edge cases properly
        match (a.is_finite(), b.is_finite()) {
            (true, true) => (a - b).abs() < 1e-9,
            (false, false) => a.is_infinite() == b.is_infinite() && a.is_sign_positive() == b.is_sign_positive(),
            _ => false,
        }
    })
}

/// More detailed correctness check with error reporting
fn verify_distances_detailed(name: &str, reference: &[f64], result: &[f64]) -> bool {
    if reference.len() != result.len() {
        eprintln!("  [{}] Length mismatch: {} vs {}", name, reference.len(), result.len());
        return false;
    }
    
    let mut errors = 0;
    for (i, (r, d)) in reference.iter().zip(result.iter()).enumerate() {
        let ok = match (r.is_finite(), d.is_finite()) {
            (true, true) => (r - d).abs() < 1e-9,
            (false, false) => r.is_infinite() == d.is_infinite() 
                           && r.is_sign_positive() == d.is_sign_positive(),
            _ => false,
        };
        if !ok {
            if errors < 3 {
                eprintln!("  [{}] Mismatch at node {}: expected {}, got {}", name, i, r, d);
            }
            errors += 1;
        }
    }
    
    if errors > 0 {
        eprintln!("  [{}] Total mismatches: {}", name, errors);
    }
    errors == 0
}

struct BenchResult {
    name: &'static str,
    time_us: u128,
    correct: bool,
}

fn benchmark<F>(name: &'static str, graph: &Graph, source: usize, reference: &[f64], f: F) -> BenchResult 
where F: Fn(&Graph, usize) -> Vec<f64>
{
    // Handle empty graph
    if graph.n == 0 {
        return BenchResult { name, time_us: 0, correct: true };
    }
    
    // Warmup
    let _ = f(graph, source);
    
    // Timed run (average of 5)
    let runs = 5;
    let mut total = 0u128;
    let mut result = Vec::new();
    
    for _ in 0..runs {
        let start = Instant::now();
        result = f(graph, source);
        total += start.elapsed().as_micros();
    }
    
    let correct = verify_distances_detailed(name, reference, &result);
    
    BenchResult {
        name,
        time_us: total / runs as u128,
        correct,
    }
}

use std::collections::HashMap;

fn run_benchmark_suite(sizes: &[usize], seed: u64) {
    println!("================================================================================");
    println!("SSSP BENCHMARK (Rust) - No Python Overhead");
    println!("Dijkstra vs Manifold (Davis) - Pure Algorithmic Comparison");
    println!("================================================================================");
    
    // Track wins: (algorithm_name, graph_type) -> [(n, speedup)]
    let mut wins: HashMap<(&str, &str), Vec<(usize, f64)>> = HashMap::new();
    let mut total_tests = 0;
    let mut dijkstra_wins = 0;
    
    for &n in sizes {
        println!("\n============================================================");
        println!("Testing n = {}", n);
        println!("============================================================");
        
        // Compute grid dimensions
        let mut rows = (n as f64).sqrt() as usize;
        while n % rows != 0 && rows > 1 {
            rows -= 1;
        }
        let cols = n / rows;
        
        let n_clusters = (n / 50).max(1);
        let graphs: Vec<(&str, Graph)> = vec![
            ("sparse", random_sparse_graph(n, 4, seed)),
            ("dense", random_dense_graph(n, 0.1, seed)),
            ("grid", grid_graph(rows, cols, seed)),
            ("clustered", clustered_graph(n_clusters, 50, 0.5, 2, seed)),
            ("road_like", road_network_like(n, seed)),
        ];
        
        for (graph_type, graph) in &graphs {
            println!("\n  {}: n={}, m={}", graph_type, graph.n, graph.m);
            
            // Handle empty graph
            if graph.n == 0 {
                println!("    (empty graph, skipping)");
                continue;
            }
            
            // Reference from Dijkstra
            let reference = dijkstra(graph, 0);
            
            let results = vec![
                benchmark("Dijkstra", graph, 0, &reference, dijkstra),
                benchmark("Davis_Δ", graph, 0, &reference, davis_delta),
                benchmark("Davis_Corridor", graph, 0, &reference, davis_corridor),
                benchmark("Davis_Deep", graph, 0, &reference, davis_deep),
                benchmark("Davis_Hub", graph, 0, &reference, davis_hub),
            ];
            
            for r in &results {
                let status = if r.correct { "✓" } else { "✗" };
                println!("    {:20}: {:8}μs {}", r.name, r.time_us, status);
            }
            
            // Track winner
            total_tests += 1;
            let correct_results: Vec<_> = results.iter().filter(|r| r.correct).collect();
            if let Some(winner) = correct_results.iter().min_by_key(|r| r.time_us) {
                let dijkstra_time = results[0].time_us;
                if dijkstra_time > 0 {
                    let speedup = dijkstra_time as f64 / winner.time_us.max(1) as f64;
                    if winner.name != "Dijkstra" {
                        println!("    🏆 {} wins! ({:.2}x vs Dijkstra)", winner.name, speedup);
                        wins.entry((winner.name, graph_type))
                            .or_insert_with(Vec::new)
                            .push((n, speedup));
                    } else {
                        dijkstra_wins += 1;
                    }
                }
            }
        }
    }
    
    // Print summary
    println!("\n================================================================================");
    println!("SUMMARY");
    println!("================================================================================");
    
    // Aggregate wins by algorithm
    let mut algo_wins: HashMap<&str, Vec<(&str, usize, f64)>> = HashMap::new();
    for ((algo, graph_type), entries) in &wins {
        for &(n, speedup) in entries {
            algo_wins.entry(algo)
                .or_insert_with(Vec::new)
                .push((graph_type, n, speedup));
        }
    }
    
    let manifold_total: usize = algo_wins.values().map(|v| v.len()).sum();
    
    println!("\n  Total tests: {}", total_tests);
    println!("  Dijkstra wins: {} ({:.1}%)", dijkstra_wins, 100.0 * dijkstra_wins as f64 / total_tests as f64);
    println!("  Davis Manifold wins: {} ({:.1}%)", manifold_total, 100.0 * manifold_total as f64 / total_tests as f64);
    
    println!("\n  Wins by algorithm:");
    for (algo, mut entries) in algo_wins {
        entries.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        println!("\n    {} ({} wins):", algo, entries.len());
        for (graph_type, n, speedup) in entries {
            println!("      - {} n={}: {:.2}x speedup", graph_type, n, speedup);
        }
    }
    
    // Best domains for Davis Manifold
    println!("\n  Best graph types for Davis Manifold:");
    let mut graph_type_wins: HashMap<&str, (usize, f64)> = HashMap::new();
    for ((_, graph_type), entries) in &wins {
        let count = entries.len();
        let avg_speedup: f64 = entries.iter().map(|(_, s)| s).sum::<f64>() / count as f64;
        let entry = graph_type_wins.entry(graph_type).or_insert((0, 0.0));
        entry.0 += count;
        entry.1 = (entry.1 * (entry.0 - count) as f64 + avg_speedup * count as f64) / entry.0 as f64;
    }
    
    let mut sorted_graph_types: Vec<_> = graph_type_wins.iter().collect();
    sorted_graph_types.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    
    for (graph_type, (count, avg_speedup)) in sorted_graph_types {
        println!("    - {}: {} wins, avg {:.2}x speedup", graph_type, count, avg_speedup);
    }
    
    println!("\n================================================================================");
}

fn main() {
    let sizes = vec![500, 1000, 2000, 5000, 10000];
    run_benchmark_suite(&sizes, 42);
}
