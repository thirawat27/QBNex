// Minimal stand-ins for the C++ string type (qbs) and runtime helpers that the
// verbatim legacy code in legacy/*.inc calls. Test-only; include inside namespace legacy.
struct qbs_field {
    int32 fileno;
    int64 fileid;
    int64 size;
    int64 offset;
};
struct qbs {
    uint8 *chr;
    int32 len;
    uint8 fixed;
    qbs_field *field;
};
static qbs *qbs_new(int32 len, uint8 tmp) {
    qbs *s = new qbs();
    s->chr = (uint8 *)calloc(len + 1, 1);
    s->len = len;
    return s;
}
static qbs *qbs_new_txt_len(const char *t, int32 len) {
    qbs *s = qbs_new(len, 1);
    memcpy(s->chr, t, len);
    return s;
}
static qbs *qbs_new_txt(const char *t) { return qbs_new_txt_len(t, (int32)strlen(t)); }
static qbs *qbs_set(qbs *d, qbs *s) {
    if (d->fixed) {
        int32 n = s->len < d->len ? s->len : d->len;
        memcpy(d->chr, s->chr, n);
        memset(d->chr + n, 32, d->len - n);
        return d;
    }
    uint8 *c = (uint8 *)calloc(s->len + 1, 1);
    memcpy(c, s->chr, s->len);
    d->chr = c;
    d->len = s->len;
    return d;
}
static void qbs_free(qbs *s) {}
static qbs *qbs_add(qbs *a, qbs *b) {
    qbs *s = qbs_new(a->len + b->len, 1);
    memcpy(s->chr, a->chr, a->len);
    memcpy(s->chr + a->len, b->chr, b->len);
    return s;
}
static qbs *qbs_ucase(qbs *a) {
    qbs *s = qbs_new_txt_len((char *)a->chr, a->len);
    for (int i = 0; i < s->len; i++)
        if (s->chr[i] >= 'a' && s->chr[i] <= 'z') s->chr[i] -= 32;
    return s;
}
static qbs *qbs_left(qbs *a, int32 n) {
    if (n < 0) n = 0;
    if (n > a->len) n = a->len;
    return qbs_new_txt_len((char *)a->chr, n);
}
static qbs *qbs_str(int64 v) {
    char b[32];
    snprintf(b, 32, "% lld", (long long)v);
    return qbs_new_txt(b);
}
static qbs *func_space(int32 n) {
    qbs *s = qbs_new(n < 0 ? 0 : n, 1);
    memset(s->chr, 32, s->len);
    return s;
}
static int32 func_instr(int32 start, qbs *a, qbs *b, int32 passed) {
    for (int32 i = 0; i + b->len <= a->len; i++)
        if (memcmp(a->chr + i, b->chr, b->len) == 0) return i + 1;
    return 0;
}
static int64 qbr(double v) { return (int64)llround(v); }
static qbs *printed = nullptr;
static std::vector<std::string> printed_lines;
static void qbs_print(qbs *s, int32 nl) {
    printed = s;
    printed_lines.push_back(std::string((char *)s->chr, s->len));
}
static qbs *qbs_inkey() { return qbs_new(0, 1); }
static void evnt(int32) {}
static int32 stop_program = 0;
static int32 vwatch = 0;
static void showvalue(int32) {}

struct byte_element_struct {
    uint64 offset;
    int32 length;
};
static void *byte_element(uint64 offset, int32 length, byte_element_struct *info) {
    if (length < 0) length = 0;
    info->length = length;
    info->offset = offset;
    return info;
}
struct special_handle_struct {
    int32 type;
    ptrszint index;
};
struct stream_struct {
    int32 type, in_size, eof;
    uint8 *in;
};
struct list {
    int32 indexes;
};
static list special_handles_v = {0};
static list *special_handles = &special_handles_v;
static void *list_get(list *, int32) { return NULL; }
static void stream_update(stream_struct *) {}
static void stream_out(stream_struct *, void *, int32) {}
static void connection_close(int32) {}
static qbs *qbs_right(qbs *a, int32 n) {
    if (n < 0) n = 0;
    if (n > a->len) n = a->len;
    return qbs_new_txt_len((char *)a->chr + a->len - n, n);
}
static int32 qbs_equal(qbs *a, qbs *b) { return a->len == b->len && memcmp(a->chr, b->chr, a->len) == 0 ? -1 : 0; }
