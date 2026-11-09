//! Bug-for-bug ports of the stb_image v2.27 loaders for the simple formats
//! (BMP, TGA, GIF, PNM) and of its format dispatch (`stbi__load_main`).
//! PNG and JPEG go to the `png` and `zune-jpeg` crates (see `codecs.rs`).
//!
//! Every function mirrors the C code it names; comments only mark the places
//! where C behaviour is undefined and a fixed choice was made.

/// `stbi__context` over a memory buffer: reads past the end give 0,
/// `skip` may move past the end, negative skips jump to the end.
pub struct Ctx<'a> {
    d: &'a [u8],
    pos: usize,
}

impl<'a> Ctx<'a> {
    pub fn new(d: &'a [u8]) -> Self {
        Ctx { d, pos: 0 }
    }

    pub fn rewind(&mut self) {
        self.pos = 0;
    }

    pub fn get8(&mut self) -> u8 {
        if self.pos < self.d.len() {
            self.pos += 1;
            self.d[self.pos - 1]
        } else {
            0
        }
    }

    pub fn get16le(&mut self) -> i32 {
        let z = self.get8() as i32;
        z | (self.get8() as i32) << 8
    }

    pub fn get32le(&mut self) -> u32 {
        let z = self.get16le() as u32;
        z.wrapping_add((self.get16le() as u32) << 16)
    }

    pub fn skip(&mut self, n: i64) {
        if n == 0 {
            return;
        }
        if n < 0 {
            self.pos = self.d.len();
            return;
        }
        self.pos = self.pos.saturating_add(n as usize);
    }

    /// `stbi__getn`: all or nothing; on failure nothing is consumed.
    pub fn getn(&mut self, out: &mut [u8]) -> bool {
        match self.pos.checked_add(out.len()) {
            Some(end) if end <= self.d.len() => {
                out.copy_from_slice(&self.d[self.pos..end]);
                self.pos = end;
                true
            }
            _ => false,
        }
    }

    pub fn at_eof(&self) -> bool {
        self.pos >= self.d.len()
    }

    fn consumed(&self) -> i64 {
        self.pos as i64
    }
}

pub const MAX_DIMENSIONS: u32 = 1 << 24;

/// `stbi__mul2sizes_valid`
fn mul2_valid(a: i64, b: i64) -> bool {
    if a < 0 || b < 0 {
        return false;
    }
    b == 0 || a <= i32::MAX as i64 / b
}

/// `stbi__mad3sizes_valid(a, b, c, 0)`
pub fn mad3_valid(a: i64, b: i64, c: i64) -> bool {
    mul2_valid(a, b) && mul2_valid(a * b, c)
}

/// Zeroed buffer, or None where malloc would fail. Every buffer sized from
/// file data goes through this (or `try_reserve`): an allocation failure must
/// make `_LOADIMAGE` return -1 like the C code, not abort the program.
pub fn alloc(n: usize) -> Option<Vec<u8>> {
    let mut v = Vec::new();
    v.try_reserve_exact(n).ok()?;
    v.resize(n, 0);
    Some(v)
}

/// `stbi__convert_format(.., req_comp = 4)`
pub fn to_rgba(mut data: Vec<u8>, comp: usize, n: usize) -> Option<Vec<u8>> {
    if comp == 4 {
        data.truncate(n * 4);
        return Some(data);
    }
    let mut out = Vec::new();
    out.try_reserve_exact(n * 4).ok()?;
    for p in data.chunks_exact(comp).take(n) {
        match comp {
            1 => out.extend_from_slice(&[p[0], p[0], p[0], 255]),
            2 => out.extend_from_slice(&[p[0], p[0], p[0], p[1]]),
            _ => out.extend_from_slice(&[p[0], p[1], p[2], 255]),
        }
    }
    Some(out)
}

/// Every other byte (the high byte of big-endian 16-bit samples).
pub fn high_bytes(raw: &[u8]) -> Option<Vec<u8>> {
    let mut out = alloc(raw.len() / 2)?;
    for (o, p) in out.iter_mut().zip(raw.chunks_exact(2)) {
        *o = p[0];
    }
    Some(out)
}

