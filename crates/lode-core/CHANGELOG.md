# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.0](https://github.com/carvalhosauro/lode/releases/tag/lode-core-v0.0.0) - 2026-07-05

### Added

- *(core)* DrainState with FNV routing, similarity, widen-only (T1.2)
- *(parse)* char-class masker with composite rules (T1.1b)
- *(core)* add CorpusMiner::begin_format for per-format mining state
- *(core)* add in-memory corpus evaluation harness
- *(core)* add mining types, Attributes, and fingerprint hashing
- *(core)* add domain types and severity model

### Fixed

- satisfy clippy -D warnings in drain and mask modules
- *(core)* fingerprint from event masked tokens (DEC-005)
- *(ci)* resolve rustfmt, clippy, and test failures on PR 4

### Other

- *(core)* enforce append-only template registry invariant
- add cargo workspace and crate skeletons
