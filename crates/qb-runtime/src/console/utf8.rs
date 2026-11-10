//! Code point helpers for text output: UTF-8 decoding with a code page 437
//! fallback, the code page 437 <-> Unicode table (`_MAPUNICODE` changes it at
//! run time), and the width classes used to lay out SCREEN 0 cells.

/// Code page 437 -> Unicode (BMP). Generated code and libqb.cpp (`_MAPUNICODE`,
/// `_UPRINTSTRING`, the text renderer) read and write it through this symbol.
#[unsafe(no_mangle)]
pub static mut codepage437_to_unicode16: [u16; 256] = CP437_DEFAULT;

const CP437_DEFAULT: [u16; 256] = [
    0x0020, 0x263A, 0x263B, 0x2665, 0x2666, 0x2663, 0x2660, 0x2022, 0x25D8, 0x25CB, 0x25D9, 0x2642, 0x2640, 0x266A,
    0x266B, 0x263C, 0x25BA, 0x25C4, 0x2195, 0x203C, 0x00B6, 0x00A7, 0x25AC, 0x21A8, 0x2191, 0x2193, 0x2192, 0x2190,
    0x221F, 0x2194, 0x25B2, 0x25BC, 0x0020, 0x0021, 0x0022, 0x0023, 0x0024, 0x0025, 0x0026, 0x0027, 0x0028, 0x0029,
    0x002A, 0x002B, 0x002C, 0x002D, 0x002E, 0x002F, 0x0030, 0x0031, 0x0032, 0x0033, 0x0034, 0x0035, 0x0036, 0x0037,
    0x0038, 0x0039, 0x003A, 0x003B, 0x003C, 0x003D, 0x003E, 0x003F, 0x0040, 0x0041, 0x0042, 0x0043, 0x0044, 0x0045,
    0x0046, 0x0047, 0x0048, 0x0049, 0x004A, 0x004B, 0x004C, 0x004D, 0x004E, 0x004F, 0x0050, 0x0051, 0x0052, 0x0053,
    0x0054, 0x0055, 0x0056, 0x0057, 0x0058, 0x0059, 0x005A, 0x005B, 0x005C, 0x005D, 0x005E, 0x005F, 0x0060, 0x0061,
    0x0062, 0x0063, 0x0064, 0x0065, 0x0066, 0x0067, 0x0068, 0x0069, 0x006A, 0x006B, 0x006C, 0x006D, 0x006E, 0x006F,
    0x0070, 0x0071, 0x0072, 0x0073, 0x0074, 0x0075, 0x0076, 0x0077, 0x0078, 0x0079, 0x007A, 0x007B, 0x007C, 0x007D,
    0x007E, 0x2302, 0x00C7, 0x00FC, 0x00E9, 0x00E2, 0x00E4, 0x00E0, 0x00E5, 0x00E7, 0x00EA, 0x00EB, 0x00E8, 0x00EF,
    0x00EE, 0x00EC, 0x00C4, 0x00C5, 0x00C9, 0x00E6, 0x00C6, 0x00F4, 0x00F6, 0x00F2, 0x00FB, 0x00F9, 0x00FF, 0x00D6,
    0x00DC, 0x00A2, 0x00A3, 0x00A5, 0x20A7, 0x0192, 0x00E1, 0x00ED, 0x00F3, 0x00FA, 0x00F1, 0x00D1, 0x00AA, 0x00BA,
    0x00BF, 0x2310, 0x00AC, 0x00BD, 0x00BC, 0x00A1, 0x00AB, 0x00BB, 0x2591, 0x2592, 0x2593, 0x2502, 0x2524, 0x2561,
    0x2562, 0x2556, 0x2555, 0x2563, 0x2551, 0x2557, 0x255D, 0x255C, 0x255B, 0x2510, 0x2514, 0x2534, 0x252C, 0x251C,
    0x2500, 0x253C, 0x255E, 0x255F, 0x255A, 0x2554, 0x2569, 0x2566, 0x2560, 0x2550, 0x256C, 0x2567, 0x2568, 0x2564,
    0x2565, 0x2559, 0x2558, 0x2552, 0x2553, 0x256B, 0x256A, 0x2518, 0x250C, 0x2588, 0x2584, 0x258C, 0x2590, 0x2580,
    0x03B1, 0x00DF, 0x0393, 0x03C0, 0x03A3, 0x03C3, 0x00B5, 0x03C4, 0x03A6, 0x0398, 0x03A9, 0x03B4, 0x221E, 0x03C6,
    0x03B5, 0x2229, 0x2261, 0x00B1, 0x2265, 0x2264, 0x2320, 0x2321, 0x00F7, 0x2248, 0x00B0, 0x2219, 0x00B7, 0x221A,
    0x207F, 0x00B2, 0x25A0, 0x0020,
];

