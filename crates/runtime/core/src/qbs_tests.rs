//! Unit tests for the string engine (`qbs.rs`). Expected values follow the
//! legacy libqb.cpp engine (bug-for-bug) and QBasic semantics.
//!
//! The engine is one global, single-threaded allocator, so every test takes
//! `LOCK` and starts from `qb_qbs_reset()`.

use super::*;
use std::sync::Mutex;

pub(crate) static LOCK: Mutex<()> = Mutex::new(());

fn fresh() -> std::sync::MutexGuard<'static, ()> {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { qb_qbs_reset() };
    host::take_error();
    g
}

fn txt(s: &'static str) -> *mut Qbs {
    unsafe { qb_qbs_new_txt_len(s.as_ptr().cast(), s.len() as i32) }
}

/// A non-tmp variable-length string holding `s` (like a BASIC variable).
fn var(s: &str) -> *mut Qbs {
    unsafe {
        let v = qb_qbs_new(0, 0);
        let t = qb_qbs_new(s.len() as i32, 1);
        std::ptr::copy_nonoverlapping(s.as_ptr(), (*t).chr, s.len());
        qb_qbs_set(v, t)
    }
}

fn text(q: *mut Qbs) -> String {
    unsafe {
        let q = &*q;
        if q.len <= 0 {
            return String::new();
        }
        String::from_utf8_lossy(std::slice::from_raw_parts(q.chr, q.len as usize)).into_owned()
    }
}

fn nexti() -> u32 {
    unsafe { *std::ptr::addr_of!(qbs_tmp_list_nexti) }
}

#[test]
fn layout_matches_common_h() {
    // struct qbs in internal/c/common.h, read field by field by generated C++.
    use std::mem::{offset_of, size_of};
    let p = size_of::<usize>();
    assert_eq!(offset_of!(Qbs, chr), 0);
    assert_eq!(offset_of!(Qbs, len), p);
    assert_eq!(offset_of!(Qbs, in_cmem), p + 4);
    assert_eq!(offset_of!(Qbs, cmem_descriptor), 2 * p);
    assert_eq!(offset_of!(Qbs, cmem_descriptor_offset), 3 * p);
    assert_eq!(offset_of!(Qbs, listi), 3 * p + 4);
    assert_eq!(offset_of!(Qbs, tmp), 3 * p + 8);
    assert_eq!(offset_of!(Qbs, tmplisti), 3 * p + 12);
    assert_eq!(offset_of!(Qbs, fixed), 3 * p + 16);
    assert_eq!(offset_of!(Qbs, readonly), 3 * p + 17);
    if p == 8 {
        assert_eq!(offset_of!(Qbs, field), 48);
        assert_eq!(size_of::<Qbs>(), 56);
    }
}

#[test]
fn new_tmp_string_is_tracked_and_reserves_32_spare_bytes() {
    let _g = fresh();
    unsafe {
        let a = qb_qbs_new(5, 1);
        assert!(!a.is_null());
        assert_eq!((*a).len, 5);
        assert_eq!((*a).tmp, 1);
        assert_eq!((*a).tmplisti, 1, "tmp list index 0 is a sentinel");
        assert_eq!((*a).listi, 0);
        assert_eq!(nexti(), 2);
        let b = qb_qbs_new(1, 0);
        assert_eq!((*b).chr.offset_from((*a).chr), 37);
        assert_eq!((*b).tmp, 0);
        assert_eq!((*b).tmplisti, 0);
        qb_qbs_free(a);
        assert_eq!(nexti(), 1);
    }
}

#[test]
fn set_acquires_tmp_source_and_frees_it() {
    let _g = fresh();
    unsafe {
        let v = qb_qbs_new(0, 0);
        let t = qb_qbs_new(3, 1);
        (*t).chr.copy_from_nonoverlapping(b"abc".as_ptr(), 3);
        let chr = (*t).chr;
        assert_eq!(qb_qbs_set(v, t), v);
        assert_eq!(text(v), "abc");
        assert_eq!((*v).chr, chr, "deststr takes over the tmp string's data");
        assert_eq!(nexti(), 1);
        qb_qbs_set(v, txt("hello world"));
        assert_eq!(text(v), "hello world");
        qb_qbs_set(v, txt("hi"));
        assert_eq!(text(v), "hi");
    }
}

