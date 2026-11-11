//! CALL ABSOLUTE: the small 16-bit x86 interpreter of libqb.cpp (`cpu_call`,
//! `call_absolute`): MOV forms, PUSH/POP, RETF (returns to BASIC) and INT
//! (INT 33h mouse queries are answered by the caller, as libqb `call_int`), over the
//! program's conventional memory. Anything else is an "X86 Error" (exit 86)
//! like libqb.

const ES: usize = 0;
const CS: usize = 1;
const SS: usize = 2;
const DS: usize = 3;
const FS: usize = 4;
const GS: usize = 5;

#[derive(Clone, Copy)]
enum Loc {
    Reg(usize),
    Mem(usize),
}

struct Cpu<'a> {
    m: &'a mut [u8],
    /// eax ecx edx ebx esp ebp esi edi
    r: [u32; 8],
    s: [u16; 8],
    ip: usize,
    seg: usize,
    seg_bp: usize,
    a32: bool,
}

impl Cpu<'_> {
    /// Byte at linear address `a`; outside the memory image it reads as 0
    /// (which also ends the program loop) instead of panicking (R14).
    fn rd(&self, a: usize) -> u8 {
        self.m.get(a).copied().unwrap_or(0)
    }
    /// Stores a byte; writes outside the memory image are dropped (R14).
    fn wr(&mut self, a: usize, v: u8) {
        if let Some(x) = self.m.get_mut(a) {
            *x = v;
        }
    }
    fn rd_n<const N: usize>(&self, a: usize) -> [u8; N] {
        core::array::from_fn(|k| self.rd(a + k))
    }
    fn wr_n(&mut self, a: usize, bytes: &[u8]) {
        for (k, &v) in bytes.iter().enumerate() {
            self.wr(a + k, v);
        }
    }
    fn b(&mut self) -> u8 {
        let v = self.rd(self.ip);
        self.ip += 1;
        v
    }
    fn w(&mut self) -> u16 {
        let v = u16::from_le_bytes(self.rd_n(self.ip));
        self.ip += 2;
        v
    }
    fn d(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.rd_n(self.ip));
        self.ip += 4;
        v
    }
    fn base(&self, s: usize) -> usize {
        usize::from(self.s[s]) * 16
    }
    fn r16(&self, i: usize) -> u16 {
        self.r[i] as u16
    }
    fn sib(&mut self, mod0: bool) -> u32 {
        let i = u32::from(self.b());
        let idx = self.r[(i >> 3 & 7) as usize] << (i >> 6);
        if mod0 && i & 7 == 5 {
            return self.d().wrapping_add(idx);
        }
        self.r[(i & 7) as usize].wrapping_add(idx)
    }
    /// ModR/M operand (libqb rm8/rm16/rm32: 16-bit offsets inside the segment)
    fn ea(&mut self) -> Loc {
        let i = usize::from(self.b());
        let (m, rm) = (i >> 6, i & 7);
        if m == 3 {
            return Loc::Reg(rm);
        }
        let (seg, sbp) = (self.seg, self.seg_bp);
        let (bx, si, di, bp) = (self.r16(3), self.r16(6), self.r16(7), self.r16(5));
        let off: (usize, u16) = if self.a32 {
            let regv = |c: &Self, k: usize| c.r[k];
            let disp: u32 = match m {
                0 => 0,
                1 => self.b() as i8 as i32 as u32,
                _ => self.d(),
            };
            match rm {
                4 => {
                    let v = self.sib(m == 0);
                    (seg, v.wrapping_add(disp) as u16)
                }
                5 if m == 0 => (seg, self.d() as u16),
                5 => (sbp, regv(self, 5).wrapping_add(disp) as u16),
                k => (seg, regv(self, k).wrapping_add(disp) as u16),
            }
        } else {
            let disp: u16 = match m {
                0 => 0,
                1 => self.b() as i8 as i16 as u16,
                _ => self.w(),
            };
            match rm {
                0 => (seg, bx.wrapping_add(si).wrapping_add(disp)),
                1 => (seg, bx.wrapping_add(di).wrapping_add(disp)),
                2 => (sbp, bp.wrapping_add(si).wrapping_add(disp)),
                3 => (sbp, bp.wrapping_add(di).wrapping_add(disp)),
                4 => (seg, si.wrapping_add(disp)),
                5 => (seg, di.wrapping_add(disp)),
                6 if m == 0 => (seg, self.w()),
                6 => (sbp, bp.wrapping_add(disp)),
                _ => (seg, bx.wrapping_add(disp)),
            }
        };
        Loc::Mem(off.0 + usize::from(off.1))
    }
    fn get(&self, l: Loc, size: u32) -> u32 {
        match l {
            Loc::Reg(k) => match size {
                8 => {
                    if k < 4 { self.r[k] & 255 } else { self.r[k - 4] >> 8 & 255 }
                }
                16 => self.r[k] & 0xFFFF,
                _ => self.r[k],
            },
            Loc::Mem(a) => match size {
                8 => u32::from(self.rd(a)),
                16 => u32::from(u16::from_le_bytes(self.rd_n(a))),
                _ => u32::from_le_bytes(self.rd_n(a)),
            },
        }
    }
    fn set(&mut self, l: Loc, size: u32, v: u32) {
        match l {
            Loc::Reg(k) => match size {
                8 => {
                    if k < 4 {
                        self.r[k] = self.r[k] & !255 | (v & 255);
                    } else {
                        self.r[k - 4] = self.r[k - 4] & !0xFF00 | (v & 255) << 8;
                    }
                }
                16 => self.r[k] = self.r[k] & !0xFFFF | (v & 0xFFFF),
                _ => self.r[k] = v,
            },
            Loc::Mem(a) => match size {
                8 => self.wr(a, v as u8),
                16 => self.wr_n(a, &(v as u16).to_le_bytes()),
                _ => self.wr_n(a, &v.to_le_bytes()),
            },
        }
    }
    fn sp(&self) -> u16 {
        self.r16(4)
    }
    fn set_sp(&mut self, v: u16) {
        self.r[4] = self.r[4] & !0xFFFF | u32::from(v);
    }
    fn push(&mut self, v: u32, size: u32) {
        let sp = self.sp().wrapping_sub((size / 8) as u16);
        self.set_sp(sp);
        let a = self.base(SS) + usize::from(sp);
        self.set(Loc::Mem(a), size, v);
    }
    fn pop(&mut self, size: u32) -> u32 {
        let a = self.base(SS) + usize::from(self.sp());
        let v = self.get(Loc::Mem(a), size);
        self.set_sp(self.sp().wrapping_add((size / 8) as u16));
        v
    }
}

