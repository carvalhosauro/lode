//! Reproducible mining benchmark over the golden corpus (RFC-0003 §12, task T1.4).
//!
//! Two layers, both driven by the *same fixed golden corpus* used by the PA gate, so
//! every run is comparable:
//!
//! * **Determinism/quality fingerprint** — `template_set_hash` + per-format PA + template
//!   count deltas. This is bit-identical on any machine (RFC-0003 determinism invariant),
//!   committed as `benches/mining.golden.json`, and compared exactly. It is the golden gate.
//! * **Throughput** — lines/s and MiB/s of the tokenize→mask→drain hot path. Machine-relative;
//!   reported for same-machine before/after comparison, never used to fail a run.
//!
//! Usage:
//! ```text
//! cargo bench --bench mining                    # report + compare vs committed golden
//! cargo bench --bench mining -- --check         # exit 1 on determinism/quality drift (CI gate)
//! cargo bench --bench mining -- --update-golden # rewrite the committed fingerprint baseline
//! ```
//! Each run also writes the full result to `target/lode-bench/mining.json`.

// Benchmarks favour terse ratio maths over exhaustive lossless-cast ceremony.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

// Reuse the exact corpus loader the integration tests use — the bench mines the same
// golden set as the PA gate, with zero duplication.
#[path = "../tests/common/corpus_loader.rs"]
mod corpus_loader;

use std::hint::black_box;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use corpus_loader::{corpus_root, load_corpus};
use lode_core::{CorpusInput, CorpusMiner, run_corpus};
use lode_parse::DrainMiner;

/// Approximate number of line-mines to time per round (the corpus is replayed to reach it).
const TARGET_LINES_PER_ROUND: usize = 300_000;
/// Timed rounds; the median is reported to damp scheduler noise.
const ROUNDS: usize = 25;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let update_golden = args.iter().any(|a| a == "--update-golden");
    let check = args.iter().any(|a| a == "--check");

    let input = load_corpus(corpus_root());
    let total_lines: usize = input.formats.iter().map(|f| f.lines.len()).sum();
    let total_bytes: usize = input
        .formats
        .iter()
        .flat_map(|f| &f.lines)
        .map(String::len)
        .sum();

    let fingerprint = Fingerprint::compute(&input);
    let throughput = measure_throughput(&input, total_lines, total_bytes);

    fingerprint.print(total_lines, total_bytes);
    throughput.print();
    write_run_json(&fingerprint, &throughput, total_lines, total_bytes);

    let golden_path = golden_path();
    if update_golden {
        fingerprint.write_golden(&golden_path);
        println!("\n  golden baseline written to {}", golden_path.display());
        return ExitCode::SUCCESS;
    }

    match Fingerprint::read_golden(&golden_path) {
        None => {
            println!(
                "\n  no golden baseline at {} — run `-- --update-golden` to create one",
                golden_path.display()
            );
            ExitCode::SUCCESS
        }
        Some(golden) => {
            let drift = fingerprint.diff(&golden);
            if drift.is_empty() {
                println!("\n  vs golden: MATCH ✓ (determinism + quality unchanged)");
                ExitCode::SUCCESS
            } else {
                println!("\n  vs golden: DRIFT ✗");
                for line in &drift {
                    println!("    - {line}");
                }
                if check {
                    println!("  (mining output changed; if intended, re-run with --update-golden)");
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
        }
    }
}

/// Bit-reproducible mining fingerprint (independent of hardware).
struct Fingerprint {
    template_set_hash: u64,
    global_pa: f64,
    per_format: Vec<(String, f64, i64)>, // (id, pa, template_count_delta)
}

impl Fingerprint {
    fn compute(input: &CorpusInput) -> Self {
        let mut miner = DrainMiner::new();
        let result = run_corpus(input, &mut miner).expect("golden corpus evaluates");
        let mut per_format: Vec<(String, f64, i64)> = input
            .formats
            .iter()
            .map(|f| {
                let id = f.spec.id.clone();
                let pa = result.per_format_pa[&id];
                let delta = result.per_format_template_count_delta[&id];
                (id, pa, delta)
            })
            .collect();
        per_format.sort_by(|a, b| a.0.cmp(&b.0));
        Self {
            template_set_hash: result.template_set_hash,
            global_pa: result.global_pa,
            per_format,
        }
    }

    fn print(&self, total_lines: usize, total_bytes: usize) {
        println!(
            "lode mining benchmark — golden corpus ({} formats, {total_lines} lines, {:.1} KiB)",
            self.per_format.len(),
            total_bytes as f64 / 1024.0
        );
        println!(
            "  determinism: template_set_hash={:#018x}  global_pa={:.4}",
            self.template_set_hash, self.global_pa
        );
        for (id, pa, delta) in &self.per_format {
            println!("    {id:<16} pa={pa:.4}  template_count_delta={delta:+}");
        }
    }

    /// Human-comparable drift lines vs a golden baseline; empty = identical.
    fn diff(&self, golden: &Self) -> Vec<String> {
        let mut out = Vec::new();
        if self.template_set_hash != golden.template_set_hash {
            out.push(format!(
                "template_set_hash {:#018x} != golden {:#018x}",
                self.template_set_hash, golden.template_set_hash
            ));
        }
        if (self.global_pa - golden.global_pa).abs() > 1e-9 {
            out.push(format!(
                "global_pa {:.6} != golden {:.6}",
                self.global_pa, golden.global_pa
            ));
        }
        for (id, pa, delta) in &self.per_format {
            match golden.per_format.iter().find(|g| &g.0 == id) {
                None => out.push(format!("format {id} absent from golden")),
                Some((_, gpa, gdelta)) => {
                    if (pa - gpa).abs() > 1e-9 {
                        out.push(format!("{id}: pa {pa:.6} != golden {gpa:.6}"));
                    }
                    if delta != gdelta {
                        out.push(format!(
                            "{id}: template_count_delta {delta:+} != golden {gdelta:+}"
                        ));
                    }
                }
            }
        }
        out
    }

    fn to_json(&self) -> String {
        let fmts = self
            .per_format
            .iter()
            .map(|(id, pa, delta)| {
                format!(
                    "    {{ \"id\": {id:?}, \"pa\": {pa:.6}, \"template_count_delta\": {delta} }}"
                )
            })
            .collect::<Vec<_>>()
            .join(",\n");
        format!(
            "{{\n  \"template_set_hash\": \"{:#018x}\",\n  \"global_pa\": {:.6},\n  \"per_format\": [\n{fmts}\n  ]\n}}\n",
            self.template_set_hash, self.global_pa
        )
    }

    fn write_golden(&self, path: &PathBuf) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create golden dir");
        }
        std::fs::write(path, self.to_json()).expect("write golden baseline");
    }

    /// Parse a golden baseline written by [`Self::to_json`]; `None` if the file is absent.
    fn read_golden(path: &PathBuf) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        let value: serde_json::Value = serde_json::from_str(&text).expect("golden json parses");
        let template_set_hash = {
            let s = value["template_set_hash"]
                .as_str()
                .expect("hash is a string");
            let hex = s.strip_prefix("0x").unwrap_or(s);
            u64::from_str_radix(hex, 16).expect("hash is hex")
        };
        let global_pa = value["global_pa"].as_f64().expect("global_pa is a number");
        let per_format = value["per_format"]
            .as_array()
            .expect("per_format is an array")
            .iter()
            .map(|e| {
                (
                    e["id"].as_str().expect("id string").to_string(),
                    e["pa"].as_f64().expect("pa number"),
                    e["template_count_delta"].as_i64().expect("delta int"),
                )
            })
            .collect();
        Some(Self {
            template_set_hash,
            global_pa,
            per_format,
        })
    }
}

