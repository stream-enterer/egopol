/// Compute the Adler-32 checksum of `data`.
///
/// Matches C++ emCalcAdler32. Produces the same output as zlib's adler32().
/// The `start` parameter allows chaining: pass the previous result to
/// continue computing over additional data.
pub fn emCalcAdler32(data: &[u8], start: u32) -> u32 {
    const MOD_ADLER: u32 = 65521;
    // C++ batches 5552 bytes to avoid overflow before taking mod.
    // 5552 is the largest n such that 255*n*(n+1)/2 + (n+1)*0xFFFF < 2^32.
    const BATCH: usize = 5552;

    let mut lo = start & 0xFFFF;
    let mut hi = start >> 16;

    let mut offset = 0;
    while offset < data.len() {
        let end = (offset + BATCH).min(data.len());
        for &byte in &data[offset..end] {
            lo += byte as u32;
            hi += lo;
        }
        lo %= MOD_ADLER;
        hi %= MOD_ADLER;
        offset = end;
    }

    (hi << 16) | lo
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
/// compatible with zlib/PNG CRC-32. Pass a previous result as `start`
/// to chain multiple calls.
pub fn emCalcCRC32(data: &[u8], start: u32) -> u32 {
    let mut r = start;
    if !data.is_empty() {
        r = !r;
        for &byte in data {
            let index = (byte ^ (r as u8)) as usize;
            r = CRC32_TABLE[index] ^ (r >> 8);
        }
        r = !r;
    }
    r
}

/// CRC-64 lookup table (polynomial 0xD800000000000000).
const CRC64_TABLE: [u64; 256] = {
    let mut table = [0u64; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut crc = i as u64;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = 0xD800_0000_0000_0000 ^ (crc >> 1);
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

/// Compute the CRC-64 checksum of `data`.
///
/// Matches C++ emCalcCRC64. Pass a previous result as `start`
/// to chain multiple calls.
pub fn emCalcCRC64(data: &[u8], start: u64) -> u64 {
    let mut r = start;
    if !data.is_empty() {
        r = !r;
        for &byte in data {
            let index = (byte ^ (r as u8)) as usize;
            r = CRC64_TABLE[index] ^ (r >> 8);
        }
        r = !r;
    }
    r
}

/// Compute a simple hash code for a null-terminated-style string.
///
/// Matches C++ emCalcHashCode exactly: multiplier 335171, processes bytes
/// until a zero byte or end of slice. Returns signed i32 like C++.
pub fn emCalcHashCode(data: &[u8], start: i32) -> i32 {
    let mut r = start as u32;
    for &byte in data {
        if byte == 0 {
            break;
        }
        r = r.wrapping_mul(335_171).wrapping_add(byte as u32);
    }
    r as i32
}

/// Compute an any-length hash name from data.
///
/// Matches C++ emCalcHashName exactly. The result is a string of letters
/// and digits. Capitalization provides extra entropy but comparisons
/// can safely ignore case.
pub fn emCalcHashName(src: &[u8], hash_len: usize) -> String {
    // Part 1: base-36 hash
    let mut hash = vec![0u8; hash_len];

    for &src_byte in src {
        for j in 0..hash_len {
            let mut a = hash[j] as u32;
            if j == hash_len - 1 {
                a += src_byte as u32;
            }
            a = a.wrapping_mul(6_795_413);
            hash[j] = (a % 36) as u8;
            a /= 36;
            // Propagate carry backwards
            let mut k = j as isize - 1;
            while k >= 0 && a != 0 {
                a += hash[k as usize] as u32;
                hash[k as usize] = (a % 36) as u8;
                a /= 36;
                k -= 1;
            }
        }
    }

    // Convert to ASCII digits/lowercase letters
    for h in &mut hash {
        if *h < 10 {
            *h += b'0';
        } else {
            *h += b'a' - 10;
        }
    }

    // Part 2: capitalization for extra entropy
    let letter_count = hash.iter().filter(|&&c| c.is_ascii_lowercase()).count();
    let b: u64 = if letter_count <= 32 {
        emCalcCRC32(src, 0) as u64
    } else {
        emCalcCRC64(src, 0)
    };
    let mut bits = b;
    if letter_count <= 16 {
        bits ^= bits >> 16;
    }
    if letter_count <= 8 {
        bits ^= bits >> 8;
    }
    if letter_count <= 4 {
        bits ^= bits >> 4;
    }
    if letter_count <= 2 {
        bits ^= bits >> 2;
    }
    if letter_count <= 1 {
        bits ^= bits >> 1;
    }

    for h in &mut hash {
        if h.is_ascii_lowercase() {
            if bits & 1 != 0 {
                *h -= b'a' - b'A';
            }
            bits >>= 1;
        }
    }

    // Safety: all bytes are ASCII digits or letters
    String::from_utf8(hash).expect("hash contains only ASCII")
}

use std::cell::RefCell;

// ── Dynamic Library API ─────────────────────────────────────────────
// Port of C++ emStd2.h/emStd2.cpp: emTryOpenLib, emTryResolveSymbolFromLib,
// emCloseLib, emTryResolveSymbol, and the emLibTable cache.

/// Error type for dynamic library operations.
/// Port of C++ exceptions thrown by emTryOpenLib and emTryResolveSymbol.
#[derive(Debug)]
pub enum LibError {
    /// Failed to load a dynamic library.
    LibraryLoad { library: String, message: String },
    /// Failed to resolve a symbol from a loaded library.
    SymbolResolve {
        library: String,
        symbol: String,
        message: String,
    },
}

impl std::fmt::Display for LibError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LibraryLoad { library, message } => {
                write!(f, "failed to load library \"{library}\": {message}")
            }
            Self::SymbolResolve {
                library,
                symbol,
                message,
            } => {
                write!(
                    f,
                    "failed to resolve \"{symbol}\" in \"{library}\": {message}"
                )
            }
        }
    }
}