#[test]
fn fixed_length_set_pads_and_truncates() {
    let _g = fresh();
    unsafe {
        let mut buf = *b"xxxxx";
        let f = qb_qbs_new_fixed(buf.as_mut_ptr(), 5, 0);
        qb_qbs_set(f, txt("ab"));
        assert_eq!(&buf, b"ab   ");
        qb_qbs_set(f, txt("abcdefg"));
        assert_eq!(&buf, b"abcde");
        assert_eq!((*f).fixed, 1);
        assert!((*f).cmem_descriptor.is_null(), "not inside DBLOCK");
    }
}

#[test]
fn add_concatenates_and_passes_empty_operands_on() {
    let _g = fresh();
    unsafe {
        let a = txt("ab");
        assert_eq!(qb_qbs_add(a, txt("")), a);
        let r = qb_qbs_add(txt("ab"), txt("cd"));
        assert_eq!(text(r), "abcd");
        assert_eq!((*r).tmp, 1);
        let v = var("x");
        let r2 = qb_qbs_add(v, txt("y"));
        assert_eq!(text(r2), "xy");
        assert_eq!(text(v), "x", "a variable operand is not freed");
    }
}

#[test]
fn concat_in_a_loop_survives_arena_growth() {
    let _g = fresh();
    unsafe {
        let v = qb_qbs_new(0, 0);
        let keep = var("keep me");
        let mut expect = String::new();
        let (p1, p2) = ("ab".repeat(75).leak() as &'static str, "cde".repeat(50).leak() as &'static str);
        for i in 0..3_000 {
            let piece = if i % 2 == 0 { p1 } else { p2 };
            expect.push_str(piece);
            let base = nexti();
            qb_qbs_set(v, qb_qbs_add(v, txt(piece)));
            qb_qbs_cleanup(base);
        }
        assert_eq!(text(v), expect);
        assert_eq!(text(keep), "keep me");
        assert!(qb_qbs_debug_data_size() > 1_048_576, "the arena grew");
    }
}

#[test]
fn free_compacts_tracking_and_rewinds_stack_pointer() {
    let _g = fresh();
    unsafe {
        let a = qb_qbs_new(10, 0);
        let b = qb_qbs_new(10, 0);
        assert_eq!(qb_qbs_debug_sp(), 84);
        qb_qbs_free(b);
        assert_eq!(qb_qbs_debug_sp(), 42);
        qb_qbs_free(a);
        assert_eq!(qb_qbs_debug_sp(), 0);
    }
}

#[test]
fn cleanup_frees_tmps_created_after_base() {
    let _g = fresh();
    unsafe {
        let keep = qb_qbs_new(1, 1);
        let base = nexti();
        qb_qbs_new(1, 1);
        qb_qbs_new(1, 1);
        assert_eq!(nexti(), base + 2);
        qb_qbs_cleanup(base);
        assert_eq!(nexti(), base);
        assert_eq!((*keep).len, 1);
    }
}

#[test]
fn left_right_mid() {
    let _g = fresh();
    unsafe {
        let v = var("hello");
        assert_eq!(text(qb_qbs_left(v, 2)), "he");
        assert_eq!(text(qb_qbs_left(v, 99)), "hello");
        assert_eq!(qb_qbs_left(v, 5), v, "whole string is passed on");
        assert_eq!(text(qb_qbs_left(v, -3)), "");
        assert_eq!(text(qb_qbs_right(v, 3)), "llo");
        assert_eq!(text(qb_func_mid(v, 2, 3, 1)), "ell");
        assert_eq!(text(qb_func_mid(v, 0, 3, 1)), "he");
        assert_eq!(text(qb_func_mid(v, 4, 0, 0)), "lo");
        assert_eq!(text(qb_func_mid(v, 9, 2, 1)), "");
        // tmp strings are cut in place
        let t = qb_qbs_new(4, 1);
        (*t).chr.copy_from_nonoverlapping(b"wxyz".as_ptr(), 4);
        let r = qb_qbs_right(t, 2);
        assert_eq!(r, t);
        assert_eq!(text(r), "yz");
    }
}