/// Current mapping of code page 437 byte `b`.
#[inline]
pub fn cp437(b: u8) -> u32 {
    // SAFETY: plain read of a u16 element; writers (`_MAPUNICODE`) run on the
    // program thread, as in legacy.
    unsafe { core::ptr::addr_of!(codepage437_to_unicode16).cast::<u16>().add(b as usize).read() as u32 }
}

/// `unicode_to_cp437`: first byte that maps to `x`, 0 if none (so U+0020 maps to 0).
pub fn unicode_to_cp437(x: u32) -> u32 {
    (0..=255u32).find(|&i| cp437(i as u8) == x).unwrap_or(0)
}

/// `qb_decode_utf8_codepoint`: returns (bytes used, code point); an invalid or
/// truncated sequence yields its first byte mapped through code page 437.
pub fn decode(text: &[u8]) -> (usize, u32) {
    let len = text.len();
    if len == 0 {
        return (0, 0);
    }
    let b0 = text[0];
    if b0 < 0x80 {
        return (1, b0 as u32);
    }
    if (0xC2..=0xDF).contains(&b0) && len >= 2 {
        let b1 = text[1];
        if b1 & 0xC0 == 0x80 {
            return (2, ((b0 as u32 & 0x1F) << 6) | (b1 as u32 & 0x3F));
        }
    }
    if (0xE0..=0xEF).contains(&b0) && len >= 3 {
        let (b1, b2) = (text[1], text[2]);
        if b1 & 0xC0 == 0x80 && b2 & 0xC0 == 0x80 {
            let v = ((b0 as u32 & 0x0F) << 12) | ((b1 as u32 & 0x3F) << 6) | (b2 as u32 & 0x3F);
            if v >= 0x800 && !(0xD800..=0xDFFF).contains(&v) && !(b0 == 0xE0 && b1 < 0xA0) && !(b0 == 0xED && b1 >= 0xA0)
            {
                return (3, v);
            }
        }
    }
    if (0xF0..=0xF4).contains(&b0) && len >= 4 {
        let (b1, b2, b3) = (text[1], text[2], text[3]);
        if b1 & 0xC0 == 0x80 && b2 & 0xC0 == 0x80 && b3 & 0xC0 == 0x80 {
            let v = ((b0 as u32 & 0x07) << 18) | ((b1 as u32 & 0x3F) << 12) | ((b2 as u32 & 0x3F) << 6) | (b3 as u32 & 0x3F);
            if (0x10000..=0x10FFFF).contains(&v) && !(b0 == 0xF0 && b1 < 0x90) && !(b0 == 0xF4 && b1 > 0x8F) {
                return (4, v);
            }
        }
    }
    (1, cp437(b0))
}

/// `qb_append_utf8_codepoint`.
pub fn push_utf8(out: &mut Vec<u8>, mut cp: u32) {
    if cp > 0x10FFFF {
        cp = 0xFFFD;
    }
    if cp <= 0x7F {
        out.push(cp as u8);
    } else if cp <= 0x7FF {
        out.extend_from_slice(&[0xC0 | (cp >> 6) as u8, 0x80 | (cp & 0x3F) as u8]);
    } else if cp <= 0xFFFF {
        out.extend_from_slice(&[0xE0 | (cp >> 12) as u8, 0x80 | ((cp >> 6) & 0x3F) as u8, 0x80 | (cp & 0x3F) as u8]);
    } else {
        out.extend_from_slice(&[
            0xF0 | (cp >> 18) as u8,
            0x80 | ((cp >> 12) & 0x3F) as u8,
            0x80 | ((cp >> 6) & 0x3F) as u8,
            0x80 | (cp & 0x3F) as u8,
        ]);
    }
}

pub fn is_emoji_like(cp: u32) -> bool {
    (0x1F1E6..=0x1F1FF).contains(&cp)
        || (0x1F300..=0x1FAFF).contains(&cp)
        || (0x2600..=0x27BF).contains(&cp)
        || (0x2300..=0x23FF).contains(&cp)
        || cp == 0x20E3
}

/// Combining marks and joiners: attached to the previous cell, never given one.
pub fn is_zero_width(cp: u32) -> bool {
    (0x0300..=0x036F).contains(&cp)
        || (0x1AB0..=0x1AFF).contains(&cp)
        || (0x1DC0..=0x1DFF).contains(&cp)
        || (0x20D0..=0x20FF).contains(&cp)
        || (0xFE20..=0xFE2F).contains(&cp)
        || (0xFE00..=0xFE0F).contains(&cp)
        || (0xE0100..=0xE01EF).contains(&cp)
        || matches!(cp, 0x0E31 | 0x200C | 0x200D | 0xFEFF)
        || (0x0E34..=0x0E3A).contains(&cp)
        || (0x0E47..=0x0E4E).contains(&cp)
}

