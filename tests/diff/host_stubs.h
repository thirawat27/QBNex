// Stand-ins for the symbols libqb.cpp/qbx.cpp define for compiled programs and that
// libqb_runtime.a refers to. libqb_runtime.a is one object file, so every differential
// test must provide them even if it never calls that code. Include once, at file scope,
// after the harness's own code. Define QB_DIFF_OWN_STRING_HOST when the harness defines
// cmem/cmem_sp/dblock/qb_host_error/qb_host_field_free itself.
#pragma once
#include <cstdint>
#include <cstdlib>
extern "C" {
#ifndef QB_DIFF_OWN_STRING_HOST
uint8_t cmem[1114099];
uint32_t cmem_sp = 65536;
intptr_t dblock = 0;
void qb_host_error(int32_t) { abort(); }
void qb_host_field_free(void *) {}
#endif
uint8_t *cmem_static_pointer = nullptr;
uint8_t *cmem_dynamic_base = nullptr;
int32_t qb_host_message_box(const char *, const char *, int32_t) { return 0; }
void qb_host_end() {}
void qb_host_evnt() {}
void qb_host_devices_init() {}
}
