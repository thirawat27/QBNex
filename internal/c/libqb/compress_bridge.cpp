// compress_bridge.cpp — _DEFLATE$ / _INFLATE$ (#included into libqb.cpp under DEPENDENCY_ZLIB).
// R10: the system zlib (-l:libz.a / -lz) is gone. compress()/uncompress() are replaced by
// qb_deflate/qb_inflate (crates/qb-runtime/src/compress.rs, same result codes); the
// buffer handling below is the legacy parts/zlib/src.c code unchanged.
// Deviation D-R10-1: the compressed bytes may differ from zlib's (still a valid zlib stream).

#define QB_Z_BUF_ERROR (-5)

qbs *func__deflate(qbs *text) {
    uint32_t filesize = (uint32)text->len; // length of the text
    uint32_t compsize = qb_deflate_bound(filesize);
    unsigned char *dest = (unsigned char *)malloc(compsize);
    int32 result = qb_deflate(text->chr, filesize, dest, &compsize);
    qbs *ret = qbs_new(compsize, 1);
    memcpy(ret->chr, dest, compsize);
    free(dest);
    return ret;
}

qbs *func__inflate(qbs *text, int64 originalsize, int32 passed) {
    int32 result = 0;
    if (passed == 1) {
        uint32_t uncompsize = originalsize;
        unsigned char *dest = (unsigned char *)malloc(originalsize);
        int32 result = qb_inflate(text->chr, text->len, dest, &uncompsize);
        qbs *ret = qbs_new(uncompsize, 1);
        memcpy(ret->chr, dest, uncompsize);
        free(dest);
        return ret;
    } else {
        uint32_t uncompsize = 0;
        unsigned char *dest;
        do {
            uncompsize = uncompsize + 10000000; // 10 mb original buffer, resized by 10 mb each pass until it's large enough to hold the uncompressed data.
            dest = (unsigned char *)malloc(uncompsize);
            result = qb_inflate(text->chr, text->len, dest, &uncompsize);
            if (result == QB_Z_BUF_ERROR) free(dest); // if the buffer is too small, free the old buffer
        } while (result == QB_Z_BUF_ERROR);           // and try again with a larger buffer
        qbs *ret = qbs_new(uncompsize, 1);
        memcpy(ret->chr, dest, uncompsize);
        free(dest);
        return ret;
    }
}
