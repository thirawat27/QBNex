//! Builtin functions called with `FN <name> <argc>`. Result types follow the
//! QB64 C++ backend (checked against conformance golden output).

use crate::{float_result, str_of, Val};

/// First N bytes of a string argument (CVx); shorter strings are error 5.
fn fixed<const N: usize>(v: Val) -> Result<[u8; N], i32> {
    let s = v.s()?;
    s.get(..N).and_then(|b| b.try_into().ok()).ok_or(5)
}

fn count(v: &Val) -> Result<usize, i32> {
    let n = v.i()?;
    usize::try_from(n).map_err(|_| 5)
}

fn float_fn(a: &Val, f: impl Fn(f64) -> f64) -> Result<Val, i32> {
    Ok(float_result(f(a.f()?), a.rank().max(2)))
}

/// HEX$/OCT$: INTEGER prints 16 bits, everything else 32 bits.
fn radix(a: &Val, hex: bool) -> Result<Val, i32> {
    let v = a.i()?;
    let bits = match a {
        Val::I(_) => u64::from(v as i16 as u16),
        _ => u64::from(v as i32 as u32),
    };
    Ok(Val::S(if hex { format!("{bits:X}") } else { format!("{bits:o}") }.into_bytes()))
}

fn instr(start: usize, s: &[u8], t: &[u8]) -> i32 {
    if start == 0 || start > s.len() + usize::from(t.is_empty()) {
        return 0;
    }
    if t.is_empty() {
        return start as i32;
    }
    s[start - 1..]
        .windows(t.len())
        .position(|w| w == t)
        .map_or(0, |p| (p + start) as i32)
}