#[test]
fn trims_and_case() {
    let _g = fresh();
    unsafe {
        assert_eq!(text(qb_qbs_ltrim(var("  a b  "))), "a b  ");
        assert_eq!(text(qb_qbs_rtrim(var("  a b  "))), "  a b");
        assert_eq!(text(qb_qbs__trim(var("  a b  "))), "a b");
        assert_eq!(text(qb_qbs_ltrim(var("    "))), "");
        assert_eq!(text(qb_qbs_ucase(var("aZ9{z"))), "AZ9{Z");
        assert_eq!(text(qb_qbs_lcase(var("Az9[Z"))), "az9[z");
        let ro = txt("abc");
        let up = qb_qbs_ucase(ro);
        assert_ne!(up, ro, "read-only literals are copied");
        assert_eq!(text(up), "ABC");
    }
}

#[test]
fn chr_space_string_varptr() {
    let _g = fresh();
    unsafe {
        assert_eq!(text(qb_func_chr(65)), "A");
        assert_eq!(host::take_error(), 0);
        let bad = qb_func_chr(256);
        assert_eq!((*bad).len, 0);
        assert_eq!(host::take_error(), 5);
        assert_eq!(text(qb_func_space(3)), "   ");
        assert_eq!(text(qb_func_space(-1)), "");
        assert_eq!(text(qb_func_string(3, 65 + 256)), "AAA");
        let p = qb_func_varptr_helper(7, 0x1234);
        assert_eq!(std::slice::from_raw_parts((*p).chr, 3), &[7, 0x34, 0x12]);
    }
}

#[test]
fn instr_and_instrrev() {
    let _g = fresh();
    unsafe {
        let h = var("hello");
        let l = var("l");
        let e = var("");
        assert_eq!(qb_func_instr(0, h, l, 0), 3);
        assert_eq!(qb_func_instr(4, h, l, 1), 4);
        assert_eq!(qb_func_instr(5, h, l, 1), 0);
        assert_eq!(qb_func_instr(-7, h, l, 1), 3, "QBNex: start below 1 means 1");
        assert_eq!(qb_func_instr(0, h, e, 1), 0);
        assert_eq!(qb_func_instr(2, h, e, 1), 2);
        assert_eq!(qb_func_instr(6, h, e, 1), 0);
        assert_eq!(qb_func_instr(1, e, e, 1), 0);
        assert_eq!(qb_func_instr(1, h, var("lo"), 1), 4);
        assert_eq!(qb_func_instr(1, h, var("lx"), 1), 0);
        assert_eq!(qb_func__instrrev(0, h, l, 0), 4);
        assert_eq!(qb_func__instrrev(3, h, l, 1), 3);
        assert_eq!(qb_func__instrrev(2, h, l, 1), 0);
        assert_eq!(qb_func__instrrev(0, h, e, 0), 5);
        assert_eq!(qb_func__instrrev(0, h, h, 0), 1);
    }
}

#[test]
fn mid_statement() {
    let _g = fresh();
    unsafe {
        let d = var("abcdef");
        qb_sub_mid(d, 2, 3, var("XYZW"), 1);
        assert_eq!(text(d), "aXYZef");
        qb_sub_mid(d, 5, 0, var("123"), 0);
        assert_eq!(text(d), "aXYZ12");
        qb_sub_mid(d, 0, 3, var("pq"), 1);
        assert_eq!(text(d), "qXYZ12", "start 0 skips one source byte");
        qb_sub_mid(d, 2, 4, d, 1);
        assert_eq!(text(d), "qqXYZ2", "overlapping self copy");
    }
}