pub struct Rgba {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

// ------------------------------------------------------------------ BMP ---

pub fn bmp_test(s: &mut Ctx) -> bool {
    let r = (|| {
        if s.get8() != b'B' || s.get8() != b'M' {
            return false;
        }
        s.get32le();
        s.get16le();
        s.get16le();
        s.get32le();
        let sz = s.get32le();
        matches!(sz, 12 | 40 | 56 | 108 | 124)
    })();
    s.rewind();
    r
}

fn high_bit(mut z: u32) -> i32 {
    let mut n = 0;
    if z == 0 {
        return -1;
    }
    if z >= 0x10000 {
        n += 16;
        z >>= 16;
    }
    if z >= 0x00100 {
        n += 8;
        z >>= 8;
    }
    if z >= 0x00010 {
        n += 4;
        z >>= 4;
    }
    if z >= 0x00004 {
        n += 2;
        z >>= 2;
    }
    if z >= 0x00002 {
        n += 1;
    }
    n
}

fn shiftsigned(mut v: u32, shift: i32, bits: i32) -> u32 {
    const MUL: [u32; 9] = [0, 0xff, 0x55, 0x49, 0x11, 0x21, 0x41, 0x81, 0x01];
    const SHIFT: [u32; 9] = [0, 0, 0, 1, 0, 2, 4, 6, 0];
    if shift < 0 {
        v <<= -shift;
    } else {
        v >>= shift;
    }
    v >>= 8 - bits;
    (v * MUL[bits as usize]) >> SHIFT[bits as usize]
}

struct BmpInfo {
    bpp: i32,
    offset: i32,
    hsz: i32,
    mr: u32,
    mg: u32,
    mb: u32,
    ma: u32,
    all_a: u32,
    extra_read: i32,
}

fn bmp_set_mask_defaults(info: &mut BmpInfo, compress: u32) {
    if compress == 0 {
        if info.bpp == 16 {
            info.mr = 31 << 10;
            info.mg = 31 << 5;
            info.mb = 31;
        } else if info.bpp == 32 {
            info.mr = 0xff << 16;
            info.mg = 0xff << 8;
            info.mb = 0xff;
            info.ma = 0xff << 24;
            info.all_a = 0;
        } else {
            info.mr = 0;
            info.mg = 0;
            info.mb = 0;
            info.ma = 0;
        }
    }
}

pub fn bmp_load(s: &mut Ctx) -> Option<Rgba> {
    let mut info = BmpInfo {
        bpp: 0,
        offset: 0,
        hsz: 0,
        mr: 0,
        mg: 0,
        mb: 0,
        ma: 0,
        all_a: 255,
        extra_read: 14,
    };
    // stbi__bmp_parse_header
    if s.get8() != b'B' || s.get8() != b'M' {
        return None;
    }
    s.get32le();
    s.get16le();
    s.get16le();
    info.offset = s.get32le() as i32;
    info.hsz = s.get32le() as i32;
    let hsz = info.hsz;
    if info.offset < 0 || !matches!(hsz, 12 | 40 | 56 | 108 | 124) {
        return None;
    }
    let (img_x, img_y) = if hsz == 12 {
        (s.get16le() as u32, s.get16le() as u32)
    } else {
        (s.get32le(), s.get32le())
    };
    if s.get16le() != 1 {
        return None;
    }
    info.bpp = s.get16le();
    if hsz != 12 {
        let compress = s.get32le();
        if compress == 1 || compress == 2 || compress >= 4 {
            return None;
        }
        if compress == 3 && info.bpp != 16 && info.bpp != 32 {
            return None;
        }
        for _ in 0..5 {
            s.get32le();
        }
        if hsz == 40 || hsz == 56 {
            if hsz == 56 {
                for _ in 0..4 {
                    s.get32le();
                }
            }
            if info.bpp == 16 || info.bpp == 32 {
                if compress == 0 {
                    bmp_set_mask_defaults(&mut info, compress);
                } else {
                    info.mr = s.get32le();
                    info.mg = s.get32le();
                    info.mb = s.get32le();
                    info.extra_read += 12;
                    if info.mr == info.mg && info.mg == info.mb {
                        return None;
                    }
                }
            }
        } else {
            info.mr = s.get32le();
            info.mg = s.get32le();
            info.mb = s.get32le();
            info.ma = s.get32le();
            if compress != 3 {
                bmp_set_mask_defaults(&mut info, compress);
            }
            for _ in 0..13 {
                s.get32le();
            }
            if hsz == 124 {
                for _ in 0..4 {
                    s.get32le();
                }
            }
        }
    }

    // stbi__bmp_load
    let flip_vertically = (img_y as i32) > 0;
    let img_y = (img_y as i32).wrapping_abs() as u32;
    if img_y > MAX_DIMENSIONS || img_x > MAX_DIMENSIONS {
        return None;
    }
    let (mr, mg, mb, ma) = (info.mr, info.mg, info.mb, info.ma);
    let mut all_a = info.all_a;
    let offset = info.offset as i64;
    let extra = info.extra_read as i64;
    let mut psize: i64 = 0;
    if hsz == 12 {
        if info.bpp < 24 {
            psize = (offset - extra - 24) / 3;
        }
    } else if info.bpp < 16 {
        psize = (offset - extra - hsz as i64) >> 2;
    }
    if psize == 0 && offset != s.consumed() {
        return None;
    }
    let (w, h) = (img_x as usize, img_y as usize);
    if !mad3_valid(4, w as i64, h as i64) {
        return None;
    }
    let mut out = alloc(4 * w * h)?;
    let mut z = 0usize;
    if info.bpp < 16 {
        if psize == 0 || psize > 256 {
            return None;
        }
        // Entries past psize are uninitialised stack memory in C; 0 here.
        let mut pal = [[0u8; 4]; 256];
        for e in pal.iter_mut().take(psize.max(0) as usize) {
            e[2] = s.get8();
            e[1] = s.get8();
            e[0] = s.get8();
            if hsz != 12 {
                s.get8();
            }
            e[3] = 255;
        }
        s.skip(offset - extra - hsz as i64 - psize * if hsz == 12 { 3 } else { 4 });
        let width: i64 = match info.bpp {
            1 => (img_x as i64 + 7) >> 3,
            4 => (img_x as i64 + 1) >> 1,
            8 => img_x as i64,
            _ => return None,
        };
        let pad = (-width) & 3;
        let put = |out: &mut [u8], z: &mut usize, c: usize| {
            out[*z..*z + 3].copy_from_slice(&pal[c][..3]);
            out[*z + 3] = 255;
            *z += 4;
        };
        if info.bpp == 1 {
            for _ in 0..h {
                let mut bit_offset = 7i32;
                let mut v = s.get8() as i32;
                for i in 0..w {
                    let color = ((v >> bit_offset) & 1) as usize;
                    put(&mut out, &mut z, color);
                    if i + 1 == w {
                        break;
                    }
                    bit_offset -= 1;
                    if bit_offset < 0 {
                        bit_offset = 7;
                        v = s.get8() as i32;
                    }
                }
                s.skip(pad);
            }
        } else {
            for _ in 0..h {
                let mut i = 0;
                while i < w {
                    let mut v = s.get8() as usize;
                    let mut v2 = 0;
                    if info.bpp == 4 {
                        v2 = v & 15;
                        v >>= 4;
                    }
                    put(&mut out, &mut z, v);
                    if i + 1 == w {
                        break;
                    }
                    let v = if info.bpp == 8 { s.get8() as usize } else { v2 };
                    put(&mut out, &mut z, v);
                    i += 2;
                }
                s.skip(pad);
            }
        }
    } else {
        s.skip(offset - extra - hsz as i64);
        let width: i64 = match info.bpp {
            24 => 3 * img_x as i64,
            16 => 2 * img_x as i64,
            _ => 0,
        };
        let pad = (-width) & 3;
        let easy = if info.bpp == 24 {
            1
        } else if info.bpp == 32
            && mb == 0xff
            && mg == 0xff00
            && mr == 0x00ff0000
            && ma == 0xff000000
        {
            2
        } else {
            0
        };
        let (mut rs, mut gs, mut bs, mut as_, mut rc, mut gc, mut bc, mut ac) =
            (0, 0, 0, 0, 0, 0, 0, 0);
        if easy == 0 {
            if mr == 0 || mg == 0 || mb == 0 {
                return None;
            }
            rs = high_bit(mr) - 7;
            rc = mr.count_ones() as i32;
            gs = high_bit(mg) - 7;
            gc = mg.count_ones() as i32;
            bs = high_bit(mb) - 7;
            bc = mb.count_ones() as i32;
            as_ = high_bit(ma) - 7;
            ac = ma.count_ones() as i32;
            if rc > 8 || gc > 8 || bc > 8 || ac > 8 {
                return None;
            }
        }
        for _ in 0..h {
            if easy != 0 {
                for _ in 0..w {
                    out[z + 2] = s.get8();
                    out[z + 1] = s.get8();
                    out[z] = s.get8();
                    z += 3;
                    let a = if easy == 2 { s.get8() } else { 255 };
                    all_a |= a as u32;
                    out[z] = a;
                    z += 1;
                }
            } else {
                for _ in 0..w {
                    let v = if info.bpp == 16 {
                        s.get16le() as u32
                    } else {
                        s.get32le()
                    };
                    out[z] = shiftsigned(v & mr, rs, rc) as u8;
                    out[z + 1] = shiftsigned(v & mg, gs, gc) as u8;
                    out[z + 2] = shiftsigned(v & mb, bs, bc) as u8;
                    let a = if ma != 0 {
                        shiftsigned(v & ma, as_, ac)
                    } else {
                        255
                    };
                    all_a |= a;
                    out[z + 3] = a as u8;
                    z += 4;
                }
            }
            s.skip(pad);
        }
    }
    if all_a == 0 {
        for p in out.chunks_exact_mut(4) {
            p[3] = 255;
        }
    }
    if flip_vertically {
        flip_rows(&mut out, w * 4, h);
    }
    Some(Rgba {
        w: img_x,
        h: img_y,
        px: out,
    })
}

fn flip_rows(px: &mut [u8], stride: usize, h: usize) {
    for j in 0..h / 2 {
        let (top, bottom) = px.split_at_mut((h - 1 - j) * stride);
        top[j * stride..(j + 1) * stride].swap_with_slice(&mut bottom[..stride]);
    }
}

// ------------------------------------------------------------------ TGA ---

/// Returns (components, is_rgb16); 0 components = unsupported.
fn tga_get_comp(bits_per_pixel: i32, is_grey: bool) -> (usize, bool) {
    match bits_per_pixel {
        8 => (1, false),
        16 if is_grey => (2, false),
        15 | 16 => (3, true),
        24 | 32 => (bits_per_pixel as usize / 8, false),
        _ => (0, false),
    }
}

pub fn tga_test(s: &mut Ctx) -> bool {
    let r = (|| {
        s.get8();
        let color_type = s.get8();
        if color_type > 1 {
            return false;
        }
        let sz = s.get8();
        if color_type == 1 {
            if sz != 1 && sz != 9 {
                return false;
            }
            s.skip(4);
            let sz = s.get8();
            if !matches!(sz, 8 | 15 | 16 | 24 | 32) {
                return false;
            }
            s.skip(4);
        } else {
            if !matches!(sz, 2 | 3 | 10 | 11) {
                return false;
            }
            s.skip(9);
        }
        if s.get16le() < 1 || s.get16le() < 1 {
            return false;
        }
        let sz = s.get8();
        if color_type == 1 && sz != 8 && sz != 16 {
            return false;
        }
        matches!(sz, 8 | 15 | 16 | 24 | 32)
    })();
    s.rewind();
    r
}

fn tga_read_rgb16(s: &mut Ctx, out: &mut [u8]) {
    let px = s.get16le() as u16;
    let (r, g, b) = ((px >> 10) & 31, (px >> 5) & 31, px & 31);
    out[0] = (r as u32 * 255 / 31) as u8;
    out[1] = (g as u32 * 255 / 31) as u8;
    out[2] = (b as u32 * 255 / 31) as u8;
}

pub fn tga_load(s: &mut Ctx) -> Option<Rgba> {
    let offset = s.get8() as i64;
    let indexed = s.get8() != 0;
    let mut image_type = s.get8() as i32;
    let palette_start = s.get16le() as i64;
    let palette_len = s.get16le() as usize;
    let palette_bits = s.get8() as i32;
    s.get16le();
    s.get16le();
    let width = s.get16le() as usize;
    let height = s.get16le() as usize;
    let bits_per_pixel = s.get8() as i32;
    let inverted_byte = s.get8() as i32;
    let mut is_rle = false;
    if image_type >= 8 {
        image_type -= 8;
        is_rle = true;
    }
    let inverted = 1 - ((inverted_byte >> 5) & 1) != 0;
    let (comp, rgb16) = if indexed {
        tga_get_comp(palette_bits, false)
    } else {
        tga_get_comp(bits_per_pixel, image_type == 3)
    };
    if comp == 0 || !mad3_valid(width as i64, height as i64, comp as i64) {
        return None;
    }
    let mut data = alloc(width * height * comp)?;
    s.skip(offset);
    let row_len = width * comp;
    if !indexed && !is_rle && !rgb16 {
        for i in 0..height {
            let row = if inverted { height - i - 1 } else { i };
            // A short read leaves the row as it was (uninitialised in C, zero here).
            s.getn(&mut data[row * row_len..(row + 1) * row_len]);
        }
    } else {
        let mut palette = Vec::new();
        if indexed {
            if palette_len == 0 {
                return None;
            }
            s.skip(palette_start);
            palette = alloc(palette_len * comp)?;
            if rgb16 {
                for e in palette.chunks_exact_mut(comp) {
                    tga_read_rgb16(s, e);
                }
            } else if !s.getn(&mut palette) {
                return None;
            }
        }
        let mut raw = [0u8; 4];
        let (mut rle_count, mut rle_repeating, mut read_next) = (0i32, false, true);
        for i in 0..width * height {
            if is_rle {
                if rle_count == 0 {
                    let cmd = s.get8() as i32;
                    rle_count = 1 + (cmd & 127);
                    rle_repeating = cmd >> 7 != 0;
                    read_next = true;
                } else if !rle_repeating {
                    read_next = true;
                }
            } else {
                read_next = true;
            }
            if read_next {
                if indexed {
                    let mut idx = if bits_per_pixel == 8 {
                        s.get8() as usize
                    } else {
                        s.get16le() as usize
                    };
                    if idx >= palette_len {
                        idx = 0;
                    }
                    raw[..comp].copy_from_slice(&palette[idx * comp..idx * comp + comp]);
                } else if rgb16 {
                    tga_read_rgb16(s, &mut raw);
                } else {
                    for b in raw.iter_mut().take(comp) {
                        *b = s.get8();
                    }
                }
                read_next = false;
            }
            data[i * comp..i * comp + comp].copy_from_slice(&raw[..comp]);
            rle_count -= 1;
        }
        if inverted {
            flip_rows(&mut data, row_len, height);
        }
    }
    if comp >= 3 && !rgb16 {
        for p in data.chunks_exact_mut(comp) {
            p.swap(0, 2);
        }
    }
    Some(Rgba {
        w: width as u32,
        h: height as u32,
        px: to_rgba(data, comp, width * height)?,
    })
}

// ------------------------------------------------------------------ GIF ---

pub fn gif_test(s: &mut Ctx) -> bool {
    let r = (|| {
        if s.get8() != b'G' || s.get8() != b'I' || s.get8() != b'F' || s.get8() != b'8' {
            return false;
        }
        let sz = s.get8();
        (sz == b'9' || sz == b'7') && s.get8() == b'a'
    })();
    s.rewind();
    r
}

#[derive(Clone, Copy, Default)]
struct Lzw {
    prefix: i16,
    first: u8,
    suffix: u8,
}

struct Gif {
    w: i32,
    h: i32,
    out: Vec<u8>,
    history: Vec<u8>,
    flags: u8,
    bgindex: u8,
    pal: [[u8; 4]; 256],
    lpal: [[u8; 4]; 256],
    use_lpal: bool,
    codes: Vec<Lzw>,
    parse: i32,
    step: i32,
    lflags: u8,
    start_x: i32,
    start_y: i32,
    max_x: i32,
    max_y: i32,
    cur_x: i32,
    cur_y: i32,
    line_size: i32,
    eflags: u8,
    transparent: i32,
}

fn gif_parse_colortable(s: &mut Ctx, pal: &mut [[u8; 4]; 256], num_entries: usize, transp: i32) {
    for (i, e) in pal.iter_mut().enumerate().take(num_entries) {
        e[2] = s.get8();
        e[1] = s.get8();
        e[0] = s.get8();
        e[3] = if transp == i as i32 { 0 } else { 255 };
    }
}

impl Gif {
    fn out_code(&mut self, code: u16) {
        // stbi__out_gif_code recurses into the prefix first; walk the chain.
        let mut chain = Vec::new();
        let mut c = code as i32;
        loop {
            chain.push(c as usize);
            let p = self.codes[c as usize].prefix;
            if p < 0 {
                break;
            }
            c = p as i32;
        }
        for &c in chain.iter().rev() {
            if self.cur_y >= self.max_y {
                continue;
            }
            let idx = (self.cur_x + self.cur_y) as usize;
            self.history[idx / 4] = 1;
            let s = self.codes[c].suffix as usize;
            let col = if self.use_lpal {
                self.lpal[s]
            } else {
                self.pal[s]
            };
            if col[3] > 128 {
                self.out[idx] = col[2];
                self.out[idx + 1] = col[1];
                self.out[idx + 2] = col[0];
                self.out[idx + 3] = col[3];
            }
            self.cur_x += 4;
            if self.cur_x >= self.max_x {
                self.cur_x = self.start_x;
                self.cur_y += self.step;
                while self.cur_y >= self.max_y && self.parse > 0 {
                    self.step = (1 << self.parse) * self.line_size;
                    self.cur_y = self.start_y + (self.step >> 1);
                    self.parse -= 1;
                }
            }
        }
    }

