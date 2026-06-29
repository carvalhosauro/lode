# Lode Roadmap

> **Name:** `Lode`

> **Status: alpha.** Pre-1.0 — contracts may still change.
> No dates. Order ≈ priority. Each item links the RFC that specifies it.
> See [`RFC/`](./RFC) for the full design.

Lode is a **local-first log investigation engine**: point it at a log stream and it
automatically mines the recurring structure, lets you query it, and surfaces what
changed — all in one binary, no server, no cluster. The design bet is
*template mining + insights + structured query, integrated locally*, not feature
count. The moat is integration and locality; the differentiator is the mining engine.

Implementation language: **Rust** — chosen after a practical mining-core spike (`spike/`)
that showed Rust leads on the per-line masking/ingest hot path. RFC contracts are
expressed in Rust idiom (`Result`, traits).

---

## Now — v1 (MVP)

The smallest version that beats `grep`/`lnav` and proves the central bet: that
automatic template mining on a laptop is genuinely useful. The mining engine ships
in v1 **on purpose** — it is the differentiator and the biggest risk, so it must be
validated first, not last.

- [ ] Ingestion: file + stdin, with tail semantics (rotation / truncation / resume) — RFC-0001
- [ ] Append-only segment storage with offset + search index — RFC-0002
- [ ] **Automatic template mining (online, Drain-style) + fingerprint fallback** — RFC-0003
- [ ] Minimal LodeQL: filter by `template_id` / severity / time-range / text, `AND` / `OR` — RFC-0004
- [ ] Minimal insight set: **frequency spike + rare-event** detection (falls straight out of mining) — RFC-0005
- [ ] Timestamp parsing, `nil` fallback, per-stream ordering — RFC-0006
- [ ] Ephemeral Workspace: active filters, selection, query history — RFC-0007
- [ ] TUI: virtualized viewport, fuzzy (fzf-like) navigation, pattern/template panel, follow mode — RFC-0008
- [ ] OTP supervision, per-stream fault isolation (one stream down ≠ all down) — RFC-0012
- [ ] Failure handling: `unparsed` marking, raw always preserved, local containment — RFC-0013
- [ ] Domain + telemetry events, structured internal logging — RFC-0009, RFC-0011
- [ ] **Redaction at ingest** (mask secrets / tokens / PII) — RFC-0015
- [ ] CLI: `ingest` / `open` / `query` / `version`, plus a config surface — RFC-0016

**Explicitly not in v1:** docker/journald sources, the full insight engine, temporal /
cross-stream query operators, Workspace persistence, plugins, retention/GC policy,
encryption-at-rest, distribution. v1 ships a **minimal insight set** (spike + rare
only), mirroring "validate the brain before building the body."

---

## Next — v2

Where Lode stops being "a smarter pager" and becomes a tool people adopt.

- [ ] Sources: docker / journald / podman / compose — RFC-0001
- [ ] Full Insight engine: anomaly, regression, baseline-per-template, cross-stream correlation — RFC-0005, RFC-0006
- [ ] LodeQL temporal `AND`/`OR` + sequence + cross-stream correlation operators — RFC-0004
- [ ] Workspace snapshot persistence (save / restore an investigation) — RFC-0007
- [ ] Retention / GC policy (time- and size-based eviction) — RFC-0002
- [ ] Insight suppression + confidence thresholds + precision tuning (anti false-positive) — RFC-0005
- [ ] Plugin system + public Behaviour contracts (custom parsers / insights / sources) — RFC-0010, RFC-0014

---

## Later

- [ ] Encryption-at-rest + full access model (authz, multi-tenant, stream-level access) — RFC-0015
- [ ] Distribution / multi-host stream aggregation (still local-first, merges remote streams)
- [ ] Web UI / HTTP API as an alternate renderer over the same engine — RFC-0008
- [ ] Spill-to-disk for large correlation / sequence queries over wide windows — RFC-0004
- [ ] More sources: Kubernetes pods, syslog, cloud log APIs
- [ ] Historical replay / backtest of insight rules over sealed segments
- [ ] Saved queries + alerting on LodeQL conditions

---

## Non-Goals

Lode deliberately does **not** do these. Saying no keeps the core sharp.

- **Not a clustered log warehouse or SIEM.** That is Loki / Quickwit / Elastic territory. Lode is local-first and single-node.
- **Not a pipeline / forwarder.** That is Vector / Fluent Bit. Lode ingests to *investigate*, not to route data elsewhere.
- **Not long-term compliance retention storage.** Lode is an investigation engine, not an archive.
- **Not a metrics / tracing system.** Logs (events) only — though insights are log-derived.
- **No fleet / agent management** in the core.
- **Not a hosted SaaS.** A local binary you run yourself.

---

## Versioning

- [Semantic Versioning](https://semver.org/spec/v2.0.0.html). `0.x` until contracts stabilize; `1.0` marks a stable **LodeQL grammar + Behaviour contracts + segment `schema_version`**.
- Each roadmap item maps to a GitHub milestone backed by its RFC.

> Replace `carvalhosauro/lode` references with the real repo path when publishing.
