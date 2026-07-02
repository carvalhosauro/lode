# Phase 1 — Mining Engine Implementation Plan

> **For agentic workers:** Implement task-by-task. Checkboxes (`- [ ]`) track progress.  
> **Spec:** [`docs/phase-1-spec.md`](../phase-1-spec.md) · **Tasks:** T1.1–T1.4 in [`TASKS.md`](../../TASKS.md)

**Goal:** Implement the full mining pipeline (tokenize → mask → drain → fingerprint) so the golden corpus gate passes with PA ≥ 0.90 per format and bit-identical determinism.

**Architecture:** `lode-parse` owns tokenization and masking (IO-adjacent, may grow regex later). `lode-core` owns pure Drain state (routing, similarity, widen, registry, pattern string). `DrainMiner` in `lode-parse` wires both and implements `CorpusMiner`. Per-format `d=5` override for `nginx-access` via `begin_format`.

**Tech stack:** Rust 1.85, workspace crates, `cargo test`, optional `criterion` bench for T1.4.

**Branch deliverables:**

| ID | Deliverable | Gate |
|----|-------------|------|
| T1.1 | Tokenizer + masker | Unit tests per format fixture lines |
| T1.2 | `DrainState` + registry | Unit tests for route/match/widen |
| T1.3 | `DrainMiner` + `CorpusMiner` | Integration smoke |
| T1.4 | Corpus PA + determinism | `corpus_pa_meets_floor` un-ignored, all green |

---

## File map (create / modify)

| File | Responsibility |
|------|----------------|
| `crates/lode-parse/src/error.rs` | `ParseError` (empty line, line too long) |
| `crates/lode-parse/src/tokenize.rs` | Structural scanner + JSON fast-path |
| `crates/lode-parse/src/mask.rs` | Char-class predicates + composite rules |
| `crates/lode-parse/src/miner.rs` | `DrainMiner`, `CorpusMiner` impl |
| `crates/lode-parse/src/lib.rs` | Module exports |
| `crates/lode-parse/tests/tokenize_corpus.rs` | Tokenizer vs expected token splits |
| `crates/lode-parse/tests/mask_corpus.rs` | Mask-only PA per format (pre-drain ceiling) |
| `crates/lode-core/src/mining/drain.rs` | `DrainState`, `ProcessResult`, routing/match/widen |
| `crates/lode-core/src/mining/pattern.rs` | `pattern_to_string(&[Token]) -> String` |
| `crates/lode-core/src/mining/mod.rs` | Re-exports |
| `crates/lode-core/src/corpus/miner.rs` | Add `begin_format` to trait |
| `crates/lode-core/src/corpus/mod.rs` | Call `begin_format` in `run_corpus` |
| `crates/lode-parse/tests/corpus.rs` | Enable PA + determinism tests |
| `RFC/RFC-0003-Template-Mining-System.md` | Note on nginx `d` (doc-only, small) |
| `docs/phase-1-spec.md` | Status → Accepted when gate passes |

---

## Task 0 — Plumbing & trait hook (prerequisite)

**Why:** `run_corpus` uses one miner across formats; nginx needs `d=5` only during nginx lines and a clean tree per format.

**Files:**
- Modify: `crates/lode-core/src/corpus/miner.rs`
- Modify: `crates/lode-core/src/corpus/mod.rs`

- [ ] **Step 1: Extend `CorpusMiner` with `begin_format`**

```rust
// crates/lode-core/src/corpus/miner.rs
pub trait CorpusMiner {
    /// Reset per-format state (new parse tree, format-specific params). Default: no-op.
    fn begin_format(&mut self, _format_id: &str) {}

    fn mine_line(&mut self, raw: &str) -> String;
}
```

- [ ] **Step 2: Call at start of each format in `run_corpus`**

```rust
// crates/lode-core/src/corpus/mod.rs — inside `for format in &input.formats`
let format_id = &format.spec.id;
miner.begin_format(format_id);
```

