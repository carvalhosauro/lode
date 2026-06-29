// Lode regex-masking spike (Rust, regex crate). Two passes in ONE binary:
//   charclass = byte-class masking in the timed loop
//   regex     = regex masking in the timed loop
// Masking is in the hot loop here (unlike the routing-only spikes) because real
// ingest masks every line once. The delta between the two passes = regex overhead.
use std::collections::HashMap;
use std::time::Instant;
use regex::Regex;

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
fn mask_cc(tok: &str) -> String {
    if is_uuid(tok) { "<UUID>".into() } else if is_ip(tok) { "<IP>".into() }
    else if is_ts(tok) { "<TS>".into() } else if is_path(tok) { "<PATH>".into() }
    else if is_hex(tok) { "<HEX>".into() } else if is_num(tok) { "<NUM>".into() }
    else { tok.to_string() }
}

fn intern(s: String, m: &mut HashMap<String, usize>, v: &mut Vec<String>) -> usize {
    if let Some(&id) = m.get(&s) { return id; }
    let id = v.len(); m.insert(s.clone(), id); v.push(s); id
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
    a.iter().zip(b).filter(|(x, y)| x == y).count() as f64 / a.len() as f64
}

fn run(label: &str, n: usize, mask_fn: &dyn Fn(&str) -> String) {
    let d = 4usize; let st = 0.5f64;
    let mut interner: HashMap<String, usize> = HashMap::new();
    let mut id_to_str: Vec<String> = Vec::new();
    let wildcard = intern("<*>".into(), &mut interner, &mut id_to_str);
    let mut templates: Vec<(Vec<usize>, u64)> = Vec::with_capacity(256);
    let mut index: HashMap<u64, Vec<usize>> = HashMap::with_capacity(256);
    let mut idbuf: Vec<usize> = Vec::with_capacity(32);

    let start = Instant::now();
    for i in 0..n {
        let line = SAMPLE[i % SAMPLE.len()];
        idbuf.clear();
        for t in line.split_whitespace() {
            let m = mask_fn(t);
            idbuf.push(intern(m, &mut interner, &mut id_to_str));
        }
        let ids = &idbuf;
        let len = ids.len();
        let pref = d.min(len);
        let key = route_key(ids, pref, len);
        let cands = index.entry(key).or_default();
        let mut best: i64 = -1; let mut bs = st;
        for &ti in cands.iter() { let s = sim(&templates[ti].0, ids); if s >= bs { bs = s; best = ti as i64; } }
        if best >= 0 {
            let t = &mut templates[best as usize];
            for j in 0..len { if t.0[j] != ids[j] && t.0[j] != wildcard { t.0[j] = wildcard; } }
            t.1 += 1;
        } else {
            let id = templates.len();
            templates.push((ids.clone(), 1));
            cands.push(id);
        }
    }
    let secs = start.elapsed().as_secs_f64();
    let mut pats: Vec<String> = templates.iter()
        .map(|t| t.0.iter().map(|&x| id_to_str[x].as_str()).collect::<Vec<_>>().join(" ")).collect();
    pats.sort();
    let mut h: u64 = 0xcbf29ce484222325;
    for s in &pats { for b in s.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); } h ^= 10; h = h.wrapping_mul(0x100000001b3); }
    println!("{:<10} templates={} time={:.3}s throughput={:.0} l/s hash={:016x}", label, templates.len(), secs, n as f64 / secs, h);
}

fn build_regex_masker() -> impl Fn(&str) -> String {
    let uuid = Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$").unwrap();
    let ip = Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$").unwrap();
    let ts = Regex::new(r"[0-9]{1,2}:[0-9]{2}:[0-9]{2}").unwrap();
    let path = Regex::new(r"/").unwrap();
    let hex = Regex::new(r"^[0-9a-fA-F]{8,}$").unwrap();
    let num = Regex::new(r"^-?[0-9.]*[0-9][0-9.]*$").unwrap();
    move |tok: &str| -> String {
        if uuid.is_match(tok) { "<UUID>".into() }
        else if ip.is_match(tok) { "<IP>".into() }
        else if ts.is_match(tok) { "<TS>".into() }
        else if path.is_match(tok) { "<PATH>".into() }
        else if hex.is_match(tok) { "<HEX>".into() }
        else if num.is_match(tok) { "<NUM>".into() }
        else { tok.to_string() }
    }
}

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(1_000_000);
    run("charclass", n, &mask_cc);
    let rem = build_regex_masker();
    run("regex", n, &rem);
}
