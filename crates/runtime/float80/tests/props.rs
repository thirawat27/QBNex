//! Property tests (proptest) for the software x87 float. These check algebraic
//! invariants that must hold on every target, including the software path.

#![allow(clippy::pedantic)] // test code: casts and short names on purpose

use proptest::prelude::*;
use qb_float80::{F80, Round, dec, libm};

/// Any 80-bit encoding.
fn any_f80() -> impl Strategy<Value = F80> {
    (any::<u64>(), any::<u16>()).prop_map(|(m, se)| F80 { mant: m, se })
}

/// Finite values (normal or denormal), the ones decimal text round-trips.
fn finite_f80() -> impl Strategy<Value = F80> {
    prop_oneof![
        (any::<u64>(), 1u16..0x7FFF, any::<bool>()).prop_map(|(m, e, s)| F80::from_parts(s, e, m | (1 << 63))),
        (1u64..(1 << 63), any::<bool>()).prop_map(|(m, s)| F80::from_parts(s, 0, m)),
        (any::<u64>(), 0x3FBFu16..0x403F, any::<bool>()).prop_map(|(m, e, s)| F80::from_parts(s, e, m | (1 << 63))),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 20_000, ..ProptestConfig::default() })]

    #[test]
    fn f64_widening_is_exact(bits in any::<u64>()) {
        let d = f64::from_bits(bits);
        prop_assume!(!d.is_nan());
        prop_assert_eq!(F80::from_f64(d).to_f64().to_bits(), bits);
    }

    #[test]
    fn integers_round_trip(v in any::<i64>()) {
        prop_assert_eq!(F80::from_i64(v).to_i64(Round::Nearest), v);
        prop_assert_eq!(F80::from_u64(v as u64).to_u64_cast(), v as u64);
    }

    #[test]
    fn add_and_mul_commute_for_non_nan(a in any_f80(), b in any_f80()) {
        prop_assume!(!a.is_nan() && !b.is_nan());
        prop_assert_eq!(a + b, b + a);
        prop_assert_eq!(a * b, b * a);
    }

    #[test]
    fn sub_is_add_of_negation(a in any_f80(), b in any_f80()) {
        prop_assume!(!a.is_nan() && !b.is_nan());
        prop_assert_eq!(a - b, a + (-b));
    }

}

proptest! {
    // Exact decimal expansion of extreme exponents is slow (big integers).
    #![proptest_config(ProptestConfig { cases: 400, ..ProptestConfig::default() })]

    #[test]
    fn decimal_text_round_trips(x in finite_f80()) {
        // 21 significant digits identify every 64-bit significand.
        let s = dec::format_e(x, 20);
        prop_assert_eq!(dec::parse(s.as_bytes()).0, x, "{}", s);
        let short = dec::format_shortest(x);
        prop_assert_eq!(dec::parse(short.as_bytes()).0, x, "{}", short);
    }

}

proptest! {
    #![proptest_config(ProptestConfig { cases: 20_000, ..ProptestConfig::default() })]

    #[test]
    fn ordering_matches_subtraction_sign(a in finite_f80(), b in finite_f80()) {
        let d = a - b;
        match a.compare(b) {
            Some(core::cmp::Ordering::Less) => prop_assert!(d.sign() && !d.is_zero()),
            Some(core::cmp::Ordering::Greater) => prop_assert!(!d.sign() && !d.is_zero()),
            Some(core::cmp::Ordering::Equal) => prop_assert!(d.is_zero()),
            None => prop_assert!(false, "finite values are ordered"),
        }
    }

    #[test]
    fn sqrt_of_exact_squares(n in 0u32..u32::MAX) {
        let x = F80::from_u64(n as u64);
        prop_assert_eq!((x * x).sqrt(), x);
    }

    #[test]
    fn integer_powers_match_repeated_multiplication(m in any::<u64>(), e in 0x3FF0u16..0x4010, k in 0i32..4) {
        let x = F80::from_parts(false, e, m | (1 << 63));
        let mut want = F80::ONE;
        // __powil squares: x^2 = x*x, x^3 = x * (x*x), x^4 = (x*x)*(x*x)
        let sq = x * x;
        want = match k { 0 => want, 1 => x, 2 => sq, _ => x * sq };
        prop_assert_eq!(libm::powl(x, F80::from_i32(k)), want);
    }

    #[test]
    fn rounding_modes_bracket_the_value(x in finite_f80()) {
        let down = x.round_int(Round::Down);
        let up = x.round_int(Round::Up);
        prop_assert!(down <= x && x <= up);
        let near = x.round_int(Round::Nearest);
        prop_assert!(near == down || near == up);
    }

    #[test]
    fn no_operation_panics(a in any_f80(), b in any_f80(), p in 0usize..40) {
        let _ = (a + b, a - b, a * b, a / b, a.sqrt(), a.scale(b), a.compare(b));
        let _ = (a.to_f64(), a.to_f32(), a.to_i16(Round::Zero), a.to_i32(Round::Up), a.to_u64_cast());
        let _ = (libm::expl(a), libm::powl(a, b), libm::log2l(a), libm::exp2l(a), libm::ldexpl(a, p as i32 - 20));
        let _ = (dec::format_e(a, p), dec::format_g(a, p));
        let _ = qb_float80::qb::pow2(a, b);
    }
}
