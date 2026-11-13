//! `_DEFLATE$` / `_INFLATE$`: zlib's `compress()` and `uncompress()` through
//! `zlib-rs`, a Rust port of zlib whose `uncompress` is zlib's `uncompress2` line
//! by line. The caller does the buffer sizing (the 10 MB retry loop of
//! `_INFLATE$` without a size).
//!
//! * [`deflate`]: zlib stream (RFC 1950), default level 6. zlib-rs compresses with
//!   zlib-ng's algorithms, so the bytes can differ from zlib's (documented deviation);
//!   every zlib inflates them.
//! * [`inflate`]: zlib's result codes, output length and partial output, including
//!   corrupt streams and the "zero-length destination" special case.

use zlib_rs::{DeflateConfig, InflateConfig, ReturnCode};

/// zlib result codes returned by [`inflate`].
pub const Z_OK: i32 = 0;
pub const Z_DATA_ERROR: i32 = -3;
pub const Z_BUF_ERROR: i32 = -5;

/// zlib 1.3's `compressBound()`.
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
}
