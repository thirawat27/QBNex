//----------------------------------------------------------------------------------------------------
//    ___  ___   __ _ _    ___                       _    _ _
//   / _ \| _ ) / /| | |  |_ _|_ __  __ _ __ _ ___  | |  (_) |__ _ _ __ _ _ _ _  _
//  | (_) | _ \/ _ \_  _|  | || '  \/ _` / _` / -_) | |__| | '_ \ '_/ _` | '_| || |
//   \__\_\___/\___/ |_|  |___|_|_|_\__,_\__, \___| |____|_|_.__/_| \__,_|_|  \_, |
//                                       |___/                                |__/
//
//  QBNex Image Library
//  Decoding by the Rust runtime (crates/qb-runtime/src/image, ported from stb_image & dr_pcx)
//
//  Copyright (c) 2022 Samuel Gomes
//  https://github.com/a740g
//
//-----------------------------------------------------------------------------------------------------

#ifndef DEPENDENCY_IMAGE_CODEC
// Stub(s):
int32_t func__loadimage(qbs *f, int32_t bpp, int32_t passed);

#else
// The decoders (dr_pcx 0.3.1 and stb_image 2.27 ports, PNG/JPEG crates) and the 8bpp
// quantizer/palette remapper live in Rust since phase R7: crates/qb-runtime/src/image/,
// declared in qb_runtime.h. Deviations from the old decoders: docs/rust-port/progress.md (D-R7-1..D-R7-5).

/// <summary>
/// This function loads an image into memory and returns valid LONG image handle values that are less than -1.
/// </summary>
/// <param name="f">The filename of the image</param>
/// <param name="bpp">Mode: 32=32bpp, 33=hardware acclerated 32bpp, 256=8bpp or 257=8bpp without palette remap</param>
/// <param name="passed">How many parameters were passed?</param>
/// <returns>Valid LONG image handle values that are less than -1 or -1 on failure</returns>
int32_t func__loadimage(qbs *f, int32_t bpp, int32_t passed)
{
    if (new_error)
        return 0;

    auto isHardware = false;
    auto dontRemapPalette = false;

    // Handle special cases
    if (bpp == 33)
    {
        bpp = 32;
        isHardware = true;
    }
    else if (bpp == 257)
    {
        bpp = 256;
        dontRemapPalette = true;
    }

    // Validate bpp
    if (passed)
    {
        if ((bpp != 32) && (bpp != 256))
        {
            error(5);
            return 0;
        }
    }
    else
    {
        if (write_page->text)
        {
            error(5);
            return 0;
        }
        bpp = -1;
    }
    if (!f->len)
        return -1; // return invalid handle if null length string

    // Load the file
    auto fh = gfs_open(f, 1, 0, 0);
    if (fh < 0)
        return -1;
    auto lof = gfs_lof(fh);
    auto content = (uint8 *)malloc(lof);
    if (!content)
    {
        gfs_close(fh);
        return -1;
    }
    auto result = gfs_read(fh, -1, content, lof);
    gfs_close(fh);
    if (result < 0)
    {
        free(content);
        return -1;
    }

    int32_t x, y;
    // Decode (PCX first, then the stb_image formats); the pixels come back as BGRA
    auto pixels = qb_image_decode(content, lof, &x, &y);

    // Free the memory holding the file
    free(content);

    // Return failure if nothing was able to load the image
    if (!pixels)
        return -1;

    int32_t i; // Image handle to be returned

    // Convert image to 8bpp if requested by the user
    if (bpp == 256)
    {
        i = func__newimage(x, y, 256, 1);
        if (i == -1)
        {
            free(pixels);
            return -1;
        }

        auto palette = (uint32_t *)malloc(256 * sizeof(uint32_t)); // 3 bytes for bgr + 1 for alpha (basically a uint32_t)
        if (!palette)
        {
            free(pixels);
            return -1;
        }

        auto pixels256 = qb_image_to_8bpp(pixels, x, y, palette);
        if (!pixels256)
        {
            free(palette);
            free(pixels);
            return -1;
        }

        if (dontRemapPalette)
        {
            // Copy the 8bpp pixel data and then free it
            memcpy(img[-i].offset, pixels256, x * y);
            free(pixels256);

            // Copy the palette and then free it
            memcpy(img[-i].pal, palette, 256 * sizeof(uint32_t));
            free(palette);
        }
        else
        {
            // Remap the image indexes to QBNex default palette and then free our palette
            qb_image_remap_palette(pixels256, x, y, palette, palette_256);
            free(palette);

            // Copy the 8bpp pixel data and then free it
            memcpy(img[-i].offset, pixels256, x * y);
            free(pixels256);

            // Copy the default QBNex palette
            memcpy(img[-i].pal, palette_256, 256 * sizeof(uint32_t));
        }
    }
    else
    {
        i = func__newimage(x, y, 32, 1);
        if (i == -1)
        {
            free(pixels);
            return -1;
        }
        memcpy(img[-i].offset, pixels, x * y * sizeof(uint32_t));
    }

    // Free pixel memory (qb-runtime allocates it with malloc)
    free(pixels);

    // This only executes if bpp is 32
    if (isHardware)
    {
        auto iHardware = func__copyimage(i, 33, 1);
        sub__freeimage(i, 1);
        i = iHardware;
    }

    return i;
}

#endif
