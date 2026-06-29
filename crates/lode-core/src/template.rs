//! [`Template`] — a semantic grouping of structurally similar events (RFC-0000 §5.4,
//! RFC-0003).

use crate::ids::{IndexTime, TemplateId};

/// Lifecycle state of a template (RFC-0003 §6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateState {
    /// Newly created, accumulating evidence.
    Emerging,
    /// Recurring reliably past the stabilization threshold.
    Stable,
    /// Pattern is widening; a new version is forming.
    Evolving,
    /// Not seen within the retention window; kept for history.
    Retired,
    /// Demoted under the memory bound; revivable in batch re-mine.
    Cold,
}

/// A derived grouping of similar events. Never ingested (RFC-0003 DEC-001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub id: TemplateId,
    /// Constant skeleton with typed placeholders and `<*>` wildcards.
    pub pattern: Box<str>,
    /// Bumped on widen / split / merge (RFC-0003 §6.4).
    pub version: u32,
    /// Eventually-consistent accumulator (RFC-0003 DEC-007).
    pub occurrence_count: u64,
    pub first_seen: IndexTime,
    pub last_seen: IndexTime,
    pub state: TemplateState,
}
