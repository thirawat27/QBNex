//! `_DEFLATE$` / `_INFLATE$` (was `internal/c/parts/zlib/src.c` linked against the
//! system zlib, `-l:libz.a` / `-lz`).
//!
//! The C++ shim (`internal/c/libqb/compress_bridge.cpp`) keeps the legacy control
//! flow (buffer sizing, the 10 MB retry loop of `_INFLATE$` without a size); only
//! zlib's `compress()` and `uncompress()` are replaced, by `zlib-rs`, a Rust port of
//! zlib whose `uncompress` is zlib's `uncompress2` line by line:
//!
//! * [`deflate`]: zlib stream (RFC 1950), default level 6. zlib-rs compresses with
//!   zlib-ng's algorithms, so the bytes can differ from zlib's (documented deviation);
//!   every zlib inflates them.
//! * [`inflate`]: zlib's result codes, output length and partial output, including
//!   corrupt streams and the "zero-length destination" special case.

use zlib_rs::{DeflateConfig, InflateConfig, ReturnCode};

/// zlib result codes returned through the C ABI.
pub const Z_OK: i32 = 0;
pub const Z_DATA_ERROR: i32 = -3;
pub const Z_BUF_ERROR: i32 = -5;

/// zlib 1.3's `compressBound()` (the C ABI promises this exact value).
pub fn deflate_bound(len: usize) -> usize {
    len + (len >> 12) + (len >> 14) + (len >> 25) + 13
}

/// zlib's `compress()` (level `Z_DEFAULT_COMPRESSION` = 6).
pub fn deflate(src: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; zlib_rs::compress_bound(src.len()).max(deflate_bound(src.len()))];
    let (z, rc) = zlib_rs::compress_slice(&mut out, src, DeflateConfig::default());
    assert_eq!(rc, ReturnCode::Ok, "compress_slice with a compress_bound buffer");
    let n = z.len();
    out.truncate(n);
    out
}

/// zlib's `uncompress()` into `dst`. Returns `(code, bytes written)`.
///
/// * `Z_OK`: complete stream.
/// * `Z_BUF_ERROR`: `dst` is full before the stream ends.
/// * `Z_DATA_ERROR`: corrupt stream, or the input ends early while `dst` still has room.
///
/// With an empty `dst` zlib decompresses into a 1-byte scratch buffer: a stream of at
/// most one byte is `Z_OK`, a longer one `Z_DATA_ERROR`; 0 bytes are reported.
pub fn inflate(src: &[u8], dst: &mut [u8]) -> (i32, usize) {
    let (out, rc) = zlib_rs::decompress_slice(dst, src, InflateConfig::default());
    (rc as i32, out.len())
}

// ---------------------------------------------------------------------------
// C ABI
// ---------------------------------------------------------------------------

/// `compressBound(len)`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_deflate_bound(len: u32) -> u32 {
    deflate_bound(len as usize) as u32
}

/// `compress(dst, dst_len, src, len)`: `*dst_len` is the capacity on entry and the
/// compressed size on return. Returns `Z_OK` or `Z_BUF_ERROR`.
///
/// # Safety
/// `src` must be readable for `len` bytes, `dst` writable for `*dst_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_deflate(src: *const u8, len: u32, dst: *mut u8, dst_len: *mut u32) -> i32 {
    // SAFETY: the caller guarantees the buffer sizes (see above).
    unsafe {
        let input = if len == 0 { &[][..] } else { std::slice::from_raw_parts(src, len as usize) };
        let out = deflate(input);
        if out.len() > *dst_len as usize {
            return Z_BUF_ERROR;
        }
        std::ptr::copy_nonoverlapping(out.as_ptr(), dst, out.len());
        *dst_len = out.len() as u32;
        Z_OK
    }
}

