use lode_core::MAX_RAW_LINE_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    EmptyLine,
    LineTooLong { len: usize, max: usize },
}

impl ParseError {
    #[must_use]
    pub fn line_too_long(len: usize) -> Self {
        Self::LineTooLong {
            len,
            max: MAX_RAW_LINE_BYTES,
        }
    }
}
