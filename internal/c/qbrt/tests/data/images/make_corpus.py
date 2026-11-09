#!/usr/bin/env python3
"""Regenerates the image corpus used by qbrt's image tests (R7).

Run from anywhere: python make_corpus.py  (needs Pillow for the JPEG/GIF files;
everything else is written by the small encoders below so that the corpus
covers header variants Pillow cannot produce).

Output is deterministic (fixed seed). All images are synthetic and released
with the repository under its MIT license.
"""
import io
import os
import random
import struct
import zlib

from PIL import Image

OUT = os.path.dirname(os.path.abspath(__file__))
rng = random.Random(0x5EED)


def write(name, data):
    with open(os.path.join(OUT, name), "wb") as f:
        f.write(data)


def pattern_rgba(w, h, alpha=True):
    """Gradient + noise so every channel takes many values."""
    px = []
    for y in range(h):
        for x in range(w):
            r = (x * 255 // max(1, w - 1)) ^ (rng.randrange(4))
            g = (y * 255 // max(1, h - 1))
            b = rng.randrange(256)
            a = rng.choice([0, 255, rng.randrange(256)]) if alpha else 255
            px.append((r & 255, g, b, a))
    return px


# ---------------------------------------------------------------- PNG ------
def png_chunk(t, d):
    c = struct.pack(">I", len(d)) + t + d
    return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)


def png_pack_row(vals, depth):
    if depth == 8:
        return bytes(vals)
    if depth == 16:
        return b"".join(struct.pack(">H", v) for v in vals)
    out, acc, n = bytearray(), 0, 0
    for v in vals:
        acc = (acc << depth) | v
        n += depth
        if n == 8:
            out.append(acc)
            acc, n = 0, 0
    if n:
        out.append(acc << (8 - n))
    return bytes(out)


def png_filter(raw, prev, bpp, ft):
    out = bytearray()
    for i, x in enumerate(raw):
        a = raw[i - bpp] if i >= bpp else 0
        b = prev[i] if prev else 0
        c = prev[i - bpp] if prev and i >= bpp else 0
        if ft == 0:
            p = 0
        elif ft == 1:
            p = a
        elif ft == 2:
            p = b
        elif ft == 3:
            p = (a + b) // 2
        else:
            pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
            p = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
        out.append((x - p) & 255)
    return bytes(out)


def png(name, w, h, ctype, depth, samples, interlace=False, plte=None, trns=None, gama=False):
    """samples: list of per-pixel tuples of channel values (already in range)."""
    chans = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[ctype]
    bpp = max(1, chans * depth // 8)
    passes = [(0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4), (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)] if interlace else [(0, 0, 1, 1)]
    data = bytearray()
    for (x0, y0, dx, dy) in passes:
        prev = None
        for y in range(y0, h, dy):
            vals = []
            for x in range(x0, w, dx):
                vals.extend(samples[y * w + x])
            if not vals:
                continue
            raw = png_pack_row(vals, depth)
            ft = rng.randrange(5)
            data.append(ft)
            data += png_filter(raw, prev, bpp, ft)
            prev = raw
    out = b"\x89PNG\r\n\x1a\n" + png_chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, depth, ctype, 0, 0, 1 if interlace else 0))
    if gama:
        out += png_chunk(b"gAMA", struct.pack(">I", 100000))
    if plte:
        out += png_chunk(b"PLTE", bytes(c for rgb in plte for c in rgb))
    if trns is not None:
        out += png_chunk(b"tRNS", trns)
    comp = zlib.compress(bytes(data), 9)
    half = len(comp) // 2  # split IDAT to exercise chunk concatenation
    out += png_chunk(b"IDAT", comp[:half]) + png_chunk(b"IDAT", comp[half:]) + png_chunk(b"IEND", b"")
    write(name, out)


