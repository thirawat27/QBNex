//! PRINT USING: the format string of the statement, its literal text and fields
//! (`&`, `!`, `\ \`, `#.#`, `+`, `-`, `$$`, `**`, `,`, `^^^^`), and the text each
//! field gives a value.

use super::builtins::{screen_out, suffix_ty, Host};
use super::{e, R, V};
use qb_ir::Ty;

/// The PRINT USING statement being run: its file number, format and the position
/// of the next field.
pub(crate) struct Using {
    pub(crate) ch: i32,
    pub(crate) fmt: Vec<u8>,
    pub(crate) pos: usize,
    pub(crate) used: bool,
}

/// Minimal PRINT USING: literal text, `&`, `!`, `\ \`, and numeric fields `#.#`, `+`, `-`, `$$`, `**`, `,`, `^^^^`.
pub(crate) fn using<'a>(vm: &mut dyn Host<'a>, name: &str, a: &[V]) -> R<Option<V>> {
    let Some(mut u) = vm.rt().using.take() else { return e(5) };
    let ch = u.ch;
    let mut out = Vec::new();
    if name == "USINGEND" {
        // literal text up to the next field (or the end)
        let (lit, _) = scan_literal(&u.fmt, u.pos);
        out.extend_from_slice(&lit);
        using_out(vm, ch, out)?;
        return Ok(None);
    }
    let mut wrapped = false;
    let field = loop {
        let (lit, next) = scan_literal(&u.fmt, u.pos);
        out.extend_from_slice(&lit);
        u.pos = next;
        if u.pos >= u.fmt.len() {
            if wrapped || !u.used && u.pos == 0 {
                return e(5);
            }
            wrapped = true;
            u.pos = 0;
            continue;
        }
        break field_at(&u.fmt, u.pos);
    };
    let (kind, len) = field;
    u.used = true;
    let fmt = u.fmt[u.pos..u.pos + len].to_vec();
    u.pos += len;
    match (kind, name) {
        (b'S', "USINGS") => {
            let s = a[0].s();
            match fmt[0] {
                b'!' => out.push(*s.first().unwrap_or(&b' ')),
                b'&' => out.extend_from_slice(s),
                _ => {
                    let n = fmt.len();
                    let mut t = s[..s.len().min(n)].to_vec();
                    t.resize(n, b' ');
                    out.extend_from_slice(&t);
                }
            }
        }
        (b'N', n) if n.starts_with("USINGN.") => {
            out.extend_from_slice(&format_num(&fmt, &a[0], suffix_ty(n)));
        }
        _ => return e(13),
    }
    using_out(vm, ch, out)?;
    vm.rt().using = Some(u);
    Ok(None)
}

/// PRINT USING text: the screen page when it is the destination (as PRINT), else stdout or the file.
fn using_out(vm: &mut dyn Host<'_>, ch: i32, out: Vec<u8>) -> R<()> {
    if ch == 0 && screen_out(vm)
        && let Some(r) = super::display::call(vm, "SCREENPRINT", &[V::S(out.clone())]) {
            return r.map(|_| ());
        }
    vm.rt().write(ch, &out)
}

fn is_num_start(f: &[u8], p: usize) -> bool {
    let c = f[p];
    let n = f.get(p + 1).copied().unwrap_or(0);
    let n2 = f.get(p + 2).copied().unwrap_or(0);
    c == b'#'
        || (c == b'.' && n == b'#')
        || (c == b'+' && (n == b'#' || (n == b'.' && n2 == b'#') || n == b'$' || n == b'*'))
        || (c == b'$' && n == b'$')
        || (c == b'*' && n == b'*')
}

/// Returns (literal bytes, position of next field or end).
fn scan_literal(f: &[u8], mut p: usize) -> (Vec<u8>, usize) {
    let mut lit = Vec::new();
    while p < f.len() {
        let c = f[p];
        if c == b'_' && p + 1 < f.len() {
            lit.push(f[p + 1]);
            p += 2;
            continue;
        }
        if c == b'!' || c == b'&' || is_num_start(f, p) {
            break;
        }
        if c == b'\\'
            && let Some(k) = f[p + 1..].iter().position(|x| *x == b'\\')
                && f[p + 1..p + 1 + k].iter().all(|x| *x == b' ') {
                    break;
                }
        lit.push(c);
        p += 1;
    }
    (lit, p)
}

