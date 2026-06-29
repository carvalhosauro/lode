//! [`LogStream`] — an origin of continuous or batch events (RFC-0000 §5.2).

use crate::ids::StreamId;

/// The kind of origin a stream reads from (RFC-0001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceType {
    File,
    Stdin,
    Docker,
    Journald,
}

/// How a stream is consumed (RFC-0001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamMode {
    Batch,
    Tail,
    Hybrid,
}

/// An origin of events. Carries no parsing logic and no business rules (RFC-0000).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogStream {
    pub id: StreamId,
    pub source_type: SourceType,
    pub mode: StreamMode,
    /// Free-form origin context (path, container id, unit, …).
    pub metadata: Vec<(Box<str>, Box<str>)>,
}