/// East Asian wide characters and emoji take two SCREEN 0 cells.
pub fn is_wide(cp: u32) -> bool {
    is_emoji_like(cp)
        || (0x1100..=0x115F).contains(&cp)
        || (0x2329..=0x232A).contains(&cp)
        || ((0x2E80..=0xA4CF).contains(&cp) && cp != 0x303F)
        || (0xAC00..=0xD7A3).contains(&cp)
        || (0xF900..=0xFAFF).contains(&cp)
        || (0xFE10..=0xFE19).contains(&cp)
        || (0xFE30..=0xFE6F).contains(&cp)
        || (0xFF01..=0xFF60).contains(&cp)
        || (0xFFE0..=0xFFE6).contains(&cp)
}

/// Byte stored in the SCREEN 0 character plane for code point `c`. Code
/// points that code page 437 lacks become 0 (legacy: `unicode_to_cp437`
/// returns 0, so its `'?'` fallback is never reached); the UTF-8 side table
/// keeps the real character.
pub fn screen_byte(c: u32) -> u8 {
    if c <= 0x7F {
        return c as u8;
    }
    let m = unicode_to_cp437(c);
    if m <= 0xFF {
        m as u8
    } else {
        b'?'
    }
}

/// C ABI: `unicode_to_cp437`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_unicode_to_cp437(x: u32) -> u32 {
    unicode_to_cp437(x)
}

/// C ABI: `qb_decode_utf8_codepoint` (0 for an empty input).
///
/// # Safety
/// `text` valid for `len` bytes; `codepoint` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_utf8_decode(text: *const u8, len: i32, codepoint: *mut u32) -> i32 {
    if text.is_null() || len <= 0 || codepoint.is_null() {
        return 0;
    }
    // SAFETY: caller contract.
    let (n, cp) = decode(unsafe { core::slice::from_raw_parts(text, len as usize) });
    unsafe { *codepoint = cp };
    n as i32
}

/// C ABI: `qb_is_zero_width_codepoint` (-1 / 0).
#[unsafe(no_mangle)]
pub extern "C" fn qb_utf8_is_zero_width(cp: u32) -> i32 {
    -(is_zero_width(cp) as i32)
}

/// C ABI: `qb_is_emoji_like_codepoint` (-1 / 0).
#[unsafe(no_mangle)]
pub extern "C" fn qb_utf8_is_emoji_like(cp: u32) -> i32 {
    -(is_emoji_like(cp) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_valid_and_fallback() {
        assert_eq!(decode(b"A"), (1, 65));
        assert_eq!(decode("é".as_bytes()), (2, 0xE9));
        assert_eq!(decode("ก".as_bytes()), (3, 0x0E01));
        assert_eq!(decode("😀".as_bytes()), (4, 0x1F600));
        // lone continuation / overlong / truncated: CP437 of the first byte
        assert_eq!(decode(&[0x82]), (1, 0x00E9));
        assert_eq!(decode(&[0xC0, 0x80]), (1, cp437(0xC0)));
        assert_eq!(decode(&[0xE0, 0x80, 0x80]), (1, cp437(0xE0)));
        assert_eq!(decode(&[0xED, 0xA0, 0x80]), (1, cp437(0xED)));
        assert_eq!(decode(&[0xE2, 0x82]), (1, cp437(0xE2)));
        assert_eq!(decode(&[0xF4, 0x90, 0x80, 0x80]), (1, cp437(0xF4)));
    }

    #[test]
    fn widths_and_screen_bytes() {
        assert!(is_wide(0x4E00) && is_wide(0x1F600) && !is_wide(0x303F) && !is_wide(65));
        assert!(is_zero_width(0x0301) && is_zero_width(0x0E31) && !is_zero_width(0x0E01));
        assert_eq!(screen_byte(0x2591), 0xB0);
        assert_eq!(screen_byte(0x0E01), 0); // unmapped: unicode_to_cp437 gives 0
        assert_eq!(screen_byte(0x20), 0x20);
        assert_eq!(unicode_to_cp437(0x20), 0); // first match is byte 0
        let mut v = Vec::new();
        push_utf8(&mut v, 0x0E01);
        push_utf8(&mut v, 0x110000);
        assert_eq!(v, "ก\u{FFFD}".as_bytes());
    }
}
