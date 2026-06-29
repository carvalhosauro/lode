//! [`Insight`] — an automatic, explainable observation (RFC-0000 §5.6, RFC-0005).

use crate::ids::RowAnchor;

/// The v1 detector kinds (RFC-0005 §5.2). Anomaly and correlation are deferred to v2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InsightKind {
    Spike,
    Rare,
    Emergence,
    Regression,
}

/// A normalized confidence score in `[0.0, 1.0]` (RFC-0005 §9).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Confidence(f64);

impl Confidence {
    /// Construct a confidence, clamped into `[0.0, 1.0]`.
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    #[must_use]
    pub fn value(self) -> f64 {
        self.0
    }
}

/// An automatic discovery surfaced to the Workspace (RFC-0005). Immutable once
/// surfaced; references its evidence by anchor, never by embedding raw data.
#[derive(Debug, Clone, PartialEq)]
pub struct Insight {
    pub kind: InsightKind,
    pub confidence: Confidence,
    /// Human-readable explanation, including the evidence that triggered it.
    pub description: Box<str>,
    /// The events that produced this insight (RFC-0005); non-empty in practice.
    pub related: Vec<RowAnchor>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_clamps_to_unit_interval() {
        assert!((Confidence::new(1.5).value() - 1.0).abs() < f64::EPSILON);
        assert!((Confidence::new(-0.2).value() - 0.0).abs() < f64::EPSILON);
        assert!((Confidence::new(0.42).value() - 0.42).abs() < f64::EPSILON);
    }
}
