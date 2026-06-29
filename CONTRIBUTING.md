# Contributing to Lode

## Setup

The toolchain is pinned in `rust-toolchain.toml` — `rustup` installs it automatically.
Install the dev tools and the git hooks:

```sh
# git hooks (fmt + clippy on commit, conventional-commit check, tests on push)
cargo install lefthook && lefthook install

# quality + release tooling used by CI
cargo install cargo-nextest cargo-deny git-cliff release-plz typos-cli
```

## Before you push

The pre-commit and pre-push hooks run these; you can run them by hand:

```sh
cargo fmt --all                                  # format
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace                    # tests
cargo deny check                                 # licenses + advisories
typos                                            # spelling
```

## Rules

- **`lode-core` has zero external dependencies.** Only `std`. CI fails if anything is
  added to its dependency tree. Anything touching IO, parsing, or the terminal goes in
  another crate (`lode-storage`, `lode-parse`, `lode-source`, `lode-tui`).
- **No `unsafe` in `lode-core`** (`#![forbid(unsafe_code)]`); elsewhere it is `deny` and
  must be explicitly justified.
- **Match the RFCs.** Behavior is specified in [`RFC/`](./RFC/README.md). If your change
  diverges, update the RFC in the same PR. Preserve the determinism invariants and the
  acceptance bars (mining PA ≥ 0.90, insight precision ≥ 0.80).

## Commits

[Conventional Commits](https://www.conventionalcommits.org/). Types: `feat`, `fix`,
`docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Scope is
usually the crate (`feat(parse): …`). The `commit-msg` hook enforces the format.

## Releases

Automated. `release-plz` opens a "Release vX" PR on pushes to `main` (version bump +
`CHANGELOG.md` from the commits). Merging it tags the repo; the tag triggers
`release.yml`, which builds the static binaries for Linux + macOS and attaches them to
the GitHub Release. No manual version edits. Lode is an app — nothing is published to
crates.io.
