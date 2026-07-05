# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.0](https://github.com/carvalhosauro/lode/releases/tag/lode-parse-v0.0.0) - 2026-07-05

### Added

- *(parse)* DrainMiner wiring tokenize/mask/drain (T1.3)
- *(parse)* char-class masker with composite rules (T1.1b)
- *(parse)* structural tokenizer with JSON fast-path (T1.1a)

### Fixed

- *(parse)* mask long decimals as NUM and handle HTTP/2 and HTTP/3
- *(parse)* stop tokenizer panics on multibyte and unterminated input
- satisfy clippy -D warnings in drain and mask modules
- *(ci)* resolve rustfmt, clippy, and test failures on PR 4

### Other

- *(parse)* add reproducible mining benchmark over the golden corpus
- *(parse)* short-circuit params_for override lookup
- *(parse)* apply rustfmt to corpus test files
- *(parse)* table-drive mining overrides and guard begin_format
- *(parse)* enable corpus PA gate >= 0.90 and determinism (T1.4)
- *(parse)* load golden corpus fixtures with serde and toml
- add cargo workspace and crate skeletons
