# Lode RFCs

The design of Lode, one stable piece per document. Conventions: English, Mermaid
diagrams, numbered sections, explicit Decisions (DEC-xxx), and a glossary. Contracts are
expressed in Rust idiom (`Result`, `Option`, traits).

## Core

- [RFC-0000 — Domain Model](./RFC-0000-Domain-Model.md) — entities and invariants; base of everything.

## Ingestion, storage, structure

- [RFC-0001 — Ingestion Model](./RFC-0001-Ingestion-Model.md) — file/docker/stdin/journald, tail, backpressure.
- [RFC-0002 — Storage & Indexing Engine](./RFC-0002-Storage-And-Indexing-Engine.md) — append-only segments, cursor, cold start.
- [RFC-0003 — Template Mining System](./RFC-0003-Template-Mining-System.md) — Drain-style mining, masking, determinism (PA ≥ 0.90). **Differentiator.**
- [RFC-0017 — Enrichment & Severity Model](./RFC-0017-Enrichment-And-Severity-Model.md) — timestamp/severity/attribute derivation.

## Intelligence & query

- [RFC-0004 — Query Engine (LodeQL)](./RFC-0004-Query-Engine.md) — AST, evaluation, streaming, temporal composition.
- [RFC-0005 — Insight Engine](./RFC-0005-Insight-Engine.md) — statistical detectors + precision pipeline (precision ≥ 0.80).
- [RFC-0006 — Time System & Ordering](./RFC-0006-Time-System-And-Ordering.md) — timestamps, partial ordering.

## Experience

- [RFC-0007 — Workspace Model](./RFC-0007-Workspace-Model.md) — investigation session state.
- [RFC-0008 — Rendering Layer (TUI)](./RFC-0008-Rendering-Layer-TUI.md) — virtualized viewport, fuzzy nav.

## Observability, runtime, resilience

- [RFC-0009 — Performance Budget & Telemetry Model](./RFC-0009-Performance-Budget-And-Telemetry-Model.md)
- [RFC-0011 — Event & Telemetry Bus](./RFC-0011-Event-And-Telemetry-Bus.md)
- [RFC-0012 — Execution Runtime Model](./RFC-0012-Execution-Runtime-Model.md) — supervisor + per-stream workers (Rust async/threads).
- [RFC-0013 — Failure Handling & Recovery Model](./RFC-0013-Failure-Handling-And-Recovery-Model.md)

## Extensibility & contracts

- [RFC-0010 — Plugin System](./RFC-0010-Plugin-System.md) — custom parsers/insights/sources via traits.
- [RFC-0014 — Trait Contracts](./RFC-0014-Trait-Contracts.md) — the trait seams between components.
- [RFC-0015 — Security & Access Model](./RFC-0015-Security-And-Access-Model.md) — redaction-at-ingest first.
- [RFC-0016 — Configuration & CLI Model](./RFC-0016-Configuration-And-CLI-Model.md) — config + the `lode` verbs.

## Recommended reading order

Foundation: 0000 → 0001 → 0002 → 0017 → 0003. Intelligence: 0004 → 0005 → 0006.
Experience: 0007 → 0008. Robustness: 0012 → 0013 → 0009. Ecosystem: 0010 → 0014 → 0015 → 0016.
