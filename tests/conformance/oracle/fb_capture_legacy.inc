// Framebuffer capture hook for the LEGACY oracle build (test-only).
//
// Copied into a clone of the legacy tree by tests/conformance/oracle/make_fb_oracle.sh,
// which also inserts the two calls below. The Rust runtime implements the same
// format in crates/runtime/core/src/gfx/capture.rs; both must stay byte-identical.
//
// When the environment variable QBNEX_FB_CAPTURE names a file, every _DISPLAY
// call (kind 0) and the end of the program (kind 1, graphics pages only) append
// one record describing the display page:
//   u32 'QBFB' (0x42465142), u32 kind, u32 width, u32 height,
//   u32 bytes_per_pixel, u32 compatible_mode,
//   width*height*bytes_per_pixel bytes of pixels (text pages: char/attr pairs),
//   u32 palette entries (0 or 256), then that many u32 palette values.
// All integers little-endian.
static void qbnex_fb_capture(int32 kind){
    static int32 state=-1;
    static FILE *f=NULL;
    if (state==-1){
        const char *p=getenv("QBNEX_FB_CAPTURE");
        state=0;
        if (p&&*p){ f=fopen(p,"wb"); if (f) state=1; }
    }
    if (state!=1) return;
    img_struct *im=display_page;
    if (!im) return;
    if (kind==1&&im->text) return;
    uint32 hdr[6]={0x42465142u,(uint32)kind,(uint32)im->width,(uint32)im->height,
                   (uint32)im->bytes_per_pixel,(uint32)im->compatible_mode};
    fwrite(hdr,4,6,f);
    fwrite(im->offset,1,(size_t)im->width*im->height*im->bytes_per_pixel,f);
    uint32 npal=((im->bytes_per_pixel==4)||(!im->pal))?0:256;
    fwrite(&npal,4,1,f);
    if (npal) fwrite(im->pal,4,256,f);
    fflush(f);
}