impl std::error::Error for LibError {}

/// Port of C++ `emLibTableEntry`.
struct LibTableEntry {
    filename: String,
    ref_count: u64, // 0 = infinite (never unloaded)
    handle: libloading::Library,
}

thread_local! {
    /// Port of C++ `emLibTable`. Single-threaded; no mutex needed.
    static LIBRARY_TABLE: RefCell<Vec<LibTableEntry>> = const { RefCell::new(Vec::new()) };
}

/// Opaque handle to a loaded dynamic library.
/// Port of C++ `emLibHandle`.
#[derive(Debug, Clone, Copy)]
pub struct emLibHandle {
    pub(crate) index: usize,
}

/// Convert a library pure name to a platform filename.
/// Port of C++ logic in `emTryOpenLib`: "emFoo" -> "libemFoo.so" (Linux),
/// "libemFoo.dylib" (macOS), "emFoo.dll" (Windows).
pub fn lib_name_to_filename(name: &str) -> String {
    if cfg!(target_os = "windows") || cfg!(target_os = "cygwin") {
        format!("{name}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{name}.dylib")
    } else {
        format!("lib{name}.so")
    }
}

/// Open a dynamic library. Port of C++ `emTryOpenLib`.
///
/// If `is_filename` is false, `lib_name` is a pure name converted to a
/// platform filename. Libraries are cached: opening the same library twice
/// returns the same handle with an incremented refcount.
pub fn emTryOpenLib(lib_name: &str, is_filename: bool) -> Result<emLibHandle, LibError> {
    let filename = if is_filename {
        lib_name.to_string()
    } else {
        lib_name_to_filename(lib_name)
    };

    LIBRARY_TABLE.with(|table| {
        let mut table = table.borrow_mut();

        // Check cache
        for (i, entry) in table.iter_mut().enumerate() {
            if entry.filename == filename {
                if entry.ref_count > 0 {
                    entry.ref_count += 1;
                }
                return Ok(emLibHandle { index: i });
            }
        }

        // Load new library
        let handle =
            unsafe { libloading::Library::new(&filename) }.map_err(|e| LibError::LibraryLoad {
                library: filename.clone(),
                message: e.to_string(),
            })?;

        let index = table.len();
        table.push(LibTableEntry {
            filename,
            ref_count: 1,
            handle,
        });

        Ok(emLibHandle { index })
    })
}

/// Resolve a symbol from an open library.
/// Port of C++ `emTryResolveSymbolFromLib`.
///
/// # Safety
/// The returned pointer is only valid while the library remains open.
/// Caller must ensure the pointer is transmuted to the correct function type.
pub unsafe fn emTryResolveSymbolFromLib(
    handle: &emLibHandle,
    symbol: &str,
) -> Result<*const (), LibError> {
    LIBRARY_TABLE.with(|table| {
        let table = table.borrow();
        let entry = &table[handle.index];

        let sym: libloading::Symbol<*const ()> = unsafe { entry.handle.get(symbol.as_bytes()) }
            .map_err(|e| LibError::SymbolResolve {
                library: entry.filename.clone(),
                symbol: symbol.to_string(),
                message: e.to_string(),
            })?;

        Ok(*sym)
    })
}

/// Close a dynamic library. Port of C++ `emCloseLib`.
///
/// Decrements refcount. At zero, the library is unloaded and the entry
/// removed. If refcount was already zero (infinite), this is a no-op.
pub fn emCloseLib(handle: emLibHandle) {
    LIBRARY_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        let entry = &mut table[handle.index];

        if entry.ref_count == 0 {
            return; // infinite lifetime
        }

        entry.ref_count -= 1;
        if entry.ref_count == 0 {
            table.remove(handle.index);
        }
    });
}

