//! Microsoft Binary Format (MBF) <-> IEEE 754 conversion.
//!
//! Used by `MKSMBF$`, `CVSMBF`, `MKDMBF$` and `CVDMBF`. The arithmetic is
//! byte-for-byte identical to QB64's `msbin` converter, including its quirks,
//! because files written by old programs depend on it:
//!
//! - IEEE NaN/infinity are not special-cased.
//! - Single precision: IEEE zero/denormals get MBF exponent 2 (not 0), and
//!   an IEEE exponent of 0xFF wraps to MBF exponent 1 without an error.
//! - Double precision: IEEE exponents that do not fit the 8-bit MBF field
//!   (including every tiny value and `-0.0`) report overflow.
//! - `dmsbin_to_ieee` reports overflow when the low mantissa nibble would be
//!   lost, which the caller turns into error 5.
//!
//! All functions return `Ok(bytes)` on success and `Err(bytes)` on overflow;
//! the bytes are what QB64 would have left in the destination buffer.

/// 4-byte MBF -> IEEE single (little-endian byte arrays).
pub fn fmsbin_to_ieee(msbin: [u8; 4]) -> Result<[u8; 4], [u8; 4]> {
    let mut ieee = [0u8; 4];
    let sign = msbin[2] & 0x80;
    if msbin[3] == 0 {
        return Ok(ieee);
    }
    ieee[3] |= sign;
    // MBF bias 128 with the binary point before the hidden bit; IEEE bias 127
    // with it after. QB64 subtracts 2 in `unsigned char` arithmetic.
    let ieee_exp = msbin[3].wrapping_sub(2);
    ieee[3] |= ieee_exp >> 1;
    ieee[2] |= ieee_exp << 7;
    ieee[2] |= msbin[2] & 0x7F;
    ieee[1] = msbin[1];
    ieee[0] = msbin[0];
    Ok(ieee)
}

/// IEEE single -> 4-byte MBF.
pub fn fieee_to_msbin(ieee: [u8; 4]) -> Result<[u8; 4], [u8; 4]> {
    let mut msbin = [0u8; 4];
    let sign = ieee[3] & 0x80;
    let mut msbin_exp: u8 = 0;
    msbin_exp |= ieee[3] << 1;
    msbin_exp |= ieee[2] >> 7;
    if msbin_exp == 0xFE {
        // QB64 leaves the destination untouched; callers zero the length.
        return Err(msbin);
    }
    msbin_exp = msbin_exp.wrapping_add(2);
    msbin[3] = msbin_exp;
    msbin[2] |= sign;
    msbin[2] |= ieee[2] & 0x7F;
    msbin[1] = ieee[1];
    msbin[0] = ieee[0];
    Ok(msbin)
}

/// 8-byte MBF -> IEEE double.
pub fn dmsbin_to_ieee(src: [u8; 8]) -> Result<[u8; 8], [u8; 8]> {
    let mut msbin = src;
    let mut ieee = [0u8; 8];
    let sign = msbin[6] & 0x80;
    if msbin[7] == 0 {
        return Ok(ieee);
    }
    ieee[7] |= sign;
    // QB64 evaluates this left to right in uint32, so the intermediate
    // `msbin[7] - 128` wraps for exponents below 128; the final sum is the same.
    let ieee_exp: u32 = u32::from(msbin[7])
        .wrapping_sub(128)
        .wrapping_sub(1)
        .wrapping_add(1023);
    ieee[7] |= (ieee_exp >> 4) as u8;
    ieee[6] |= (ieee_exp << 4) as u8;

    for i in (1..=6).rev() {
        msbin[i] <<= 1;
        msbin[i] |= msbin[i - 1] >> 7;
    }
    msbin[0] <<= 1;

    for i in (1..=6).rev() {
        ieee[i] |= msbin[i] >> 4;
        ieee[i - 1] |= msbin[i] << 4;
    }
    ieee[0] |= msbin[0] >> 4;

    if msbin[0] & 0x0F != 0 {
        Err(ieee)
    } else {
        Ok(ieee)
    }
}