    fn process_raster(&mut self, s: &mut Ctx) -> bool {
        let lzw_cs = s.get8();
        if lzw_cs > 12 {
            return false;
        }
        let clear: i32 = 1 << lzw_cs;
        let mut first = true;
        let mut codesize = lzw_cs as i32 + 1;
        let mut codemask = (1 << codesize) - 1;
        let (mut bits, mut valid_bits) = (0i32, 0i32);
        for init in 0..clear as usize {
            self.codes[init] = Lzw {
                prefix: -1,
                first: init as u8,
                suffix: init as u8,
            };
        }
        let mut avail = clear + 2;
        let mut oldcode: i32 = -1;
        let mut len: i32 = 0;
        loop {
            if valid_bits < codesize {
                if len == 0 {
                    len = s.get8() as i32;
                    if len == 0 {
                        return true;
                    }
                }
                len -= 1;
                bits |= (s.get8() as i32) << valid_bits;
                valid_bits += 8;
            } else {
                let code = bits & codemask;
                bits >>= codesize;
                valid_bits -= codesize;
                if code == clear {
                    codesize = lzw_cs as i32 + 1;
                    codemask = (1 << codesize) - 1;
                    avail = clear + 2;
                    oldcode = -1;
                    first = false;
                } else if code == clear + 1 {
                    s.skip(len as i64);
                    loop {
                        let l = s.get8();
                        if l == 0 {
                            break;
                        }
                        s.skip(l as i64);
                    }
                    return true;
                } else if code <= avail {
                    if first {
                        return false;
                    }
                    if oldcode >= 0 {
                        let pi = avail as usize;
                        avail += 1;
                        if avail > 8192 {
                            return false;
                        }
                        let pfirst = self.codes[oldcode as usize].first;
                        // C assigns p->first before reading codes[code].first,
                        // which is p itself when code == the new entry.
                        let suffix = if code == avail || code as usize == pi {
                            pfirst
                        } else {
                            self.codes[code as usize].first
                        };
                        self.codes[pi] = Lzw {
                            prefix: oldcode as i16,
                            first: pfirst,
                            suffix,
                        };
                    } else if code == avail {
                        return false;
                    }
                    self.out_code(code as u16);
                    if (avail & codemask) == 0 && avail <= 0x0FFF {
                        codesize += 1;
                        codemask = (1 << codesize) - 1;
                    }
                    oldcode = code;
                } else {
                    return false;
                }
            }
        }
    }
}

pub fn gif_load(s: &mut Ctx) -> Option<Rgba> {
    // stbi__gif_header
    if s.get8() != b'G' || s.get8() != b'I' || s.get8() != b'F' || s.get8() != b'8' {
        return None;
    }
    let version = s.get8();
    if (version != b'7' && version != b'9') || s.get8() != b'a' {
        return None;
    }
    let mut g = Gif {
        w: s.get16le(),
        h: s.get16le(),
        out: Vec::new(),
        history: Vec::new(),
        flags: s.get8(),
        bgindex: s.get8(),
        pal: [[0; 4]; 256],
        lpal: [[0; 4]; 256],
        use_lpal: false,
        codes: vec![Lzw::default(); 8192],
        parse: 0,
        step: 0,
        lflags: 0,
        start_x: 0,
        start_y: 0,
        max_x: 0,
        max_y: 0,
        cur_x: 0,
        cur_y: 0,
        line_size: 0,
        eflags: 0,
        transparent: -1,
    };
    s.get8(); // ratio
    if g.w as u32 > MAX_DIMENSIONS || g.h as u32 > MAX_DIMENSIONS {
        return None;
    }
    if g.flags & 0x80 != 0 {
        gif_parse_colortable(s, &mut g.pal, 2 << (g.flags & 7), -1);
    }
    // stbi__gif_load_next, first frame
    if !mad3_valid(4, g.w as i64, g.h as i64) {
        return None;
    }
    let pcount = (g.w * g.h) as usize;
    g.out = alloc(4 * pcount)?;
    g.history = alloc(pcount)?;
    loop {
        match s.get8() {
            0x2C => {
                let x = s.get16le();
                let y = s.get16le();
                let w = s.get16le();
                let h = s.get16le();
                if x + w > g.w || y + h > g.h {
                    return None;
                }
                g.line_size = g.w * 4;
                g.start_x = x * 4;
                g.start_y = y * g.line_size;
                g.max_x = g.start_x + w * 4;
                g.max_y = g.start_y + h * g.line_size;
                g.cur_x = g.start_x;
                g.cur_y = g.start_y;
                if w == 0 {
                    g.cur_y = g.max_y;
                }
                g.lflags = s.get8();
                if g.lflags & 0x40 != 0 {
                    g.step = 8 * g.line_size;
                    g.parse = 3;
                } else {
                    g.step = g.line_size;
                    g.parse = 0;
                }
                if g.lflags & 0x80 != 0 {
                    let transp = if g.eflags & 1 != 0 { g.transparent } else { -1 };
                    gif_parse_colortable(s, &mut g.lpal, 2 << (g.lflags & 7), transp);
                    g.use_lpal = true;
                } else if g.flags & 0x80 == 0 {
                    return None;
                }
                if !g.process_raster(s) {
                    return None;
                }
                if g.bgindex > 0 {
                    for pi in 0..pcount {
                        if g.history[pi] == 0 {
                            let bg = g.bgindex as usize;
                            g.pal[bg][3] = 255;
                            // stb copies the palette entry in its B,G,R,A storage order.
                            g.out[pi * 4..pi * 4 + 4].copy_from_slice(&g.pal[bg]);
                        }
                    }
                }
                return Some(Rgba {
                    w: g.w as u32,
                    h: g.h as u32,
                    px: g.out,
                });
            }
            0x21 => {
                let ext = s.get8();
                if ext == 0xF9 {
                    let len = s.get8();
                    if len == 4 {
                        g.eflags = s.get8();
                        s.get16le();
                        if g.transparent >= 0 {
                            g.pal[g.transparent as usize][3] = 255;
                        }
                        if g.eflags & 1 != 0 {
                            g.transparent = s.get8() as i32;
                            g.pal[g.transparent as usize][3] = 0;
                        } else {
                            s.skip(1);
                            g.transparent = -1;
                        }
                    } else {
                        s.skip(len as i64);
                        continue;
                    }
                }
                loop {
                    let len = s.get8();
                    if len == 0 {
                        break;
                    }
                    s.skip(len as i64);
                }
            }
            _ => return None, // 0x3B (end of stream before any image) or unknown
        }
    }
}

// ------------------------------------------------------------------ PNM ---

pub fn pnm_test(s: &mut Ctx) -> bool {
    let p = s.get8();
    let t = s.get8();
    if p != b'P' || (t != b'5' && t != b'6') {
        s.rewind();
        return false;
    }
    true
}

fn pnm_isspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

fn pnm_skip_whitespace(s: &mut Ctx, c: &mut u8) {
    loop {
        while !s.at_eof() && pnm_isspace(*c) {
            *c = s.get8();
        }
        if s.at_eof() || *c != b'#' {
            break;
        }
        while !s.at_eof() && *c != b'\n' && *c != b'\r' {
            *c = s.get8();
        }
    }
}

fn pnm_getinteger(s: &mut Ctx, c: &mut u8) -> i32 {
    let mut value: i32 = 0;
    while !s.at_eof() && c.is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((*c - b'0') as i32);
        *c = s.get8();
    }
    value
}

