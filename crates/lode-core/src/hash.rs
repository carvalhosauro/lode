//! FNV-1a 64-bit hashing (spike / corpus determinism parity).

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Incremental FNV-1a 64-bit hasher.
#[derive(Debug, Clone, Copy, Default)]
pub struct Fnv1a64(u64);

impl Fnv1a64 {
    #[must_use]
    pub const fn new() -> Self {
        Self(FNV_OFFSET)
    }

    pub fn write_byte(&mut self, byte: u8) {
        self.0 ^= u64::from(byte);
        self.0 = self.0.wrapping_mul(FNV_PRIME);
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_byte(b);
        }
    }

    #[must_use]
    pub const fn finish(self) -> u64 {
        self.0
    }
}

/// One-shot FNV-1a over a byte slice.
#[must_use]
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h = Fnv1a64::new();
    h.write_bytes(bytes);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_offset_basis() {
        assert_eq!(fnv1a_64(b""), FNV_OFFSET);
    }
}
