// _LOADFONT font engine. The FreeType 2.4.9 wrapper that lived here was ported to Rust
// (internal/c/qbrt/src/font.rs, declared in qbrt.h); FreeType is no longer compiled or linked.
// These shims keep the C++ signatures libqb.cpp calls. Behaviour and deviations: see
// docs/rust-port/progress.md (D-R7-6).

#ifndef DEPENDENCY_LOADFONT
//Stubs
int32 FontRenderTextUTF32(int32 i,uint32*codepoint,int32 codepoints,int32 options,
                          uint8**out_data,int32*out_x,int32 *out_y,int32*out_x_pre_increment,int32*out_x_post_increment){return NULL;}
int32 FontLoad (uint8 *content_original,int32 content_bytes,int32 default_pixel_height,int32 which_font,int32 options){return NULL;}
int32 FontRenderTextASCII(int32 i,uint8*codepoint,int32 codepoints,int32 options,
                          uint8**out_data,int32*out_x,int32 *out_y,int32*out_x_pre_increment,int32*out_x_post_increment){return NULL;}
int32 FontWidth(int32 i){return NULL;}
void FontFree(int32 i){return;}
#else

// options: 1=bold, 2=italic, 4=underline, 8=IGNORED, 16=monospace, 32=unicode (only 16 is used here)
int32 FontLoad(uint8 *content_original, int32 content_bytes, int32 default_pixel_height, int32 which_font, int32 options) {
    return qbrt_font_load(content_original, content_bytes, default_pixel_height, which_font, options);
}

// returns success{1}/failure{0}; options: 1=black{0}&white{255}; *out_data is malloc'ed alpha values
int32 FontRenderTextUTF32(int32 i, uint32 *codepoint, int32 codepoints, int32 options, uint8 **out_data, int32 *out_x,
                          int32 *out_y, int32 *out_x_pre_increment, int32 *out_x_post_increment) {
    return qbrt_font_render_utf32(i, codepoint, codepoints, options, out_data, out_x, out_y, out_x_pre_increment,
                                  out_x_post_increment);
}

void FontFree(int32 i) { qbrt_font_free(i); }

int32 FontRenderTextASCII(int32 i, uint8 *codepoint, int32 codepoints, int32 options, uint8 **out_data, int32 *out_x,
                          int32 *out_y, int32 *out_x_pre_increment, int32 *out_x_post_increment) {
    uint32 *utf32_codepoint = NULL;
    if (codepoints >= 1) {
        utf32_codepoint = (uint32 *)malloc(codepoints * 4);
        for (int32 x = 0; x < codepoints; x++) utf32_codepoint[x] = codepage437_to_unicode16[codepoint[x]];
    }
    int32 retval = FontRenderTextUTF32(i, utf32_codepoint, codepoints, options, out_data, out_x, out_y,
                                       out_x_pre_increment, out_x_post_increment);
    if (codepoints > 0) free(utf32_codepoint);
    return retval;
}

int32 FontWidth(int32 i) { return qbrt_font_width(i); }

#endif