- [ ] **Step 3: Verify `StubMiner` still compiles; run core tests**

```bash
cargo test -p lode-core
```

Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(core): add CorpusMiner::begin_format for per-format mining state"
```

---

## Task 1 — T1.1a Tokenizer (`lode-parse`)

**Spec ref:** phase-1-spec § T1.1, decisions #2–3.

**Files:**
- Create: `crates/lode-parse/src/error.rs`
- Create: `crates/lode-parse/src/tokenize.rs`
- Modify: `crates/lode-parse/src/lib.rs`
- Create: `crates/lode-parse/tests/tokenize_corpus.rs`

- [ ] **Step 1: `ParseError`**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    EmptyLine,
    LineTooLong { len: usize, max: usize },
}
```

Use `lode_core::MAX_RAW_LINE_BYTES` for max.

- [ ] **Step 2: Write failing tests — one line per format**

```rust
// crates/lode-parse/tests/tokenize_corpus.rs
use lode_parse::tokenize;

#[test]
fn nginx_bracket_and_quoted_tokens() {
    let raw = r#"127.0.0.1 - - [10/Oct/2024:13:55:36 +0000] "GET /api/users/12 HTTP/1.1" 200 1500"#;
    let tokens: Vec<_> = tokenize(raw).unwrap().into_iter().map(|t| t.as_str().to_string()).collect();
    assert_eq!(tokens[0], "127.0.0.1");
    assert_eq!(tokens[3], "[10/Oct/2024:13:55:36 +0000]");
    assert_eq!(tokens[4], r#""GET /api/users/12 HTTP/1.1""#);
}

#[test]
fn syslog_pri_version_iso_hostport() {
    let raw = "<134>1 2024-10-10T13:55:36.123Z host01 myapp - - - User login successful for user 100";
    let tokens: Vec<_> = tokenize(raw).unwrap().into_iter().map(|t| t.as_str().to_string()).collect();
    assert_eq!(tokens[0], "<134>1");
    assert_eq!(tokens[1], "2024-10-10T13:55:36.123Z");
    // ...
}

#[test]
fn json_preserves_delimiters_and_spaces() {
    let raw = r#"{ "level" : "info" , "msg" : "user login" , "user_id" : 100 }"#;
    let tokens: Vec<_> = tokenize(raw).unwrap().into_iter().map(|t| t.as_str().to_string()).collect();
    assert_eq!(tokens, vec!["{", "\"level\"", ":", "\"info\"", ",", /* ... */]);
}
```

- [ ] **Step 3: Run tests — expect FAIL**

```bash
cargo test -p lode-parse tokenize_corpus -- --nocapture
```

- [ ] **Step 4: Implement `tokenize`**

Scanner rules (from spec):

1. Skip whitespace.
2. **JSON fast-path** if first non-space is `{`: emit `{ } , :` as tokens; quoted strings atomic; other runs until delimiter/space.
3. **Generic path:** `[...]` atomic; `"..."` atomic; `<digits>…` PRI+version as `<134>1` single token; `IP:port` atomic when `is_host_port`; ISO `YYYY-MM-DDTHH:MM:SS.sssZ` via scan or regex-free digit/`T`/`Z` check; default run until space/structural char.

Export: `pub fn tokenize(raw: &str) -> Result<Vec<Token>, ParseError>`.

- [ ] **Step 5: Run tests — expect PASS**

