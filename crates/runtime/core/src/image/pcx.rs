//! PCX decoder, a bug-for-bug port of dr_pcx v0.3.1 (`drpcx_load_memory`
//! with `flipped = false`, `desiredComponents = 4`), which the legacy
//! `func__loadimage` tried before stb_image.
//!
//! Quirks kept on purpose (golden output and old programs depend on them):
//! - reads past the end of the file yield zero bytes, so truncated files decode
//!   with black rows instead of failing;
//! - 1-bit single-plane images ignore the palette (black/white);
//! - 4-bit images use the mask `4 << (1 - nibble)`, so only bit 2 or 3 of each
//!   nibble is read;
//! - 2-bit, 4-plane images index the 16-colour palette past its end, into the
//!   rest of the 128-byte header;
//! - 8-bit images with 2 or more than 4 planes decode as all black.
//!
//! Deviation: dr_pcx computed the buffer size in 32 bits and overflowed
//! the heap for images above 4 GiB of pixels; those now fail cleanly.

const HEADER_LEN: usize = 128;

/// Mirrors `drpcx_memory` + `drpcx__read_byte`: short reads return zeros.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> u8 {
        let b = self.data.get(self.pos).copied().unwrap_or(0);
        self.pos = (self.pos + 1).min(self.data.len());
        b
    }

    /// Reads exactly `n` bytes or returns None (consuming what was there).
    fn exact(&mut self, n: usize) -> Option<&[u8]> {
        let start = self.pos;
        let end = (start + n).min(self.data.len());
        self.pos = end;
        (end - start == n).then(|| &self.data[start..end])
    }

    fn rle(&mut self) -> (u8, u8) {
        let mut value = self.byte();
        let count;
        if value & 0xC0 == 0xC0 {
            count = value & 0x3F;
            value = self.byte();
        } else {
            count = 1;
        }
        (count, value)
    }
}

struct Pcx<'a> {
    rd: Reader<'a>,
    header: [u8; HEADER_LEN],
    width: usize,
    height: usize,
    components: usize,
    /// Pixel data plus 4 bytes of slack: some dr_pcx paths write one pixel
    /// past the last row (heap overflow in C); the slack absorbs it.
    px: Vec<u8>,
    rle_count: u8,
    rle_value: u8,
}

