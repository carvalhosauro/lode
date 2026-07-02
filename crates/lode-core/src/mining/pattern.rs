//! Pattern string helpers for masked token sequences.

use super::Token;

/// Join masked tokens with a single space (Drain pattern representation).
#[must_use]
pub fn pattern_to_string(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(Token::as_str)
        .collect::<Vec<_>>()
        .join(" ")
}
