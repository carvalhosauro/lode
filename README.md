# Lode

**Local-first log investigation engine.** Point it at a log stream and it automatically
mines the recurring structure, lets you query it, and surfaces what changed — in one
static binary, no server, no cluster.

> **Status: pre-alpha (design phase).** The design is specified in [`RFC/`](./RFC); the
> engine is being built. Contracts will change until `1.0`.

## Why

Between `grep`/`lnav` (no intelligence) and Loki/Elastic (intelligence, but a cluster to
run and operate) there's a gap: a local tool that *understands* your logs. Lode fills it.

The bet — five pillars in one binary:

- **Ingest** — file, stdin, docker, journald (batch + tail).
- **Store** — append-only segments with their own index, local.
- **Mine** — automatic Drain-style template mining (the differentiator): groups raw lines into evolving patterns. See [RFC-0003](./RFC/RFC-0003-Template-Mining-System.md).
- **Insights** — statistical, explainable spike / rare / emergence / regression detection, precision-first. See [RFC-0005](./RFC/RFC-0005-Insight-Engine.md).
- **TUI** — virtualized viewport, fuzzy navigation, multi-panel. See [RFC-0008](./RFC/RFC-0008-Rendering-Layer-TUI.md).

## Design

The full design lives in [`RFC/`](./RFC/README.md) (18 RFCs) and the plan in
[`ROADMAP.md`](./ROADMAP.md). Architecture in one line: **an event-analysis engine with a
pluggable UI** — the TUI is one interface of several.

## Build

Requires the toolchain pinned in [`rust-toolchain.toml`](./rust-toolchain.toml) (stable).

```sh
cargo build --release
./target/release/lode --version
```

Static single binary (Linux):

```sh
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
ldd target/x86_64-unknown-linux-musl/release/lode   # => "not a dynamic executable"
```

Prebuilt binaries for Linux and macOS are attached to each [release](https://github.com/carvalhosauro/lode/releases).

## Workspace

| Crate | Role | Deps |
| ----- | ---- | ---- |
| `lode-core` | pure domain types + algorithms (mining, index, query, insight) | **none** (enforced in CI) |
| `lode-storage` | append-only segments, cursor, on-disk index | yes |
| `lode-parse` | tokenization + masking dictionary | yes |
| `lode-source` | file / stdin / docker / journald adapters | yes |
| `lode-tui` | terminal UI | yes |
| `lode-cli` | the `lode` binary | yes |

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md). Commits follow
[Conventional Commits](https://www.conventionalcommits.org/); hooks via `lefthook`.

## License

[MIT](./LICENSE).
