//! Argument classification and the call thunks.
//!
//! Every ABI uses one [`Frame`]: up to 8 integer and 8 float registers, a block
//! of stack words, and the function pointer. The thunk (assembly below) copies
//! the stack words to an aligned outgoing area, loads the registers, calls,
//! and stores the integer and float return registers back into the frame.
//!
//! | ABI | integer regs | float regs | stack |
//! | --- | --- | --- | --- |
//! | Win64 | RCX RDX R8 R9 by position | XMM0-3 by position (also copied to the integer register, for `...`) | 8-byte slots after 32 bytes of shadow space |
//! | SysV x86-64 | RDI RSI RDX RCX R8 R9 | XMM0-7 (AL = count) | 8-byte slots |
//! | AAPCS64 | X0-X7 | D0-D7 (S for f32) | 8-byte slots; Apple: packed by size and every variadic argument on the stack; Windows: variadic floats in X registers |

use crate::Ty;
use std::ffi::c_void;

#[repr(C)]
pub(crate) struct Frame {
    func: usize,      // 0
    gpr: [u64; 8],    // 8
    fpr: [u64; 8],    // 72
    stack: *const u64, // 136
    nstack: usize,    // 144
    nfpr: u64,        // 152
    ret_gpr: u64,     // 160
    ret_fpr: u64,     // 168
    ret_gpr2: u64,    // 176
}

/// Register/stack layout of one call, before it runs.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Layout {
    pub gpr: [u64; 8],
    pub fpr: [u64; 8],
    pub nfpr: u64,
    pub stack: Vec<u64>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Abi {
    Win64,
    SysV,
    Aapcs,
    AapcsApple,
    AapcsWindows,
}

#[allow(dead_code)]
pub(crate) const NATIVE: Abi = if cfg!(all(target_arch = "x86_64", windows)) {
    Abi::Win64
} else if cfg!(target_arch = "x86_64") {
    Abi::SysV
} else if cfg!(all(target_arch = "aarch64", target_vendor = "apple")) {
    Abi::AapcsApple
} else if cfg!(all(target_arch = "aarch64", windows)) {
    Abi::AapcsWindows
} else {
    Abi::Aapcs
};

fn size_of(ty: Ty) -> usize {
    match ty {
        Ty::I8 | Ty::U8 => 1,
        Ty::I16 | Ty::U16 => 2,
        Ty::I32 | Ty::U32 | Ty::F32 => 4,
        _ => 8,
    }
}

/// Places the arguments (`(type, 64-bit register image)`) for `abi`.
pub(crate) fn layout(abi: Abi, args: &[(Ty, u64)], variadic_from: usize) -> Layout {
    let mut l = Layout::default();
    match abi {
        Abi::Win64 => {
            for (i, &(ty, bits)) in args.iter().enumerate() {
                if i < 4 {
                    l.gpr[i] = bits;
                    if ty.is_float() {
                        l.fpr[i] = bits;
                    }
                } else {
                    l.stack.push(bits);
                }
            }
            l.nfpr = 4;
        }
        Abi::SysV => {
            let (mut ng, mut nf) = (0, 0);
            for &(ty, bits) in args {
                if ty.is_float() {
                    if nf < 8 {
                        l.fpr[nf] = bits;
                        nf += 1;
                        continue;
                    }
                } else if ng < 6 {
                    l.gpr[ng] = bits;
                    ng += 1;
                    continue;
                }
                l.stack.push(bits);
            }
            l.nfpr = nf as u64;
        }
        Abi::Aapcs | Abi::AapcsWindows => {
            let (mut ng, mut nf) = (0, 0);
            for (i, &(ty, bits)) in args.iter().enumerate() {
                let float_in_gpr = abi == Abi::AapcsWindows && i >= variadic_from;
                if ty.is_float() && !float_in_gpr {
                    if nf < 8 {
                        l.fpr[nf] = bits;
                        nf += 1;
                        continue;
                    }
                    nf = 8;
                } else if ng < 8 {
                    let bits = if ty == Ty::F32 && float_in_gpr {
                        // Variadic floats are promoted to double by the C caller.
                        (f32::from_bits(bits as u32) as f64).to_bits()
                    } else {
                        bits
                    };
                    l.gpr[ng] = bits;
                    ng += 1;
                    continue;
                } else {
                    ng = 8;
                }
                l.stack.push(bits);
            }
            l.nfpr = nf as u64;
        }
        Abi::AapcsApple => {
            // Named arguments: registers, then the stack packed by natural size
            // and alignment. Variadic arguments: all on the stack, 8-byte slots.
            let (mut ng, mut nf) = (0, 0);
            let mut bytes: Vec<u8> = Vec::new();
            for (i, &(ty, bits)) in args.iter().enumerate() {
                if i >= variadic_from {
                    let align = (bytes.len() + 7) & !7;
                    bytes.resize(align, 0);
                    bytes.extend_from_slice(&bits.to_le_bytes());
                    continue;
                }
                if ty.is_float() && nf < 8 {
                    l.fpr[nf] = bits;
                    nf += 1;
                    continue;
                }
                if !ty.is_float() && ng < 8 {
                    l.gpr[ng] = bits;
                    ng += 1;
                    continue;
                }
                if ty.is_float() {
                    nf = 8;
                } else {
                    ng = 8;
                }
                let size = size_of(ty);
                let align = (bytes.len() + size - 1) & !(size - 1);
                bytes.resize(align, 0);
                bytes.extend_from_slice(&bits.to_le_bytes()[..size]);
            }
            bytes.resize((bytes.len() + 7) & !7, 0);
            l.stack = bytes.chunks_exact(8).map(|c| u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]])).collect();
            l.nfpr = nf as u64;
        }
    }
    l
}

