//! [`Fingerprint`] hashing and corpus [`template_set_hash`] (RFC-0003, spike parity).

use crate::hash::Fnv1a64;
use crate::ids::Fingerprint;
use crate::mining::{Token, hash_masked_tokens};

impl Fingerprint {
    /// Stable identifier over a masked token sequence (RFC-0003 §5.4).
    #[must_use]
    pub fn from_masked_tokens(tokens: &[Token]) -> Self {
        Self(hash_masked_tokens(tokens))
    }
}

/// Deterministic hash over a template set for corpus gates (spike `template_set_hash`).
///
/// Patterns must be pre-sorted lexicographically; each pattern is hashed followed by `\n`.
#[must_use]
pub fn template_set_hash(sorted_patterns: &[&str]) -> u64 {
    let mut h = Fnv1a64::new();
    for pattern in sorted_patterns {
        h.write_bytes(pattern.as_bytes());
        h.write_byte(b'\n');
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_set_hash_matches_spike_sample() {
        let patterns = [
            "<IP> - - <TS> <*> <PATH> <PATH> <NUM> <NUM>",
            "ERROR 2024-10-10 <TS> db connection failed after <NUM> ms id <UUID>",
            "GET <PATH> <NUM>",
            "INFO 2024-10-10 <TS> user <NUM> logged in from <IP>",
            "WARN 2024-10-10 <TS> cache miss for key <HEX>",
        ];
        assert_eq!(template_set_hash(&patterns), 0x7ae1_d59d_88c3_26ce);
    }
}
