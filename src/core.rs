//! core.rs — Robust lightweight checksum utility routines.

/// Computes a standard 32-bit polynomial checksum over a byte payload sequence.
pub fn simple_checksum(bytes: &[u8]) -> u32 {
    let mut hash = 5381u32;
    for &byte in bytes {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
    }
    hash
}