def make_png():
    w, h = 19, 13
    px = pattern_rgba(w, h)
    png("png_rgba8.png", w, h, 6, 8, px)
    png("png_rgba8_interlaced.png", w, h, 6, 8, px, interlace=True)
    png("png_rgb8.png", w, h, 2, 8, [p[:3] for p in px], gama=True)
    png("png_rgb8_trns.png", w, h, 2, 8, [p[:3] for p in px], trns=struct.pack(">HHH", *px[5][:3]))
    png("png_gray8.png", w, h, 0, 8, [(p[1],) for p in px])
    png("png_gray8_trns.png", w, h, 0, 8, [(p[1],) for p in px], trns=struct.pack(">H", px[w][1]))
    png("png_graya8.png", w, h, 4, 8, [(p[0], p[3]) for p in px])
    for d in (1, 2, 4):
        m = (1 << d) - 1
        png("png_gray%d.png" % d, w, h, 0, d, [((p[2] >> (8 - d)) & m,) for p in px])
        png("png_gray%d_interlaced.png" % d, w, h, 0, d, [((p[2] >> (8 - d)) & m,) for p in px], interlace=True)
        pal = [(rng.randrange(256), rng.randrange(256), rng.randrange(256)) for _ in range(1 << d)]
        png("png_pal%d.png" % d, w, h, 3, d, [((p[0] >> (8 - d)) & m,) for p in px], plte=pal)
    pal = [(rng.randrange(256), rng.randrange(256), rng.randrange(256)) for _ in range(256)]
    png("png_pal8.png", w, h, 3, 8, [(p[2],) for p in px], plte=pal)
    png("png_pal8_trns.png", w, h, 3, 8, [(p[2],) for p in px], plte=pal, trns=bytes(rng.randrange(256) for _ in range(100)))
    png("png_pal8_interlaced.png", w, h, 3, 8, [(p[2],) for p in px], plte=pal, interlace=True)
    s16 = [tuple((c << 8) | rng.randrange(256) for c in p) for p in px]
    png("png_rgba16.png", w, h, 6, 16, s16)
    png("png_rgb16.png", w, h, 2, 16, [p[:3] for p in s16])
    png("png_rgb16_trns.png", w, h, 2, 16, [p[:3] for p in s16], trns=struct.pack(">HHH", *s16[7][:3]))
    png("png_gray16.png", w, h, 0, 16, [(p[0],) for p in s16])
    png("png_gray16_trns.png", w, h, 0, 16, [(p[0],) for p in s16], trns=struct.pack(">H", s16[3][0]))
    png("png_graya16_interlaced.png", w, h, 4, 16, [(p[0], p[3]) for p in s16], interlace=True)
    png("png_1x1.png", 1, 1, 6, 8, [(1, 2, 3, 4)])
    png("png_wide.png", 300, 2, 2, 8, [p[:3] for p in pattern_rgba(300, 2, False)])