/// Runs the call; returns the integer and float return registers.
///
/// # Safety
/// See [`crate::call`].
pub(crate) unsafe fn invoke(func: *const c_void, args: &[(Ty, u64)], variadic_from: usize) -> (u64, u64) {
    let l = layout(NATIVE, args, variadic_from);
    let mut f = Frame {
        func: func as usize,
        gpr: l.gpr,
        fpr: l.fpr,
        stack: l.stack.as_ptr(),
        nstack: l.stack.len(),
        nfpr: l.nfpr,
        ret_gpr: 0,
        ret_fpr: 0,
        ret_gpr2: 0,
    };
    // SAFETY: the frame is fully initialised; the thunk follows the platform
    // ABI; the callee contract is the caller's.
    unsafe { qb_ffi_thunk(&raw mut f) };
    let _ = f.ret_gpr2;
    (f.ret_gpr, f.ret_fpr)
}

unsafe extern "C" {
    fn qb_ffi_thunk(frame: *mut Frame);
}

/// C symbol of the thunk: Mach-O prefixes an underscore.
#[cfg(all(not(all(target_arch = "x86_64", windows)), target_vendor = "apple"))]
macro_rules! thunk_name {
    () => {
        "_qb_ffi_thunk"
    };
}
#[cfg(all(not(all(target_arch = "x86_64", windows)), not(target_vendor = "apple")))]
macro_rules! thunk_name {
    () => {
        "qb_ffi_thunk"
    };
}

#[cfg(all(target_arch = "x86_64", windows))]
core::arch::global_asm!(
    ".globl qb_ffi_thunk",
    ".p2align 4",
    "qb_ffi_thunk:",
    "push rbp",
    "mov rbp, rsp",
    "push rbx",
    "push rsi",
    "push rdi",
    "push r12",
    "mov rbx, rcx",
    "and rsp, -16",
    "mov rcx, [rbx + 144]",
    "lea rax, [rcx*8 + 47]",
    "and rax, -16",
    "sub rsp, rax",
    "mov rsi, [rbx + 136]",
    "lea rdi, [rsp + 32]",
    "cld",
    "rep movsq",
    "mov rcx, [rbx + 8]",
    "mov rdx, [rbx + 16]",
    "mov r8, [rbx + 24]",
    "mov r9, [rbx + 32]",
    "movq xmm0, qword ptr [rbx + 72]",
    "movq xmm1, qword ptr [rbx + 80]",
    "movq xmm2, qword ptr [rbx + 88]",
    "movq xmm3, qword ptr [rbx + 96]",
    "call qword ptr [rbx]",
    "mov [rbx + 160], rax",
    "movq qword ptr [rbx + 168], xmm0",
    "mov [rbx + 176], rdx",
    "lea rsp, [rbp - 32]",
    "pop r12",
    "pop rdi",
    "pop rsi",
    "pop rbx",
    "pop rbp",
    "ret",
);

