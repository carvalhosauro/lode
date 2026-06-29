<!-- Title must follow Conventional Commits, e.g. "feat(core): add parse tree". -->

## What & why

<!-- What does this change and why. Link the RFC/issue it relates to. -->

## Checklist

- [ ] `cargo fmt --check`, `cargo clippy -- -D warnings`, and tests pass locally
- [ ] `lode-core` still has zero external dependencies (if touched)
- [ ] Behavior matches the relevant RFC (link it), or the RFC is updated in this PR
- [ ] Determinism / acceptance bars preserved (PA ≥ 0.90 mining, precision ≥ 0.80 insights) where relevant
