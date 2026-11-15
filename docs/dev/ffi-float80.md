# `qb-float80` and `qb-ffi` (API for the emitter and the VM)

Two Rust-only crates the QIR backend needs. Neither depends on a native library
beyond the platform loader. Both use `edition = "2024"`.

## `crates/runtime/float80` — `_FLOAT`

`qb_float80::F80` is the x87 80-bit extended format (`mant: u64` with explicit
integer bit, `se: u16` sign + biased exponent; `from_bytes`/`to_bytes` give the
10-byte little-endian image). Results are bit-identical to the
legacy build (FPU control word `0x037F`).

### What the VM calls for each BASIC operation

| BASIC | Rust API | Legacy function it replaced |
| --- | --- | --- |
| `a + b`, `-`, `*`, `/` on `_FLOAT` | `a + b` ... (`core::ops`) | x87 `fadd`... |
| unary minus | `-a` | `fchs` |
| `=`, `<`, ... | `a.compare(b) -> Option<Ordering>` (`None` = unordered) | `fucomi` |
| `a ^ b` (**every** numeric type: legacy promotes to 80 bits) | `qb::pow2(a, b) -> Result<F80, i32>` | `pow2` -> `powl` |
| `EXP(x##)` | `qb::exp_float` | `func_exp_float` -> `expl` |
| `FIX(x##)`, `INT(x##)` | `qb::fix_float`, `qb::int_float` | `ceill`/`floorl` |
| `SIN/COS/TAN/ATN(x##)` | `qb::sin` ... (through `f64`, like legacy) | generated code called the double `sin` |
| `LOG`, `SQR` | `qb::log`, `qb::sqr` (error 5) | `func_log`, `func_sqr` |
| `CINT`/`CLNG`/assignment to integers | `qb::qbr(x) -> i64` | `qbr` |
| truncating casts to integer types | `x.to_i16/to_i32/to_i64(Round::Zero)`, `x.to_u64_cast()` | `fisttp` |
| widen/narrow | `F80::from_i64/from_u64/from_f32/from_f64`, `x.to_f32()/to_f64()` | `fild`, `fld`, `fst` |
| `VAL` | qb-runtime `val::val` (uses `dec::decimal_to_f80`) | `func_val` |
| `STR$`/`PRINT` of `_FLOAT` | qb-runtime `numfmt::str_f80` (formats the value as `DOUBLE`) | `qbs_str` |
| text | `dec::parse`, `dec::format_e/f/g` (`%.*Le/Lf/Lg`), `Display` (shortest) | the legacy parser and formatter |

`Err(code)` is the BASIC error the legacy runtime raised (`5` illegal function
call, `6` overflow); the result is then 0. The lower-level `libm::{expl, exp2l,
log2l, powl, powil, ldexpl, floorl, ceill}`, `F80::sqrt`, `round_int(Round)`
and `scale` are public too.

### Exactness

- Arithmetic, comparisons, conversions, `sqrt`, rounding, decimal text: pure
  software, every target.
- `expl`/`exp2l`/`log2l`/`powl` are the legacy math library's algorithms
  transcribed instruction by instruction. Three x87 primitives inside them (`f2xm1`,
  `fyl2x`, `fyl2xp1`) are **not** correctly rounded by the hardware (about 3%
  of inputs are one ulp off, and the pattern is CPU-specific), so on x86/x86_64
  they run on the x87 unit (`x87::HARDWARE`), which makes `EXP` and `^` with a
  non-integer exponent identical to the legacy build on the same machine.
  On other targets (or with feature `soft-transcendentals`) a 120-bit software
  version is used, an intentional deviation (identical to the hardware for ~97% of
  inputs, never more than one ulp off; measured by the former `diff_float80` differential test). Integer
  exponents (`x ^ 2`) never use these primitives and are exact everywhere.

Evidence: a former differential test against the legacy build's hardware
80-bit arithmetic ran 87,481,972 checks with 0 mismatches; its mutation run
(reference at 53-bit precision) reported 573,263 mismatches.

## `crates/engine/ffi` — `DECLARE LIBRARY`

```rust
use qb_ffi::{Library, Signature, Ty, Value, call, find_default_symbol, find_header, header_error};
```

### Emitter: what to check at compile time

- `DECLARE LIBRARY "spec"` (and `CUSTOMTYPE`/`STATIC`) that names a
  header file is a compile error (an intentional deviation). The BASIC compiler
  (`compiler/qbnex.bas`) looks for the header itself, with the file names of
  `find_header(spec, Some(source_dir))` (`name.h`, `name.hpp`, next to the
  path, then bare, and in `/usr/include` on Linux/macOS), and reports:

  > DECLARE LIBRARY "spec" names the header file *path*: header files are not supported. Build the functions into a DLL (.so, .dylib) and declare that library

  `find_header` and `header_error` are the same check for Rust callers.