fn field_at(f: &[u8], p: usize) -> (u8, usize) {
    match f[p] {
        b'!' | b'&' => (b'S', 1),
        b'\\' => (b'S', f[p + 1..].iter().position(|x| *x == b'\\').unwrap() + 2),
        _ => {
            let mut q = p;
            if f[q] == b'+' {
                q += 1;
            }
            if f[q..].starts_with(b"**$") {
                q += 3;
            } else if f[q..].starts_with(b"**") || f[q..].starts_with(b"$$") {
                q += 2;
            }
            while q < f.len() && (f[q] == b'#' || f[q] == b',') {
                q += 1;
            }
            if q < f.len() && f[q] == b'.' {
                q += 1;
                while q < f.len() && f[q] == b'#' {
                    q += 1;
                }
            }
            if f[q..].starts_with(b"^^^^^") {
                q += 5;
            } else if f[q..].starts_with(b"^^^^") {
                q += 4;
            }
            if q < f.len() && (f[q] == b'+' || f[q] == b'-') && f[p] != b'+' {
                q += 1;
            }
            (b'N', q - p)
        }
    }
}

fn format_num(fmt: &[u8], val: &V, t: Ty) -> Vec<u8> {
    let v = val.f();
    let x80 = (t == Ty::F80).then(|| val.x().abs());
    let lead_plus = fmt[0] == b'+';
    let trail = match fmt.last() {
        Some(b'+') if !lead_plus => Some(b'+'),
        Some(b'-') => Some(b'-'),
        _ => None,
    };
    let body: &[u8] = &fmt[usize::from(lead_plus)..fmt.len() - usize::from(trail.is_some())];
    let stars = body.starts_with(b"**");
    let dollar = body.starts_with(b"$$") || body.starts_with(b"**$");
    let comma = body.contains(&b',');
    let exp = body.iter().filter(|c| **c == b'^').count();
    let dot = body.iter().position(|c| *c == b'.');
    let int_w = dot.unwrap_or(body.len() - exp);
    let int_w = body[..int_w].iter().filter(|c| matches!(c, b'#' | b'*' | b'$' | b',')).count();
    let dec = dot.map_or(0, |d| body[d + 1..].iter().filter(|c| **c == b'#').count());
    let neg = v < 0.0;
    let a = v.abs();
    let a = if t == Ty::F32 { f64::from(a as f32) } else { a };
    // Infinity and NaN fit no field: the overflow mark `%` (as for a number too
    // wide for its field), the sign and INF / NAN
    let (special, nan) = match x80 {
        Some(x) => (x.exp() == 0x7FFF, x.mant << 1 != 0),
        None => (!a.is_finite(), a.is_nan()),
    };
    if special {
        let mut o = vec![b'%'];
        if neg && !nan {
            o.push(b'-');
        } else if lead_plus && !nan {
            o.push(b'+');
        }
        o.extend_from_slice(if nan { b"NAN" } else { b"INF" });
        return o;
    }
    let mut digits;
    if exp > 0 {
        let p = int_w.saturating_sub(usize::from(!lead_plus && trail.is_none())).max(1);
        let s = match x80 {
            Some(x) => qb_float80::dec::format_e(x, p - 1 + dec),
            None => format!("{:.*e}", p - 1 + dec, a),
        };
        let Some((m, ex)) = s.split_once('e') else { return vec![b'%'] };
        let ex: i32 = ex.parse().unwrap_or(0) - (p as i32 - 1);
        let m: String = m.chars().filter(|c| *c != '.').collect();
        let (ip, fp) = m.split_at(p.min(m.len()));
        digits = ip.to_string();
        if dot.is_some() {
            digits.push('.');
            digits.push_str(fp);
        }
        let w = if exp >= 5 { 3 } else { 2 };
        let letter = match t {
            Ty::F32 => 'E',
            Ty::F80 => 'F',
            _ => 'D',
        };
        digits.push_str(&format!("{letter}{}{:0w$}", if ex < 0 { '-' } else { '+' }, ex.abs(), w = w));
    } else {
        digits = match x80 {
            Some(x) => qb_float80::dec::format_f(x, dec),
            None => format!("{:.*}", dec, a),
        };
        if dec == 0 && dot.is_some() {
            digits.push('.');
        }
        if digits.starts_with("0.") && int_w == 0 {
            digits.remove(0);
        }
        if comma {
            let (ip, rest) = digits.split_at(digits.find('.').unwrap_or(digits.len()));
            let mut g = String::new();
            for (k, c) in ip.chars().enumerate() {
                if k > 0 && (ip.len() - k) % 3 == 0 {
                    g.push(',');
                }
                g.push(c);
            }
            digits = g + rest;
        }
    }
    let mut s = String::new();
    if dollar {
        s.push('$');
    }
    s.push_str(&digits);
    if lead_plus {
        // the sign takes the first column of the field; the number is right-aligned after it
        let width = fmt.len() - 1;
        let body = s.into_bytes();
        let mut r = vec![if neg { b'-' } else { b'+' }];
        if body.len() > width {
            let mut o = vec![b'%'];
            o.extend_from_slice(&r);
            o.extend_from_slice(&body);
            return o;
        }
        r.extend(std::iter::repeat_n(if stars { b'*' } else { b' ' }, width - body.len()));
        r.extend_from_slice(&body);
        return r;
    } else if neg && trail.is_none() {
        s.insert(0, '-');
    }
    if let Some(tc) = trail {
        s.push(if neg { '-' } else if tc == b'+' { '+' } else { ' ' });
    }
    let width = fmt.len();
    let mut out = s.into_bytes();
    if out.len() > width {
        let mut o = vec![b'%'];
        o.extend_from_slice(&out);
        return o;
    }
    let fillc = if stars { b'*' } else { b' ' };
    let mut r = vec![fillc; width - out.len()];
    r.append(&mut out);
    r
}

