//! Decimal text <-> [`F80`], exact (correctly rounded, ties to even), like MinGW
//! `__mingw_strtold` and `__mingw_snprintf` with `%.*Le` / `%.*Lf` / `%.*Lg`.

use crate::arith::{fin, round_pack, zero};
use crate::{F80, INT_BIT, QUIET_BIT};

/// Unsigned big integer, little-endian 32-bit limbs, no leading zero limbs.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Big(Vec<u32>);

impl Big {
    fn from_u128(mut v: u128) -> Big {
        let mut l = Vec::new();
        while v != 0 {
            l.push(v as u32);
            v >>= 32;
        }
        Big(l)
    }

    fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    fn trim(&mut self) {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
    }

    fn mul_add_small(&mut self, m: u32, add: u32) {
        let mut carry = add as u64;
        for limb in &mut self.0 {
            let t = *limb as u64 * m as u64 + carry;
            *limb = t as u32;
            carry = t >> 32;
        }
        if carry != 0 {
            self.0.push(carry as u32);
        }
    }

    fn mul_pow(&mut self, base: u32, mut n: u32) {
        // Largest power of base that fits in u32.
        let (chunk, k) = match base {
            5 => (1_220_703_125u32, 13),
            10 => (1_000_000_000u32, 9),
            _ => unreachable!(),
        };
        while n >= k {
            self.mul_add_small(chunk, 0);
            n -= k;
        }
        if n > 0 {
            self.mul_add_small(base.pow(n), 0);
        }
    }

    fn bit_len(&self) -> u32 {
        match self.0.last() {
            None => 0,
            Some(&top) => (self.0.len() as u32 - 1) * 32 + 32 - top.leading_zeros(),
        }
    }

    fn shl(&mut self, bits: u32) {
        if self.is_zero() {
            return;
        }
        let limbs = (bits / 32) as usize;
        let rest = bits % 32;
        if rest != 0 {
            let mut carry = 0u32;
            for limb in &mut self.0 {
                let next = *limb >> (32 - rest);
                *limb = (*limb << rest) | carry;
                carry = next;
            }
            if carry != 0 {
                self.0.push(carry);
            }
        }
        if limbs != 0 {
            self.0.splice(0..0, core::iter::repeat_n(0, limbs));
        }
    }

    /// Shift right by one bit.
    fn shr1(&mut self) {
        let mut carry = 0u32;
        for limb in self.0.iter_mut().rev() {
            let next = *limb & 1;
            *limb = (*limb >> 1) | (carry << 31);
            carry = next;
        }
        self.trim();
    }

    fn cmp(&self, o: &Big) -> core::cmp::Ordering {
        self.0.len().cmp(&o.0.len()).then_with(|| self.0.iter().rev().cmp(o.0.iter().rev()))
    }

    fn sub_assign(&mut self, o: &Big) {
        let mut borrow = 0i64;
        for i in 0..self.0.len() {
            let t = self.0[i] as i64 - borrow - *o.0.get(i).unwrap_or(&0) as i64;
            if t < 0 {
                self.0[i] = (t + (1 << 32)) as u32;
                borrow = 1;
            } else {
                self.0[i] = t as u32;
                borrow = 0;
            }
        }
        self.trim();
    }

    /// `self /= d`, returns the remainder.
    fn divmod_small(&mut self, d: u32) -> u32 {
        let mut rem = 0u64;
        for limb in self.0.iter_mut().rev() {
            let cur = (rem << 32) | *limb as u64;
            *limb = (cur / d as u64) as u32;
            rem = cur % d as u64;
        }
        self.trim();
        rem as u32
    }

    /// Decimal digits (values 0..=9), most significant first; empty for zero.
    fn into_digits(mut self) -> Vec<u8> {
        let mut chunks = Vec::new();
        while !self.is_zero() {
            chunks.push(self.divmod_small(1_000_000_000));
        }
        let mut out = Vec::with_capacity(chunks.len() * 9);
        for (i, c) in chunks.iter().rev().enumerate() {
            let s = format!("{c:09}");
            let s = if i == 0 { s.trim_start_matches('0').to_string() } else { s };
            out.extend(s.bytes().map(|b| b - b'0'));
        }
        out
    }
}

