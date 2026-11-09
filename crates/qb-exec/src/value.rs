//! Tagged values for the generic builtin path (`builtins::call`). The VM keeps
//! typed registers; values are boxed into `Val` only when a builtin without a
//! dedicated instruction is called.

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Val {
    I(i16),
    L(i32),
    Q(i64),
    /// SINGLE, carried at double precision
    F(f64),
    D(f64),
    S(Vec<u8>),
}

impl Val {
    pub(crate) fn rank(&self) -> u8 {
        match self {
            Val::I(_) => 0,
            Val::L(_) | Val::Q(_) => 1,
            Val::F(_) => 2,
            Val::D(_) => 3,
            Val::S(_) => 9,
        }
    }
    pub(crate) fn is_int(&self) -> bool {
        matches!(self, Val::I(_) | Val::L(_) | Val::Q(_))
    }
    pub(crate) fn f(&self) -> Result<f64, i32> {
        match self {
            Val::I(v) => Ok(f64::from(*v)),
            Val::L(v) => Ok(f64::from(*v)),
            Val::Q(v) => Ok(*v as f64),
            Val::F(v) | Val::D(v) => Ok(*v),
            Val::S(_) => Err(13),
        }
    }
    /// Integer value; floats round half-even like QBasic (`qb-runtime` rounding).
    pub(crate) fn i(&self) -> Result<i64, i32> {
        match self {
            Val::I(v) => Ok(i64::from(*v)),
            Val::L(v) => Ok(i64::from(*v)),
            Val::Q(v) => Ok(*v),
            Val::F(v) | Val::D(v) => Ok(qb_runtime::round::qb_qbr_f64(*v)),
            Val::S(_) => Err(13),
        }
    }
    pub(crate) fn s(self) -> Result<Vec<u8>, i32> {
        match self {
            Val::S(s) => Ok(s),
            _ => Err(13),
        }
    }
}

pub(crate) fn float_result(v: f64, rank: u8) -> Val {
    if rank >= 3 {
        Val::D(v)
    } else {
        Val::F(v)
    }
}

pub(crate) fn str_of(v: &Val) -> Result<Vec<u8>, i32> {
    Ok(match v {
        Val::I(x) => qb_runtime::numfmt::str_i64(i64::from(*x)),
        Val::L(x) => qb_runtime::numfmt::str_i64(i64::from(*x)),
        Val::Q(x) => qb_runtime::numfmt::str_i64(*x),
        Val::F(x) => qb_runtime::numfmt::str_f32(*x as f32),
        Val::D(x) => qb_runtime::numfmt::str_f64(*x),
        Val::S(_) => return Err(13),
    })
}
