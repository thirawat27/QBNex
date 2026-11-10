//! `qb-ext` — QBNex extension library for the BASIC standard library.
//!
//! Regular expressions (`regex`), hashing (CRC32, MD5, SHA-1, SHA-256,
//! SHA-512), Base64 and hex, zip archives (`zip`) and HTTP/HTTPS requests
//! (`ureq` + rustls, no OpenSSL). Each area has a plain Rust API in its
//! module; [`ffi`] exposes them as `qb_ext_*` C functions for the BASIC
//! wrappers in `compiler/stdlib/ext/*.bas` (declared in `include/qb_ext.h`).
//!
//! BASIC strings are byte strings, so every function works on bytes; text
//! given to the regex engine and to HTTP URLs is read as UTF-8 (invalid
//! sequences are replaced, see each function).

#![deny(unsafe_op_in_unsafe_fn)]

pub mod codec;
pub mod ffi;
pub mod hash;
pub mod http;
pub mod re;
pub mod ziparc;
