//! Template-mining domain types (RFC-0003). Algorithms land in T1.

mod fingerprint;

pub use fingerprint::template_set_hash;

use crate::hash::Fnv1a64;

/// RFC-0003 §6.1 built-in placeholders, in evaluation order (most-specific-first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaskKind {
    Ts,
    Uuid,
    Ip,
    Url,
    Email,
    Path,
    Hex,
    Num,
    /// Optional rich mask (RFC-0003 §6.1).
    Quoted,
    /// Widened constant position (`<*>`).
    Wildcard,
}

impl MaskKind {
    /// Mask kinds tried before falling through to the next (RFC-0003 §6.1 diagram).
    pub const EVAL_ORDER: [Self; 8] = [
        Self::Ts,
        Self::Uuid,
        Self::Ip,
        Self::Url,
        Self::Email,
        Self::Path,
        Self::Hex,
        Self::Num,
    ];

    #[must_use]
    pub const fn placeholder(self) -> &'static str {
        match self {
            Self::Ts => "<TS>",
            Self::Uuid => "<UUID>",
            Self::Ip => "<IP>",
            Self::Url => "<URL>",
            Self::Email => "<EMAIL>",
            Self::Path => "<PATH>",
            Self::Hex => "<HEX>",
            Self::Num => "<NUM>",
            Self::Quoted => "<QUOTED>",
            Self::Wildcard => "<*>",
        }
    }
}

/// A structural token; delimiters are preserved as their own tokens (RFC-0003 §5.2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token(pub Box<str>);

impl Token {
    #[must_use]
    pub fn new(s: impl Into<Box<str>>) -> Self {
        Self(s.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Masked token sequence produced before clustering (RFC-0003 §5.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaskedTokens {
    pub tokens: Vec<Token>,
    /// Captured variable values keyed by placeholder name (e.g. `"<NUM>"` → `"47"`).
    pub placeholders: Vec<(Box<str>, Box<str>)>,
}

impl MaskedTokens {
    #[must_use]
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            placeholders: Vec::new(),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}

/// Tunables fixed for determinism (RFC-0003 §6.6, §11).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MiningParams {
    /// Parse-tree depth (`d`).
    pub depth: u8,
    /// Minimum sequence similarity for a leaf match (`st`).
    pub similarity_threshold: f64,
    /// Maximum live templates before eviction (`T_max`).
    pub max_templates: u32,
    /// Occurrences before `emerging` → `stable` (`N`).
    pub stable_threshold: u32,
}

impl Default for MiningParams {
    fn default() -> Self {
        Self {
            depth: 4,
            similarity_threshold: 0.5,
            max_templates: 10_000,
            stable_threshold: 5,
        }
    }
}

/// Token separator for per-event [`crate::Fingerprint`] (phase-0 spec).
const FINGERPRINT_SEP: u8 = 0x1e;

/// Hash masked token text for fingerprinting.
#[must_use]
pub fn hash_masked_tokens(tokens: &[Token]) -> u64 {
    let mut h = Fnv1a64::new();
    for (i, tok) in tokens.iter().enumerate() {
        if i > 0 {
            h.write_byte(FINGERPRINT_SEP);
        }
        h.write_bytes(tok.as_str().as_bytes());
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::Fingerprint;

    #[test]
    fn mining_params_default_matches_rfc() {
        let p = MiningParams::default();
        assert_eq!(p.depth, 4);
        assert!((p.similarity_threshold - 0.5).abs() < f64::EPSILON);
        assert_eq!(p.max_templates, 10_000);
        assert_eq!(p.stable_threshold, 5);
    }

    #[test]
    fn mask_eval_order_matches_rfc_diagram() {
        assert_eq!(
            MaskKind::EVAL_ORDER,
            [
                MaskKind::Ts,
                MaskKind::Uuid,
                MaskKind::Ip,
                MaskKind::Url,
                MaskKind::Email,
                MaskKind::Path,
                MaskKind::Hex,
                MaskKind::Num,
            ]
        );
    }

    #[test]
    fn fingerprint_is_deterministic() {
        let tokens = vec![Token::new("GET"), Token::new("<PATH>"), Token::new("<NUM>")];
        let a = Fingerprint::from_masked_tokens(&tokens);
        let b = Fingerprint::from_masked_tokens(&tokens);
        assert_eq!(a, b);
    }

    #[test]
    fn fingerprint_differs_for_different_shapes() {
        let a = Fingerprint::from_masked_tokens(&[Token::new("GET"), Token::new("<PATH>")]);
        let b = Fingerprint::from_masked_tokens(&[Token::new("POST"), Token::new("<PATH>")]);
        assert_ne!(a, b);
    }
}