/// Deviation D-R7-4: 16-bit PNM (maxval > 255) used to read past its buffer in
/// stb_image 2.27 (it converted 8-bit channels then treated them as 16-bit);
/// it now decodes to the high byte of each big-endian sample.
pub fn pnm_load(s: &mut Ctx) -> Option<Rgba> {
    s.rewind();
    let _p = s.get8();
    let t = s.get8();
    let comp = if t == b'6' { 3 } else { 1 };
    let mut c = s.get8();
    pnm_skip_whitespace(s, &mut c);
    let x = pnm_getinteger(s, &mut c);
    pnm_skip_whitespace(s, &mut c);
    let y = pnm_getinteger(s, &mut c);
    pnm_skip_whitespace(s, &mut c);
    let maxv = pnm_getinteger(s, &mut c);
    let bytes = if maxv > 65535 {
        return None;
    } else if maxv > 255 {
        2
    } else {
        1
    };
    if x as u32 > MAX_DIMENSIONS || y as u32 > MAX_DIMENSIONS {
        return None;
    }
    let (w, h) = (x as usize, y as usize);
    if !mad3_valid(comp as i64, w as i64, h as i64) || !mul2_valid((comp * w * h) as i64, bytes) {
        return None;
    }
    let mut raw = alloc(comp * w * h * bytes as usize)?;
    s.getn(&mut raw); // short read: buffer stays as it was (uninitialised in C)
    let samples = if bytes == 2 { high_bytes(&raw)? } else { raw };
    Some(Rgba {
        w: w as u32,
        h: h as u32,
        px: to_rgba(samples, comp, w * h)?,
    })
}

