//! Digests: CRC32 (IEEE, as used by zip/PNG), MD5, SHA-1, SHA-256, SHA-512.

use md5::Digest;

/// Hash algorithms, numbered as in the C ABI (`kind` argument).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Algorithm {
    Crc32 = 1,
    Md5 = 2,
    Sha1 = 3,
    Sha256 = 4,
    Sha512 = 5,
}

impl Algorithm {
    pub fn from_code(code: i32) -> Option<Self> {
        Some(match code {
            1 => Self::Crc32,
            2 => Self::Md5,
            3 => Self::Sha1,
            4 => Self::Sha256,
            5 => Self::Sha512,
            _ => return None,
        })
    }
}

/// CRC32 (IEEE 802.3 polynomial, reflected, as zlib's `crc32`).
pub fn crc32(data: &[u8]) -> u32 {
    crc32fast::hash(data)
}

/// Raw digest bytes (CRC32 big-endian, 4 bytes).
pub fn digest(alg: Algorithm, data: &[u8]) -> Vec<u8> {
    match alg {
        Algorithm::Crc32 => crc32(data).to_be_bytes().to_vec(),
        Algorithm::Md5 => md5::Md5::digest(data).to_vec(),
        Algorithm::Sha1 => sha1::Sha1::digest(data).to_vec(),
        Algorithm::Sha256 => sha2::Sha256::digest(data).to_vec(),
        Algorithm::Sha512 => sha2::Sha512::digest(data).to_vec(),
    }
}

/// Lower-case hex digest.
pub fn hex_digest(alg: Algorithm, data: &[u8]) -> String {
    hex::encode(digest(alg, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors() {
        let abc = b"abc";
        assert_eq!(hex_digest(Algorithm::Crc32, b"123456789"), "cbf43926");
        assert_eq!(hex_digest(Algorithm::Md5, abc), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(hex_digest(Algorithm::Sha1, abc), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            hex_digest(Algorithm::Sha256, abc),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex_digest(Algorithm::Sha512, abc),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
        assert_eq!(hex_digest(Algorithm::Md5, b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(crc32(b""), 0);
        assert_eq!(Algorithm::from_code(9), None);
    }
}
