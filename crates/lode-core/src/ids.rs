//! Identifier and position newtypes (RFC-0000), kept distinct so the three former
//! meanings of "offset" can never be confused again (RFC-0000 / RFC-0007):
//! `source_offset` (origin position), `segment_position` (storage-internal), and
//! `row_anchor` (a Workspace bookmark/viewport anchor).

/// Identifies a [`crate::LogStream`]. User-facing, so a string name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StreamId(pub String);

/// Identifies a single [`crate::LogEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EventId(pub u64);

/// Identifies a [`crate::Template`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TemplateId(pub u64);

/// Identifies an `IndexSegment` (owned by `lode-storage`, RFC-0002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SegmentId(pub u64);

/// Position of an event in its **origin** stream (bytes/records). Owned by ingestion
/// (RFC-0001); the field on [`crate::LogEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceOffset(pub u64);

/// Physical position of an event within an `IndexSegment` (storage-internal,
/// RFC-0002). Never an event's identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SegmentPosition(pub u64);

/// The time an event was committed to a segment — assigned by Storage at commit,
/// monotonic per segment (RFC-0002 / RFC-0006). A logical counter, not wall-clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IndexTime(pub u64);

/// Event time, in Unix nanoseconds. May be absent (`Option<Timestamp>`) when it
/// cannot be resolved (RFC-0006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Timestamp(pub i64);

/// Stable structural identifier over masked tokens; the fallback identity when no
/// template matches (RFC-0003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fingerprint(pub u64);

/// A stable Workspace anchor for bookmarks/viewport (RFC-0007): `(stream, source_offset)`.
/// Never a [`SegmentPosition`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RowAnchor {
    pub stream: StreamId,
    pub source_offset: SourceOffset,
}
