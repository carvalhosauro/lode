# Lode — Mining-Core Spike (Rust vs Swift)

Throwaway language-comparison spike. Not product code. It implements the **core of
RFC-0003** (tokenize → mask → fixed-depth Drain-style routing → widen-only templates)
identically in Rust and Swift, over the same embedded log sample, and reports:

- the mined template set + occurrence counts,
- a deterministic `template_set_hash` (FNV-1a over sorted patterns),
- throughput in lines/sec.

The two implementations use the **same algorithm and the same FNV hash**, so:

> **If the Swift and Rust `template_set_hash` match, both implement the identical
> algorithm.** That is the correctness cross-check. Then compare throughput and
> ergonomics.

## Run

Rust:
```
cd rust
rustc -O mining_spike.rs -o mining_spike
./mining_spike 1000000
```

Swift:
```
cd swift
swiftc -O mining_spike.swift -o mining_spike
./mining_spike 1000000
# or, no build step: swift mining_spike.swift 1000000
```

Both take an optional line count (default 1,000,000).

## Results (this sandbox, single thread, stdlib only)

All four variants emit identical templates and `template_set_hash = 7ae1d59d88c326ce`
(algorithm parity proven). Files: `*_spike.{rs,swift}` (naive), `*_spike_opt.{rs,swift}` (optimized).

| Variant | 1M lines | 5M lines |
| ------- | -------- | -------- |
| Swift naive | 1.26M l/s | 1.26M l/s |
| Rust naive  | 9.16M l/s | 9.17M l/s |
| Swift opt   | 12.4M l/s | 12.5M l/s |
| Rust opt    | 14.7M l/s | 18.2M l/s |

Optimization = intern tokens to Int ids + integer hash routing key + no per-line
allocation. It gave Swift ~9.8× and Rust ~2×.

**Conclusion (routing-only):** at the ceiling Rust is only ~1.2–1.5× faster than
Swift, not 7×. The naive gap was allocation overhead in the Swift hot loop, not a
language ceiling. The tree/routing core is effectively at parity.

## Round 2 — multi-thread + regex (the realistic hot path)

Files: `*_mt.{rs,swift}` (per-stream parallel), `rust-regex/`, `*_regex.swift`
(masking IN the timed loop, since real ingest masks every line). All hashes match.

**Multi-thread** (16M lines, per-stream isolation, routing-only):

| | 1 thread | 16 threads |
| --- | -------- | ---------- |
| Rust | 17.7M l/s | 51.7M l/s |
| Swift | 23.0M l/s | 44.1M l/s |

Scales ~2–3× here (tiny per-thread workload + WSL2 undersell it); determinism holds
per stream. Maps to RFC-0012 one-worker-per-stream.

**Full per-line pipeline** (tokenize + mask + route, masking in loop, 1M, 1 thread):

| masking | Rust | Swift | Rust edge |
| ------- | ---- | ----- | --------- |
| char-class | 1.05M l/s | 0.106M l/s | ~10× |
| regex | 0.86M l/s | 0.030M l/s | ~29× |

Findings:
- The **tree is cheap; per-line masking+tokenize is the real cost** (~17× drop from routing-only).
- **Rust dominates the masking hot path as written** (10× / 29×). Regex penalty within a language: Rust 1.2×, Swift 3.6×.
- **Swift native `Regex` is slow** (~30k l/s) — avoid in the hot path; use `NSRegularExpression` or hand-rolled.
- Caveat: the Swift mask helpers are allocation-naive (`Array(utf8)` + `String` per token) — the char-class 10× gap is largely fixable (as the routing opt was), the regex gap mostly intrinsic.

**Real-code guidance:** char-class fast-path for common masks (NUM/IP/UUID/HEX),
regex only for rich/custom masks; in Swift avoid native `Regex` in ingest.

## What to compare (the actual decision)

| Axis | Look for |
| ---- | -------- |
| Correctness | identical `template_set_hash` (must match) |
| Throughput | lines/sec on the same machine |
| Ergonomics | how each language felt for: token/string handling, the byte-level mask checks, the `Template`/`HashMap` data model, value vs reference semantics, determinism |
| Friction | build/run setup, error messages, iteration speed |

## Notes / simplifications (identical in both)

- Masking uses byte-class checks (no regex engine) so the spike is dependency-free in both languages.
- Routing uses a flat `(len, prefix)` bucket instead of a literal multi-level tree — same work, simpler; fidelity to full Drain is not the point here, language feel and the mining core are.
- Ingestion/enrichment are not benchmarked; the sample is pre-tokenized conceptually inside the timed loop the same way in both.