/// Open, resolve, and set library to infinite lifetime.
/// Port of C++ `emTryResolveSymbol`.
///
/// The library is never closed after this call (refcount set to 0 = infinite).
///
/// # Safety
/// Same as `emTryResolveSymbolFromLib`.
pub unsafe fn emTryResolveSymbol(
    lib_name: &str,
    is_filename: bool,
    symbol: &str,
) -> Result<*const (), LibError> {
    let handle = emTryOpenLib(lib_name, is_filename)?;
    let ptr = unsafe { emTryResolveSymbolFromLib(&handle, symbol)? };

    // Set to infinite lifetime (matching C++: e->RefCount=0)
    LIBRARY_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        table[handle.index].ref_count = 0;
    });

    Ok(ptr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adler32_empty() {
        assert_eq!(emCalcAdler32(&[], 1), 1);
    }

    #[test]
    fn adler32_known() {
        // "Wikipedia" -> 0x11E60398 (well-known test vector)
        assert_eq!(emCalcAdler32(b"Wikipedia", 1), 0x11E6_0398);
    }

    #[test]
    fn adler32_chaining() {
        let data = b"Hello, World!";
        let full = emCalcAdler32(data, 1);
        let part1 = emCalcAdler32(&data[..5], 1);
        let part2 = emCalcAdler32(&data[5..], part1);
        assert_eq!(full, part2);
    }

    #[test]
    fn crc32_empty() {
        assert_eq!(emCalcCRC32(&[], 0), 0);
    }

    #[test]
    fn crc32_known() {
        // "123456789" -> 0xCBF43926 (ISO 3309 / ITU-T V.42 test vector)
        assert_eq!(emCalcCRC32(b"123456789", 0), 0xCBF4_3926);
    }

    #[test]
    fn crc32_chaining() {
        let data = b"Hello, World!";
        let full = emCalcCRC32(data, 0);
        let part1 = emCalcCRC32(&data[..5], 0);
        let part2 = emCalcCRC32(&data[5..], part1);
        assert_eq!(full, part2);
    }

    #[test]
    fn crc64_empty() {
        assert_eq!(emCalcCRC64(&[], 0), 0);
    }

    #[test]
    fn crc64_nonempty() {
        // Non-zero result for non-empty input
        let result = emCalcCRC64(b"test", 0);
        assert_ne!(result, 0);
    }

    #[test]
    fn crc64_chaining() {
        let data = b"Hello, World!";
        let full = emCalcCRC64(data, 0);
        let part1 = emCalcCRC64(&data[..5], 0);
        let part2 = emCalcCRC64(&data[5..], part1);
        assert_eq!(full, part2);
    }

    #[test]
    fn hash_code_empty() {
        assert_eq!(emCalcHashCode(&[], 0), 0);
    }

    #[test]
    fn hash_code_stops_at_null() {
        // C++ emCalcHashCode stops at null byte
        let with_null = emCalcHashCode(b"abc\0xyz", 0);
        let without = emCalcHashCode(b"abc", 0);
        assert_eq!(with_null, without);
    }

    #[test]
    fn hash_code_specific_value() {
        // Verify the formula: r = r * 335171 + c for each byte
        // "A" (0x41=65): 0 * 335171 + 65 = 65
        assert_eq!(emCalcHashCode(b"A", 0), 65);
        // "AB": (65 * 335171 + 66) = 21786181
        assert_eq!(
            emCalcHashCode(b"AB", 0),
            65_i32.wrapping_mul(335_171).wrapping_add(66)
        );
    }

    #[test]
    fn hash_code_differs() {
        assert_ne!(emCalcHashCode(b"abc", 0), emCalcHashCode(b"xyz", 0));
    }

    #[test]
    fn hash_name_length() {
        let name = emCalcHashName(b"test", 8);
        assert_eq!(name.len(), 8);
        // All chars should be alphanumeric
        assert!(name.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn hash_name_deterministic() {
        let a = emCalcHashName(b"hello", 10);
        let b = emCalcHashName(b"hello", 10);
        assert_eq!(a, b);
    }

    #[test]
    fn hash_name_differs() {
        let a = emCalcHashName(b"hello", 8);
        let b = emCalcHashName(b"world", 8);
        assert_ne!(a.to_ascii_lowercase(), b.to_ascii_lowercase());
    }

    #[test]
    fn test_try_open_lib_nonexistent() {
        let result = super::emTryOpenLib("nonexistent_library_12345", false);
        assert!(result.is_err());
        match result.unwrap_err() {
            super::LibError::LibraryLoad { library, .. } => {
                assert!(library.contains("nonexistent_library_12345"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn test_lib_filename_construction() {
        // On Linux: "emFoo" -> "libemFoo.so"
        assert_eq!(super::lib_name_to_filename("emFoo"), "libemFoo.so");
    }
}