```bash
cargo test -p lode-parse tokenize_corpus
```

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(parse): structural tokenizer with JSON fast-path (T1.1a)"
```

---

## Task 2 — T1.1b Masker (`lode-parse`)

**Spec ref:** phase-1-spec decisions #4–7, `MaskKind::EVAL_ORDER`.

**Files:**
- Create: `crates/lode-parse/src/mask.rs`
- Modify: `crates/lode-parse/src/lib.rs`
- Create: `crates/lode-parse/tests/mask_corpus.rs`

- [ ] **Step 1: Predicate helpers** (`is_num`, `is_ip`, `is_uuid`, `is_hex`, `is_path`, `is_ts_bracket`, `is_ts_iso`, `is_host_port`, `mask_pri_ver`)

Implement char-class checks per spike / phase-1-spec. **Order:** iterate `MaskKind::EVAL_ORDER` for plain tokens.

- [ ] **Step 2: Composite rules**

| Token shape | Output |
|-------------|--------|
| `<131>1` (PRI+ver) | `<NUM>1` (partial replace in-token) |
| `10.0.1.1:8001` | `<IP>:<NUM>` |
| `"GET /path HTTP/1.1"` | `"GET <PATH> HTTP/1.1"` |
| `"uuid"` / `"hex"` quoted | `"<UUID>"` / `"<HEX>"` |

Record captures in `MaskedTokens.placeholders` as `(placeholder, original_value)`.

- [ ] **Step 3: Failing test — mask-only PA per format**

```rust
// crates/lode-parse/tests/mask_corpus.rs
use lode_parse::{tokenize_and_mask, pattern_to_string}; // pattern_to_string from core or local join

fn mask_pattern(raw: &str) -> String {
    let masked = tokenize_and_mask(raw).unwrap();
    pattern_to_string(&masked.tokens)
}

#[test]
fn mask_only_pa_nginx_sample() {
    let raw = r#"127.0.0.1 - - [10/Oct/2024:13:55:36 +0000] "GET /api/users/12 HTTP/1.1" 200 1500"#;
    let exp = r#"<IP> - - <TS> "GET <PATH> HTTP/1.1" <NUM> <NUM>"#;
    assert_eq!(mask_pattern(raw), exp);
}
```

Add loop test loading all 165 lines + labels (reuse `corpus_loader`) asserting mask-only PA == 1.0.

- [ ] **Step 4: Implement `mask` + `tokenize_and_mask`**

```rust
pub fn mask(tokens: Vec<Token>) -> MaskedTokens;
pub fn tokenize_and_mask(raw: &str) -> Result<MaskedTokens, ParseError>;
```

- [ ] **Step 5: Run mask corpus tests**

```bash
cargo test -p lode-parse mask_corpus
```

Expected: 165/165 mask-only match.

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(parse): char-class masker with composite rules (T1.1b)"
```

---

## Task 3 — T1.2 Drain engine (`lode-core`)

**Spec ref:** phase-1-spec § T1.2, decisions #8–10, #12.

**Files:**
- Create: `crates/lode-core/src/mining/pattern.rs`
- Create: `crates/lode-core/src/mining/drain.rs`
- Modify: `crates/lode-core/src/mining/mod.rs`
- Modify: `crates/lode-core/src/lib.rs` (re-export `DrainState`, `ProcessResult`)

- [ ] **Step 1: `pattern_to_string`**

```rust
pub fn pattern_to_string(tokens: &[Token]) -> String {
    tokens.iter().map(|t| t.as_str()).collect::<Vec<_>>().join(" ")
}
```

- [ ] **Step 2: Failing drain unit tests**

```rust
// in drain.rs #[cfg(test)]
#[test]
fn route_separates_nginx_verbs_at_d5() {
    let mut state = DrainState::new(MiningParams { depth: 5, ..Default::default() });
    let get = masked_tokens(r#"<IP> - - <TS> "GET <PATH> HTTP/1.1" <NUM> <NUM>"#);
    let post = masked_tokens(r#"<IP> - - <TS> "POST <PATH> HTTP/1.1" <NUM> <NUM>"#);
    let p1 = state.process(&get).pattern;
    let p2 = state.process(&post).pattern;
    assert_ne!(p1, p2);
    assert!(!p1.contains("<*>"));
}

#[test]
fn widen_does_not_touch_routing_prefix() { /* d=4, force mismatch at pos >= 4 */ }

#[test]
fn determinism_same_input_same_template_id() {
    let mut a = DrainState::new(MiningParams::default());
    let mut b = DrainState::new(MiningParams::default());
    let m = sample_masked();
    assert_eq!(a.process(&m).pattern, b.process(&m).pattern);
}
```

