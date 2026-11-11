//! Default host functions: strings, console output and arrays over
//! `qb-runtime`, for embedders (and tests) that do not bring their own.
//!
//! The VM registers its own table (its string representation); these
//! follow the same calling contract: `extern "C" fn(*mut Ctx, args...) -> ret`,
//! errors reported by setting `Ctx::err`.
//!
//! Strings are handles: 0 = empty string, otherwise `Box<Vec<u8>>` raw pointer.
//! Every function returning a string returns a new handle the caller owns.

use crate::ctx::{qberr, ArrDesc, Ctx};
use crate::mir::{HostFn, HostSig, Ty};

/// Host state reached through `Ctx::user`.
#[derive(Debug, Default)]
pub struct Rt {
    pub out: Vec<u8>,
}

fn rt(ctx: *mut Ctx) -> &'static mut Rt {
    // SAFETY: the embedder sets `ctx.user` to a live `Rt` before running code that calls these.
    unsafe { &mut *((*ctx).user as *mut Rt) }
}

fn fail(ctx: *mut Ctx, code: i32) {
    // SAFETY: valid context from native code.
    unsafe { (*ctx).err = code };
}

pub fn bytes<'a>(h: u64) -> &'a [u8] {
    if h == 0 {
        &[]
    } else {
        // SAFETY: non-zero handles are live `Box<Vec<u8>>` pointers.
        unsafe { &*(h as *const Vec<u8>) }
    }
}

pub fn new_str(v: Vec<u8>) -> u64 {
    if v.is_empty() {
        0
    } else {
        Box::into_raw(Box::new(v)) as u64
    }
}

pub fn free_str(h: u64) {
    if h != 0 {
        // SAFETY: handle created by `new_str`.
        drop(unsafe { Box::from_raw(h as *mut Vec<u8>) });
    }
}

extern "C" fn str_lit(_c: *mut Ctx, p: u64, len: i64) -> u64 {
    // SAFETY: the code generator passes a pointer to `len` static bytes.
    new_str(unsafe { core::slice::from_raw_parts(p as *const u8, len as usize) }.to_vec())
}
extern "C" fn str_free(_c: *mut Ctx, s: u64) {
    free_str(s)
}
extern "C" fn str_copy(_c: *mut Ctx, s: u64) -> u64 {
    new_str(bytes(s).to_vec())
}
extern "C" fn str_concat(_c: *mut Ctx, a: u64, b: u64) -> u64 {
    new_str([bytes(a), bytes(b)].concat())
}
extern "C" fn str_cmp(_c: *mut Ctx, a: u64, b: u64) -> i32 {
    bytes(a).cmp(bytes(b)) as i32
}
extern "C" fn str_len(_c: *mut Ctx, s: u64) -> i32 {
    bytes(s).len() as i32
}
extern "C" fn str_left(c: *mut Ctx, s: u64, n: i32) -> u64 {
    if n < 0 {
        fail(c, qberr::ILLEGAL_CALL);
        return 0;
    }
    let b = bytes(s);
    new_str(b[..(n as usize).min(b.len())].to_vec())
}
/// `MID$(s, start, len)`; `len < 0` = rest of the string.
extern "C" fn str_mid(c: *mut Ctx, s: u64, start: i32, len: i32) -> u64 {
    if start < 1 {
        fail(c, qberr::ILLEGAL_CALL);
        return 0;
    }
    let b = bytes(s);
    let from = (start as usize - 1).min(b.len());
    let to = if len < 0 { b.len() } else { (from + len as usize).min(b.len()) };
    new_str(b[from..to].to_vec())
}
extern "C" fn str_chr(c: *mut Ctx, n: i32) -> u64 {
    if !(0..=255).contains(&n) {
        fail(c, qberr::ILLEGAL_CALL);
        return 0;
    }
    new_str(vec![n as u8])
}
extern "C" fn str_asc(c: *mut Ctx, s: u64) -> i32 {
    match bytes(s).first() {
        Some(b) => i32::from(*b),
        None => {
            fail(c, qberr::ILLEGAL_CALL);
            0
        }
    }
}
extern "C" fn str_i64(_c: *mut Ctx, v: i64) -> u64 {
    new_str(qb_runtime::numfmt::str_i64(v))
}
extern "C" fn str_f64(_c: *mut Ctx, v: f64) -> u64 {
    new_str(qb_runtime::numfmt::str_f64(v))
}
extern "C" fn str_f32(_c: *mut Ctx, v: f32) -> u64 {
    new_str(qb_runtime::numfmt::str_f32(v))
}
extern "C" fn print_str(c: *mut Ctx, s: u64) {
    rt(c).out.extend_from_slice(bytes(s));
}
extern "C" fn print_nl(c: *mut Ctx) {
    rt(c).out.extend_from_slice(b"\n");
}
extern "C" fn arr_new1(c: *mut Ctx, elem: i32, lb: i64, ub: i64) -> u64 {
    if ub < lb - 1 {
        fail(c, qberr::SUBSCRIPT);
        return 0;
    }
    Box::into_raw(ArrDesc::new(elem as u32, &[(lb, ub)])) as u64
}
extern "C" fn arr_new2(c: *mut Ctx, elem: i32, lb1: i64, ub1: i64, lb2: i64, ub2: i64) -> u64 {
    if ub1 < lb1 - 1 || ub2 < lb2 - 1 {
        fail(c, qberr::SUBSCRIPT);
        return 0;
    }
    Box::into_raw(ArrDesc::new(elem as u32, &[(lb1, ub1), (lb2, ub2)])) as u64
}
extern "C" fn arr_free(_c: *mut Ctx, a: u64) {
    if a != 0 {
        // SAFETY: created by `arr_new*`.
        drop(unsafe { Box::from_raw(a as *mut ArrDesc) });
    }
}

