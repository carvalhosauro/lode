# Lode RFCs

The design of Lode, one stable piece per document. Conventions: English, Mermaid
diagrams, numbered sections, explicit Decisions (DEC-xxx), and a glossary. Contracts are
expressed in Rust idiom (`Result`, `Option`, traits).

**Scope tags** mirror [TASKS.md](../TASKS.md) so the risk-first ordering is visible here:
`[MVP]` is on the v1 critical path, `[MVP-minimal]` ships a reduced subset for v1 (full
spec deferred), and `[post-MVP]` is out of v1 scope. Read `[MVP]` before `[post-MVP]`.

## Core

- `[MVP]` [RFC-0000 — Domain Model](./RFC-0000-Domain-Model.md) — entities and invariants; base of everything.

## Ingestion, storage, structure

- `[MVP]` [RFC-0001 — Ingestion Model](./RFC-0001-Ingestion-Model.md) — file/docker/stdin/journald, tail, backpressure.
- `[MVP]` [RFC-0002 — Storage & Indexing Engine](./RFC-0002-Storage-And-Indexing-Engine.md) — append-only segments, cursor, cold start.
- `[MVP]` [RFC-0003 — Template Mining System](./RFC-0003-Template-Mining-System.md) — Drain-style mining, masking, determinism (PA ≥ 0.90). **Differentiator.**
- `[MVP]` [RFC-0017 — Enrichment & Severity Model](./RFC-0017-Enrichment-And-Severity-Model.md) — timestamp/severity/attribute derivation.

## Intelligence & query

- `[MVP-minimal]` [RFC-0004 — Query Engine (LodeQL)](./RFC-0004-Query-Engine.md) — AST, evaluation, streaming, temporal composition. **v1 = filter model only (template_id/severity/time/text + AND/OR); full LodeQL later.**
- `[MVP]` [RFC-0005 — Insight Engine](./RFC-0005-Insight-Engine.md) — statistical detectors + precision pipeline (precision ≥ 0.80).
- `[MVP]` [RFC-0006 — Time System & Ordering](./RFC-0006-Time-System-And-Ordering.md) — timestamps, partial ordering.

## Experience

- `[MVP]` [RFC-0007 — Workspace Model](./RFC-0007-Workspace-Model.md) — investigation session state.
- `[MVP]` [RFC-0008 — Rendering Layer (TUI)](./RFC-0008-Rendering-Layer-TUI.md) — virtualized viewport, fuzzy nav.

## Observability, runtime, resilience

- `[MVP-minimal]` [RFC-0009 — Performance Budget & Telemetry Model](./RFC-0009-Performance-Budget-And-Telemetry-Model.md)
- `[post-MVP]` [RFC-0011 — Event & Telemetry Bus](./RFC-0011-Event-And-Telemetry-Bus.md)
- `[MVP]` [RFC-0012 — Execution Runtime Model](./RFC-0012-Execution-Runtime-Model.md) — supervisor + per-stream workers (Rust async/threads).
- `[MVP]` [RFC-0013 — Failure Handling & Recovery Model](./RFC-0013-Failure-Handling-And-Recovery-Model.md)

## Extensibility & contracts

- `[post-MVP]` [RFC-0010 — Plugin System](./RFC-0010-Plugin-System.md) — custom parsers/insights/sources via traits.
- `[MVP]` [RFC-0014 — Trait Contracts](./RFC-0014-Trait-Contracts.md) — the trait seams between components.
- `[post-MVP]` [RFC-0015 — Security & Access Model](./RFC-0015-Security-And-Access-Model.md) — redaction-at-ingest first.
- `[MVP-minimal]` [RFC-0016 — Configuration & CLI Model](./RFC-0016-Configuration-And-CLI-Model.md) — config + the `lode` verbs.

## Recommended reading order

Foundation: 0000 → 0001 → 0002 → 0017 → 0003. Intelligence: 0004 → 0005 → 0006.
Experience: 0007 → 0008. Robustness: 0012 → 0013 → 0009. Ecosystem: 0010 → 0014 → 0015 → 0016.

## Status & amendments

RFCs are `Draft` until ratified. **RFC-0000** and **RFC-0003** are `Accepted` —
their derived spec is implemented and underway in PR #6. Changing an `Accepted`
RFC requires an explicit amendment: a new `DEC-xxx` decision or a changelog entry
in the RFC itself, never a silent edit.
