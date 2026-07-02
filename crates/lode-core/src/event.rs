//! [`LogEvent`] — the fundamental analyzable unit (RFC-0000 §5.3), plus the canonical
//! [`Severity`] scale and its [`Provenance`] (RFC-0017).

use crate::attributes::Attributes;
use crate::ids::Timestamp;
use crate::ids::{EventId, Fingerprint, IndexTime, SourceOffset, StreamId, TemplateId};

/// Maximum admitted raw line length in bytes.
pub const MAX_RAW_LINE_BYTES: usize = 64 * 1024;

/// The single canonical severity scale (RFC-0017 §5.2), ordered low → high.
/// "unknown" is **not** a variant here — it is modeled as `Option::<Severity>::None`,
/// so unknown is excluded from every ordered comparison by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Severity {
    Trace,
    Debug,
    Info,
    Notice,
    Warn,
    Error,
    Fatal,
}

/// How a severity was derived (RFC-0017 §5.3); consumers weight by trust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provenance {
    /// Read from an explicit source field (highest trust).
    Structured,
    /// Inferred from a text marker (medium trust).
    Pattern,
    /// Nothing matched; severity is unknown (no trust).
    Default,
}

impl Provenance {
    /// Whether this provenance is trusted enough to count as error-bearing evidence
    /// (RFC-0017 §6.3): `structured` or `pattern`, never `default`.
    #[must_use]
    pub fn is_trusted(self) -> bool {
        matches!(self, Provenance::Structured | Provenance::Pattern)
    }
}

/// The fundamental unit of the system (RFC-0000 §5.3).
///
/// `raw` is immutable: it is set once at construction and exposed only as `&str`
/// (RFC-0000 DEC-002). Enrichment (RFC-0017) and mining (RFC-0003) fill the derived
/// fields; Storage (RFC-0002) assigns `index_time` at commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEvent {
    id: EventId,
    stream: StreamId,
    raw: Box<str>,
    source_offset: SourceOffset,

    /// Event time (RFC-0006); `None` when unresolved.
    pub timestamp: Option<Timestamp>,
    /// Commit time, assigned by Storage (RFC-0002); `None` until committed.
    pub index_time: Option<IndexTime>,
    /// Canonical severity (RFC-0017); `None` means unknown.
    pub severity: Option<Severity>,
    /// How `severity` was derived (RFC-0017).
    pub severity_source: Provenance,
    /// Derived key/value attributes (RFC-0017 §7).
    pub attributes: Attributes,
    /// Inferred template (RFC-0003); `None` until classified.
    pub template_id: Option<TemplateId>,
    /// Structural fallback identity (RFC-0003); `None` until mined.
    pub fingerprint: Option<Fingerprint>,
}

impl LogEvent {
    /// Create a raw event at ingestion. Derived fields start empty and are filled by
    /// enrichment, mining, and storage downstream.
    #[must_use]
    pub fn new(
        id: EventId,
        stream: StreamId,
        raw: impl Into<Box<str>>,
        source_offset: SourceOffset,
    ) -> Self {
        Self {
            id,
            stream,
            raw: raw.into(),
            source_offset,
            timestamp: None,
            index_time: None,
            severity: None,
            severity_source: Provenance::Default,
            attributes: Attributes::new(),
            template_id: None,
            fingerprint: None,
        }
    }

    /// The immutable raw line, exactly as admitted (post-redaction, RFC-0015).
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    #[must_use]
    pub fn id(&self) -> EventId {
        self.id
    }

    #[must_use]
    pub fn stream(&self) -> &StreamId {
        &self.stream
    }

    #[must_use]
    pub fn source_offset(&self) -> SourceOffset {
        self.source_offset
    }

    /// Whether this event is error-bearing for the Regression detector (RFC-0017 §6.3):
    /// `severity >= floor` **and** trusted provenance. Unknown severity is never
    /// error-bearing.
    #[must_use]
    pub fn is_error_bearing(&self, floor: Severity) -> bool {
        self.severity_source.is_trusted() && self.severity.is_some_and(|s| s >= floor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{EventId, SourceOffset, StreamId};

    fn ev() -> LogEvent {
        LogEvent::new(
            EventId(1),
            StreamId("app".to_string()),
            "boom",
            SourceOffset(0),
        )
    }

    #[test]
    fn severity_is_ordered_low_to_high() {
        assert!(Severity::Trace < Severity::Info);
        assert!(Severity::Warn < Severity::Error);
        assert!(Severity::Error < Severity::Fatal);
        assert!(Severity::Error >= Severity::Warn);
    }

    #[test]
    fn unknown_severity_is_never_error_bearing() {
        let e = ev(); // severity None, provenance Default
        assert!(!e.is_error_bearing(Severity::Warn));
    }

    #[test]
    fn error_bearing_requires_trusted_provenance() {
        let mut e = ev();
        e.severity = Some(Severity::Error);
        e.severity_source = Provenance::Default;
        assert!(!e.is_error_bearing(Severity::Warn)); // untrusted provenance
        e.severity_source = Provenance::Structured;
        assert!(e.is_error_bearing(Severity::Warn)); // trusted + above floor
        e.severity = Some(Severity::Info);
        assert!(!e.is_error_bearing(Severity::Warn)); // below floor
    }

    #[test]
    fn raw_is_preserved() {
        assert_eq!(ev().raw(), "boom");
    }
}