/// Exact decimal expansion of a finite non-zero value: `value = 0.d1d2... * 10^point`.
fn exact_digits(x: F80) -> (Vec<u8>, i32) {
    let mut f = fin(x);
    if x.exp() == 0 && x.mant & INT_BIT != 0 {
        f.exp -= 1; // printf reads a pseudo-denormal with exponent 0
    }
    let tz = f.mant.trailing_zeros();
    let (m, e) = (f.mant >> tz, f.exp + tz as i32);
    let mut n = Big::from_u128(m as u128);
    let (digits, point) = if e >= 0 {
        n.shl(e as u32);
        let d = n.into_digits();
        let p = d.len() as i32;
        (d, p)
    } else {
        n.mul_pow(5, (-e) as u32);
        let d = n.into_digits();
        let p = d.len() as i32 + e;
        (d, p)
    };
    let mut d = digits;
    while d.len() > 1 && d.last() == Some(&0) {
        d.pop();
    }
    (d, point)
}

/// Rounds `0.digits * 10^point` to `keep` digits (ties to even). The result
/// has exactly `max(keep,0)` digits (zero padded), or is `[1]` with
/// `point+1`... digits when rounding carried out. A result of zero is `([], point)`.
fn round_digits(digits: &[u8], point: i32, keep: i32) -> (Vec<u8>, i32) {
    if keep < 0 {
        return (Vec::new(), point);
    }
    let keep = keep as usize;
    if keep >= digits.len() {
        let mut d = digits.to_vec();
        d.resize(keep, 0);
        return (d, point);
    }
    let mut kept = digits[..keep].to_vec();
    let rest = &digits[keep..];
    let up = match rest[0].cmp(&5) {
        core::cmp::Ordering::Greater => true,
        core::cmp::Ordering::Less => false,
        core::cmp::Ordering::Equal => {
            if rest[1..].iter().any(|&d| d != 0) {
                true
            } else {
                kept.last().is_some_and(|&d| d & 1 == 1)
            }
        }
    };
    if !up {
        return (kept, point);
    }
    let mut i = kept.len();
    loop {
        if i == 0 {
            // All nines (or keep == 0): becomes 1 followed by zeros.
            let mut d = vec![1u8];
            d.resize(keep.max(1), 0);
            if keep == 0 {
                // Kept nothing; the result is one unit at the rounding position.
                return (vec![1], point + 1);
            }
            return (d, point + 1);
        }
        i -= 1;
        if kept[i] == 9 {
            kept[i] = 0;
        } else {
            kept[i] += 1;
            return (kept, point);
        }
    }
}

/// How MinGW's printf sees an encoding: exponent 0x7FFF is `inf` (also the
/// pseudo-infinity) or `nan` (NaNs never get a sign); a zero significand is
/// zero; anything else is `mant * 2^(e-16446)`, with `e = 1` for a denormal
/// but `e = 0` for a pseudo-denormal (half its x87 value), so unnormals print as
/// numbers.
fn special_text(x: F80) -> Option<String> {
    if x.exp() != 0x7FFF {
        return None;
    }
    if x.mant << 1 == 0 {
        return Some(if x.sign() { "-inf" } else { "inf" }.to_string());
    }
    Some("nan".to_string())
}

/// Zero for printing purposes.
fn print_zero(x: F80) -> bool {
    x.mant == 0
}

fn digit_str(d: &[u8]) -> String {
    d.iter().map(|&b| (b + b'0') as char).collect()
}

/// `printf("%.*Le", prec, x)`.
pub fn format_e(x: F80, prec: usize) -> String {
    if let Some(s) = special_text(x) {
        return s;
    }
    let sign = if x.sign() { "-" } else { "" };
    let (d, exp10) = if print_zero(x) {
        (vec![0u8; prec + 1], 0)
    } else {
        let (digits, point) = exact_digits(x);
        let (d, point) = round_digits(&digits, point, prec as i32 + 1);
        (d, point - 1)
    };
    let mut out = String::from(sign);
    out.push((d[0] + b'0') as char);
    if prec > 0 {
        out.push('.');
        out.push_str(&digit_str(&d[1..]));
    }
    out.push('e');
    out.push(if exp10 < 0 { '-' } else { '+' });
    out.push_str(&format!("{:02}", exp10.abs()));
    out
}

