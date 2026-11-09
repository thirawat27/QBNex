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

// --- System clock (was GetTicks and the start-of-day setup in main) -----------
// Monotonic milliseconds; TIMER's start-up reference values.
int64_t qbrt_get_ticks(void);
void qbrt_time_init(uint32_t *clock_first, uint32_t *day_first);

// --- Buffers returned by qbrt (free with these, giving the element count) -----
void qbrt_free_pixels(uint32_t *p, int64_t count);
void qbrt_free_bytes(uint8_t *p, int64_t count);

// --- TCP/IP (was tcp_* in libqb.cpp and myip.cpp) -------------------------------
// Handle tables stay in libqb.cpp; a qbrt_tcp is one socket.
typedef struct qbrt_tcp qbrt_tcp;
// Parses "TCP/IP:port[:host]"; method 0 = _OPENCLIENT, 1 = _OPENHOST. Returns -1
// for invalid arguments (error 5). *val_error: error VAL raised on the port.
int32_t qbrt_net_parse_info(const uint8_t *info, int32_t len, int32_t method, int32_t ide_allowed,
                            int32_t *port, int32_t *host_off, int32_t *host_len, int32_t *val_error);
qbrt_tcp *qbrt_tcp_host_open(int64_t port);
qbrt_tcp *qbrt_tcp_client_open(const uint8_t *host, int32_t len, int64_t port);
qbrt_tcp *qbrt_tcp_accept(qbrt_tcp *host);
void qbrt_tcp_close(qbrt_tcp *c);
void qbrt_tcp_send(qbrt_tcp *c, const uint8_t *data, int64_t bytes);
int64_t qbrt_tcp_recv(qbrt_tcp *c, uint8_t *buf, int64_t cap);
int32_t qbrt_tcp_connected(qbrt_tcp *c);
// _CONNECTIONADDRESS text; type 1 client, 2 host listener, 3 accepted. Returns the
// full length (writes at most cap bytes).
int32_t qbrt_tcp_address(qbrt_tcp *c, int32_t type, uint8_t *out, int32_t cap);

// --- Clipboard (was sub__clipboard/func__clipboard/_CLIPBOARDIMAGE) -----------
void qbrt_clipboard_set_text(const uint8_t *text, int32_t len);
uint8_t *qbrt_clipboard_get_text(int64_t *len);
// Images are top-down 0xAARRGGBB pixels.
void qbrt_clipboard_set_image(const uint32_t *pixels, int32_t w, int32_t h);
uint32_t *qbrt_clipboard_get_image(int32_t *w, int32_t *h);

// --- _SCREENIMAGE / _SCREENCLICK / _SCREENPRINT ------------------------------------
// NULL with *w,*h > 0: capture unsupported (make a blank image of the screen size);
// NULL with *w or *h <= 0: invalid region.
uint32_t *qbrt_screenimage(int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t passed, int32_t *w, int32_t *h);
void qbrt_screenclick(int32_t x, int32_t y, int32_t button, int32_t passed);
void qbrt_screenprint(const uint8_t *txt, int32_t len);

// --- Printer (was libqb/printer.cpp; Windows only) -------------------------------
void *qbrt_printer_begin(int32_t *w, int32_t *h);
void qbrt_printer_finish(void *p, const uint32_t *pixels, int32_t w, int32_t h);
void qbrt_printer_abort(void *p);

// --- Test hooks used only by internal/c/qbrt/tests/diff_*.cpp ------------------
uint32_t qbrt_day_ms(uint32_t hour, uint32_t minute, uint32_t second, int32_t valid);
void qbrt_test_screen_region(int32_t sw, int32_t sh, int32_t x1, int32_t y1, int32_t x2, int32_t y2,
                             int32_t passed, int32_t *out4);
int32_t qbrt_test_win_screenclick(int32_t x, int32_t y, int32_t button, int32_t passed, int32_t sw,
                                  int32_t sh, int32_t *out, int32_t cap);
int32_t qbrt_test_win_screenprint(const uint8_t *txt, int32_t len, const int16_t *scan256, int32_t *out,
                                  int32_t cap);
int32_t qbrt_test_mac_screenprint(const uint8_t *txt, int32_t len, int32_t *out, int32_t cap);
int32_t qbrt_test_printer_trace(uint32_t fail, int32_t page_w, int32_t page_h, int32_t image_ok,
                                int32_t *out, int32_t cap);
int32_t qbrt_net_format_address(int32_t type, int32_t port, const uint8_t *ip4, const uint8_t *host,
                                int32_t host_len, const uint8_t *listener_ip, int32_t ip_len,
                                uint8_t *out, int32_t cap);

#ifdef __cplusplus
}
#endif

#endif // QBNEX_QBRT_H
