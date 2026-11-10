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

#ifdef __cplusplus
}
#endif
#endif
