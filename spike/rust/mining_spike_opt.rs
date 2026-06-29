// Lode mining-core spike (Rust, OPTIMIZED). Same algorithm + same template_set_hash
// as mining_spike.rs, with the identical optimization applied to Swift's opt version:
//   - tokens interned to ids ONCE (outside the timed loop)
//   - integer hash routing key (no per-line String build)
//   - similarity over ids
// Build: rustc -O mining_spike_opt.rs -o mining_spike_opt
// Run:   ./mining_spike_opt [N]   (default 1_000_000)
use std::collections::HashMap;
use std::time::Instant;

const SAMPLE: &[&str] = &[
    "127.0.0.1 - - [10/Oct/2024:13:55:36] \"GET /api/users/12 HTTP/1.1\" 200 1534",
    "127.0.0.1 - - [10/Oct/2024:13:55:37] \"GET /api/users/47 HTTP/1.1\" 200 1422",
    "192.168.1.5 - - [10/Oct/2024:13:55:38] \"POST /api/login HTTP/1.1\" 401 88",
    "INFO 2024-10-10 13:55:36 user 12 logged in from 10.0.0.3",
    "INFO 2024-10-10 13:55:37 user 47 logged in from 10.0.0.9",
    "ERROR 2024-10-10 13:55:40 db connection failed after 3000 ms id 550e8400-e29b-41d4-a716-446655440000",
    "WARN 2024-10-10 13:55:41 cache miss for key a1b2c3d4e5f6a7b8",
    "GET /static/app.css 200",
];

fn is_hex(s: &str) -> bool { s.len() >= 8 && s.bytes().all(|b| b.is_ascii_hexdigit()) }
fn is_num(s: &str) -> bool {
    let t = s.trim_start_matches('-');
    !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit() || b == b'.') && t.bytes().any(|b| b.is_ascii_digit())
}
fn is_ip(s: &str) -> bool {
    let p: Vec<&str> = s.split('.').collect();
    p.len() == 4 && p.iter().all(|x| !x.is_empty() && x.bytes().all(|b| b.is_ascii_digit()))
}
fn is_uuid(s: &str) -> bool {
    s.len() == 36 && s.as_bytes().iter().enumerate().all(|(i, &b)| {
        if i == 8 || i == 13 || i == 18 || i == 23 { b == b'-' } else { (b as char).is_ascii_hexdigit() }
    })
}
fn is_ts(s: &str) -> bool { s.matches(':').count() >= 2 && s.bytes().any(|b| b.is_ascii_digit()) }
fn is_path(s: &str) -> bool { s.contains('/') && s.len() > 1 }

fn mask(tok: &str) -> String {
    if is_uuid(tok) { "<UUID>".into() }
    else if is_ip(tok) { "<IP>".into() }
    else if is_ts(tok) { "<TS>".into() }
    else if is_path(tok) { "<PATH>".into() }
    else if is_hex(tok) { "<HEX>".into() }
    else if is_num(tok) { "<NUM>".into() }
    else { tok.to_string() }
}

fn intern(s: String, interner: &mut HashMap<String, usize>, id_to_str: &mut Vec<String>) -> usize {
    if let Some(&id) = interner.get(&s) { return id; }
    let id = id_to_str.len();
    interner.insert(s.clone(), id);
    id_to_str.push(s);
    id
}

#[inline(always)]
fn route_key(ids: &[usize], end: usize, len: usize) -> u64 {
    let mut h: u64 = 1469598103934665603 ^ (len as u64);
    for k in 0..end { h = h.wrapping_mul(1099511628211) ^ (ids[k] as u64); }
    h
}
#[inline(always)]
fn sim(a: &[usize], b: &[usize]) -> f64 {
    if a.len() != b.len() { return 0.0; }
    if a.is_empty() { return 1.0; }
    let m = a.iter().zip(b).filter(|(x, y)| x == y).count();
    m as f64 / a.len() as f64
}

fn main() {
    let d = 4usize;
    let st = 0.5f64;
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(1_000_000);

    let mut interner: HashMap<String, usize> = HashMap::new();
    let mut id_to_str: Vec<String> = Vec::new();
    let wildcard = intern("<*>".into(), &mut interner, &mut id_to_str);

    let masked: Vec<Vec<usize>> = SAMPLE.iter()
        .map(|line| line.split_whitespace().map(|t| intern(mask(t), &mut interner, &mut id_to_str)).collect())
        .collect();

    let mut templates: Vec<(Vec<usize>, u64)> = Vec::with_capacity(256);
    let mut index: HashMap<u64, Vec<usize>> = HashMap::with_capacity(256);

    let start = Instant::now();
    for i in 0..n {
        let ids = &masked[i % masked.len()];
        let len = ids.len();
        let pref_end = d.min(len);
        let key = route_key(ids, pref_end, len);
        let cands = index.entry(key).or_default();

        let mut best: i64 = -1;
        let mut best_sim = st;
        for &ti in cands.iter() {
            let s = sim(&templates[ti].0, ids);
            if s >= best_sim { best_sim = s; best = ti as i64; }
        }
        if best >= 0 {
            let t = &mut templates[best as usize];
            for j in 0..len {
                if t.0[j] != ids[j] && t.0[j] != wildcard { t.0[j] = wildcard; }
            }
            t.1 += 1;
        } else {
            let id = templates.len();
            templates.push((ids.clone(), 1));
            cands.push(id);
        }
    }
    let elapsed = start.elapsed();

    let mut patterns: Vec<String> = templates.iter()
        .map(|t| t.0.iter().map(|&id| id_to_str[id].as_str()).collect::<Vec<_>>().join(" "))
        .collect();
    patterns.sort();
    let mut h: u64 = 0xcbf29ce484222325;
    for s in &patterns {
        for b in s.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        h ^= b'\n' as u64; h = h.wrapping_mul(0x100000001b3);
    }

    let mut out: Vec<String> = templates.iter()
        .map(|t| format!("{:>10}  {}", t.1, t.0.iter().map(|&id| id_to_str[id].as_str()).collect::<Vec<_>>().join(" ")))
        .collect();
    out.sort();
    println!("=== templates ({}) ===", templates.len());
    for l in &out { println!("{}", l); }
    let secs = elapsed.as_secs_f64();
    println!("---");
    println!("lines={} time={:.3}s throughput={:.0} lines/sec", n, secs, n as f64 / secs);
    println!("template_set_hash={:016x}", h);
}