#[cfg(all(target_arch = "x86_64", not(windows)))]
core::arch::global_asm!(
    concat!(".globl ", thunk_name!()),
    ".p2align 4",
    concat!(thunk_name!(), ":"),
    "push rbp",
    "mov rbp, rsp",
    "push rbx",
    "push r12",
    "mov rbx, rdi",
    "and rsp, -16",
    "mov rcx, [rbx + 144]",
    "lea rax, [rcx*8 + 15]",
    "and rax, -16",
    "sub rsp, rax",
    "mov rsi, [rbx + 136]",
    "mov rdi, rsp",
    "cld",
    "rep movsq",
    "mov rdi, [rbx + 8]",
    "mov rsi, [rbx + 16]",
    "mov rdx, [rbx + 24]",
    "mov rcx, [rbx + 32]",
    "mov r8, [rbx + 40]",
    "mov r9, [rbx + 48]",
    "movq xmm0, qword ptr [rbx + 72]",
    "movq xmm1, qword ptr [rbx + 80]",
    "movq xmm2, qword ptr [rbx + 88]",
    "movq xmm3, qword ptr [rbx + 96]",
    "movq xmm4, qword ptr [rbx + 104]",
    "movq xmm5, qword ptr [rbx + 112]",
    "movq xmm6, qword ptr [rbx + 120]",
    "movq xmm7, qword ptr [rbx + 128]",
    "mov rax, [rbx + 152]",
    "call qword ptr [rbx]",
    "mov [rbx + 160], rax",
    "movq qword ptr [rbx + 168], xmm0",
    "mov [rbx + 176], rdx",
    "lea rsp, [rbp - 16]",
    "pop r12",
    "pop rbx",
    "pop rbp",
    "ret",
);

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    concat!(".globl ", thunk_name!()),
    ".p2align 4",
    concat!(thunk_name!(), ":"),
    "stp x29, x30, [sp, #-32]!",
    "mov x29, sp",
    "str x19, [sp, #16]",
    "mov x19, x0",
    "ldr x9, [x19, #144]",
    "lsl x10, x9, #3",
    "add x10, x10, #15",
    "and x10, x10, #-16",
    "sub sp, sp, x10",
    "ldr x11, [x19, #136]",
    "mov x12, sp",
    "cbz x9, 2f",
    "1:",
    "ldr x13, [x11], #8",
    "str x13, [x12], #8",
    "subs x9, x9, #1",
    "b.ne 1b",
    "2:",
    "ldp x0, x1, [x19, #8]",
    "ldp x2, x3, [x19, #24]",
    "ldp x4, x5, [x19, #40]",
    "ldp x6, x7, [x19, #56]",
    "ldp d0, d1, [x19, #72]",
    "ldp d2, d3, [x19, #88]",
    "ldp d4, d5, [x19, #104]",
    "ldp d6, d7, [x19, #120]",
    "ldr x16, [x19]",
    "blr x16",
    "str x0, [x19, #160]",
    "str d0, [x19, #168]",
    "str x1, [x19, #176]",
    "mov sp, x29",
    "ldr x19, [sp, #16]",
    "ldp x29, x30, [sp], #32",
    "ret",
);

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: f64) -> (Ty, u64) {
        (Ty::F64, v.to_bits())
    }
    fn i(v: u64) -> (Ty, u64) {
        (Ty::I64, v)
    }

    #[test]
    fn win64_assigns_by_position() {
        let l = layout(Abi::Win64, &[i(1), f(2.0), i(3), f(4.0), i(5), f(6.0)], usize::MAX);
        assert_eq!(l.gpr[..4], [1, 2f64.to_bits(), 3, 4f64.to_bits()]);
        assert_eq!(l.fpr[1], 2f64.to_bits());
        assert_eq!(l.fpr[0], 0);
        assert_eq!(l.stack, vec![5, 6f64.to_bits()]);
    }

    #[test]
    fn sysv_fills_classes_independently() {
        let args: Vec<_> = (0..8).map(i).chain((0..10).map(|k| f(k as f64))).collect();
        let l = layout(Abi::SysV, &args, usize::MAX);
        assert_eq!(l.gpr[..6], [0, 1, 2, 3, 4, 5]);
        assert_eq!(l.nfpr, 8);
        assert_eq!(l.stack, vec![6, 7, 8f64.to_bits(), 9f64.to_bits()]);
    }

    #[test]
    fn aapcs_variants() {
        let args: Vec<_> = (0..9).map(i).collect();
        assert_eq!(layout(Abi::Aapcs, &args, usize::MAX).stack, vec![8]);
        // Apple packs named stack arguments by size.
        let mut named: Vec<_> = (0..8).map(i).collect();
        named.push((Ty::I32, 0x1111_1111));
        named.push((Ty::I32, 0x2222_2222));
        assert_eq!(layout(Abi::AapcsApple, &named, usize::MAX).stack, vec![0x2222_2222_1111_1111]);
        // Apple variadics go to the stack even when registers are free.
        let v = layout(Abi::AapcsApple, &[i(1), f(2.0)], 1);
        assert_eq!((v.gpr[0], v.nfpr, v.stack.clone()), (1, 0, vec![2f64.to_bits()]));
        // Windows on ARM: variadic floats in X registers.
        let w = layout(Abi::AapcsWindows, &[i(1), f(2.0)], 1);
        assert_eq!((w.gpr[1], w.nfpr), (2f64.to_bits(), 0));
    }
}