#[cfg(test)]
mod using_tests {
    use super::super::builtins::print_count;
    use super::{format_num, V};
    use qb_ir::Ty;

    #[test]
    fn spc_and_tab_counts_stay_bounded() {
        // stdout: MOD the 80-column width past it; files: at most 32767
        assert_eq!(print_count(0, 1_000_000_000_000), (1_000_000_000_000i64 % 80) as usize);
        assert_eq!(print_count(0, 80), 80);
        assert_eq!(print_count(0, 85), 5);
        assert_eq!(print_count(0, -3), 0);
        assert_eq!(print_count(1, 1_000_000_000_000), 32767);
        assert_eq!(print_count(1, 100), 100);
    }

    #[test]
    fn print_using_infinity_and_nan_get_the_overflow_mark() {
        for f in [&b"#.##^^^^"[..], b"##.##", b"+#.##^^^^^"] {
            let plus = f[0] == b'+';
            let pos: &[u8] = if plus { b"%+INF" } else { b"%INF" };
            assert_eq!(format_num(f, &V::F(f64::INFINITY), Ty::F64), pos);
            assert_eq!(format_num(f, &V::F(f64::NEG_INFINITY), Ty::F64), b"%-INF");
            assert_eq!(format_num(f, &V::F(f64::NAN), Ty::F32), b"%NAN");
            let x = V::X(qb_float80::F80::from_f64(f64::INFINITY));
            assert_eq!(format_num(f, &x, Ty::F80), pos);
        }
        // finite values still go through the exponent format
        let f = String::from_utf8(format_num(b"#.##^^^^", &V::F(1234.5), Ty::F64)).unwrap();
        assert!(f.contains("1.23D+03") && !f.starts_with('%'), "{f}");
    }
}