/// `uncompress(dst, dst_len, src, len)`: `*dst_len` is the capacity on entry and the
/// number of bytes written on return. Returns `Z_OK`, `Z_BUF_ERROR` or `Z_DATA_ERROR`.
///
/// # Safety
/// `src` must be readable for `len` bytes, `dst` writable for `*dst_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_inflate(src: *const u8, len: u32, dst: *mut u8, dst_len: *mut u32) -> i32 {
    // SAFETY: the caller guarantees the buffer sizes (see above).
    unsafe {
        let input = if len == 0 { &[][..] } else { std::slice::from_raw_parts(src, len as usize) };
        let cap = *dst_len as usize;
        let out = if cap == 0 { &mut [][..] } else { std::slice::from_raw_parts_mut(dst, cap) };
        let (code, written) = inflate(input, out);
        *dst_len = written as u32;
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // zlib 1.3.1 `compress()` of "hello hello hello hello" (MinGW libz.a).
    const HELLO_Z: [u8; 16] = [
        0x78, 0x9c, 0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x57, 0xc8, 0x40, 0x27, 0x01, 0x68, 0x03, 0x08, 0xb1,
    ];

    fn sample(n: usize) -> Vec<u8> {
        let mut x: u32 = 12345;
        (0..n)
            .map(|i| {
                x = x.wrapping_mul(1_103_515_245).wrapping_add(12345);
                if i % 7 < 4 { b"QBNex "[i % 6] } else { (x >> 16) as u8 }
            })
            .collect()
    }

    #[test]
    fn bound_matches_zlib() {
        assert_eq!(deflate_bound(0), 13);
        assert_eq!(deflate_bound(23), 36);
        assert_eq!(deflate_bound(100_000), 100_000 + 24 + 6 + 13);
    }

    #[test]
    fn deflate_is_a_zlib_stream_that_round_trips() {
        for n in [0usize, 1, 23, 4096, 200_000] {
            let src = sample(n);
            let z = deflate(&src);
            assert!(z.len() <= deflate_bound(n), "n={n}");
            assert_eq!(z[0], 0x78, "zlib header, 32K window");
            assert_eq!(((z[0] as u16) << 8 | z[1] as u16) % 31, 0, "FCHECK");
            let mut out = vec![0u8; n];
            assert_eq!(inflate(&z, &mut out), (Z_OK, n));
            assert_eq!(out, src);
        }
    }

    #[test]
    fn inflate_reads_zlib_output() {
        let mut out = [0u8; 23];
        assert_eq!(inflate(&HELLO_Z, &mut out), (Z_OK, 23));
        assert_eq!(&out, b"hello hello hello hello");
        let mut big = [0u8; 100];
        assert_eq!(inflate(&HELLO_Z, &mut big), (Z_OK, 23));
    }

    #[test]
    fn inflate_small_buffer_is_buf_error_with_partial_output() {
        let src = sample(5000);
        let z = deflate(&src);
        let mut out = vec![0u8; 1000];
        assert_eq!(inflate(&z, &mut out), (Z_BUF_ERROR, 1000));
        assert_eq!(&out[..], &src[..1000]);
    }

    #[test]
    fn inflate_exact_buffer_is_ok() {
        let src = sample(777);
        let z = deflate(&src);
        let mut out = vec![0u8; 777];
        assert_eq!(inflate(&z, &mut out), (Z_OK, 777));
    }

    #[test]
    fn inflate_truncated_input_with_room_is_data_error() {
        let src = sample(5000);
        let z = deflate(&src);
        let mut out = vec![0u8; 10_000];
        let (code, n) = inflate(&z[..z.len() / 2], &mut out);
        assert_eq!(code, Z_DATA_ERROR);
        assert!(n < 5000);
        assert_eq!(&out[..n], &src[..n]);
    }

    #[test]
    fn inflate_garbage_is_data_error() {
        let mut out = [0u8; 64];
        assert_eq!(inflate(b"not a zlib stream", &mut out).0, Z_DATA_ERROR);
        assert_eq!(inflate(b"", &mut out), (Z_DATA_ERROR, 0));
    }

    #[test]
    fn inflate_into_empty_buffer_follows_zlib() {
        let mut none: [u8; 0] = [];
        assert_eq!(inflate(&deflate(b""), &mut none), (Z_OK, 0));
        assert_eq!(inflate(&deflate(b"x"), &mut none), (Z_OK, 0));
        assert_eq!(inflate(&deflate(b"xy"), &mut none), (Z_DATA_ERROR, 0));
        assert_eq!(inflate(b"junk", &mut none), (Z_DATA_ERROR, 0));
    }

    #[test]
    fn c_abi_round_trip() {
        let src = sample(3000);
        let mut zlen = qb_deflate_bound(3000);
        let mut z = vec![0u8; zlen as usize];
        assert_eq!(unsafe { qb_deflate(src.as_ptr(), 3000, z.as_mut_ptr(), &mut zlen) }, Z_OK);
        let mut olen: u32 = 3000;
        let mut out = vec![0u8; 3000];
        assert_eq!(unsafe { qb_inflate(z.as_ptr(), zlen, out.as_mut_ptr(), &mut olen) }, Z_OK);
        assert_eq!((olen, out), (3000, src));
    }
}
