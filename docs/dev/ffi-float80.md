# R11d — `qb-float80` and `qb-ffi` (API for the emitter and the VM)

Two Rust-only crates the QIR backend needs. Neither depends on a C compiler or a
C library beyond the platform loader. Both use `edition = "2024"` (D16).

## `crates/runtime/float80` — `_FLOAT` (decision D12)

`qb_float80::F80` is the x87 80-bit extended format (`mant: u64` with explicit
integer bit, `se: u16` sign + biased exponent; `from_bytes`/`to_bytes` give the
10-byte little-endian `long double` image). Results are bit-identical to the
legacy MinGW build (FPU control word `0x037F`).

### What the VM calls for each BASIC operation

| BASIC | Rust API | C ABI (JIT) | Legacy C++ it replaces |
| --- | --- | --- | --- |
| `a + b`, `-`, `*`, `/` on `_FLOAT` | `a + b` ... (`core::ops`) | `qb_f80_add/sub/mul/div(a, b, out)` | x87 `fadd`... |
| unary minus | `-a` | `qb_f80_neg` | `fchs` |
| `=`, `<`, ... | `a.compare(b) -> Option<Ordering>` (`None` = unordered) | `qb_f80_cmp` (-1/0/1, 2 unordered) | `fucomi` |
| `a ^ b` (**every** numeric type: legacy promotes to long double) | `qb::pow2(a, b) -> Result<F80, i32>` | `qb_f80_pow` (returns error) | `pow2` -> `powl` |
| `EXP(x##)` | `qb::exp_float` | `qb_f80_exp` | `func_exp_float` -> `expl` |
| `FIX(x##)`, `INT(x##)` | `qb::fix_float`, `qb::int_float` | `qb_f80_fix`, `qb_f80_int` | `ceill`/`floorl` |
| `SIN/COS/TAN/ATN(x##)` | `qb::sin` ... (through `f64`, like legacy) | `qb_f80_sin` ... | generated code called the double `sin` |
| `LOG`, `SQR` | `qb::log`, `qb::sqr` (error 5) | `qb_f80_log`, `qb_f80_sqr` | `func_log(double)`, `func_sqr(double)` |
| `CINT`/`CLNG`/assignment to integers | `qb::qbr(x) -> i64` | — (qb-runtime `qb_qbr_f80`) | `qbr(long double)` |
| C casts `(int)x` etc. | `x.to_i16/to_i32/to_i64(Round::Zero)`, `x.to_u64_cast()` | `qb_f80_to_i64_trunc` | `fisttp` |
| widen/narrow | `F80::from_i64/from_u64/from_f32/from_f64`, `x.to_f32()/to_f64()` | `qb_f80_from_*`, `qb_f80_to_f64/f32` | `fild`, `fld`, `fst` |
| `VAL` | qb-runtime `val::val` (uses `dec::decimal_to_f80`) | `qb_val` | `func_val` |
| `STR$`/`PRINT` of `_FLOAT` | qb-runtime `numfmt::str_f80` (formats `(double)x`) | `qb_str_f80` | `qbs_str(long double)` |
| text | `dec::parse` (`strtold`), `dec::format_e/f/g` (`%.*Le/Lf/Lg`), `Display` (shortest) | `qb_f80_parse`, `qb_f80_format` | `__mingw_strtold`, `__mingw_snprintf` |

`Err(code)` is the BASIC error the legacy runtime raised (`5` illegal function
call, `6` overflow); the result is then 0. The lower-level `libm::{expl, exp2l,
log2l, powl, powil, ldexpl, floorl, ceill}`, `F80::sqrt`, `round_int(Round)`
and `scale` are public too.

### Exactness

- Arithmetic, comparisons, conversions, `sqrt`, rounding, decimal text: pure
  software, every target.
- `expl`/`exp2l`/`log2l`/`powl` are MinGW-w64's algorithms transcribed
  instruction by instruction. Three x87 primitives inside them (`f2xm1`,
  `fyl2x`, `fyl2xp1`) are **not** correctly rounded by the hardware (about 3%
  of inputs are one ulp off, and the pattern is CPU-specific), so on x86/x86_64
  they run on the x87 unit (`x87::HARDWARE`), which makes `EXP` and `^` with a
  non-integer exponent identical to the legacy build on the same machine.
  On other targets (or with feature `soft-transcendentals`) a 120-bit software
  version is used: **deviation D-111** (identical to the hardware for ~97% of
  inputs, never more than one ulp off; measured by `diff_float80`). Integer
  exponents (`x ^ 2`) never use these primitives and are exact everywhere.