- Otherwise emit, per declared function, the library spec (empty for a bare
  `DECLARE LIBRARY`), the symbol name (ALIAS or the BASIC name) and a `Signature`.
  Types: `_BYTE` I8, `_UNSIGNED _BYTE` U8, `INTEGER` I16, `_UNSIGNED INTEGER` U16,
  `LONG` I32, `_UNSIGNED LONG` U32, `_INTEGER64` I64, `_UNSIGNED _INTEGER64` U64,
  `SINGLE` F32, `DOUBLE` F64, `_OFFSET` Ptr; every `BYREF` parameter and
  `BYVAL x AS STRING` is Ptr (the VM passes the address of the variable / of the
  string bytes); `SUB` returns Void. `_FLOAT` parameters or results are a
  compile error (an intentional deviation: the calling convention of an 80-bit value
  differs per platform; the legacy build generated glue code for it).

### VM: run time

- Bare `DECLARE LIBRARY`: `find_default_symbol(name)` (program, system runtime,
  kernel32/user32/gdi32/advapi32/shell32/ole32/oleaut32/comdlg32/winmm/ws2_32/
  ntdll, or libc/libm/libdl/libpthread, libSystem on macOS; on Windows
  `getpid` also finds `_getpid`). The compiler's own `getpid` works.
- `DECLARE DYNAMIC LIBRARY "spec"` and `DECLARE LIBRARY "spec"` without a
  header: `Library::open_dynamic(spec, None)` once per library (the compiler has
  already made a `./` spec an absolute path in the source folder), then
  `lib.symbol(name)`. `library_candidates` lists the files tried:
  - a spec with a path (`lib/name`, `C:\libs\name`) first there, as in QB64:
    `path\name.dll`, `System32\path\name.dll`; `lib<name>.so.<ver>`,
    `lib<name>.so` in the path, `/usr/lib64/path`, `/usr/lib/path` (`.dylib`
    first on macOS). A relative path is relative to the current directory.
  - then the name alone, **never in the current directory** (QB64 tried
    `name.dll` / `./libname.so` there first, so a file of that name in the
    folder the program was started from, such as a download folder, was
    loaded instead: library planting): the program's folder (`program_dir()`:
    the executable's folder; `qb-exec prog.qir` sets the folder of `prog.qir`
    and qb-dap the folder of the `.bas` file with `set_program_dir`), then on
    Windows `System32`, the Windows folder and the absolute folders of `PATH`;
    on Linux `/usr/lib64`, `/usr/lib`; on macOS `/usr/lib`, the absolute
    folders of `DYLD_FALLBACK_LIBRARY_PATH` and `/usr/local/lib`.
  - finally the system loader's own search for the plain file name: Windows
    `LoadLibraryExW(name.dll, LOAD_LIBRARY_SEARCH_APPLICATION_DIR |
    LOAD_LIBRARY_SEARCH_SYSTEM32)` (API-set names), Linux `dlopen("libname.so")`
    (`LD_LIBRARY_PATH`, the `ld.so` cache). Not on macOS, where `dlopen` of a
    bare name searches the current directory.

  Folders of `PATH` / `DYLD_FALLBACK_LIBRARY_PATH` that are relative (`.`,
  `bin`, an empty entry) are skipped for the same reason. The libraries a
  loaded library needs in turn are found by the system loader as usual.
  A static `.a`/`.lib`/`.o`
  cannot be loaded without a linker, so `DECLARE LIBRARY "spec"` needs a shared
  library at run time. Failure: `LibraryError` (its text lists the paths).
- `unsafe { call(ptr, &sig, &args) } -> Result<Value, CallError>`: integers are
  truncated to the declared width then sign/zero-extended; floats passed as
  F32/F64; results come back as `Value::I` (signed), `U` (unsigned), `F32`, `F64`,
  `Ptr`, `Void`. `Signature::variadic(args, ret, fixed)` marks a variadic `...` list.
  `Function { ptr, sig }` bundles both.

Thunks: one `global_asm!` routine per ABI (Win64, SysV x86-64, AAPCS64 with the
Apple and Windows-on-ARM variadic rules) behind a common frame of 8 integer +
8 float registers and a stack block. Win64 copies float arguments into the
integer register of the same position as well (needed for variadic callees).

Evidence: `cargo test -p qb-ffi` (real `getpid`, `strlen`,
`GetCurrentProcessId`, `MessageBeep`, 20 mixed arguments, a cdylib built from
`tests/testlib.rs` by the test) passes on Windows x64 (`*-windows-msvc` and `*-windows-gnu` targets),
Linux x86-64 (WSL Debian) and Linux AArch64 (qemu-aarch64). macOS and Windows
on ARM are compiled (`cargo build --target ...`) but not run.
