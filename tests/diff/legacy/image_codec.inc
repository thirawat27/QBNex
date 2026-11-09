// Verbatim copy of the legacy decode helpers from internal/c/parts/video/image/src.c (commit 4aa6f1f, lines 21-201).
// Test-only reference oracle for diff_image.cpp; needs dr_pcx.h and stb_image.h, which run_diff extracts from git
// history (they are no longer in the tree). Production uses crates/qb-runtime/src/image/.
#define DR_PCX_IMPLEMENTATION
#include "dr_pcx.h"
#define STB_IMAGE_IMPLEMENTATION
#include "stb_image.h"

// The byte ordering here are straight from libqb.cpp. So, if libqb.cpp is wrong, then we are wrong! ;)
#define IMAGE_GET_BGRA_RED(c) (uint32_t(c) >> 16 & 0xFF)
#define IMAGE_GET_BGRA_GREEN(c) (uint32_t(c) >> 8 & 0xFF)
#define IMAGE_GET_BGRA_BLUE(c) (uint32_t(c) & 0xFF)
#define IMAGE_GET_BGRA_ALPHA(c) (uint32_t(c) >> 24)
#define IMAGE_MAKE_BGRA(r, g, b, a) (uint32_t((uint8_t(b) | (uint16_t(uint8_t(g)) << 8)) | (uint32_t(uint8_t(r)) << 16) | (uint32_t(uint8_t(a)) << 24)))
// Calculates the RGB distance in the RGB color cube
#define IMAGE_CALCULATE_RGB_DISTANCE(r1, g1, b1, r2, g2, b2)                                                         \
    sqrt(((float(r2) - float(r1)) * (float(r2) - float(r1))) + ((float(g2) - float(g1)) * (float(g2) - float(g1))) + \
         ((float(b2) - float(b1)) * (float(b2) - float(b1))))

/// <summary>
/// Decodes a PCX image using the dr_pcx library.
/// </summary>
/// <param name="content">A pointer to the file in memory</param>
/// <param name="bytes">The length of the file</param>
/// <param name="result">Out: bit 1=Success, bit 2=32bit. This cannot be NULL[BGRA]</param>
/// <param name="x">Out: width in pixels. This cannot be NULL</param>
/// <param name="y">Out: height in pixels. This cannot be NULL</param>
/// <returns>A pointer to the raw pixel data in RGBA format or NULL on failure</returns>
static uint8_t *image_decode_drpcx(uint8_t *content, int32_t bytes, int32_t *result, int32_t *x, int32_t *y)
{
    auto h = 0, w = 0, comp = 0;
    *result = 0;

    auto out = drpcx_load_memory(content, bytes, DRPCX_FALSE, &w, &h, &comp, 4);
    if (!out)
        return nullptr;

    *result = 1 + 2;
    *x = w;
    *y = h;
    return out;
}

/// <summary>
/// Decodes an image using the stb_image library.
/// </summary>
/// <param name="content">A pointer to the file in memory</param>
/// <param name="bytes">The length of the file</param>
/// <param name="result">Out: bit 1=Success, bit 2=32bit[BGRA]. This cannot be NULL</param>
/// <param name="x">Out: width in pixels. This cannot be NULL</param>
/// <param name="y">Out: height in pixels. This cannot be NULL</param>
/// <returns>A pointer to the raw pixel data in RGBA format or NULL on failure</returns>
static uint8_t *image_decode_stbi(uint8_t *content, int32_t bytes, int32_t *result, int32_t *x, int32_t *y)
{
    auto h = 0, w = 0, comp = 0;
    *result = 0;

    auto out = stbi_load_from_memory(content, bytes, &w, &h, &comp, 4);
    if (!out)
        return nullptr;

    *result = 1 + 2;
    *x = w;
    *y = h;
    return out;
}

/// <summary>
/// Clamps a color channel to the range 0 - 255.
/// </summary>
/// <param name="n">The color component</param>
/// <returns>The clamped value</returns>
static inline uint8_t image_clamp_component(int32_t n)
{
    n &= -(n >= 0);
    return n | ((255 - n) >> 31);
}

