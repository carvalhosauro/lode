# Lode — Implementation Tasks

Order follows three principles: **(1) risk-first** — the mining engine is validated
against the golden corpus before the body is built; **(2) dependency-respecting** — the
RFC graph (core → IO → runtime → query/insight → TUI/CLI); **(3) vertical slice early**
— a headless end-to-end pipeline as soon as possible, then thicken.

Each task: crate · RFC · gate. 🏁 = non-return milestone.

## Phase 0 — Foundation

- [x] **T0.1** Rebalance RFC-0012 to `Result`+ownership primary, supervisor as panic safety-net · docs · RFC-0012
- [x] **T0.2** Domain types in `lode-core` (LogEvent/LogStream/Template/Insight + canonical newtypes `source_offset`/`index_time`/`row_anchor`, Severity) · RFC-0000/0017 · gate: core-no-deps
- [x] **T0.3** Golden corpus harness + fixtures (labeled standard formats, PA metric, determinism test) · RFC-0003/0005

## Phase 1 — Mining engine (differentiator + risk #1)

- [ ] **T1.1** Tokenizer + char-class masking dictionary (NUM/IP/UUID/HEX/PATH/TS) · `lode-parse` · RFC-0003 §6.1
- [ ] **T1.2** Drain parse tree + template registry (fixed-depth, similarity, widen-only, bounded, deterministic) · `lode-core` · RFC-0003
- [ ] **T1.3** Fingerprint + `template_id` resolution · `lode-core`
- [ ] 🏁 **T1.4 GATE: PA ≥ 0.90 + determinism on the golden corpus** (+ criterion throughput)

## Phase 2 — Ingest + enrich + storage

- [ ] **T2.1** `SourceAdapter` trait + file/stdin adapters (tail: rotation/truncation/resume, `source_offset`) · `lode-source` · RFC-0001
- [ ] **T2.2** Enrichment: timestamp (RFC-0006 minimal) + severity model + attributes · `lode-parse` · RFC-0017
- [ ] **T2.3** Append-only storage: IndexSegment + durable cursor + `index_time` + cold start · `lode-storage` · RFC-0002
- [ ] 🏁 **T2.4** Wire headless: file → ingest → enrich → mine → segment

## Phase 3 — Runtime (daemon, per-stream)

- [ ] **T3.1** Runtime: one worker per stream, `Result` errors, thin supervisor respawn-from-cursor, bounded channels/backpressure · RFC-0012
- [ ] **T3.2** Failure handling: `unparsed`, degraded mode · RFC-0013
- [ ] **T3.3** Event/telemetry bus + counters (minimal) · RFC-0011/0009

## Phase 4 — Query + insight

- [ ] **T4.1** Filter model: template_id/severity/time/text + AND/OR, streaming results · `lode-core` · RFC-0004
- [ ] **T4.2** Insight v1: spike + rare + emergence (EWMA baseline + warmup + precision pipeline), then regression · RFC-0005
- [ ] 🏁 **T4.3 GATE: insight precision ≥ 0.80 on the corpus**

## Phase 5 — TUI + CLI (the face / demo)

- [ ] **T5.1** CLI verbs `ingest`/`open`/`query`/`version` + config · `lode-cli` · RFC-0016
- [ ] **T5.2** TUI (editxr-style custom render): virtualized viewport, template panel, fuzzy nav, follow tail, insights · `lode-tui` · RFC-0008
- [ ] **T5.3** Redaction at ingest (mask secrets/PII before raw commit) · RFC-0015
- [ ] 🏁 **T5.4** The demo: logs in → auto-cluster → spike flagged → fuzzy-jump (= v1 MVP + GIF)

## Phase 6 — Release v0.1

- [ ] **T6.1** Retention basics + perf budget checks + extra masks · RFC-0009
- [ ] 🏁 **T6.2** First release (binaries via release pipeline) + README demo GIF

---

**Critical path:** T0.2 → T0.3 → T1.\* → 🏁 PA → T2.\* → vertical slice → daemon →
intelligence → TUI → release. The two gates (PA ≥ 0.90, precision ≥ 0.80) are the
points of no return — if mining misses PA, rethink before spending on Phases 2–5.