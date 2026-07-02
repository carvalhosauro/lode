//! Lode parsing & masking — tokenization plus the typed-variable masking dictionary
//! (RFC-0003 §6.1) and severity markers (RFC-0017). May take dependencies (e.g. regex)
//! for rich/custom masks; common masks stay char-class fast-paths (see `spike/`).

pub mod error;
pub mod tokenize;

pub use error::ParseError;
pub use tokenize::tokenize;
