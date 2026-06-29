//! Lode core — pure, dependency-free domain types and algorithms.
//!
//! This crate MUST NOT take external dependencies; only the standard library.
//! The emptiness of its dependency tree is enforced in CI (`core-no-deps`).
//! It holds the domain model (RFC-0000) and the pure algorithms: template mining
//! (RFC-0003), index/segment logic (RFC-0002), query evaluation (RFC-0004),
//! insight statistics (RFC-0005), and time/ordering (RFC-0006). Anything that
//! touches IO, the filesystem, or the terminal belongs in another crate.
#![forbid(unsafe_code)]

pub mod event;
pub mod ids;
pub mod insight;
pub mod stream;
pub mod template;

pub use event::{LogEvent, Provenance, Severity};
pub use ids::{
    EventId, Fingerprint, IndexTime, RowAnchor, SegmentId, SegmentPosition, SourceOffset, StreamId,
    TemplateId, Timestamp,
};
pub use insight::{Confidence, Insight, InsightKind};
pub use stream::{LogStream, SourceType, StreamMode};
pub use template::{Template, TemplateState};

/// Crate version, surfaced by the CLI.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
