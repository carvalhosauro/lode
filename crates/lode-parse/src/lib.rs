//! Lode parsing & masking — tokenization plus the typed-variable masking dictionary
//! (RFC-0003 §6.1) and severity markers (RFC-0017). May take dependencies (e.g. regex)
//! for rich/custom masks; common masks stay char-class fast-paths (see `spike/`).

pub mod error;
pub mod mask;
pub mod miner;
pub mod tokenize;

pub use error::ParseError;
pub use lode_core::pattern_to_string;
pub use mask::{mask, tokenize_and_mask};
pub use miner::DrainMiner;
pub use tokenize::tokenize;