impl Pcx<'_> {
    fn bit_planes(&self) -> u32 {
        self.header[65] as u32
    }

    fn bytes_per_line(&self) -> usize {
        u16::from_le_bytes([self.header[66], self.header[67]]) as usize
    }

    /// `header.palette16[i]`; indices past the palette read the rest of the
    /// header struct like the C code, and past the header give 0.
    fn palette16(&self, i: usize) -> u8 {
        self.header.get(16 + i).copied().unwrap_or(0)
    }

    fn next_value(&mut self) -> u8 {
        if self.rle_count == 0 {
            let (c, v) = self.rd.rle();
            self.rle_count = c;
            self.rle_value = v;
        }
        self.rle_count = self.rle_count.wrapping_sub(1);
        self.rle_value
    }

    fn row(&self, y: usize) -> usize {
        y * self.width * self.components
    }

    fn apply_palette16(&mut self, y: usize, channels: usize) {
        let mut p = self.row(y);
        for _ in 0..self.width {
            let index = self.px[p] as usize;
            for c in 0..channels {
                self.px[p + c] = self.palette16(index * 3 + c);
            }
            p += self.components;
        }
    }

    fn decode_1bit(&mut self) -> bool {
        let (bpl, w) = (self.bytes_per_line(), self.width);
        match self.bit_planes() {
            1 => {
                for y in 0..self.height {
                    let mut p = self.row(y);
                    for x in 0..bpl {
                        let v = self.next_value();
                        let mut bit = 0;
                        while bit < 8 && x * 8 + bit < w {
                            let index = (v >> (7 - bit)) & 1;
                            let g = index.wrapping_mul(255);
                            self.px[p..p + 3].fill(g);
                            p += 3;
                            bit += 1;
                        }
                    }
                }
                true
            }
            planes @ 2..=4 => {
                for y in 0..self.height {
                    for c in 0..planes {
                        let mut p = self.row(y);
                        for x in 0..bpl {
                            let v = self.next_value();
                            let mut bit = 0;
                            while bit < 8 && x * 8 + bit < w {
                                let index = (v >> (7 - bit)) & 1;
                                self.px[p] |= (index & 1) << c;
                                p += self.components;
                                bit += 1;
                            }
                        }
                    }
                    self.apply_palette16(y, self.components);
                }
                true
            }
            _ => false,
        }
    }

    fn decode_2bit(&mut self) -> bool {
        const CGA: [u8; 48] = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0xAA, 0x00, 0xAA, 0x00, 0x00, 0xAA, 0xAA, 0xAA, 0x00,
            0x00, 0xAA, 0x00, 0xAA, 0xAA, 0x55, 0x00, 0xAA, 0xAA, 0xAA, 0x55, 0x55, 0x55, 0x55,
            0x55, 0xFF, 0x55, 0xFF, 0x55, 0x55, 0xFF, 0xFF, 0xFF, 0x55, 0x55, 0xFF, 0x55, 0xFF,
            0xFF, 0xFF, 0x55, 0xFF, 0xFF, 0xFF,
        ];
        let (bpl, w) = (self.bytes_per_line(), self.width);
        match self.bit_planes() {
            1 => {
                let bg = self.palette16(0) >> 4;
                let i = (self.palette16(3) & 0x20) >> 5;
                let p_bit = (self.palette16(3) & 0x40) >> 6;
                for y in 0..self.height {
                    let mut p = self.row(y);
                    for x in 0..bpl {
                        let v = self.next_value();
                        for bit in 0..4 {
                            if x * 4 + bit < w {
                                let index = (v >> ((3 - bit) * 2)) & 3;
                                let cga = if index == 0 {
                                    bg
                                } else {
                                    ((index << 1) + p_bit) + (i << 3)
                                } as usize;
                                self.px[p..p + 3].copy_from_slice(&CGA[cga * 3..cga * 3 + 3]);
                                p += 3;
                            }
                        }
                    }
                }
                if self.header[1] == 5 {
                    // dr_pcx reads the palette marker but never uses the palette.
                    self.rd.byte();
                }
                true
            }
            4 => {
                for y in 0..self.height {
                    for c in 0..4u32 {
                        let mut p = self.row(y);
                        for x in 0..bpl {
                            let v = self.next_value();
                            let mut pair = 0;
                            while pair < 4 && x * 4 + pair < w {
                                let mask = 4u8 << (3 - pair);
                                let index = (v & mask) >> (3 - pair);
                                self.px[p] |= (index & 3) << (c * 2);
                                p += self.components;
                                pair += 1;
                            }
                        }
                    }
                    // dr_pcx loops over bitPlanes (4) channels here although a
                    // pixel has 3: the 4th write lands on the next pixel's red.
                    self.apply_palette16(y, 4);
                }
                true
            }
            _ => false,
        }
    }

    fn decode_4bit(&mut self) -> bool {
        let planes = self.bit_planes();
        if planes > 1 {
            return false;
        }
        let (bpl, w) = (self.bytes_per_line(), self.width);
        for y in 0..self.height {
            for c in 0..planes {
                let mut p = self.row(y);
                for x in 0..bpl {
                    let v = self.next_value();
                    let mut nibble = 0;
                    while nibble < 2 && x * 2 + nibble < w {
                        let mask = 4u8 << (1 - nibble);
                        let index = (v & mask) >> (1 - nibble);
                        self.px[p] |= (index & 0x0F) << (c * 4);
                        p += self.components;
                        nibble += 1;
                    }
                }
            }
            self.apply_palette16(y, self.components);
        }
        true
    }

    fn decode_8bit(&mut self) -> bool {
        let (bpl, w) = (self.bytes_per_line(), self.width);
        match self.bit_planes() {
            1 => {
                for y in 0..self.height {
                    let mut p = self.row(y);
                    for x in 0..bpl {
                        let v = self.next_value();
                        if x < w {
                            self.px[p..p + 3].fill(v);
                            p += 3;
                        }
                    }
                }
                if self.rd.byte() == 0x0C {
                    let Some(pal) = self.rd.exact(768) else {
                        return false;
                    };
                    let mut palette = [0u8; 768];
                    palette.copy_from_slice(pal);
                    let n = self.width * self.height;
                    for px in self.px[..n * 3].as_chunks_mut::<3>().0 {
                        let i = px[0] as usize * 3;
                        px.copy_from_slice(&palette[i..i + 3]);
                    }
                }
                true
            }
            3 | 4 => {
                let comps = self.components;
                for y in 0..self.height {
                    for c in 0..comps {
                        let mut p = self.row(y);
                        for x in 0..bpl {
                            let v = self.next_value();
                            if x < w {
                                self.px[p + c] = v;
                                p += comps;
                            }
                        }
                    }
                }
                true
            }
            _ => true,
        }
    }
}