# ---------------------------------------------------------------- BMP ------
def bmp(name, w, h, bpp, pixels, palette=None, top_down=False, header=40, masks=None, compression=0, raw=None, colors_used=None):
    """pixels: rows top->bottom of ints (index) or (r,g,b,a)."""
    rows = []
    for y in range(h):
        row = bytearray()
        line = pixels[y * w:(y + 1) * w]
        if bpp < 8:
            acc, n = 0, 0
            for v in line:
                acc = (acc << bpp) | v
                n += bpp
                if n == 8:
                    row.append(acc)
                    acc, n = 0, 0
            if n:
                row.append(acc << (8 - n))
        elif bpp == 8:
            row += bytes(line)
        elif bpp == 16:
            for (r, g, b, a) in line:
                if masks and masks[1] == 0x07E0:
                    v = ((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3)
                else:
                    v = ((r >> 3) << 10) | ((g >> 3) << 5) | (b >> 3) | ((a >> 7) << 15 if masks and masks[3] else 0)
                row += struct.pack("<H", v)
        elif bpp == 24:
            for (r, g, b, a) in line:
                row += bytes((b, g, r))
        else:
            for (r, g, b, a) in line:
                row += bytes((b, g, r, a))
        while len(row) % 4:
            row.append(0)
        rows.append(bytes(row))
    if raw is not None:
        body = raw
    else:
        body = b"".join(rows if top_down else rows[::-1])
    pal = b""
    if palette:
        if header == 12:
            pal = b"".join(bytes((b, g, r)) for (r, g, b) in palette)
        else:
            pal = b"".join(bytes((b, g, r, 0)) for (r, g, b) in palette)
    if header == 12:
        info = struct.pack("<IHHHH", 12, w, h, 1, bpp)
    else:
        info = struct.pack("<IiiHHIIiiII", header, w, -h if top_down else h, 1, bpp, compression, len(body), 2835, 2835,
                           colors_used if colors_used is not None else (len(palette) if palette else 0), 0)
        if header >= 52 or compression == 3:
            m = masks or (0, 0, 0, 0)
            extra = struct.pack("<IIII", *m)
            if header == 40:
                extra = extra[:12]
            info += extra
        if header > 56:
            info += b"\x00" * (header - 56)
    off = 14 + len(info) + len(pal)
    write(name, b"BM" + struct.pack("<IHHI", off + len(body), 0, 0, off) + info + pal + body)


def make_bmp():
    w, h = 21, 11
    px = pattern_rgba(w, h)
    for bpp in (1, 4, 8):
        n = 1 << bpp
        pal = [(rng.randrange(256), rng.randrange(256), rng.randrange(256)) for _ in range(n)]
        idx = [rng.randrange(n) for _ in range(w * h)]
        bmp("bmp_pal%d.bmp" % bpp, w, h, bpp, idx, pal)
        if bpp > 1:
            # stb_image reads 4 palette entries fewer than an OS/2 BMP holds and
            # the rest is uninitialised memory, so only use the entries it reads.
            bmp("bmp_pal%d_os2.bmp" % bpp, w, h, bpp, [i % (n - 4) for i in idx], pal, header=12)
    bmp("bmp_rgb24_os2.bmp", w, h, 24, px, header=12)
    pal = [(rng.randrange(256), rng.randrange(256), rng.randrange(256)) for _ in range(16)]
    bmp("bmp_pal8_short_palette.bmp", w, h, 8, [rng.randrange(16) for _ in range(w * h)], pal, colors_used=16)
    bmp("bmp_rgb24.bmp", w, h, 24, px)
    bmp("bmp_rgb24_topdown.bmp", w, h, 24, px, top_down=True)
    bmp("bmp_rgb32_alpha.bmp", w, h, 32, px)
    bmp("bmp_rgb32_zero_alpha.bmp", w, h, 32, [(r, g, b, 0) for (r, g, b, a) in px])
    bmp("bmp_rgb16_555.bmp", w, h, 16, px)
    bmp("bmp_rgb16_565.bmp", w, h, 16, px, compression=3, masks=(0xF800, 0x07E0, 0x001F, 0))
    bmp("bmp_bitfields32_v4.bmp", w, h, 32, px, header=108, compression=3, masks=(0x00FF0000, 0x0000FF00, 0x000000FF, 0xFF000000))
    bmp("bmp_bitfields32_v5.bmp", w, h, 32, px, header=124, compression=3, masks=(0x00FF0000, 0x0000FF00, 0x000000FF, 0xFF000000))
    bmp("bmp_rgb24_v5.bmp", w, h, 24, px, header=124)
    # RLE8: stb_image rejects compressed BMPs, so both decoders must fail on it.
    rle = bytearray()
    for y in range(4):
        rle += bytes((4, 7, 0, 0))
    rle += bytes((0, 1))
    pal = [(i, i, i) for i in range(256)]
    bmp("bmp_rle8.bmp", 4, 4, 8, [0] * 16, pal, compression=1, raw=bytes(rle))


# ---------------------------------------------------------------- TGA ------
def tga(name, w, h, itype, bpp, pixels, cmap=None, cmap_bpp=24, top=False, rle=False, alpha_bits=None):
    """pixels: rows top->bottom; each item is an int (index/gray) or a tuple."""
    def enc(p):
        if itype in (1, 9, 3, 11):
            if bpp == 16 and itype in (3, 11):
                return bytes((p[0], p[1]))
            return bytes((p,)) if isinstance(p, int) else bytes((p[0],))
        r, g, b, a = p
        if bpp == 15 or bpp == 16:
            v = ((r >> 3) << 10) | ((g >> 3) << 5) | (b >> 3) | (0x8000 if a >= 128 else 0)
            return struct.pack("<H", v)
        if bpp == 24:
            return bytes((b, g, r))
        return bytes((b, g, r, a))

    rows = [pixels[y * w:(y + 1) * w] for y in range(h)]
    if not top:
        rows = rows[::-1]
    body = bytearray()
    for row in rows:
        items = [enc(p) for p in row]
        if not rle:
            body += b"".join(items)
            continue
        i = 0
        while i < len(items):
            j = i + 1
            while j < len(items) and j - i < 128 and items[j] == items[i]:
                j += 1
            if j - i >= 2:
                body.append(0x80 | (j - i - 1))
                body += items[i]
                i = j
            else:
                j = i + 1
                while j < len(items) and j - i < 128 and (j + 1 >= len(items) or items[j] != items[j + 1]):
                    j += 1
                body.append(j - i - 1)
                body += b"".join(items[i:j])
                i = j
    cm = b""
    if cmap:
        for (r, g, b, a) in cmap:
            if cmap_bpp == 24:
                cm += bytes((b, g, r))
            elif cmap_bpp == 32:
                cm += bytes((b, g, r, a))
            else:
                cm += struct.pack("<H", ((r >> 3) << 10) | ((g >> 3) << 5) | (b >> 3))
    desc = (0x20 if top else 0) | (alpha_bits if alpha_bits is not None else (8 if bpp == 32 else (1 if bpp == 16 and itype not in (3, 11) else 0)))
    hdr = struct.pack("<BBBHHBHHHHBB", 0, 1 if cmap else 0, itype, 0, len(cmap) if cmap else 0, cmap_bpp if cmap else 0, 0, 0, w, h, bpp, desc)
    write(name, hdr + cm + bytes(body))


def make_tga():
    w, h = 17, 9
    px = pattern_rgba(w, h)
    # make runs for RLE
    px = [px[(i // 3) * 3] if (i % 5) < 3 else px[i] for i in range(len(px))]
    for rle in (False, True):
        s = "_rle" if rle else ""
        t = 8 if rle else 0
        tga("tga_rgb24%s.tga" % s, w, h, 2 + t, 24, px, rle=rle)
        tga("tga_rgba32%s.tga" % s, w, h, 2 + t, 32, px, rle=rle)
        tga("tga_rgba32_top%s.tga" % s, w, h, 2 + t, 32, px, rle=rle, top=True)
        tga("tga_rgb16%s.tga" % s, w, h, 2 + t, 16, px, rle=rle)
        tga("tga_rgb15%s.tga" % s, w, h, 2 + t, 15, px, rle=rle)
        tga("tga_gray8%s.tga" % s, w, h, 3 + t, 8, [p[1] for p in px], rle=rle)
        tga("tga_graya16%s.tga" % s, w, h, 3 + t, 16, [(p[1], p[3]) for p in px], rle=rle, alpha_bits=8)
        cmap = [(rng.randrange(256), rng.randrange(256), rng.randrange(256), rng.randrange(256)) for _ in range(40)]
        idx = [rng.randrange(40) for _ in range(w * h)]
        idx = [idx[(i // 4) * 4] for i in range(len(idx))]
        tga("tga_cmap24%s.tga" % s, w, h, 1 + t, 8, idx, cmap=cmap, rle=rle)
        tga("tga_cmap32%s.tga" % s, w, h, 1 + t, 8, idx, cmap=cmap, cmap_bpp=32, rle=rle)
        tga("tga_cmap16%s.tga" % s, w, h, 1 + t, 8, idx, cmap=cmap, cmap_bpp=16, rle=rle)
    tga("tga_rgba32_noalphabits.tga", w, h, 2, 32, px, alpha_bits=0)


# ---------------------------------------------------------------- PCX ------
def pcx_rle(data):
    out = bytearray()
    i = 0
    while i < len(data):
        j = i + 1
        while j < len(data) and j - i < 63 and data[j] == data[i]:
            j += 1
        n = j - i
        if n > 1 or data[i] >= 0xC0:
            out += bytes((0xC0 | n, data[i]))
        else:
            out.append(data[i])
        i = j
    return bytes(out)


def pcx(name, w, h, bpp, planes, plane_rows, palette16=None, palette256=None, version=5, xy=(0, 0), swap=False):
    """plane_rows: list over y of list over planes of bytes (bytesPerLine each)."""
    bpl = len(plane_rows[0][0])
    left, top = xy
    right, bottom = left + w - 1, top + h - 1
    if swap:
        left, right, top, bottom = right, left, bottom, top
    p16 = bytes(palette16 or b"\x00" * 48)
    hdr = struct.pack("<BBBBHHHHHH", 10, version, 1, bpp, left, top, right, bottom, 72, 72) + p16[:48].ljust(48, b"\x00")
    hdr += struct.pack("<BBHHHH", 0, planes, bpl, 1, 0, 0) + b"\x00" * 54
    assert len(hdr) == 128
    body = b"".join(pcx_rle(b"".join(r)) for r in plane_rows)
    tail = b""
    if palette256:
        tail = b"\x0c" + bytes(c for rgb in palette256 for c in rgb)
    write(name, hdr + body + tail)


def make_pcx():
    w, h = 23, 7
    rand = lambda n: bytes(rng.randrange(256) for _ in range(n))
    runny = lambda n: bytes([rng.choice((0, 0xC5, 0xFF, rng.randrange(256)))] * 3 + list(rand(n - 3)))[:n]
    bpl8 = w + (w & 1)
    pal = [(rng.randrange(256), rng.randrange(256), rng.randrange(256)) for _ in range(256)]
    pcx("pcx_8bit_pal.pcx", w, h, 8, 1, [[runny(bpl8)] for _ in range(h)], palette256=pal)
    pcx("pcx_8bit_gray.pcx", w, h, 8, 1, [[runny(bpl8)] for _ in range(h)])
    pcx("pcx_8bit_rgb.pcx", w, h, 8, 3, [[runny(bpl8) for _ in range(3)] for _ in range(h)])
    pcx("pcx_8bit_rgba.pcx", w, h, 8, 4, [[runny(bpl8) for _ in range(4)] for _ in range(h)])
    pcx("pcx_8bit_rgb_offset_swapped.pcx", w, h, 8, 3, [[runny(bpl8) for _ in range(3)] for _ in range(h)], xy=(5, 9), swap=True)
    bpl1 = ((w + 7) // 8 + 1) & ~1
    pal16 = rand(48)
    pcx("pcx_1bit_mono.pcx", w, h, 1, 1, [[rand(bpl1)] for _ in range(h)])
    for planes in (2, 3, 4):
        pcx("pcx_1bit_%dplanes.pcx" % planes, w, h, 1, planes, [[rand(bpl1) for _ in range(planes)] for _ in range(h)], palette16=pal16)
    bpl2 = ((w + 3) // 4 + 1) & ~1
    cga = bytes([0x30, 0, 0, 0x60]) + rand(44)
    pcx("pcx_2bit_cga.pcx", w, h, 2, 1, [[rand(bpl2)] for _ in range(h)], palette16=cga)
    pcx("pcx_2bit_cga_v3.pcx", w, h, 2, 1, [[rand(bpl2)] for _ in range(h)], palette16=bytes([0x50, 0, 0, 0x20]) + rand(44), version=3)
    bpl4 = ((w + 1) // 2 + 1) & ~1
    pcx("pcx_4bit_1plane.pcx", w, h, 4, 1, [[rand(bpl4)] for _ in range(h)], palette16=pal16)
    # Truncated file: decoder reads zeros past the end.
    pcx("pcx_8bit_truncated.pcx", 40, 30, 8, 1, [[rand(40)] for _ in range(3)])


# ------------------------------------------------------------ PNM / misc --
def make_pnm():
    w, h = 13, 5
    px = pattern_rgba(w, h, False)
    write("pnm_p6.ppm", b"P6\n# comment\n%d %d\n255\n" % (w, h) + bytes(c for p in px for c in p[:3]))
    write("pnm_p5.pgm", b"P5 %d %d 255 " % (w, h) + bytes(p[1] for p in px))


def make_pillow():
    w, h = 29, 21
    px = pattern_rgba(w, h, False)
    im = Image.new("RGB", (w, h))
    im.putdata([p[:3] for p in px])
    for name, kw in (("jpg_420.jpg", dict(quality=90)), ("jpg_444.jpg", dict(quality=95, subsampling=0)),
                     ("jpg_422.jpg", dict(quality=75, subsampling=1)), ("jpg_progressive.jpg", dict(quality=85, progressive=True)),
                     ("jpg_optimized.jpg", dict(quality=60, optimize=True))):
        b = io.BytesIO()
        im.save(b, "JPEG", **kw)
        write(name, b.getvalue())
    b = io.BytesIO()
    im.convert("L").save(b, "JPEG", quality=88)
    write("jpg_gray.jpg", b.getvalue())
    # GIF: still, transparent, interlaced, animated (first frame is used)
    pim = im.quantize(64)
    b = io.BytesIO()
    pim.save(b, "GIF", interlace=False)
    write("gif_still.gif", b.getvalue())
    b = io.BytesIO()
    pim.save(b, "GIF", interlace=True, transparency=3)
    write("gif_transparent_interlaced.gif", b.getvalue())
    frames = [pim, im.rotate(90).quantize(16), im.transpose(Image.FLIP_LEFT_RIGHT).quantize(32)]
    b = io.BytesIO()
    frames[0].save(b, "GIF", save_all=True, append_images=frames[1:], duration=50, loop=0)
    write("gif_animated.gif", b.getvalue())


def make_garbage():
    write("bad_empty.bin", b"")
    write("bad_random.bin", bytes(rng.randrange(256) for _ in range(333)))
    with open(os.path.join(OUT, "png_rgba8.png"), "rb") as f:
        good = f.read()
    write("bad_png_truncated.png", good[:len(good) // 2])


def make_extra():
    """Added after the first corpus; generated last so earlier files stay byte-identical."""
    # PCX with long RLE runs (up to 63) and run bytes >= 0xC0.
    w, h = 130, 4
    rows = []
    for y in range(h):
        row = bytes([0xC7] * 63 + [y * 40] * 40 + [rng.randrange(256) for _ in range(w - 103)])
        rows.append([row])
    pcx("pcx_8bit_long_runs.pcx", w, h, 8, 1, rows, palette256=[(i, 255 - i, i // 2) for i in range(256)])


if __name__ == "__main__":
    make_png()
    make_bmp()
    make_tga()
    make_pcx()
    make_pnm()
    make_pillow()
    make_garbage()
    make_extra()
    print("corpus written to", OUT)
