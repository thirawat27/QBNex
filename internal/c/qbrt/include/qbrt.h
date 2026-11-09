// qbrt.h — C ABI of the Rust runtime (internal/c/qbrt).
//
// Every function declared here is implemented in Rust and replaces code that
// used to live in libqb.cpp. Keep this file in sync with the #[no_mangle]
// exports in internal/c/qbrt/src and bump QBRT_ABI_VERSION on breaking changes.
#ifndef QBNEX_QBRT_H
#define QBNEX_QBRT_H

#include <stdint.h>

#define QBRT_ABI_VERSION 1u

#ifdef __cplusplus
extern "C" {
#endif

uint32_t qbrt_abi_version(void);

// --- MBF <-> IEEE (was internal/c/msbin.cpp) -------------------------------
// Return 0 on success, 1 on overflow. Pointers may be unaligned.
int32_t _fmsbintoieee(float *src4, float *dest4);
int32_t _fieeetomsbin(float *src4, float *dest4);
int32_t _dmsbintoieee(double *src8, double *dest8);
int32_t _dieeetomsbin(double *src8, double *dest8);

// --- QBasic rounding (was qbr* in libqb.cpp, x87 fistp) ---------------------
// *_f80 take a pointer to the 10 bytes of an x87 long double (may be unaligned).
// *_f64 are used where long double is not the x87 80-bit format.
int64_t qbrt_qbr_f80(const unsigned char *f80);
int64_t qbrt_fistp_i64_f80(const unsigned char *f80);
int64_t qbrt_qbr_f64(double v);
int64_t qbrt_fistp_i64_f64(double v);
int32_t qbrt_fistp_i32(double v);
void qbrt_fpu_reinit(void);

// --- RND / RANDOMIZE (was func_rnd / sub_randomize / rnd_seed) --------------
float qbrt_rnd(float n, int32_t passed);
void qbrt_randomize(double seed, int32_t use_using);
void qbrt_randomize_prompt(int16_t seed);
void qbrt_rnd_reset(void);

// --- VAL (was func_val) -------------------------------------------------------
// Writes the 10-byte x87 extended result to out_f80; returns 0 or a QBasic error code.
int32_t qbrt_val(const unsigned char *chr, int32_t len, unsigned char *out_f80);
double qbrt_f80_to_f64(const unsigned char *f80);

// --- STR$ (was the qbs_str overloads) -----------------------------------------
// Write the text (no terminator) into out[QBRT_STR_MAX] and return its length.
#define QBRT_STR_MAX 64
int32_t qbrt_str_i64(int64_t v, unsigned char *out);
int32_t qbrt_str_u64(uint64_t v, unsigned char *out);
int32_t qbrt_str_f32(float v, unsigned char *out);
int32_t qbrt_str_f64(double v, unsigned char *out);

// --- TIMER (was the arithmetic of func_timer) ---------------------------------
// Writes the value to *out and returns 0, or returns a QBasic error code.
int32_t qbrt_timer(uint32_t ticks, uint32_t clock_first, uint32_t day_first,
                   double accuracy, int32_t passed, double *out);

// --- String engine (was qbs_* in libqb.cpp, qbs_cleanup in qbx.cpp) ------------
// struct qbs is defined in internal/c/common.h; generated code uses its fields.
// The engine also exports the globals qbs_tmp_list_nexti, qbs_cmem_sp and
// qbs_cmem_descriptor_space (uint32; declared where they are used, not here, to
// keep their existing C++ declarations valid). It needs from the host:
//   uint8 cmem[]; uint32 cmem_sp; ptrszint dblock;
//   void qbrt_host_error(int32_t); void qbrt_host_field_free(struct qbs*);
struct qbs;
struct qbs *qbrt_qbs_new_descriptor(void);
void qbrt_qbs_free_descriptor(struct qbs *s);
void qbrt_qbs_free(struct qbs *s);
void qbrt_qbs_cmem_concat_list(void);
void qbrt_qbs_concat_list(void);
void qbrt_qbs_tmp_concat_list(void);
void qbrt_qbs_concat(uint32_t bytesrequired);
void qbrt_qbs_concat_cmem(uint32_t bytesrequired);
struct qbs *qbrt_qbs_new_cmem(int32_t size, uint8_t tmp);
struct qbs *qbrt_qbs_new_txt(const char *txt);
struct qbs *qbrt_qbs_new_txt_len(const char *txt, int32_t len);
struct qbs *qbrt_qbs_new_fixed(uint8_t *offset, uint32_t size, uint8_t tmp);
struct qbs *qbrt_qbs_new(int32_t size, uint8_t tmp);
void qbrt_qbs_maketmp(struct qbs *s);
struct qbs *qbrt_qbs_set(struct qbs *dest, struct qbs *src);
void qbrt_qbs_cleanup(uint32_t base); // slow path; callers test qbs_tmp_list_nexti==base first
struct qbs *qbrt_qbs_add(struct qbs *a, struct qbs *b);
struct qbs *qbrt_qbs_ucase(struct qbs *s);
struct qbs *qbrt_qbs_lcase(struct qbs *s);
struct qbs *qbrt_func_chr(int32_t value);
struct qbs *qbrt_func_varptr_helper(uint8_t type, uint16_t offset);
struct qbs *qbrt_qbs_left(struct qbs *s, int32_t l);
struct qbs *qbrt_qbs_right(struct qbs *s, int32_t l);
struct qbs *qbrt_func_space(int32_t spaces);
struct qbs *qbrt_func_string(int32_t characters, int32_t asciivalue);
void qbrt_set_qbs_size(intptr_t *target_qbs, int32_t newlength);
int32_t qbrt_func_instr(int32_t start, struct qbs *str, struct qbs *substr, int32_t passed);
int32_t qbrt_func__instrrev(int32_t start, struct qbs *str, struct qbs *substr, int32_t passed);
// MID$ statement without the new_error / nothingstring checks (done by the shim).
void qbrt_sub_mid(struct qbs *dest, int32_t start, int32_t l, struct qbs *src, int32_t passed);
struct qbs *qbrt_func_mid(struct qbs *str, int32_t start, int32_t l, int32_t passed);
struct qbs *qbrt_qbs_ltrim(struct qbs *s);
struct qbs *qbrt_qbs_rtrim(struct qbs *s);
struct qbs *qbrt_qbs__trim(struct qbs *s);
int32_t qbrt_func__str_nc_compare(struct qbs *a, struct qbs *b);
int32_t qbrt_func__str_compare(struct qbs *a, struct qbs *b);
int32_t qbrt_qbs_equal(struct qbs *a, struct qbs *b);
int32_t qbrt_qbs_notequal(struct qbs *a, struct qbs *b);
int32_t qbrt_qbs_greaterthan(struct qbs *a, struct qbs *b);
int32_t qbrt_qbs_lessthan(struct qbs *a, struct qbs *b);
int32_t qbrt_qbs_lessorequal(struct qbs *a, struct qbs *b);
int32_t qbrt_qbs_greaterorequal(struct qbs *a, struct qbs *b);
int32_t qbrt_qbs_asc(struct qbs *s, uint32_t i);
int32_t qbrt_qbs_asc1(struct qbs *s);
int32_t qbrt_qbs_len(struct qbs *s);
// Introspection (tests / differential harness only).
typedef struct QbrtQbsDebugState {
    const intptr_t *list;
    uint32_t list_nexti, list_lasti;
    const intptr_t *cmem_list;
    uint32_t cmem_list_nexti, cmem_list_lasti;
    const intptr_t *tmp_list;
    uint32_t tmp_list_lasti;
    const uint8_t *data;
    uint32_t data_size, sp;
    struct qbs *const *freed;
    uint32_t freed_count, blocks, malloc_next;
} QbrtQbsDebugState;
void qbrt_qbs_debug_state(QbrtQbsDebugState *out);
int64_t qbrt_qbs_debug_descriptor_index(const struct qbs *s);
void qbrt_qbs_reset(void);
// ============================================================================
// R4: files, file system, environment, SHELL, _MEM
// Functions return 0 or a QBasic error code that the C++ shim raises.
// ============================================================================

// --- _MEM (was new_mem_lock / free_mem_lock / func__mem* / sub__mem*) --------
// Same layout as mem_block / mem_lock in common.h (generated code uses them).
typedef struct qbrt_mem_block {
    intptr_t offset, size;
    int64_t lock_id;
    intptr_t lock_offset, type, elementsize;
    int32_t image, sound;
} qbrt_mem_block;
typedef struct qbrt_mem_lock {
    uint64_t id;
    int32_t type;
    void *offset;
} qbrt_mem_lock;
// Defined in Rust; generated code reads and writes them directly.
extern uint64_t mem_lock_id;
extern qbrt_mem_lock *mem_lock_tmp;
extern qbrt_mem_lock *mem_lock_base;
qbrt_mem_lock *qbrt_new_mem_lock(void); // also sets mem_lock_tmp
void qbrt_free_mem_lock(qbrt_mem_lock *lock);
qbrt_mem_lock *qbrt_mem_lock_invalid(void);
int32_t qbrt_memfree(qbrt_mem_block *blk);
int32_t qbrt_mem_at_offset(intptr_t offset, intptr_t size, int32_t new_error, qbrt_mem_block *out);
int32_t qbrt_memnew(intptr_t bytes, int32_t new_error, qbrt_mem_block *out);
int32_t qbrt_memexists(const qbrt_mem_block *blk);
void *qbrt_memget(const qbrt_mem_block *blk, intptr_t off, intptr_t bytes, int32_t *err);
int32_t qbrt_memfill(const qbrt_mem_block *blk, intptr_t doff, intptr_t dbytes, const uint8_t *src, intptr_t sbytes);
void qbrt_memfill_nochecks(intptr_t doff, intptr_t dbytes, const uint8_t *src, intptr_t sbytes);
int32_t qbrt_memcopy(const qbrt_mem_block *sblk, intptr_t soff, intptr_t bytes, const qbrt_mem_block *dblk,
                     intptr_t doff);
void qbrt_mem(intptr_t offset, intptr_t size, int32_t type, intptr_t elementsize, qbrt_mem_lock *lock,
              qbrt_mem_block *out);

// --- GFS file table (was gfs_* in libqb.cpp) ---------------------------------
// Entry of the Rust-owned file table; same fields as the legacy
// gfs_file_struct minus the fstream pointers.
typedef struct qbrt_gfs_file {
    int64_t id;
    uint8_t open, read, write, lock_read, lock_write;
    int64_t pos;
    uint8_t eof_reached, eof_passed;
    int32_t fileno;
    uint8_t type; // 1 RANDOM, 2 BINARY, 3 INPUT, 4 OUTPUT/APPEND
    int64_t record_length;
    uint8_t *field_buffer;
    void **field_strings; // qbs* linked by FIELD
    int32_t field_strings_n;
    int64_t column;
    uint8_t com_port;
    int32_t com_baud_rate;
    int8_t com_parity, com_data_bits_per_byte, com_stop_bits, com_bin_asc, com_asc_lf, com_rs;
    int32_t com_cd_x, com_cs_x, com_ds_x, com_op_x;
    uint8_t scrn;
} qbrt_gfs_file;
qbrt_gfs_file *qbrt_gfs_file_at(int32_t i);
int32_t qbrt_gfs_fileno_valid(int32_t f);
int32_t qbrt_gfs_fileno(int32_t f);
int32_t qbrt_gfs_fileno_freefile(void);
void qbrt_gfs_fileno_use(int32_t f, int32_t i);
int32_t qbrt_gfs_fileno_n(void);
int32_t qbrt_gfs_open(const uint8_t *name, int32_t len, int32_t access, int32_t restrictions, int32_t how);
int32_t qbrt_gfs_close(int32_t i);
int64_t qbrt_gfs_lof(int32_t i);
int32_t qbrt_gfs_setpos(int32_t i, int64_t position);
int64_t qbrt_gfs_getpos(int32_t i);
int32_t qbrt_gfs_write(int32_t i, int64_t position, const uint8_t *data, int64_t size);
int32_t qbrt_gfs_read(int32_t i, int64_t position, uint8_t *data, int64_t size);
int64_t qbrt_gfs_read_bytes(void);
int32_t qbrt_gfs_eof_reached(int32_t i);
int32_t qbrt_gfs_eof_passed(int32_t i);
int32_t qbrt_gfs_lock(int32_t i, int64_t start, int64_t end);
int32_t qbrt_gfs_unlock(int32_t i, int64_t start, int64_t end);

// --- File statements (OPEN, CLOSE, PRINT #, INPUT #, GET/PUT, FIELD, ...) ----
// i / fileno are QBasic file numbers (> 0); TCP/IP handles stay in C++.
// Functions producing a new string set *assign=1 and point *out at a buffer
// that stays valid until the next such call.
typedef struct qbrt_num_repr { // n_digits, n_digit, n_exp, n_neg, n_hex
    uint16_t digits;
    uint8_t digit[256];
    int64_t exp;
    uint8_t neg, hex;
} qbrt_num_repr;
typedef struct qbrt_field_slot {
    int32_t fileno;
    int64_t fileid, size, offset;
} qbrt_field_slot;
#define QBRT_PRINT_TO_SCREEN (-1)
int32_t qbrt_file_open(const uint8_t *name, int32_t len, int32_t type, int32_t access, int32_t sharing, int32_t i,
                       int64_t record_length, int32_t passed);
int32_t qbrt_file_open_gwbasic(const uint8_t *typestr, int32_t typelen, int32_t i, const uint8_t *name, int32_t len,
                               int64_t record_length, int32_t passed);
void qbrt_file_close(int32_t i);
void qbrt_file_close_all(void);
int32_t qbrt_file_freefile(void);
int32_t qbrt_file_print(int32_t i, const uint8_t *data, int32_t len, int32_t extraspace, int32_t tab, int32_t newline);
int32_t qbrt_file_input_number(int32_t fileno, qbrt_num_repr *n, int32_t *result);
int32_t qbrt_file_input_string(int32_t fileno, const uint8_t **out, int32_t *out_len, int32_t *assign);
int32_t qbrt_file_line_input(int32_t fileno, const uint8_t **out, int32_t *out_len, int32_t *assign);
int32_t qbrt_file_input_n(int32_t n, int32_t fileno, uint8_t *buf, int32_t buflen, int32_t *len);
int32_t qbrt_file_get(int32_t i, int64_t offset, uint8_t *data, int32_t len, int32_t passed);
int32_t qbrt_file_put(int32_t i, int64_t offset, const uint8_t *data, int32_t len, int32_t passed);
int32_t qbrt_file_get_string(int32_t i, int64_t offset, uint8_t *chr, int32_t len, int32_t passed, const uint8_t **out,
                             int32_t *out_len, int32_t *assign);
int32_t qbrt_file_put_string(int32_t i, int64_t offset, const uint8_t *data, int32_t len, int32_t passed);
int32_t qbrt_file_lof(int32_t i, int64_t *out);
int32_t qbrt_file_eof(int32_t i, int32_t *out);
int32_t qbrt_file_seek_set(int32_t i, int64_t pos);
int32_t qbrt_file_seek_get(int32_t i, int64_t *out);
int32_t qbrt_file_loc(int32_t i, int64_t *out);
int32_t qbrt_file_lock(int32_t i, int64_t start, int64_t end, int32_t passed, int32_t lock_it);
int32_t qbrt_generic_put(int32_t i, int32_t offset, const uint8_t *data, int32_t bytes);
int32_t qbrt_generic_get(int32_t i, int32_t offset, uint8_t *data, int32_t bytes, int32_t *read);
int32_t qbrt_field_new(int32_t fileno, int32_t new_error);
int32_t qbrt_field_add(int64_t size, int32_t new_error, qbrt_field_slot *slot); // -1 skip, 0 ok, else error
void qbrt_field_link(int32_t fileno, void *qbs_ptr, int64_t size);
int32_t qbrt_field_store(int32_t fileno, int64_t fileid, int64_t size, int64_t offset, const uint8_t *data, int32_t len);
void qbrt_field_forget(int32_t fileno, int64_t fileid, void *qbs_ptr);
int32_t qbrt_field_get(int32_t fileno, int64_t offset, int32_t passed);
int32_t qbrt_field_put(int32_t fileno, int64_t offset, int32_t passed);
void qbrt_lset(uint8_t *dest, int32_t dlen, const uint8_t *src, int32_t slen);
void qbrt_rset(uint8_t *dest, int32_t dlen, const uint8_t *src, int32_t slen);

// --- File system (was sub_kill/name/chdir/mkdir/rmdir/files, _FILEEXISTS, _DIREXISTS, _CWD$)
int32_t qbrt_kill(const uint8_t *s, int32_t len);
int32_t qbrt_name(const uint8_t *a, int32_t alen, const uint8_t *b, int32_t blen);
int32_t qbrt_chdir(const uint8_t *s, int32_t len);
int32_t qbrt_mkdir(const uint8_t *s, int32_t len);
int32_t qbrt_rmdir(const uint8_t *s, int32_t len);
int32_t qbrt_exists(const uint8_t *s, int32_t len, int32_t dir); // -1 / 0
int32_t qbrt_cwd(const uint8_t **out, int32_t *out_len);
// FILES: lines to print joined with '\n' (directory, entries, "n Bytes free").
int32_t qbrt_files(const uint8_t *s, int32_t len, int32_t passed, const uint8_t **out, int32_t *out_len);

// --- ENVIRON$ / ENVIRON / COMMAND$ (was func_environ, sub_environ, func_command)
void qbrt_environ_get(const uint8_t *name, int32_t len, const uint8_t **out, int32_t *out_len);
int32_t qbrt_environ_index(char **envp, int32_t number, const uint8_t **out, int32_t *out_len);
int32_t qbrt_environ_count(char **envp);
int32_t qbrt_environ_set(const uint8_t *s, int32_t len);
void qbrt_command_arg(char **array, int32_t count, int32_t index, const uint8_t **out, int32_t *out_len);

// --- SHELL family (was func_shell, func__shellhide, sub_shell, sub_shell2-4)
int64_t qbrt_shell_run(const uint8_t *cmd, int32_t len, int32_t show, int32_t wait, int32_t use_console);
int64_t qbrt_shell_interactive(int32_t use_console);
// --- Software graphics (R5, was the drawing code of libqb.cpp) ----------------
// Surfaces are `struct img_struct` (common.h); Rust mirrors its layout exactly.
// Statement entry points act on the page passed in (the C++ shims pass
// write_page/read_page) and return a QBasic error code (0 = none) that the
// shim raises with error().
struct img_struct;
// Built-in data (were filled from embedded arrays at start-up).
extern const uint32_t palette_256[256];
extern const uint32_t palette_64[64];
extern const uint32_t pal_mode10[2][9];
// The fonts are immutable Rust statics; they are declared non-const because
// legacy text code walks them with plain uint8* cursors. Never write to them.
extern uint8_t charset8x8[256][8][8];
extern uint8_t charset8x16[256][16][8];
// Blending tables (cblend: 16M, ablend: 64K), built on first use.
void qbrt_gfx_blend_tables(const uint8_t **cblend, const uint8_t **ablend);
// Pixel writers (no range check beyond the VIEW for pset_and_clip).
void qbrt_gfx_pset(const struct img_struct *im, int32_t x, int32_t y, uint32_t col);
uint32_t qbrt_gfx_point_raw(const struct img_struct *im, int32_t x, int32_t y);
void qbrt_gfx_fast_boxfill(const struct img_struct *im, int32_t x1, int32_t y1, int32_t x2, int32_t y2, uint32_t col);
void qbrt_gfx_boxfill(const struct img_struct *im, float x1, float y1, float x2, float y2, uint32_t col);
void qbrt_gfx_fast_line(const struct img_struct *im, int32_t x1, int32_t y1, int32_t x2, int32_t y2, uint32_t col);
// Palettes and colors.
void qbrt_gfx_restorepalette(const struct img_struct *im);
int32_t qbrt_gfx_palette(struct img_struct *im, uint32_t attribute, uint32_t col, int32_t passed);
int32_t qbrt_gfx_color(struct img_struct *im, uint32_t col1, uint32_t col2, int32_t passed);
void qbrt_gfx_defaultcolors(struct img_struct *im);
// Statements and functions.
int32_t qbrt_gfx_line(struct img_struct *im, float x1, float y1, float x2, float y2, uint32_t col, int32_t bf,
                      uint32_t style, int32_t passed);
int32_t qbrt_gfx_circle(struct img_struct *im, double x, double y, double r, uint32_t col, double start, double end,
                        double aspect, int32_t passed);
int32_t qbrt_gfx_paint(struct img_struct *im, float x, float y, uint32_t fillcol, uint32_t bordercol, int32_t passed);
int32_t qbrt_gfx_paint_tile(struct img_struct *im, float x, float y, const uint8_t *tile, int32_t len,
                            uint32_t bordercol, int32_t passed);
int32_t qbrt_gfx_pset_stmt(struct img_struct *im, float x, float y, uint32_t col, int32_t passed);
int32_t qbrt_gfx_preset_stmt(struct img_struct *im, float x, float y, uint32_t col, int32_t passed);
double qbrt_gfx_point(const struct img_struct *write_page, const struct img_struct *read_page, float x, float y,
                      int32_t passed, int32_t *err);
int32_t qbrt_gfx_window(struct img_struct *im, float x1, float y1, float x2, float y2, int32_t passed);
int32_t qbrt_gfx_view(struct img_struct *im, int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t fillcolor,
                      int32_t bordercolor, int32_t passed);
float qbrt_gfx_pmap(const struct img_struct *im, float val, int32_t option, int32_t *err);
// Image operations (handles resolved by the C++ shims). which: 0 red 1 green 2 blue 3 alpha.
uint32_t qbrt_gfx_rgb(const struct img_struct *im, int32_t r, int32_t g, int32_t b);
uint32_t qbrt_gfx_rgba(const struct img_struct *im, int32_t r, int32_t g, int32_t b, int32_t a);
uint32_t qbrt_gfx_matchcol(const struct img_struct *im, int32_t r, int32_t g, int32_t b);
int32_t qbrt_gfx_component(const struct img_struct *im, uint32_t col, int32_t which, int32_t *err);
int32_t qbrt_gfx_setalpha(struct img_struct *im, int32_t a, uint32_t c, uint32_t c2, int32_t passed);
int32_t qbrt_gfx_clearcolor(struct img_struct *im, uint32_t c, int32_t passed);
uint32_t qbrt_gfx_palettecolor_get(const struct img_struct *im, int32_t n, int32_t *err);
int32_t qbrt_gfx_palettecolor_set(struct img_struct *im, int32_t n, uint32_t c);
// Image registry. `img`/`nimg`/`nextimg` are owned by Rust; `img` moves when
// the registry grows (compare nimg before/after a call and refresh cached
// page pointers). Index 0 is never returned by a successful allocation once
// libqb has reserved slots 0 and 1 at start-up.
extern struct img_struct *img;
extern int32_t nimg;
extern int32_t nextimg;
void qbrt_img_init(void);
uint32_t qbrt_img_new(int32_t *err);
void qbrt_img_free(uint32_t i);
int32_t qbrt_img_grow(int32_t required);
void qbrt_img_alloc_pixels(int32_t i, intptr_t bytes);
void qbrt_img_alloc_palette(int32_t i);
void qbrt_img_release_pixels(int32_t i);
void qbrt_img_release_palette(int32_t i);
void qbrt_img_set_external_offset(int32_t i, uint8_t *offset);
void qbrt_img_set_external_palette(int32_t i, uint32_t *pal);
void qbrt_img_release_owned(int32_t i);
int32_t qbrt_imgframe(uint8_t *o, int32_t x, int32_t y, int32_t bpp, int32_t *err);
int32_t qbrt_imgnew(int32_t x, int32_t y, int32_t bpp, int32_t *err);
void qbrt_imgrevert(int32_t i);
// DRAW: cmem is the conventional memory block (VARPTR$ arguments point into it).
int32_t qbrt_gfx_draw(struct img_struct *im, const uint8_t *s, int32_t len, const uint8_t *cmem);
double qbrt_gfx_draw_f80(const uint8_t *p); // diagnostics: x87 long double -> double load
// _PUTIMAGE on resolved, distinct surfaces (passed: legacy bits). With hw_out
// (8 int32: sx1,sy1,sx2,sy2,dx1,dy1,dx2,dy2) only the coordinates are resolved.
typedef struct qbrt_put_coords { double dx1, dy1, dx2, dy2, sx1, sy1, sx2, sy2; } qbrt_put_coords;
int32_t qbrt_gfx_putimage(struct img_struct *s, struct img_struct *d, const qbrt_put_coords *c, int32_t passed,
                          int32_t *hw_out);
uint64_t qbrt_gfx_putimage_oob_count(void); // diagnostics (deviation D-10)
// Graphics GET/PUT on read_page/write_page; data/len = the array memory.
int32_t qbrt_gfx_get(struct img_struct *im, float x1, float y1, float x2, float y2, uint8_t *data, int32_t len,
                     uint32_t mask, int32_t passed);
int32_t qbrt_gfx_put(struct img_struct *im, float x1, float y1, const uint8_t *data, int32_t len, int32_t option,
                     uint32_t mask, int32_t passed);
// PCOPY on resolved pages; built-in font glyph at the text cursor of a graphics page.
int32_t qbrt_gfx_pcopy(const struct img_struct *s, const struct img_struct *d);
/* SCREEN decisions (gfx/screen.rs): mode check (mode >= 0), graphics mode
   size + font override (out[3] = w, h, font or 0; returns 0 if not a graphics
   mode), SCREEN 0 layout (out[5] = cols, rows, font, cmem bytes per page,
   cmem pages); the override pointers are reset to -1 where legacy did. */
int32_t qbrt_gfx_screen_mode_valid(int32_t mode);
int32_t qbrt_gfx_screen_geometry(int32_t mode, int32_t prev_w, int32_t prev_h, int32_t *out);
void qbrt_gfx_screen0_layout(int32_t prev_w, int32_t prev_h, int32_t *width, int32_t *height, int32_t *font, int32_t *out);
void qbrt_gfx_printchr(const struct img_struct *im, int32_t character);
// Software _MAPTRIANGLE on resolved surfaces: s = sx1,sy1,..,sy3; d = dx1,dy1,..,dy3.
int32_t qbrt_gfx_maptriangle(const struct img_struct *src, const struct img_struct *dst, const float *s, const float *d,
                             int32_t seamless);
// Headless framebuffer capture (QBNEX_FB_CAPTURE); kind 0 = _DISPLAY, 1 = end.
void qbrt_fb_capture(const struct img_struct *display_page, int32_t kind);

#ifdef __cplusplus
}
#endif

#endif // QBNEX_QBRT_H