/// IEEE double -> 8-byte MBF.
pub fn dieee_to_msbin(src: [u8; 8]) -> Result<[u8; 8], [u8; 8]> {
    let mut ieee = src;
    let mut msbin = [0u8; 8];
    if ieee.iter().all(|b| *b == 0) {
        return Ok(msbin);
    }
    let sign = ieee[7] & 0x80;
    msbin[6] |= sign;

    let mut msbin_exp: u32 = u32::from(ieee[7] & 0x7F) << 4;
    msbin_exp += u32::from(ieee[6] >> 4);
    msbin_exp = msbin_exp.wrapping_sub(0x3FF).wrapping_add(0x80).wrapping_add(1);
    if msbin_exp & 0xFF00 != 0 {
        return Err(msbin);
    }
    msbin[7] = msbin_exp as u8;

    ieee[6] &= 0x0F;
    for i in (1..=6).rev() {
        msbin[i] |= ieee[i] << 3;
        msbin[i] |= ieee[i - 1] >> 5;
    }
    msbin[0] |= ieee[0] << 3;
    Ok(msbin)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: f32) -> [u8; 4] {
        v.to_le_bytes()
    }
    fn d(v: f64) -> [u8; 8] {
        v.to_le_bytes()
    }

    #[test]
    fn single_known_values() {
        // MBF 1.0 = 00 00 00 81, -1.0 = 00 00 80 81, 0.5 = 00 00 00 80
        assert_eq!(fieee_to_msbin(s(1.0)), Ok([0, 0, 0, 0x81]));
        assert_eq!(fieee_to_msbin(s(-1.0)), Ok([0, 0, 0x80, 0x81]));
        assert_eq!(fieee_to_msbin(s(0.5)), Ok([0, 0, 0, 0x80]));
        assert_eq!(fmsbin_to_ieee([0, 0, 0, 0x81]), Ok(s(1.0)));
        assert_eq!(fmsbin_to_ieee([0, 0, 0x80, 0x82]), Ok(s(-2.0)));
    }

    #[test]
    fn single_zero_quirk_is_preserved() {
        // QB64 never special-cases IEEE zero: exponent 0 + 2 = 2.
        assert_eq!(fieee_to_msbin(s(0.0)), Ok([0, 0, 0, 2]));
        // MBF exponent 0 means zero regardless of mantissa.
        assert_eq!(fmsbin_to_ieee([0xFF, 0xFF, 0xFF, 0]), Ok([0; 4]));
    }

    #[test]
    fn single_overflow_and_wrap() {
        // IEEE exponent 0xFE (|x| in [2^127, 2^128)) overflows.
        assert!(fieee_to_msbin(s(2f32.powi(127))).is_err());
        // 0xFF (inf) wraps to exponent 1 silently.
        assert_eq!(fieee_to_msbin(s(f32::INFINITY)), Ok([0, 0, 0, 1]));
    }

    #[test]
    fn single_round_trip_normal_range() {
        for bits in (0x0080_0000u32..0x7F00_0000).step_by(7919) {
            for sign in [0u32, 0x8000_0000] {
                let v = f32::from_bits(bits | sign);
                let m = fieee_to_msbin(s(v)).expect("in range");
                assert_eq!(fmsbin_to_ieee(m), Ok(s(v)), "value {v:e}");
            }
        }
    }

    #[test]
    fn double_known_values() {
        assert_eq!(dieee_to_msbin(d(1.0)), Ok([0, 0, 0, 0, 0, 0, 0, 0x81]));
        assert_eq!(dieee_to_msbin(d(-2.5)), Ok([0, 0, 0, 0, 0, 0, 0xA0, 0x82]));
        assert_eq!(dmsbin_to_ieee([0, 0, 0, 0, 0, 0, 0, 0x81]), Ok(d(1.0)));
    }

    #[test]
    fn double_quirks_are_preserved() {
        // -0.0 is "not all zero" and its exponent wraps: overflow.
        assert!(dieee_to_msbin(d(-0.0)).is_err());
        // Tiny and huge values do not fit the 8-bit MBF exponent.
        assert!(dieee_to_msbin(d(1e-300)).is_err());
        assert!(dieee_to_msbin(d(1e300)).is_err());
        // Low mantissa bits that IEEE cannot hold are reported as overflow.
        assert!(dmsbin_to_ieee([0x01, 0, 0, 0, 0, 0, 0, 0x81]).is_err());
        assert!(dmsbin_to_ieee([0x08, 0, 0, 0, 0, 0, 0, 0x81]).is_ok());
    }

    #[test]
    fn double_round_trip_in_mbf_range() {
        // MBF exponent covers roughly 2^-128 .. 2^126.
        let mut v = 1.000_000_123_f64 * 2f64.powi(-120);
        while v < 2f64.powi(125) {
            for x in [v, -v] {
                let m = dieee_to_msbin(d(x)).expect("in range");
                // IEEE -> MBF drops nothing, MBF -> IEEE drops 3 bits.
                let back = f64::from_le_bytes(dmsbin_to_ieee(m).expect("no lost bits"));
                assert_eq!(back.to_bits() >> 3, x.to_bits() >> 3, "value {x:e}");
            }
            v *= 3.7;
        }
    }
}