- [ ] **Step 3: Implement `DrainState`**

Core logic:

```rust
pub struct ProcessResult {
    pub template_id: Option<TemplateId>,
    pub pattern: String,
    pub fingerprint: Fingerprint,
}

pub struct DrainState {
    registry: TemplateRegistry,
    params: MiningParams,
    next_id: u64,
}

impl DrainState {
    pub fn new(params: MiningParams) -> Self;
    pub fn reset(&mut self, params: MiningParams); // for begin_format
    pub fn process(&mut self, masked: &MaskedTokens) -> ProcessResult;
}
```

**Routing key:** FNV-1a over `len` + `0x1e` + `tok[0..min(d,len))` bytes.

**Leaf match:** `simSeq = equal_positions / len` (wildcard `<*>` matches any); best ≥ `st`; tie-break lower `TemplateId`.

**Widen:** differing indices `j >= min(d, len)` only; set token to `MaskKind::Wildcard.placeholder()`.

**Registry:** `Vec<Template>` + `HashMap<u64, Vec<TemplateId>>` buckets; `occurrence_count++`; state `Emerging` (promote to `Stable` at `stable_threshold` — optional for gate).

**Eviction:** stub — if `len(registry) > max_templates`, no-op or `todo!` with comment T6.1 (corpus never hits limit).

- [ ] **Step 4: Run core mining tests**

```bash
cargo test -p lode-core mining::drain
```

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(core): DrainState with FNV routing, similarity, widen-only (T1.2)"
```

---

## Task 4 — T1.3 Wire `DrainMiner` (`lode-parse`)

**Spec ref:** phase-1-spec § T1.3.

**Files:**
- Create: `crates/lode-parse/src/miner.rs`
- Modify: `crates/lode-parse/src/lib.rs`

- [ ] **Step 1: Implement `DrainMiner`**

```rust
pub struct DrainMiner {
    state: DrainState,
}

impl DrainMiner {
    pub fn new() -> Self {
        Self { state: DrainState::new(MiningParams::default()) }
    }
}

impl CorpusMiner for DrainMiner {
    fn begin_format(&mut self, format_id: &str) {
        let params = if format_id == "nginx-access" {
            MiningParams { depth: 5, ..MiningParams::default() }
        } else {
            MiningParams::default()
        };
        self.state.reset(params);
    }

    fn mine_line(&mut self, raw: &str) -> String {
        match tokenize_and_mask(raw) {
            Ok(masked) => self.state.process(&masked).pattern,
            Err(_) => String::new(), // corpus has no bad lines; T3.2 adds degraded handling
        }
    }
}
```

- [ ] **Step 2: Export from `lib.rs`**

```rust
pub mod error;
pub mod mask;
pub mod miner;
pub mod tokenize;