Evidence (before the C++ backend was removed): a differential test against real
MinGW `long double` ran 87,481,972 checks with 0 mismatches; its mutation run
(reference at 53-bit precision) reported 573,263 mismatches.

## `crates/engine/ffi` — `DECLARE LIBRARY` (decision D11)

```rust
use qb_ffi::{Library, Signature, Ty, Value, call, find_default_symbol, find_header, header_error};
```

### Emitter (R11a): what to check at compile time

- `DECLARE LIBRARY "spec"` (and `CUSTOMTYPE`/`STATIC`) with a C header
  (`name.h`/`name.hpp`, where the legacy compiler looked or next to the
  source): the BASIC compiler (`IrBuildHeaderShim` in `ir_emit.bas`) writes a
  C++ file that includes the header and exports `qbnex_hdr_<name>` per called
  function, builds it with g++ / c++ into `<program>_headers` next to the
  executable, and emits `meta extfn qbnex_hdr_<name> ./<program>_headers ...`;
  the VM opens a `./` spec from the program's folder. No C++ compiler is a
  compile error naming the header (**deviation D-110**, now only without a
  compiler). `find_header` / `header_error` are no longer called by the
  emitter.
- Otherwise emit, per declared function, the library spec (empty for a bare
  `DECLARE LIBRARY`), the C name (ALIAS or the BASIC name) and a `Signature`.
  Types: `_BYTE` I8, `_UNSIGNED _BYTE` U8, `INTEGER` I16, `_UNSIGNED INTEGER` U16,
  `LONG` I32, `_UNSIGNED LONG` U32, `_INTEGER64` I64, `_UNSIGNED _INTEGER64` U64,
  `SINGLE` F32, `DOUBLE` F64, `_OFFSET` Ptr; every `BYREF` parameter and
  `BYVAL x AS STRING` is Ptr (the VM passes the address of the variable / of the
  string bytes); `SUB` returns Void. `_FLOAT` parameters or results are a
  compile error (**deviation D-112**: the C ABI of `long double` differs per
  compiler; legacy passed it through g++).

### VM (R11b): run time

- Bare `DECLARE LIBRARY`: `find_default_symbol(name)` (program, C runtime,
  kernel32/user32/gdi32/advapi32/shell32/ole32/oleaut32/comdlg32/winmm/ws2_32/
  ntdll, or libc/libm/libdl/libpthread, libSystem on macOS; on Windows
  `getpid` also finds `_getpid`). The compiler's own `getpid` works.
- `DECLARE DYNAMIC LIBRARY "spec"` and `DECLARE LIBRARY "spec"` without a
  header: `Library::open_dynamic(spec, Some(source_dir))` once per library, then
  `lib.symbol(name)`. `library_candidates` lists the files tried (legacy order:
  `path\name.dll`, `System32\path\name.dll`, `name.dll`, `System32\name.dll`;
  `lib<name>.so.<ver>`, `lib<name>.so` in the path, `/usr/lib64`, `/usr/lib`,
  then without the path; `.dylib` first on macOS). A static `.a`/`.lib`/`.o`
  cannot be loaded without a linker, so `DECLARE LIBRARY "spec"` needs a shared
  library at run time. Failure: `LibraryError` (its text lists the paths).
- `unsafe { call(ptr, &sig, &args) } -> Result<Value, CallError>`: integers are
  truncated to the declared width then sign/zero-extended; floats passed as
  F32/F64; results come back as `Value::I` (signed), `U` (unsigned), `F32`, `F64`,
  `Ptr`, `Void`. `Signature::variadic(args, ret, fixed)` marks C `...`.
  `Function { ptr, sig }` bundles both.

Thunks: one `global_asm!` routine per ABI (Win64, SysV x86-64, AAPCS64 with the
Apple and Windows-on-ARM variadic rules) behind a common frame of 8 integer +
8 float registers and a stack block. Win64 copies float arguments into the
integer register of the same position as well (needed for variadic callees).

Evidence: `cargo test -p qb-ffi` (real `getpid`, `strlen`,
`GetCurrentProcessId`, `MessageBeep`, 20 mixed arguments, a cdylib built from
`tests/testlib.rs` by the test) passes on Windows x64 (msvc and gnu targets),
Linux x86-64 (WSL Debian) and Linux AArch64 (qemu-aarch64). macOS and Windows
on ARM are compiled (`cargo build --target ...`) but not run.
