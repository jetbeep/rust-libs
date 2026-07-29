//! CRC-64/XZ checksums.
//!
//! Uses the `CRC_64_XZ` algorithm, which matches the vectors produced by
//! `crc64fast`. Works on both `platform-desktop` and `platform-zephyr`
//! (the `crc` crate is built with default features disabled, so it is
//! `no_std`).

use crc::{Crc, CRC_64_XZ};

/// The underlying digest type, re-exported for incremental (streaming) input.
///
/// Create one with [`digest`], feed it with [`Digest::update`], and finish
/// with [`Digest::finalize`].
pub use crc::Digest;

const CRC64_XZ: Crc<u64> = Crc::<u64>::new(&CRC_64_XZ);

/// Computes the CRC-64/XZ checksum of a complete buffer.
pub fn checksum(bytes: &[u8]) -> u64 {
    CRC64_XZ.checksum(bytes)
}

/// Creates a fresh digest for incremental input.
///
/// Feed it with [`Digest::update`] and read the result with
/// [`Digest::finalize`]. The result equals [`checksum`] over the concatenated
/// input.
pub fn digest() -> Digest<'static, u64> {
    CRC64_XZ.digest()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_crc64fast_vectors() {
        assert_eq!(checksum(b"123456789"), 0x995d_c9bb_df19_39fa);
        assert_eq!(checksum(b"hello"), 0x9b1e_dae5_dbb9_37b1);
    }

    #[test]
    fn empty_input() {
        assert_eq!(checksum(b""), 0);
    }

    #[test]
    fn incremental_matches_one_shot() {
        let full = b"the quick brown fox jumps over the lazy dog";

        let mut d = digest();
        d.update(&full[..10]);
        d.update(&full[10..25]);
        d.update(&full[25..]);

        assert_eq!(d.finalize(), checksum(full));
    }
}