pub(crate) fn call(name: &str, mut a: Vec<Val>) -> Result<Val, i32> {
    let argc = a.len();
    Ok(match (name, argc) {
        ("LEN", 1) => Val::L(a.pop().unwrap().s()?.len() as i32),
        ("LEFT$", 2) => {
            let n = count(&a[1])?;
            let s = a.swap_remove(0).s()?;
            Val::S(s[..n.min(s.len())].to_vec())
        }
        ("RIGHT$", 2) => {
            let n = count(&a[1])?;
            let s = a.swap_remove(0).s()?;
            Val::S(s[s.len() - n.min(s.len())..].to_vec())
        }
        ("MID$", 2) | ("MID$", 3) => {
            let start = a[1].i()?;
            if start < 1 {
                return Err(5);
            }
            let n = if argc == 3 { Some(count(&a[2])?) } else { None };
            let s = a.swap_remove(0).s()?;
            let from = (start as usize - 1).min(s.len());
            let to = n.map_or(s.len(), |n| (from + n).min(s.len()));
            Val::S(s[from..to].to_vec())
        }
        // MID$(s$, start[, n]) = r$  ->  FN MID$SET (s, start, [n,] r)
        ("MID$SET", 3) | ("MID$SET", 4) => {
            let r = a.pop().unwrap().s()?;
            let n = if argc == 4 { count(&a.pop().unwrap())? } else { r.len() };
            let start = a.pop().unwrap().i()?;
            let mut s = a.pop().unwrap().s()?;
            if start < 1 || start as usize > s.len() {
                return Err(5);
            }
            let from = start as usize - 1;
            let k = n.min(r.len()).min(s.len() - from);
            s[from..from + k].copy_from_slice(&r[..k]);
            Val::S(s)
        }
        ("UCASE$", 1) => Val::S(a.pop().unwrap().s()?.to_ascii_uppercase()),
        ("LCASE$", 1) => Val::S(a.pop().unwrap().s()?.to_ascii_lowercase()),
        ("LTRIM$", 1) => {
            let s = a.pop().unwrap().s()?;
            let k = s.iter().take_while(|&&c| c == b' ').count();
            Val::S(s[k..].to_vec())
        }
        ("RTRIM$", 1) => {
            let mut s = a.pop().unwrap().s()?;
            while s.last() == Some(&b' ') {
                s.pop();
            }
            Val::S(s)
        }
        ("_TRIM$", 1) => {
            let s = call("RTRIM$", a)?;
            return call("LTRIM$", vec![s]);
        }
        ("SPACE$", 1) => Val::S(vec![b' '; count(&a[0])?]),
        ("STRING$", 2) => {
            let n = count(&a[0])?;
            let c = match &a[1] {
                Val::S(s) => *s.first().ok_or(5)?,
                v => u8::try_from(v.i()?).map_err(|_| 5)?,
            };
            Val::S(vec![c; n])
        }
        ("INSTR", 2) | ("INSTR", 3) => {
            let t = a.pop().unwrap().s()?;
            let s = a.pop().unwrap().s()?;
            let start = if argc == 3 { a[0].i()? } else { 1 };
            if start < 1 {
                return Err(5);
            }
            Val::L(instr(start as usize, &s, &t))
        }
        ("STR$", 1) => Val::S(str_of(&a[0])?),
        ("VAL", 1) => {
            let s = a.pop().unwrap().s()?;
            let f80 = qbrt::val::val(&s)?;
            Val::D(qbrt::val::f64_from_f80(f80))
        }
        ("CHR$", 1) => Val::S(vec![u8::try_from(a[0].i()?).map_err(|_| 5)?]),
        ("ASC", 1) | ("ASC", 2) => {
            let pos = if argc == 2 { a.pop().unwrap().i()? } else { 1 };
            let s = a.pop().unwrap().s()?;
            if pos < 1 {
                return Err(5);
            }
            Val::L(i32::from(*s.get(pos as usize - 1).ok_or(5)?))
        }
        ("HEX$", 1) => radix(&a[0], true)?,
        ("OCT$", 1) => radix(&a[0], false)?,
        ("ABS", 1) => match a.pop().unwrap() {
            Val::I(v) => Val::I(v.wrapping_abs()),
            Val::L(v) => Val::L(v.wrapping_abs()),
            Val::F(v) => Val::F(v.abs()),
            Val::D(v) => Val::D(v.abs()),
            _ => return Err(13),
        },
        ("SGN", 1) => {
            let v = a[0].f()?;
            Val::I(if v > 0.0 {
                1
            } else if v < 0.0 {
                -1
            } else {
                0
            })
        }
        ("SQR", 1) => {
            if a[0].f()? < 0.0 {
                return Err(5);
            }
            float_fn(&a[0], f64::sqrt)?
        }
        ("LOG", 1) => {
            if a[0].f()? <= 0.0 {
                return Err(5);
            }
            float_fn(&a[0], f64::ln)?
        }
        ("EXP", 1) => float_fn(&a[0], f64::exp)?,
        ("SIN", 1) => float_fn(&a[0], f64::sin)?,
        ("COS", 1) => float_fn(&a[0], f64::cos)?,
        ("TAN", 1) => float_fn(&a[0], f64::tan)?,
        ("ATN", 1) => float_fn(&a[0], f64::atan)?,
        ("INT", 1) | ("FIX", 1) => {
            let v = a.pop().unwrap();
            if v.is_int() {
                v
            } else {
                let x = v.f()?;
                float_result(if name == "INT" { x.floor() } else { x.trunc() }, v.rank())
            }
        }
        ("CINT", 1) => Val::I(i16::try_from(a[0].i()?).map_err(|_| 6)?),
        ("CLNG", 1) => Val::L(i32::try_from(a[0].i()?).map_err(|_| 6)?),
        ("CSNG", 1) => Val::F(f64::from(a[0].f()? as f32)),
        ("CDBL", 1) => Val::D(a[0].f()?),
        ("_ROUND", 1) => Val::L(i32::try_from(a[0].i()?).map_err(|_| 6)?),
        ("MKI$", 1) => Val::S(i16::try_from(a[0].i()?).map_err(|_| 6)?.to_le_bytes().to_vec()),
        ("MKL$", 1) => Val::S(i32::try_from(a[0].i()?).map_err(|_| 6)?.to_le_bytes().to_vec()),
        ("MKS$", 1) => Val::S((a[0].f()? as f32).to_le_bytes().to_vec()),
        ("MKD$", 1) => Val::S(a[0].f()?.to_le_bytes().to_vec()),
        ("CVI", 1) => Val::I(i16::from_le_bytes(fixed::<2>(a.pop().unwrap())?)),
        ("CVL", 1) => Val::L(i32::from_le_bytes(fixed::<4>(a.pop().unwrap())?)),
        ("CVS", 1) => Val::F(f64::from(f32::from_le_bytes(fixed::<4>(a.pop().unwrap())?))),
        ("CVD", 1) => Val::D(f64::from_le_bytes(fixed::<8>(a.pop().unwrap())?)),
        ("MKSMBF$", 1) => {
            let b = (a[0].f()? as f32).to_le_bytes();
            Val::S(qbrt::mbf::fieee_to_msbin(b).map_err(|_| 5)?.to_vec())
        }
        ("MKDMBF$", 1) => {
            let b = a[0].f()?.to_le_bytes();
            Val::S(qbrt::mbf::dieee_to_msbin(b).map_err(|_| 5)?.to_vec())
        }
        ("CVSMBF", 1) => {
            let b = qbrt::mbf::fmsbin_to_ieee(fixed::<4>(a.pop().unwrap())?).map_err(|_| 5)?;
            Val::F(f64::from(f32::from_le_bytes(b)))
        }
        ("CVDMBF", 1) => {
            let b = qbrt::mbf::dmsbin_to_ieee(fixed::<8>(a.pop().unwrap())?).map_err(|_| 5)?;
            Val::D(f64::from_le_bytes(b))
        }
        _ => return Err(51),
    })
}
