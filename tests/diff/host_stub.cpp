// Weak stand-ins for the host symbols libqb_runtime.a imports from libqb.cpp
// (crates/qb-runtime/src/qbs.rs: cmem, cmem_sp, dblock, qb_host_error, qb_host_field_free).
// libqb_runtime.a is built with codegen-units = 1, so linking any qb_* function pulls in
// these references. diff_qbs defines its own and is built without this file.
#include <cstdint>

extern "C" {
uint8_t cmem[1114099];
uint32_t cmem_sp = 65536;
intptr_t dblock = 0;
void qb_host_error(int32_t) {}
void qb_host_field_free(void *) {}
}