/// <summary>
/// This takes in a 32bpp (BGRA) image raw data and spits out an 8bpp raw image along with it's 256 color (BGRA) palette.
/// </summary>
/// <param name="src">The source raw image data. This must be in BGRA format and not NULL</param>
/// <param name="w">The widht of the image in pixels</param>
/// <param name="h">The height of the image in pixels</param>
/// <param name="paletteOut">A 256 color palette if the operation was successful. This cannot be NULL</param>
/// <returns>A pointer to a 8bpp raw image or NULL if operation failed</returns>
static uint8_t *image_convert_8bpp(uint8_t *src, int32_t w, int32_t h, uint32_t *paletteOut)
{
    static struct
    {
        uint32_t r, g, b;
        uint32_t count;
    } cubes[256];

    // https://en.wikipedia.org/wiki/Ordered_dithering
    static uint8_t bayerMatrix[16] = {0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5};

    // Allocate memory for new image (8-bit indexed)
    auto pixels = (uint8_t *)malloc(w * h);
    if (!pixels)
    {
        return nullptr;
    }

    memset(cubes, NULL, sizeof(cubes));

    // Quantization phase
    auto dst = pixels;
    for (auto y = 0; y < h; y++)
    {
        for (auto x = 0; x < w; x++)
        {
            int32_t t = bayerMatrix[((y & 3) << 2) + (x & 3)];
            int32_t b = image_clamp_component((*src++) + (t << 1));
            int32_t g = image_clamp_component((*src++) + (t << 1));
            int32_t r = image_clamp_component((*src++) + (t << 1));
            ++src; // Ignore alpha

            // Quantize
            uint8_t k = ((r >> 5) << 5) + ((g >> 5) << 2) + (b >> 6);
            (*dst++) = k;

            // Prepare RGB cubes for CLUT
            cubes[k].r += r;
            cubes[k].g += g;
            cubes[k].b += b;
            cubes[k].count++;
        }
    }

    // Generate a uniform CLUT based on the quantized colors
    for (auto i = 0; i < 256; i++)
    {
        if (cubes[i].count)
        {
            paletteOut[i] = IMAGE_MAKE_BGRA(cubes[i].r / cubes[i].count, cubes[i].g / cubes[i].count, cubes[i].b / cubes[i].count, 0xFF);
        }
        else
        {
            paletteOut[i] = IMAGE_MAKE_BGRA(0, 0, 0, 0xFF);
        }
    }

    return pixels;
}

/// <summary>
/// This modifies an *8bpp* image 'src' to use 'dst_pal' instead of 'src_pal'
/// </summary>
/// <param name="src">A pointer to the 8bpp image pixel data. This modifies data 'src' points to and cannot be NULL</param>
/// <param name="w">The width of the image in pixels</param>
/// <param name="h">The height of the image in pixels</param>
/// <param name="src_pal">The image's original palette. This cannot be NULL</param>
/// <param name="dst_pal">The destination palette. This cannot be NULL</param>
static void image_remap_palette(uint8_t *src, int32_t w, int32_t h, uint32_t *src_pal, uint32_t *dst_pal)
{
    static uint32_t palMap[256];

    memset(palMap, NULL, sizeof(palMap));

    // Match the palette
    for (auto x = 0; x < 256; x++)
    {
        auto oldDist = IMAGE_CALCULATE_RGB_DISTANCE(0, 0, 0, 255, 255, 255); // The farthest we can go in the color cube
        for (auto y = 0; y < 256; y++)
        {
            auto newDist = IMAGE_CALCULATE_RGB_DISTANCE(IMAGE_GET_BGRA_RED(src_pal[x]), IMAGE_GET_BGRA_GREEN(src_pal[x]), IMAGE_GET_BGRA_BLUE(src_pal[x]),
                                                        IMAGE_GET_BGRA_RED(dst_pal[y]), IMAGE_GET_BGRA_GREEN(dst_pal[y]), IMAGE_GET_BGRA_BLUE(dst_pal[y]));

            if (oldDist > newDist)
            {
                oldDist = newDist;
                palMap[x] = y;
            }
        }
    }

    // Update the bitmap to use the matched palette
    for (auto c = 0; c < (w * h); c++)
    {
        src[c] = palMap[src[c]];
    }
}