/// `printf("%.*Lf", prec, x)`.
pub fn format_f(x: F80, prec: usize) -> String {
    if let Some(s) = special_text(x) {
        return s;
    }
    let sign = if x.sign() { "-" } else { "" };
    let (d, point) = if print_zero(x) {
        (Vec::new(), 0)
    } else {
        let (digits, point) = exact_digits(x);
        round_digits(&digits, point, point + prec as i32)
    };
    // Digit at position i (0-based, from the decimal point) of 0.d * 10^point.
    let at = |i: i32| -> u8 {
        if i < 0 {
            return 0;
        }
        *d.get(i as usize).unwrap_or(&0)
    };
    let mut out = String::from(sign);
    if point <= 0 || d.is_empty() {
        out.push('0');
    } else {
        for i in 0..point {
            out.push((at(i) + b'0') as char);
        }
    }
    if prec > 0 {
        out.push('.');
        for k in 0..prec as i32 {
            let i = point + k;
            out.push((if d.is_empty() { 0 } else { at(i) } + b'0') as char);
        }
    }
    out
}

/// `printf("%.*Lg", prec, x)`.
pub fn format_g(x: F80, prec: usize) -> String {
    if let Some(s) = special_text(x) {
        return s;
    }
    let p = prec.max(1) as i32;
    let exp10 = if print_zero(x) {
        0
    } else {
        let (digits, point) = exact_digits(x);
        let (_, point) = round_digits(&digits, point, p);
        point - 1
    };
    let s = if p > exp10 && exp10 >= -4 {
        format_f(x, (p - 1 - exp10) as usize)
    } else {
        format_e(x, (p - 1) as usize)
    };
    strip_zeros(&s)
}

fn strip_zeros(s: &str) -> String {
    let (mant, exp) = match s.find('e') {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, ""),
    };
    if !mant.contains('.') {
        return s.to_string();
    }
    let m = mant.trim_end_matches('0').trim_end_matches('.');
    format!("{m}{exp}")
}

/// Shortest `%Lg` text (1..=21 significant digits) that parses back to `x`.
pub fn format_shortest(x: F80) -> String {
    if let Some(s) = special_text(x) {
        return s;
    }
    for p in 1..=21 {
        let s = format_g(x, p);
        if parse(s.as_bytes()).0 == x {
            return s;
        }
    }
    format_g(x, 21)
}

/// `__mingw_strtold`: returns the value and the number of bytes consumed
/// (0 when no number was found; the value is then +0).
pub fn parse(s: &[u8]) -> (F80, usize) {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\x0B' | b'\x0C' | b'\r') {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let rest = &s[i..];
    let starts = |w: &[u8]| rest.len() >= w.len() && rest[..w.len()].eq_ignore_ascii_case(w);
    if starts(b"infinity") {
        return (F80::INFINITY.with_sign(neg), i + 8);
    }
    if starts(b"inf") {
        return (F80::INFINITY.with_sign(neg), i + 3);
    }
    if starts(b"nan") {
        let (mant, used) = nan_payload(&s[i + 3..]);
        return (F80::from_parts(neg, 0x7FFF, mant), i + 3 + used);
    }
    if rest.len() >= 2 && rest[0] == b'0' && (rest[1] | 0x20) == b'x' {
        if let Some((v, n)) = parse_hex(&rest[2..], neg) {
            return (v, i + 2 + n);
        }
        return (zero(neg), i + 1);
    }
    // Decimal.
    let mut digits: Vec<u8> = Vec::new();
    let mut e10: i64 = 0;
    let mut seen = false;
    let mut j = i;
    while j < s.len() && s[j].is_ascii_digit() {
        seen = true;
        if !(digits.is_empty() && s[j] == b'0') {
            digits.push(s[j] - b'0');
        }
        j += 1;
    }
    if j < s.len() && s[j] == b'.' {
        let mut k = j + 1;
        while k < s.len() && s[k].is_ascii_digit() {
            seen = true;
            if digits.is_empty() && s[k] == b'0' {
                e10 -= 1;
            } else {
                digits.push(s[k] - b'0');
                e10 -= 1;
            }
            k += 1;
        }
        if seen {
            j = k;
        }
    }
    if !seen {
        // gdtoa keeps the sign of "-x" but not of a lone "-" at the end.
        let at_end = i >= s.len() || s[i] == 0;
        return (if neg && !at_end { F80::NEG_ZERO } else { F80::ZERO }, 0);
    }
    if j < s.len() && (s[j] | 0x20) == b'e' {
        let mut k = j + 1;
        let mut eneg = false;
        if k < s.len() && (s[k] == b'+' || s[k] == b'-') {
            eneg = s[k] == b'-';
            k += 1;
        }
        if k < s.len() && s[k].is_ascii_digit() {
            let mut ev: i64 = 0;
            while k < s.len() && s[k].is_ascii_digit() {
                ev = (ev * 10 + (s[k] - b'0') as i64).min(1_000_000_000);
                k += 1;
            }
            e10 += if eneg { -ev } else { ev };
            j = k;
        }
    }
    (decimal_to_f80(neg, &digits, e10, false), j)
}

