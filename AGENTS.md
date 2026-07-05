# AGENTS.md

## Cursor Cloud specific instructions

Lode is a **Rust Cargo workspace** that builds a single CLI binary (`lode`). It is
pre-alpha: `crates/lode-cli` is a clap-based scaffold that prints its version, and the
other crates (`lode-core`, `lode-storage`, `lode-parse`, `lode-source`, `lode-tui`)
hold domain types/algorithms under active construction. There is **no server, GUI, or
external service** — all verification is terminal-driven.

The Rust toolchain is pinned in `rust-toolchain.toml` (stable) and installed
automatically by `rustup`. The update script runs `cargo fetch` to pre-download
dependencies; `cargo-nextest` is also installed on startup (prebuilt binary).

Standard commands are documented in `README.md` / `CONTRIBUTING.md` and enforced in
`.github/workflows/ci.yml`. Quick reference:

- Build: `cargo build --workspace`
- Lint: `cargo fmt --all --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Test: `cargo nextest run --workspace` (falls back to `cargo test --workspace`), plus `cargo test --workspace --doc`
- Run: `cargo run -p lode-cli -- --help` (or `--version`)

Non-obvious caveats:
- `lode-core` must have **zero external dependencies** (CI job `core-no-deps` enforces
  this via `cargo tree`). Do not add crates to its dependency tree.
- `unsafe` is `#![forbid]` in `lode-core` and `deny` elsewhere; new `unsafe` must be
  justified.
- Do not set `panic = "abort"` in the release profile — the runtime relies on unwinding
  to isolate worker panics (see comment in `Cargo.toml`).
- `RUSTFLAGS="-D warnings"` is used in CI; keep clippy/warnings clean.
- Extra CI-only tooling (`cargo-deny`, `typos-cli`, `git-cliff`, `release-plz`,
  `lefthook`) is not installed by default; install on demand if you need to reproduce
  those specific checks.