/// Returns `(width, height, RGBA)` or `None` where dr_pcx returned NULL.
pub fn decode(data: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut rd = Reader { data, pos: 0 };
    let mut header = [0u8; HEADER_LEN];
    header.copy_from_slice(rd.exact(HEADER_LEN)?);
    if header[0] != 10 || header[2] != 1 || !matches!(header[3], 1 | 2 | 4 | 8) {
        return None;
    }
    let field = |o: usize| u16::from_le_bytes([header[o], header[o + 1]]);
    let (mut left, mut top, mut right, mut bottom) = (field(4), field(6), field(8), field(10));
    if left > right {
        std::mem::swap(&mut left, &mut right);
    }
    if top > bottom {
        std::mem::swap(&mut top, &mut bottom);
    }
    let width = (right - left) as usize + 1;
    let height = (bottom - top) as usize + 1;
    let components = if header[3] == 8 && header[65] == 4 {
        4
    } else {
        3
    };
    let size = width * height * components;
    if size > u32::MAX as usize {
        return None; // 1
    }
    // calloc in dr_pcx: allocation failure is a decode failure, not an abort.
    let mut px = Vec::new();
    px.try_reserve_exact(size + 4).ok()?;
    px.resize(size + 4, 0);
    let mut pcx = Pcx {
        rd,
        header,
        width,
        height,
        components,
        px,
        rle_count: 0,
        rle_value: 0,
    };
    let ok = match header[3] {
        1 => pcx.decode_1bit(),
        2 => pcx.decode_2bit(),
        4 => pcx.decode_4bit(),
        _ => pcx.decode_8bit(),
    };
    if !ok {
        return None;
    }
    pcx.px.truncate(size);
    let rgba = if components == 4 {
        pcx.px
    } else {
        let mut out = Vec::new();
        out.try_reserve_exact(width * height * 4).ok()?;
        for p in pcx.px.as_chunks::<3>().0 {
            out.extend_from_slice(&[p[0], p[1], p[2], 0xFF]);
        }
        out
    };
    Some((width as u32, height as u32, rgba))
}

#[cfg(test)]
mod tests {
    use super::decode;

    /// 128-byte header: bpp, planes, size, bytes per line, 16-colour palette.
    fn header(bpp: u8, planes: u8, w: u16, h: u16, bpl: u16, pal16: &[u8]) -> Vec<u8> {
        let mut v = vec![0u8; 128];
        v[0] = 10;
        v[1] = 5;
        v[2] = 1;
        v[3] = bpp;
        v[8..10].copy_from_slice(&(w - 1).to_le_bytes());
        v[10..12].copy_from_slice(&(h - 1).to_le_bytes());
        v[16..16 + pal16.len()].copy_from_slice(pal16);
        v[65] = planes;
        v[66..68].copy_from_slice(&bpl.to_le_bytes());
        v
    }

    #[test]
    fn eight_bit_with_palette() {
        let mut f = header(8, 1, 2, 1, 2, &[]);
        f.extend_from_slice(&[0xC2, 0x01, 0x0C]);
        let mut pal = vec![0u8; 768];
        pal[3..6].copy_from_slice(&[10, 20, 30]);
        f.extend_from_slice(&pal);
        assert_eq!(
            decode(&f),
            Some((2, 1, vec![10, 20, 30, 255, 10, 20, 30, 255]))
        );
    }

    #[test]
    fn eight_bit_without_palette_is_gray_and_short_palette_fails() {
        let mut f = header(8, 1, 2, 1, 2, &[]);
        f.extend_from_slice(&[0x07, 0x09]);
        assert_eq!(decode(&f), Some((2, 1, vec![7, 7, 7, 255, 9, 9, 9, 255])));
        f.extend_from_slice(&[0x0C, 1, 2, 3]);
        assert_eq!(decode(&f), None);
    }

    #[test]
    fn one_bit_mono_ignores_palette() {
        let mut f = header(1, 1, 3, 1, 2, &[9; 48]);
        f.extend_from_slice(&[0b1010_0000, 0x00]);
        assert_eq!(
            decode(&f),
            Some((
                3,
                1,
                vec![255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255]
            ))
        );
    }

    #[test]
    fn four_bit_reads_only_bit_two_or_three_of_each_nibble() {
        let mut pal = [0u8; 48];
        pal[12..15].copy_from_slice(&[40, 41, 42]); // entry 4
        let mut f = header(4, 1, 2, 1, 1, &pal);
        f.push(0xC1);
        f.push(0xFF);
        assert_eq!(
            decode(&f),
            Some((2, 1, vec![40, 41, 42, 255, 40, 41, 42, 255]))
        );
    }

    #[test]
    fn truncated_data_reads_zeros_and_eight_bit_two_planes_is_black() {
        let f = header(8, 1, 2, 2, 2, &[]);
        assert_eq!(decode(&f), Some((2, 2, [0, 0, 0, 255].repeat(4))));
        let mut f = header(8, 2, 1, 1, 1, &[]);
        f.extend_from_slice(&[0x55, 0x55]);
        assert_eq!(decode(&f), Some((1, 1, vec![0, 0, 0, 255])));
    }

    #[test]
    fn swapped_bounds_and_rejections() {
        let mut f = header(8, 3, 1, 1, 1, &[]);
        f[4..6].copy_from_slice(&7u16.to_le_bytes()); // left 7 > right 0
        f.extend_from_slice(&[1; 24]);
        assert_eq!(decode(&f).map(|(w, h, _)| (w, h)), Some((8, 1)));
        let good = header(8, 1, 1, 1, 1, &[]);
        for (i, v) in [(0usize, 11u8), (2, 0), (3, 3)] {
            let mut bad = good.clone();
            bad[i] = v;
            assert_eq!(decode(&bad), None, "byte {i} = {v}");
        }
        assert_eq!(decode(&good[..127]), None);
    }
}
