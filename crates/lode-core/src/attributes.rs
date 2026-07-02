//! [`Attributes`] — derived key/value pairs on a [`crate::LogEvent`] (RFC-0017 §7).

/// Derived key/value attributes attached to an event for filtering and correlation.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Attributes(pub Vec<(Box<str>, Box<str>)>);

impl Attributes {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, key: impl Into<Box<str>>, value: impl Into<Box<str>>) {
        self.0.push((key.into(), value.into()));
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.0.iter().map(|(k, v)| (k.as_ref(), v.as_ref()))
    }
}
