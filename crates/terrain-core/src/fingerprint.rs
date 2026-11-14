//! Stable, dependency-free content fingerprints.
//!
//! Used to answer "has this file changed since we last wrote it?" for deployed skills and
//! Litho run checkpoints. A non-cryptographic hash is enough here: a false match can only
//! cause a conservative skip, never a silent overwrite of user content.

/// FNV-1a 64-bit, rendered as fixed-width lowercase hex.
///
/// Deliberately implemented locally rather than via `DefaultHasher`, which is not guaranteed
/// to stay stable across Rust releases — these digests are persisted on disk.
pub fn fingerprint(bytes: &[u8]) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_and_content_sensitive() {
        assert_eq!(fingerprint(b"same"), fingerprint(b"same"));
        assert_ne!(fingerprint(b"same"), fingerprint(b"changed"));
        assert_eq!(fingerprint(b"").len(), 16);
    }
}
