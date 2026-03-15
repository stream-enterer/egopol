/// Compute the Adler-32 checksum of `data`.
///
/// Matches C++ emCalcAdler32. Produces the same output as zlib's adler32().
pub fn calc_adler32(data: &[u8]) -> u32 {
    const MOD_ADLER: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;

    for &byte in data {
        a = (a + byte as u32) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }

    (b << 16) | a
}

/// CRC-32 lookup table (polynomial 0xEDB88320, same as zlib/PNG).
const CRC32_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = 0xEDB8_8320 ^ (crc >> 1);
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
};

/// Compute the CRC-32 checksum of `data`.
///
/// Matches C++ emCalcCRC32. Uses the standard polynomial (0xEDB88320),
/// compatible with zlib/PNG CRC-32.
pub fn calc_crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        let index = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = CRC32_TABLE[index] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

/// Compute a simple hash code for `data`.
///
/// Matches C++ emCalcHashCode. This is a fast, non-cryptographic hash
/// suitable for hash tables and quick comparisons.
pub fn calc_hash_code(data: &[u8]) -> u32 {
    let mut hash: u32 = 0;
    for &byte in data {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adler32_empty() {
        assert_eq!(calc_adler32(&[]), 1);
    }

    #[test]
    fn adler32_known() {
        // "Wikipedia" -> 0x11E60398 (well-known test vector)
        assert_eq!(calc_adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn crc32_empty() {
        assert_eq!(calc_crc32(&[]), 0);
    }

    #[test]
    fn crc32_known() {
        // "123456789" -> 0xCBF43926 (ISO 3309 / ITU-T V.42 test vector)
        assert_eq!(calc_crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn hash_code_deterministic() {
        let data = b"hello world";
        assert_eq!(calc_hash_code(data), calc_hash_code(data));
    }

    #[test]
    fn hash_code_empty() {
        assert_eq!(calc_hash_code(&[]), 0);
    }

    #[test]
    fn hash_code_differs() {
        assert_ne!(calc_hash_code(b"abc"), calc_hash_code(b"xyz"));
    }
}