/// `nan(...)` of gdtoa's `hexnan`: returns the significand and the bytes used
/// after "nan". Hex digits (optional `0x`, at most the first 16 kept) up to the
/// first other character; the text is consumed only through a closing `)`.
fn nan_payload(s: &[u8]) -> (u64, usize) {
    const DEFAULT: u64 = INT_BIT | QUIET_BIT;
    if s.first() != Some(&b'(') {
        return (DEFAULT, 0);
    }
    let mut k = 1;
    while k < s.len() && s[k] != 0 && s[k] <= b' ' {
        k += 1;
    }
    if k + 1 < s.len() && s[k] == b'0' && (s[k + 1] | 0x20) == b'x' {
        k += 2;
    }
    let mut m: u64 = 0;
    let mut ndig = 0;
    let mut stopped = false;
    let mut closed = None;
    while k < s.len() && s[k] != 0 {
        let c = s[k];
        if c == b')' {
            closed = Some(k + 1);
            break;
        }
        if !stopped {
            if let Some(h) = (c as char).to_digit(16) {
                if ndig < 16 {
                    m = (m << 4) | h as u64;
                }
                ndig += 1;
            } else if c > b' ' {
                stopped = true;
            }
        }
        k += 1;
    }
    let mant = if ndig == 0 {
        DEFAULT
    } else if m == 0 {
        1
    } else {
        m
    };
    (mant, closed.unwrap_or(0))
}

fn parse_hex(s: &[u8], neg: bool) -> Option<(F80, usize)> {
    let hex = |b: u8| (b as char).to_digit(16);
    let mut m: u128 = 0;
    let mut sticky = false;
    let mut exp: i64 = 0;
    let mut seen = false;
    let mut j = 0;
    while j < s.len() {
        let Some(h) = hex(s[j]) else { break };
        seen = true;
        if m >> 120 == 0 {
            m = (m << 4) | h as u128;
        } else {
            sticky |= h != 0;
            exp += 4;
        }
        j += 1;
    }
    if j < s.len() && s[j] == b'.' {
        let mut k = j + 1;
        while k < s.len() {
            let Some(h) = hex(s[k]) else { break };
            seen = true;
            if m >> 120 == 0 {
                m = (m << 4) | h as u128;
                exp -= 4;
            } else {
                sticky |= h != 0;
            }
            k += 1;
        }
        if seen {
            j = k;
        }
    }
    if !seen {
        return None;
    }
    if j < s.len() && (s[j] | 0x20) == b'p' {
        let mut k = j + 1;
        let mut eneg = false;
        if k < s.len() && (s[k] == b'+' || s[k] == b'-') {
            eneg = s[k] == b'-';
            k += 1;
        }
        if k < s.len() && s[k].is_ascii_digit() {
            let mut ev: i64 = 0;
            while k < s.len() && s[k].is_ascii_digit() {
                ev = (ev * 10 + (s[k] - b'0') as i64).min(1_000_000_000);
                k += 1;
            }
            exp += if eneg { -ev } else { ev };
            j = k;
        }
    }
    if m == 0 {
        return Some((zero(neg), j));
    }
    let e = exp.clamp(-100_000, 100_000) as i32;
    Some((round_pack(neg, e, m, sticky), j))
}