// -------------------------------------------- formats only detected here ---

/// PSD, PIC and Radiance HDR: stb_image decoded them, qbrt does not
/// (deviation D-R7-3). A file that matches their test still stops the search,
/// as in `stbi__load_main`, so it is not retried as TGA.
pub fn unsupported_test(s: &mut Ctx) -> bool {
    let d = s.d;
    let psd = d.starts_with(b"8BPS");
    let pic = d.starts_with(b"\x53\x80\xF6\x34") && d.len() >= 92 && &d[88..92] == b"PICT";
    let hdr = d.starts_with(b"#?RADIANCE\n") || d.starts_with(b"#?RGBE\n");
    psd || pic || hdr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctx_reads_zero_past_end_and_skip_semantics() {
        let mut s = Ctx::new(&[1, 2, 3]);
        assert_eq!(s.get16le(), 0x0201);
        assert_eq!(s.get16le(), 3);
        assert_eq!(s.get8(), 0);
        s.rewind();
        s.skip(-5);
        assert!(s.at_eof());
        let mut buf = [9u8; 2];
        s.rewind();
        s.skip(2);
        assert!(!s.getn(&mut buf));
        assert_eq!(buf, [9, 9]);
        assert_eq!(s.get8(), 3);
    }

    #[test]
    fn shiftsigned_expands_to_full_range() {
        // 5-bit 31 -> 255, 6-bit 63 -> 255, 5-bit 16 -> 132
        assert_eq!(shiftsigned(31 << 10, high_bit(31 << 10) - 7, 5), 255);
        assert_eq!(shiftsigned(63 << 5, high_bit(63 << 5) - 7, 6), 255);
        assert_eq!(shiftsigned(16, high_bit(31) - 7, 5), 132);
        assert_eq!(high_bit(0), -1);
        assert_eq!(high_bit(0x8000_0000), 31);
    }

    #[test]
    fn mad3_valid_matches_stb_overflow_rule() {
        assert!(mad3_valid(4, 16384, 32767));
        assert!(!mad3_valid(4, 16384, 32768));
        assert!(!mad3_valid(4, -1, 1));
        assert!(mad3_valid(4, 0, 1 << 24));
    }

    #[test]
    fn gif_without_image_fails_and_background_is_bgr() {
        // 2x1 screen, global table of 2 entries, bgindex 1, one 1x1 frame at x=0.
        let mut f = b"GIF89a".to_vec();
        f.extend_from_slice(&[2, 0, 1, 0, 0x80, 1, 0]);
        f.extend_from_slice(&[10, 20, 30, 40, 50, 60]);
        let mut no_image = f.clone();
        no_image.push(0x3B);
        assert!(gif_load(&mut Ctx::new(&no_image)).is_none());
        f.extend_from_slice(&[0x2C, 0, 0, 0, 0, 1, 0, 1, 0, 0]);
        // LZW min code size 2: clear(4), 0, end(5) packed in 3-bit codes.
        f.extend_from_slice(&[2, 2, 0x44, 0x01, 0]);
        f.push(0x3B);
        let img = gif_load(&mut Ctx::new(&f)).unwrap();
        assert_eq!((img.w, img.h), (2, 1));
        assert_eq!(img.px, vec![10, 20, 30, 255, 60, 50, 40, 255]);
    }

    #[test]
    fn tga_test_rejects_and_accepts() {
        let mut ok = vec![0u8, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 24, 0];
        assert!(tga_test(&mut Ctx::new(&ok)));
        ok[16] = 12;
        assert!(!tga_test(&mut Ctx::new(&ok)));
    }
}