/// Host ids of [`host_table`].
pub mod id {
    pub const STR_LIT: u32 = 0;
    pub const STR_FREE: u32 = 1;
    pub const STR_COPY: u32 = 2;
    pub const STR_CONCAT: u32 = 3;
    pub const STR_CMP: u32 = 4;
    pub const STR_LEN: u32 = 5;
    pub const STR_LEFT: u32 = 6;
    pub const STR_MID: u32 = 7;
    pub const STR_CHR: u32 = 8;
    pub const STR_ASC: u32 = 9;
    pub const STR_I64: u32 = 10;
    pub const STR_F64: u32 = 11;
    pub const STR_F32: u32 = 12;
    pub const PRINT_STR: u32 = 13;
    pub const PRINT_NL: u32 = 14;
    pub const ARR_NEW1: u32 = 15;
    pub const ARR_NEW2: u32 = 16;
    pub const ARR_FREE: u32 = 17;
}

fn h(name: &str, params: &[Ty], ret: Option<Ty>, may_error: bool, addr: usize) -> HostFn {
    HostFn { sig: HostSig { name: name.into(), params: params.to_vec(), ret, may_error }, addr }
}

/// The default host table, indexed by [`id`].
pub fn host_table() -> Vec<HostFn> {
    use Ty::*;
    vec![
        h("str_lit", &[Ptr, I64], Some(Ptr), false, str_lit as *const () as usize),
        h("str_free", &[Ptr], None, false, str_free as *const () as usize),
        h("str_copy", &[Ptr], Some(Ptr), false, str_copy as *const () as usize),
        h("str_concat", &[Ptr, Ptr], Some(Ptr), false, str_concat as *const () as usize),
        h("str_cmp", &[Ptr, Ptr], Some(I32), false, str_cmp as *const () as usize),
        h("str_len", &[Ptr], Some(I32), false, str_len as *const () as usize),
        h("str_left", &[Ptr, I32], Some(Ptr), true, str_left as *const () as usize),
        h("str_mid", &[Ptr, I32, I32], Some(Ptr), true, str_mid as *const () as usize),
        h("str_chr", &[I32], Some(Ptr), true, str_chr as *const () as usize),
        h("str_asc", &[Ptr], Some(I32), true, str_asc as *const () as usize),
        h("str_i64", &[I64], Some(Ptr), false, str_i64 as *const () as usize),
        h("str_f64", &[F64], Some(Ptr), false, str_f64 as *const () as usize),
        h("str_f32", &[F32], Some(Ptr), false, str_f32 as *const () as usize),
        h("print_str", &[Ptr], None, false, print_str as *const () as usize),
        h("print_nl", &[], None, false, print_nl as *const () as usize),
        h("arr_new1", &[I32, I64, I64], Some(Ptr), true, arr_new1 as *const () as usize),
        h("arr_new2", &[I32, I64, I64, I64, I64], Some(Ptr), true, arr_new2 as *const () as usize),
        h("arr_free", &[Ptr], None, false, arr_free as *const () as usize),
    ]
}