/// Correctly rounded `(-1)^neg * (digits + tail) * 10^e10`, where `digits`
/// holds decimal digit values (0..=9) and `inexact_tail` says that non-zero
/// digits were dropped after them. Used by `VAL`.
pub fn decimal_to_f80(neg: bool, digits: &[u8], e10: i64, inexact_tail: bool) -> F80 {
    let first = digits.iter().position(|&d| d != 0);
    let Some(first) = first else {
        return zero(neg);
    };
    let digits = &digits[first..];
    let n = digits.len() as i64;
    if e10 + n - 1 > 4933 {
        return F80::INFINITY.with_sign(neg);
    }
    if e10 + n < -4952 {
        return zero(neg);
    }
    let mut num = Big(Vec::new());
    for &d in digits {
        if num.is_zero() {
            num = Big::from_u128(d as u128);
        } else {
            num.mul_add_small(10, d as u32);
        }
    }
    let mut den = Big::from_u128(1);
    if e10 >= 0 {
        num.mul_pow(10, e10 as u32);
    } else {
        den.mul_pow(10, (-e10) as u32);
    }
    let b = num.bit_len() as i64 - den.bit_len() as i64;
    let scale = 66 - b; // quotient has 66..=67 bits
    if scale >= 0 {
        num.shl(scale as u32);
    } else {
        den.shl((-scale) as u32);
    }
    // Long division with a quotient below 2^68.
    let mut q: u128 = 0;
    let mut d = den.clone();
    d.shl(67);
    for bit in (0..68).rev() {
        if num.cmp(&d) != core::cmp::Ordering::Less {
            num.sub_assign(&d);
            q |= 1 << bit;
        }
        d.shr1();
    }
    let sticky = !num.is_zero() || inexact_tail;
    round_pack(neg, -(scale as i32), q, sticky)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> F80 {
        parse(s.as_bytes()).0
    }

    #[test]
    fn parses_like_strtold() {
        assert_eq!(p("1"), F80::ONE);
        assert_eq!(p("0.1"), F80::from_parts(false, 0x3FFB, 0xCCCC_CCCC_CCCC_CCCD));
        assert_eq!(p("-0"), F80::NEG_ZERO);
        assert_eq!(p("1e5000"), F80::INFINITY);
        assert_eq!(p("1e-5000"), F80::ZERO);
        assert_eq!(p("3.64519953188247460253e-4951"), F80::MIN_DENORMAL);
        assert_eq!(p("0x1p-16445"), F80::MIN_DENORMAL);
        assert_eq!(p("0x1.8p1"), F80::from_f64(3.0));
        assert_eq!(p("  -inf"), F80::NEG_INFINITY);
        assert!(p("nan").is_nan());
        assert_eq!(parse(b"12abc").1, 2);
        assert_eq!(parse(b"1e+").1, 1);
        assert_eq!(parse(b".").1, 0);
        assert_eq!(parse(b"0x").1, 1);
    }

    #[test]
    fn formats_like_printf() {
        let tenth = p("0.1");
        assert_eq!(format_e(tenth, 20), "1.00000000000000000001e-01");
        assert_eq!(format_e(F80::ONE, 0), "1e+00");
        assert_eq!(format_f(p("2.5"), 0), "2");
        assert_eq!(format_f(p("3.5"), 0), "4");
        assert_eq!(format_f(p("0.125"), 2), "0.12");
        assert_eq!(format_f(p("0.375"), 2), "0.38");
        assert_eq!(format_f(p("0.5"), 0), "0");
        assert_eq!(format_f(p("1.5"), 0), "2");
        assert_eq!(format_f(p("0.00049"), 3), "0.000");
        assert_eq!(format_f(p("-0.00049"), 3), "-0.000");
        assert_eq!(format_f(p("999.9996"), 3), "1000.000");
        assert_eq!(format_f(F80::ZERO, 2), "0.00");
        assert_eq!(format_g(p("100000"), 6), "100000");
        assert_eq!(format_g(p("1000000"), 6), "1e+06");
        assert_eq!(format_g(p("0.0001"), 6), "0.0001");
        assert_eq!(format_g(p("0.00001"), 6), "1e-05");
        assert_eq!(format_e(F80::MAX, 20), "1.18973149535723176502e+4932");
        assert_eq!(format_e(F80::NEG_INFINITY, 3), "-inf");
    }

    #[test]
    fn shortest_round_trips() {
        for s in ["0.1", "1", "3.14159265358979323851", "1e-4950", "-2.5e300"] {
            let v = p(s);
            assert_eq!(p(&format_shortest(v)), v, "{s}");
        }
        assert_eq!(format_shortest(p("0.1")), "0.1");
    }
}
