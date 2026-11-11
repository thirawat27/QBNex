//! The BIOS keyboard buffer behind INKEY$ (a 15-key ring at `cmem` 0x41E,
//! head/tail words at 0x41A/0x41C, as on a PC) and INPUT$ from the keyboard.
//! Functions take the BIOS data area base (`cmem+0x400`).

use super::Host;

/// `_KEYCLEAR 1` / start-up: empties the ring.
pub unsafe fn clear(bios: *mut u8) {
    *bios.add(0x1a) = 30;
    *bios.add(0x1b) = 0;
    *bios.add(0x1c) = 30;
    *bios.add(0x1d) = 0;
}

/// Appends a key (ASCII code `b1`, scan code `b2`); dropped when the ring is full.
pub unsafe fn push(bios: *mut u8, b1: u8, b2: u8) {
    let head = *bios.add(0x1a);
    let tail = *bios.add(0x1c);
    let mut next = tail.wrapping_add(2);
    if next == 62 {
        next = 30;
    }
    if head != next {
        *bios.add(tail as usize) = b1;
        *bios.add(tail as usize + 1) = b2;
        *bios.add(0x1c) = next;
    }
}

/// `qbs_inkey`: next key as INKEY$ returns it (1 byte, or CHR$(0)+scan code),
/// None when the buffer is empty.
pub unsafe fn pop(bios: *mut u8) -> Option<Vec<u8>> {
    let head = *bios.add(0x1a);
    if head == *bios.add(0x1c) {
        return None;
    }
    let c0 = *bios.add(head as usize);
    let c1 = *bios.add(head as usize + 1);
    let mut next = head.wrapping_add(2);
    if next == 62 {
        next = 30;
    }
    *bios.add(0x1a) = next;
    Some(if c0 != 0 || c1 == 0 { vec![c0] } else { vec![0, c1] })
}

/// INPUT$(n) from the keyboard: waits for `n` keys. An extended key gives only
/// its CHR$(0). Error 52 for n < 0. When the program is stopped the rest of the
/// result stays 0.
pub unsafe fn input_n(h: &mut dyn Host, n: i32) -> Vec<u8> {
    if n < 0 {
        h.error(52);
        return Vec::new();
    }
    let mut out = vec![0u8; n as usize];
    let mut x = 0usize;
    while x < n as usize {
        let b = h.bios();
        if let Some(k) = pop(b) {
            out[x] = k[0];
            x += 1;
        }
        if h.stop_program() {
            return out;
        }
        if x < n as usize {
            h.events();
            h.sleep(10);
        }
    }
    out
}
