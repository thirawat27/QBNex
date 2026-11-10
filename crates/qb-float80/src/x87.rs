//! The three x87 transcendental primitives MinGW's `expl`/`exp2l`/`log2l` use.
//!
//! Their hardware results are not always the correctly rounded value (about 3% of
//! random inputs are one unit in the last place off on the reference machine, and
//! the pattern differs between CPU generations), so no software algorithm can
//! reproduce them in general. On x86/x86_64 they therefore run on the x87 unit,
//! with the control word forced to the legacy `0x037F` around the instruction;
//! everything else around them is software. Elsewhere (or with the feature
//! `soft-transcendentals`) the software versions in [`crate::soft`] are used
//! (deviation D-111: at most one unit in the last place per primitive).

use crate::F80;

/// `f2xm1`: `2^x - 1` for `-1 <= x <= 1`.
pub fn f2xm1(x: F80) -> F80 {
    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), not(feature = "soft-transcendentals")))]
    {
        hw::f2xm1(x)
    }
    #[cfg(not(all(any(target_arch = "x86", target_arch = "x86_64"), not(feature = "soft-transcendentals"))))]
    {
        crate::soft::f2xm1(x)
    }
}

/// `fyl2x`: `y * log2(x)` (x in ST0, y in ST1).
pub fn fyl2x(x: F80, y: F80) -> F80 {
    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), not(feature = "soft-transcendentals")))]
    {
        hw::fyl2x(x, y)
    }
    #[cfg(not(all(any(target_arch = "x86", target_arch = "x86_64"), not(feature = "soft-transcendentals"))))]
    {
        crate::soft::fyl2x(x, y)
    }
}

/// `fyl2xp1`: `y * log2(1 + x)` for `|x| < 1 - sqrt(2)/2`.
pub fn fyl2xp1(x: F80, y: F80) -> F80 {
    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), not(feature = "soft-transcendentals")))]
    {
        hw::fyl2xp1(x, y)
    }
    #[cfg(not(all(any(target_arch = "x86", target_arch = "x86_64"), not(feature = "soft-transcendentals"))))]
    {
        crate::soft::fyl2xp1(x, y)
    }
}

/// True when [`f2xm1`], [`fyl2x`] and [`fyl2xp1`] run on the x87 unit.
pub const HARDWARE: bool =
    cfg!(all(any(target_arch = "x86", target_arch = "x86_64"), not(feature = "soft-transcendentals")));

/// Direct access to the x87 unit (x86/x86_64 only), used by the primitives above
/// and by tests that compare software results with the hardware.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod hw {
    use crate::F80;
    use core::arch::asm;

    /// Control word of the legacy runtime: 64-bit precision, nearest, all masked.
    const CW: u16 = 0x037F;

    pub fn f2xm1(x: F80) -> F80 {
        let mut a = x.to_bytes();
        let mut old: u16 = 0;
        // SAFETY: loads/stores 10 bytes at `a`, saves/restores the control word;
        // every x87 register is declared clobbered.
        unsafe {
            asm!(
                "fnstcw word ptr [{old}]",
                "fldcw word ptr [{cw}]",
                "fld tbyte ptr [{a}]",
                "f2xm1",
                "fstp tbyte ptr [{a}]",
                "fnclex",
                "fldcw word ptr [{old}]",
                a = in(reg) a.as_mut_ptr(),
                old = in(reg) &mut old,
                cw = in(reg) &CW,
                out("st(0)") _, out("st(1)") _, out("st(2)") _, out("st(3)") _,
                out("st(4)") _, out("st(5)") _, out("st(6)") _, out("st(7)") _,
                options(nostack),
            );
        }
        F80::from_bytes(a)
    }

    pub fn fyl2x(x: F80, y: F80) -> F80 {
        two(x, y, false)
    }

    pub fn fyl2xp1(x: F80, y: F80) -> F80 {
        two(x, y, true)
    }

    fn two(x: F80, y: F80, p1: bool) -> F80 {
        let mut a = x.to_bytes();
        let b = y.to_bytes();
        let mut old: u16 = 0;
        // SAFETY: as in `f2xm1`.
        unsafe {
            if p1 {
                asm!(
                    "fnstcw word ptr [{old}]",
                    "fldcw word ptr [{cw}]",
                    "fld tbyte ptr [{b}]",
                    "fld tbyte ptr [{a}]",
                    "fyl2xp1",
                    "fstp tbyte ptr [{a}]",
                    "fnclex",
                    "fldcw word ptr [{old}]",
                    a = in(reg) a.as_mut_ptr(),
                    b = in(reg) b.as_ptr(),
                    old = in(reg) &mut old,
                    cw = in(reg) &CW,
                    out("st(0)") _, out("st(1)") _, out("st(2)") _, out("st(3)") _,
                    out("st(4)") _, out("st(5)") _, out("st(6)") _, out("st(7)") _,
                    options(nostack),
                );
            } else {
                asm!(
                    "fnstcw word ptr [{old}]",
                    "fldcw word ptr [{cw}]",
                    "fld tbyte ptr [{b}]",
                    "fld tbyte ptr [{a}]",
                    "fyl2x",
                    "fstp tbyte ptr [{a}]",
                    "fnclex",
                    "fldcw word ptr [{old}]",
                    a = in(reg) a.as_mut_ptr(),
                    b = in(reg) b.as_ptr(),
                    old = in(reg) &mut old,
                    cw = in(reg) &CW,
                    out("st(0)") _, out("st(1)") _, out("st(2)") _, out("st(3)") _,
                    out("st(4)") _, out("st(5)") _, out("st(6)") _, out("st(7)") _,
                    options(nostack),
                );
            }
        }
        F80::from_bytes(a)
    }

    /// `fldl2e` under the legacy control word (for tests).
    pub fn fldl2e() -> F80 {
        let mut a = [0u8; 10];
        let mut old: u16 = 0;
        // SAFETY: as in `f2xm1`.
        unsafe {
            asm!(
                "fnstcw word ptr [{old}]",
                "fldcw word ptr [{cw}]",
                "fldl2e",
                "fstp tbyte ptr [{a}]",
                "fldcw word ptr [{old}]",
                a = in(reg) a.as_mut_ptr(),
                old = in(reg) &mut old,
                cw = in(reg) &CW,
                out("st(0)") _, out("st(1)") _, out("st(2)") _, out("st(3)") _,
                out("st(4)") _, out("st(5)") _, out("st(6)") _, out("st(7)") _,
                options(nostack),
            );
        }
        F80::from_bytes(a)
    }
}