pub use error::ParseError;
pub use mask::{mask, tokenize_and_mask};
pub use miner::DrainMiner;
pub use tokenize::tokenize;
```

- [ ] **Step 3: Smoke test in `miner.rs`**

```rust
#[test]
fn mines_nginx_line() {
    let mut m = DrainMiner::new();
    m.begin_format("nginx-access");
    let pat = m.mine_line(r#"127.0.0.1 - - [10/Oct/2024:13:55:36 +0000] "GET /api/users/12 HTTP/1.1" 200 1500"#);
    assert!(pat.contains("GET <PATH>"));
}
```

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(parse): DrainMiner wiring tokenize/mask/drain (T1.3)"
```

---

## Task 5 — T1.4 Quality gate

**Files:**
- Modify: `crates/lode-parse/tests/corpus.rs`
- Modify: `RFC/RFC-0003-Template-Mining-System.md` (short note §6.2)
- Modify: `docs/phase-1-spec.md` (status)

- [ ] **Step 1: Enable `corpus_pa_meets_floor`**

```rust
use lode_core::{assert_deterministic, run_corpus};
use lode_parse::DrainMiner;

#[test]
fn corpus_pa_meets_floor() {
    let input = load_corpus(corpus_root());
    let mut miner = DrainMiner::new();
    let result = run_corpus(&input, &mut miner).expect("evaluate");
    for format in &input.formats {
        let pa = result.per_format_pa[&format.spec.id];
        assert!(pa >= format.spec.pa_floor, "{}: pa={pa}", format.spec.id);
    }
}

#[test]
fn corpus_mining_is_deterministic() {
    let input = load_corpus(corpus_root());
    assert_deterministic(&input, DrainMiner::new).expect("deterministic");
}
```

Remove `#[ignore]` from PA test.

- [ ] **Step 2: Run full parse + core tests**

```bash
cargo test -p lode-parse
cargo test -p lode-core
```

Expected: all pass including 165-line corpus.

- [ ] **Step 3: RFC note (doc-only)**

Add under RFC-0003 §6.2 after depth bullet:

> **Note:** Combined log formats (e.g. nginx `IP - - [ts] "METHOD …"`) may require `d > 4` so the HTTP method token falls within the routing prefix. Default remains `4`; per-stream override is valid (§11).

- [ ] **Step 4: Update phase-1-spec status → Accepted**

- [ ] **Step 5: Commit**

```bash
git commit -m "test(parse): enable corpus PA gate >= 0.90 and determinism (T1.4)"
```

---

## Task 6 — T1.4 optional: throughput bench

**Only if time permits on branch.**

**Files:**
- Create: `crates/lode-parse/benches/mining.rs`
- Modify: `crates/lode-parse/Cargo.toml` (`[dev-dependencies] criterion`)

- [ ] Bench `tokenize_and_mask` + `DrainState::process` over nginx sample × N lines.

- [ ] Record baseline in `docs/experimental/phase1-rust-bench.md` (compare to spike ~1M l/s target order-of-magnitude).

---

## Verification checklist (before merge)

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
# CI equivalent:
cargo test -p lode-core  # includes core-no-deps
cargo test -p lode-parse # includes corpus gate
```

| Check | Expected |
|-------|----------|
| `corpus_pa_meets_floor` | pass, each format PA ≥ 0.90 |
| `corpus_mining_is_deterministic` | pass |
| `mask_corpus` all lines | 165/165 exact pattern |
| `MiningParams::default().depth` | still `4` (RFC default) |
| `lode-core` Cargo.toml `[dependencies]` | empty |

---

## Suggested commit sequence (branch)

1. `feat(core): CorpusMiner::begin_format`
2. `feat(parse): structural tokenizer (T1.1a)`
3. `feat(parse): char-class masker (T1.1b)`
4. `feat(core): DrainState (T1.2)`
5. `feat(parse): DrainMiner (T1.3)`
6. `test(parse): corpus PA gate (T1.4)`
7. `docs: RFC-0003 nginx depth note` (can squash into 6)

---

## Spec coverage self-review

| Spec requirement | Task |
|------------------|------|
| Tokenizer structural + JSON fast-path | Task 1 |
| Mask RFC order + composites | Task 2 |
| FNV flat bucket routing | Task 3 |
| widen-only, routing no-widen | Task 3 tests |
| `d=5` nginx per-stream | Task 4 `begin_format` |
| `st=0.5`, default `d=4` | Task 3 defaults + Task 4 override |
| Fingerprint on masked tokens | Task 3 `ProcessResult` (exists in core) |
| PA ≥ 0.90 + determinism | Task 5 |
| RFC doc note | Task 5 |

**Out of scope (explicit):** eviction, batch re-mine, URL/EMAIL masks, lifecycle events, `LogEvent` mutation, criterion unless Task 6.

---

## Execution options

**Plan saved to:** `docs/plans/phase-1-mining-implementation.md`

1. **Inline** — implement Tasks 0→5 sequentially in this branch, reviewing after each task.
2. **Task-by-task agents** — one session per task with test gate between commits.

Which approach do you prefer to start implementation?