/// `call_absolute(args, offset)` with `defseg` (bytes) and the near
/// offsets of the arguments; `ds` is the data segment of the variables.
/// `Err(86)` = unknown opcode (libqb exits with code 86). `int` serves `INT n`
/// (number, registers AX CX DX BX SP BP SI DI; libqb `call_int` answers only 33h, the mouse).
pub(crate) fn call_absolute(m: &mut [u8], defseg: usize, ds: u16, offset: u16, args: &[u16], int: &mut dyn FnMut(u8, &mut [u32; 8])) -> Result<(), i32> {
    let mut c = Cpu { m, r: [0; 8], s: [0; 8], ip: 0, seg: 0, seg_bp: 0, a32: false };
    c.s[CS] = (defseg >> 4) as u16;
    c.s[SS] = 0xFFFF;
    c.s[DS] = ds;
    c.set_sp(0);
    for &a in args {
        c.push(u32::from(a), 16);
    }
    c.push(0xFFFF_FFFF, 32); // far return to BASIC
    c.ip = c.base(CS) + usize::from(offset);
    loop {
        let mut b32 = false;
        c.a32 = false;
        c.seg = c.base(DS);
        c.seg_bp = c.base(SS);
        let mut i = c.b();
        if i == 0x66 {
            b32 = true;
            i = c.b();
        }
        match i {
            0x26 => (c.seg, c.seg_bp, i) = (c.base(ES), c.base(ES), c.b()),
            0x2E => (c.seg, c.seg_bp, i) = (c.base(CS), c.base(CS), c.b()),
            0x36 => (c.seg, i) = (c.base(SS), c.b()),
            0x3E => (c.seg_bp, i) = (c.base(DS), c.b()),
            0x64 => (c.seg, c.seg_bp, i) = (c.base(FS), c.base(FS), c.b()),
            0x65 => (c.seg, c.seg_bp, i) = (c.base(GS), c.base(GS), c.b()),
            _ => {}
        }
        if i == 0x67 {
            c.a32 = true;
            i = c.b();
        }
        let wsz = if b32 { 32 } else { 16 };
        let r = usize::from(c.rd(c.ip) >> 3 & 7);
        match i {
            0x88 => {
                let l = c.ea();
                let v = c.get(Loc::Reg(r), 8);
                c.set(l, 8, v);
            }
            0x89 => {
                let l = c.ea();
                let v = c.get(Loc::Reg(r), wsz);
                c.set(l, wsz, v);
            }
            0x8A => {
                let l = c.ea();
                let v = c.get(l, 8);
                c.set(Loc::Reg(r), 8, v);
            }
            0x8B => {
                let l = c.ea();
                let v = c.get(l, wsz);
                c.set(Loc::Reg(r), wsz, v);
            }
            0x8C => {
                let l = c.ea();
                let v = u32::from(c.s[r]);
                c.set(l, 16, v);
            }
            0x8E => {
                let l = c.ea();
                if r != CS {
                    c.s[r] = c.get(l, 16) as u16;
                }
            }
            0xA0..=0xA3 => {
                let a = c.seg + usize::from(c.w());
                match i {
                    0xA0 => c.r[0] = c.r[0] & !255 | u32::from(c.rd(a)),
                    0xA1 => {
                        let v = c.get(Loc::Mem(a), wsz);
                        c.set(Loc::Reg(0), wsz, v);
                    }
                    0xA2 => c.wr(a, c.r[0] as u8),
                    _ => {
                        let v = c.get(Loc::Reg(0), wsz);
                        c.set(Loc::Mem(a), wsz, v);
                    }
                }
            }
            0xB0..=0xB7 => {
                let v = c.b();
                c.set(Loc::Reg(usize::from(i & 7)), 8, u32::from(v));
            }
            0xB8..=0xBF => {
                let v = if b32 { c.d() } else { u32::from(c.w()) };
                c.set(Loc::Reg(usize::from(i & 7)), wsz, v);
            }
            0xC6 => {
                let l = c.ea();
                let v = c.b();
                c.set(l, 8, u32::from(v));
            }
            0xC7 => {
                let l = c.ea();
                let v = if b32 { c.d() } else { u32::from(c.w()) };
                c.set(l, wsz, v);
            }
            0xCA | 0xCB => return Ok(()),
            0xCD => {
                let n = c.b();
                int(n, &mut c.r);
            }
            0xFF => {
                let l = c.ea();
                let v = c.get(l, wsz);
                c.push(v, wsz);
            }
            0x50..=0x57 => {
                let v = c.get(Loc::Reg(usize::from(i & 7)), wsz);
                c.push(v, wsz);
            }
            0x6A => {
                let v = c.b() as i8 as i16 as u16;
                c.push(u32::from(v), 16);
            }
            0x68 => {
                let v = if b32 { c.d() } else { u32::from(c.w()) };
                c.push(v, wsz);
            }
            0x0E | 0x16 | 0x1E | 0x06 => {
                let s = match i {
                    0x0E => CS,
                    0x16 => SS,
                    0x1E => DS,
                    _ => ES,
                };
                let v = u32::from(c.s[s]);
                c.push(v, 16);
            }
            0x8F => {
                let v = c.pop(wsz);
                let l = c.ea();
                c.set(l, wsz, v);
            }
            0x58..=0x5F => {
                let v = c.pop(wsz);
                c.set(Loc::Reg(usize::from(i & 7)), wsz, v);
            }
            0x1F | 0x07 | 0x17 => {
                let s = match i {
                    0x1F => DS,
                    0x07 => ES,
                    _ => SS,
                };
                c.s[s] = c.pop(16) as u16;
            }
            0x0F => {
                let j = c.b();
                match j {
                    0xA0 => c.push(u32::from(c.s[FS]), 16),
                    0xA8 => c.push(u32::from(c.s[GS]), 16),
                    0xA1 => c.s[FS] = c.pop(16) as u16,
                    0xA9 => c.s[GS] = c.pop(16) as u16,
                    _ => return Err(86),
                }
            }
            _ => return Err(86),
        }
        if c.rd(c.ip) == 0 {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn stores_through_a_near_pointer_argument() {
        // push bp / mov bp,sp / mov bx,[bp+6] / mov word [bx],1234 / pop bp / retf 2
        let code = [0x55, 0x89, 0xE5, 0x8B, 0x5E, 6, 0xC7, 7, 0xD2, 4, 0x5D, 0xCA, 2, 0];
        let mut m = vec![0u8; 0x110000];
        m[0x2000..0x2000 + code.len()].copy_from_slice(&code);
        super::call_absolute(&mut m, 0x2000, 0x300, 0, &[0x40], &mut |_, _| {}).unwrap();
        assert_eq!(u16::from_le_bytes([m[0x3040], m[0x3041]]), 1234);
    }

    /// R14: code that runs off the end of conventional memory (no RETF and
    /// no 0 byte before the end) and accesses near the top must not panic.
    #[test]
    fn running_off_the_end_of_memory_does_not_panic() {
        let mut m = vec![0u8; 0x110000];
        // DEF SEG = &HFFFF, offset &HFFF0: MOV AX, imm16 repeated to the end.
        let start = 0xFFFF0 + 0xFFF0;
        for (k, b) in m[start..].iter_mut().enumerate() {
            *b = if k % 3 == 0 { 0xB8 } else { 0x11 };
        }
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| super::call_absolute(&mut m, 0xFFFF0, 0xFFFF, 0xFFF0, &[], &mut |_, _| {})));
        assert!(r.is_ok(), "panicked");
        // A memory image smaller than 1 MB + 64 KB (stack at FFFF:xxxx).
        let mut small = vec![0u8; 0x100];
        small[0] = 0x50; // push ax
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| super::call_absolute(&mut small, 0, 0, 0, &[1, 2], &mut |_, _| {})));
        assert!(r.is_ok(), "panicked on a small image");
        // MOV AX,[FFFF] with DS = FFFF reads the last bytes of the image.
        let mut m = vec![0u8; 0x110000];
        m[..5].copy_from_slice(&[0x66, 0xA1, 0xFF, 0xFF, 0xCB]);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| super::call_absolute(&mut m, 0, 0xFFFF, 0, &[], &mut |_, _| {})));
        assert!(r.is_ok(), "panicked on a 32-bit read at the top");
    }
}
