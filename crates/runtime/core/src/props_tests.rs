//! R14 property tests (proptest) for pure runtime functions. Unit-test module:
//! the crate needs libqb host symbols that only the test build provides.

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 20_000, ..ProptestConfig::default() })]

    /// MKSMBF$/CVSMBF: any SINGLE that converts to MBF converts back unchanged.
    #[test]
    fn single_mbf_round_trip(bits in any::<u32>()) {
        let ieee = bits.to_le_bytes();
        if let Ok(m) = crate::mbf::fieee_to_msbin(ieee) {
            let back = crate::mbf::fmsbin_to_ieee(m);
            let f = f32::from_bits(bits);
            // MBF has no signed zero: -0 comes back as +0; tiny values underflow.
            let want = if f == 0.0 || m[3] == 0 { 0f32.to_bits().to_le_bytes() } else { ieee };
            prop_assert_eq!(back, Ok(want), "{:e}", f);
        }
    }

    /// MKDMBF$/CVDMBF: an MBF double that converts to IEEE converts back to the
    /// same MBF bytes (IEEE has more exponent range; MBF 3 more mantissa bits).
    #[test]
    fn double_mbf_to_ieee_and_back(bits in any::<u64>()) {
        let ieee = bits.to_le_bytes();
        if let Ok(m) = crate::mbf::dieee_to_msbin(ieee) {
            let d = f64::from_bits(bits);
            let back = crate::mbf::dmsbin_to_ieee(m);
            // Magnitudes below MBF's range underflow to zero (legacy msbin.cpp).
            let underflow = m[7] == 0;
            prop_assert!(!underflow || d.abs() < 3e-39, "{:e} became MBF zero", d);
            let want = if d == 0.0 || underflow { 0f64.to_bits().to_le_bytes() } else { ieee };
            prop_assert_eq!(back, Ok(want), "{:e}", d);
        }
    }

    /// HEX$/OCT$/_BIN$ read back as the same two's complement value.
    #[test]
    fn radix_strings_parse_back(v in any::<i64>()) {
        let as_u = v as u64;
        for (text, radix) in [(crate::mathfn::hex(v, 16), 16), (crate::mathfn::oct(v, 64), 8), (crate::mathfn::bin(v, 64), 2)] {
            let s = String::from_utf8(text).expect("ascii digits");
            prop_assert_eq!(u64::from_str_radix(&s, radix).ok(), Some(as_u), "{} radix {}", s, radix);
        }
    }

    /// STR$ of integers: sign column, digits that read back.
    #[test]
    fn str_of_integers(v in any::<i64>()) {
        let s = String::from_utf8(crate::numfmt::str_i64(v)).expect("ascii");
        prop_assert_eq!(s.starts_with(' '), v >= 0);
        prop_assert_eq!(s.trim().parse::<i64>().ok(), Some(v));
    }

    /// VAL of STR$ of a DOUBLE: STR$ keeps 15-16 significant digits.
    #[test]
    fn val_of_str_is_close(bits in any::<u64>()) {
        let d = f64::from_bits(bits);
        prop_assume!(d.is_finite() && d.abs() > 1e-300 && d.abs() < 1e300);
        let s = crate::numfmt::str_f64(d);
        let back = crate::val::f64_from_f80(crate::val::val(&s).expect("STR$ output parses"));
        prop_assert!(((back - d) / d).abs() < 1e-14, "{} -> {:?} -> {}", d, String::from_utf8_lossy(&s), back);
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 500, ..ProptestConfig::default() })]

    /// _DEFLATE$ / _INFLATE$ round trip.
    #[test]
    fn deflate_round_trip(data in proptest::collection::vec(any::<u8>(), 0..5000), rep in 1usize..8) {
        let src: Vec<u8> = data.iter().flat_map(|&b| std::iter::repeat_n(b, rep)).collect();
        let z = crate::compress::deflate(&src);
        prop_assert!(z.len() <= crate::compress::deflate_bound(src.len()));
        let mut out = vec![0u8; src.len()];
        let (code, used) = crate::compress::inflate(&z, &mut out);
        prop_assert_eq!((code, used), (0, src.len()));
        prop_assert_eq!(out, src);
    }
}