/// Timed throughput of the tokenize→mask→drain hot path (machine-relative).
struct Throughput {
    median: Duration,
    lines_per_round: usize,
    bytes_per_round: usize,
}

impl Throughput {
    fn lines_per_sec(&self) -> f64 {
        self.lines_per_round as f64 / self.median.as_secs_f64()
    }
    fn mib_per_sec(&self) -> f64 {
        (self.bytes_per_round as f64 / (1024.0 * 1024.0)) / self.median.as_secs_f64()
    }
    fn ns_per_line(&self) -> f64 {
        self.median.as_nanos() as f64 / self.lines_per_round as f64
    }

    fn print(&self) {
        println!(
            "  throughput (median of {ROUNDS} rounds, {} lines/round):",
            self.lines_per_round
        );
        println!("    {:.2} M lines/s", self.lines_per_sec() / 1_000_000.0);
        println!("    {:.1} MiB/s", self.mib_per_sec());
        println!("    {:.0} ns/line", self.ns_per_line());
    }
}

/// One full corpus pass; returns a checksum so the optimizer can't elide the work.
fn mine_pass(input: &CorpusInput) -> usize {
    let mut miner = DrainMiner::new();
    let mut acc = 0usize;
    for format in &input.formats {
        miner.begin_format(&format.spec.id);
        for line in &format.lines {
            acc = acc.wrapping_add(miner.mine_line(line).len());
        }
    }
    acc
}

fn measure_throughput(input: &CorpusInput, total_lines: usize, total_bytes: usize) -> Throughput {
    let inner = (TARGET_LINES_PER_ROUND / total_lines.max(1)).max(1);

    for _ in 0..3 {
        black_box(mine_pass(input)); // warm up caches / branch predictors
    }

    let mut times = Vec::with_capacity(ROUNDS);
    for _ in 0..ROUNDS {
        let start = Instant::now();
        let mut acc = 0usize;
        for _ in 0..inner {
            acc = acc.wrapping_add(mine_pass(input));
        }
        black_box(acc);
        times.push(start.elapsed());
    }
    times.sort_unstable();

    Throughput {
        median: times[ROUNDS / 2],
        lines_per_round: total_lines * inner,
        bytes_per_round: total_bytes * inner,
    }
}

fn write_run_json(fp: &Fingerprint, tp: &Throughput, total_lines: usize, total_bytes: usize) {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/lode-bench/mining.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = format!(
        "{{\n  \"template_set_hash\": \"{:#018x}\",\n  \"global_pa\": {:.6},\n  \"corpus_lines\": {total_lines},\n  \"corpus_bytes\": {total_bytes},\n  \"rounds\": {ROUNDS},\n  \"lines_per_round\": {},\n  \"median_ns\": {},\n  \"lines_per_sec\": {:.0},\n  \"mib_per_sec\": {:.2},\n  \"ns_per_line\": {:.2}\n}}\n",
        fp.template_set_hash,
        fp.global_pa,
        tp.lines_per_round,
        tp.median.as_nanos(),
        tp.lines_per_sec(),
        tp.mib_per_sec(),
        tp.ns_per_line(),
    );
    let _ = std::fs::write(&path, json);
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benches/mining.golden.json")
}