#[test]
fn comparisons() {
    let _g = fresh();
    unsafe {
        let (a, b, ab, e) = (var("a"), var("b"), var("ab"), var(""));
        assert_eq!(qb_qbs_equal(a, a), -1);
        assert_eq!(qb_qbs_equal(a, ab), 0);
        assert_eq!(qb_qbs_notequal(a, b), -1);
        assert_eq!(qb_qbs_lessthan(a, b), -1);
        assert_eq!(qb_qbs_lessthan(a, ab), -1);
        assert_eq!(qb_qbs_lessthan(ab, a), 0);
        assert_eq!(qb_qbs_lessthan(e, a), -1);
        assert_eq!(qb_qbs_lessthan(e, e), 0);
        assert_eq!(qb_qbs_greaterthan(b, a), -1);
        assert_eq!(qb_qbs_greaterthan(a, a), 0);
        assert_eq!(qb_qbs_lessorequal(a, a), -1);
        assert_eq!(qb_qbs_lessorequal(e, a), -1);
        assert_eq!(qb_qbs_greaterorequal(ab, a), -1);
        assert_eq!(qb_qbs_greaterorequal(a, ab), 0);
        assert_eq!(qb_func__str_compare(a, ab), -1);
        assert_eq!(qb_func__str_compare(b, a), 1);
        assert_eq!(qb_func__str_nc_compare(var("ABC"), var("abc")), 0);
        assert_eq!(qb_func__str_nc_compare(var("ABD"), var("abc")), 1);
        assert_eq!(qb_qbs_asc(var("AZ"), 2), 90);
        assert_eq!(qb_qbs_asc(var("AZ"), 3), 0);
        assert_eq!(host::take_error(), 5);
        assert_eq!(qb_qbs_asc1(e), 0);
        assert_eq!(host::take_error(), 5);
        assert_eq!(qb_qbs_len(ab), 2);
    }
}

#[test]
fn cmem_strings_keep_their_dblock_descriptor_current() {
    let _g = fresh();
    unsafe {
        let start_sp = host::cmem_sp_value();
        let c = qb_qbs_new_cmem(0, 0);
        assert_eq!((*c).in_cmem, 1);
        assert_eq!(host::cmem_sp_value(), start_sp - 4);
        assert_eq!((*c).cmem_descriptor_offset as u32, start_sp - 4);
        qb_qbs_set(c, txt("dblock!"));
        assert_eq!(text(c), "dblock!");
        let d = (*c).cmem_descriptor;
        assert_eq!(d.read_unaligned(), 7);
        let off = d.add(1).read_unaligned() as isize;
        assert_eq!((*c).chr as isize - host::dblock_value(), off);
        // a tmp cmem string assigned to a cmem variable is acquired
        let t = qb_qbs_new_cmem(2, 1);
        (*t).chr.copy_from_nonoverlapping(b"ok".as_ptr(), 2);
        qb_qbs_set(c, t);
        assert_eq!(text(c), "ok");
        assert_eq!(d.read_unaligned(), 2);
    }
}

#[test]
fn set_qbs_size_resizes_a_variable() {
    let _g = fresh();
    unsafe {
        let v = var("abc");
        let mut slot = v as isize;
        qb_set_qbs_size(&mut slot, 6);
        assert_eq!(text(v), "      ");
    }
}

/// R14: `start + length` near `i32::MAX` must not wrap (it read and wrote past
/// the string: `MID$(a$, 2, 2147483647)`, `MID$(a$, 2, 2147483647) = b$`).
#[test]
fn mid_with_huge_lengths_stays_inside_the_string() {
    let _g = fresh();
    unsafe {
        assert_eq!(text(qb_func_mid(var("HELLO"), 2, i32::MAX, 1)), "ELLO");
        assert_eq!(text(qb_func_mid(var("HELLO"), i32::MIN, i32::MAX, 1)), "");
        assert_eq!(text(qb_func_mid(var("HELLO"), -3, 6, 1)), "HE");
        assert_eq!(text(qb_func_mid(var("HELLO"), i32::MAX, 5, 1)), "");
        let d = var("abcde");
        qb_sub_mid(d, 2, i32::MAX, var("0123456789"), 1);
        assert_eq!(text(d), "a0123");
        let d = var("abcde");
        qb_sub_mid(d, i32::MIN, i32::MAX, var("0123456789"), 1);
        assert_eq!(text(d), "abcde");
        let d = var("abcde");
        qb_sub_mid(d, -1, 4, var("XYZW"), 1);
        assert_eq!(text(d), "ZWcde"); // positions -1 and 0 are dropped
        let d = var("abcde");
        qb_sub_mid(d, 4, 1, var("Q"), 0);
        assert_eq!(text(d), "abcQe");
    }
}
