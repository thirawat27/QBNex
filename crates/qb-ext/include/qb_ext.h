// QBNex extension library (crates/qb-ext): regex, hashing, Base64/hex, zip, HTTP(S).
// Linked into libqb_runtime.a. qbx.cpp includes this header so BASIC programs can
// declare the functions in a header-less DECLARE LIBRARY block (see
// compiler/stdlib/ext/*.bas). Pointer arguments are intptr_t because BASIC passes
// them as _OFFSET values; lengths are int64 (_INTEGER64).
#ifndef QB_EXT_H
#define QB_EXT_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

int64_t qb_ext_result_len(void);
int64_t qb_ext_result_copy(intptr_t dst, int64_t cap);
int64_t qb_ext_error_len(void);
int64_t qb_ext_error_copy(intptr_t dst, int64_t cap);
int32_t qb_ext_part_count(void);
int64_t qb_ext_part(int32_t i);

int32_t qb_ext_regex_match(intptr_t p, int64_t pl, intptr_t t, int64_t tl);
int64_t qb_ext_regex_find(intptr_t p, int64_t pl, intptr_t t, int64_t tl, int64_t start);
int64_t qb_ext_regex_replace(intptr_t p, int64_t pl, intptr_t t, int64_t tl, intptr_t r, int64_t rl, int32_t all);
int32_t qb_ext_regex_split(intptr_t p, int64_t pl, intptr_t t, int64_t tl);

int64_t qb_ext_hash(int32_t kind, intptr_t d, int64_t dl);
uint32_t qb_ext_crc32(intptr_t d, int64_t dl);
int64_t qb_ext_codec(int32_t op, intptr_t d, int64_t dl);

int32_t qb_ext_zip_list(intptr_t path, int64_t pl);
int64_t qb_ext_zip_entry_size(int32_t i);
int64_t qb_ext_zip_read(intptr_t path, int64_t pl, intptr_t name, int64_t nl);
int32_t qb_ext_zip_extract(intptr_t path, int64_t pl, intptr_t dest, int64_t dl);
int32_t qb_ext_zip_create(intptr_t path, int64_t pl);
int32_t qb_ext_zip_add(int32_t h, intptr_t name, int64_t nl, intptr_t data, int64_t dl, int32_t kind);
int32_t qb_ext_zip_close(int32_t h);

int32_t qb_ext_http(intptr_t method, int64_t ml, intptr_t url, int64_t ul, intptr_t headers, int64_t hl,
                    intptr_t body, int64_t bl, int32_t timeout_ms);
int64_t qb_ext_http_headers(void);
int32_t qb_ext_test_http_server(int32_t count); // test hook: local echo server, returns its port

int64_t qb_ext_utf8_len(intptr_t p, int64_t l);
int64_t qb_ext_utf8_slice(int32_t op, intptr_t p, int64_t l, int64_t a, int64_t b);
int64_t qb_ext_utf8_transform(int32_t op, intptr_t p, int64_t l);
int32_t qb_ext_utf8_valid(intptr_t p, int64_t l);
int64_t qb_ext_utf8_codepoint(intptr_t p, int64_t l, int64_t index);
int64_t qb_ext_utf8_chr(int64_t cp);
int64_t qb_ext_cp437(int32_t op, intptr_t p, int64_t l);

int32_t qb_ext_list_new(void);
int32_t qb_ext_dict_new(void);
int32_t qb_ext_coll_free(int32_t h);
int32_t qb_ext_coll_clear(int32_t h);
int64_t qb_ext_list_count(int32_t h);
int64_t qb_ext_list_add_str(int32_t h, intptr_t p, int64_t l);
int64_t qb_ext_list_add_num(int32_t h, double n);
int32_t qb_ext_list_put_str(int32_t h, int64_t i, intptr_t p, int64_t l, int32_t insert);
int32_t qb_ext_list_put_num(int32_t h, int64_t i, double n, int32_t insert);
int32_t qb_ext_list_type(int32_t h, int64_t i);
int64_t qb_ext_list_get_str(int32_t h, int64_t i);
double qb_ext_list_get_num(int32_t h, int64_t i);
int32_t qb_ext_list_remove(int32_t h, int64_t i);
int64_t qb_ext_list_find_str(int32_t h, intptr_t p, int64_t l);
int64_t qb_ext_list_find_num(int32_t h, double n);
int64_t qb_ext_list_join(int32_t h, intptr_t p, int64_t l);
int32_t qb_ext_list_sort(int32_t h, int32_t descending, int32_t ignore_case);
int64_t qb_ext_dict_count(int32_t h);
int32_t qb_ext_dict_set_str(int32_t h, intptr_t k, int64_t kl, intptr_t p, int64_t l);
int32_t qb_ext_dict_set_num(int32_t h, intptr_t k, int64_t kl, double n);
int32_t qb_ext_dict_type(int32_t h, intptr_t k, int64_t kl);
int64_t qb_ext_dict_get_str(int32_t h, intptr_t k, int64_t kl);
double qb_ext_dict_get_num(int32_t h, intptr_t k, int64_t kl);
int32_t qb_ext_dict_remove(int32_t h, intptr_t k, int64_t kl);
int64_t qb_ext_dict_key(int32_t h, int64_t i);

double qb_ext_dt_now(void);
int32_t qb_ext_dt_offset(double ms);
int64_t qb_ext_dt_iso(double ms, int32_t local);
int64_t qb_ext_dt_format(double ms, intptr_t f, int64_t fl, int32_t local);
double qb_ext_dt_parse(intptr_t t, int64_t tl, intptr_t f, int64_t fl, int32_t local);
double qb_ext_dt_make(int32_t year, int32_t month, int32_t day, int32_t hour, int32_t minute, int32_t second,
                      int32_t millis, int32_t local);
int64_t qb_ext_dt_part(double ms, int32_t part, int32_t local);
#ifdef __cplusplus
}
#endif
#endif
