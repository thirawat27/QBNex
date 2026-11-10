# Cross-platform audit (Linux / macOS)

Branch `claude/xplat-audit-484eb4`, base rust-port ae57d8c. Rust 1.99.1.

## cargo check (from Windows, check only)

`cargo check --workspace --all-targets` with `CARGO_BUILD_JOBS=1`, one target at a time.

| Target | Result |
|---|---|
| x86_64-unknown-linux-gnu | pass |
| aarch64-unknown-linux-gnu | pass |
| x86_64-apple-darwin | pass |
| aarch64-apple-darwin | pass |

There were no warnings.

### Errors found and fixed

| File:line | Targets | Cause | Fix |
|---|---|---|---|
| crates/qb-runtime/src/env.rs:19 | linux, macos | `extern "C"` block without `unsafe` in the `cfg(not(windows))` branch. Edition 2024 rejects it, and it was never compiled on Windows. | Changed to `unsafe extern "C"` (commit 23722fe). |
| crates/qb-runtime/src/screen.rs:383, 391 | macos | Same cause, in the CoreGraphics / CoreFoundation blocks. | Same fix. |

No other Rust-level errors in any crate.

### Build-script blockers (not Rust errors)

A cross check from Windows needs these stand-ins. On a real Linux or macOS machine none of them are needed.

| Crate | Pulled in by | Problem | Workaround for check-only |
|---|---|---|---|
| alsa-sys 0.4 | qb-audio → cpal | pkg-config probe for `alsa`. | `PKG_CONFIG_ALLOW_CROSS=1`, plus a `PKG_CONFIG` stub that exits 0 and prints `1.0.0` for `--modversion`. |
| libudev-sys 0.1 | qb-runtime → gilrs | pkg-config probe for `libudev`. | Same stub. |
| **ring 0.17** | qb-ext → ureq → rustls | Compiles C and asm with `cc` for the target. It needs `x86_64-linux-gnu-gcc` or an Apple clang. | `CC_<target>` / `AR_<target>` stubs that write empty objects. |

## Issues to act on

1. **ring brings C back into the runtime** (qb-ext, R12b's area, not edited here). `libqb_runtime.a` now needs a C compiler for each target, which goes against the "no C/C++" goal. Cross builds also need a cross gcc or clang.
   - Suggested fix: change ureq to `default-features = false, features = ["rustls-no-provider"]` (or similar) with a pure-Rust crypto provider such as `rustls-rustcrypto`.
   - Or put HTTPS behind a cargo feature.
2. **qb-link on macOS** (not edited). It writes code as RW, then makes it RX, through `region`. That works unsigned, but under the hardened runtime or notarization it needs `MAP_JIT`, `pthread_jit_write_protect_np` and the `com.apple.security.cs.allow-jit` entitlement.
3. **qb-ffi.** The SysV x86_64 and AAPCS64 thunks build for all four targets. Apple arm64 variadic calls pass all variadic arguments on the stack. The thunk uses the plain AAPCS64 layout, so `DECLARE LIBRARY` calls to variadic C functions (printf and similar) would be wrong on macOS arm64. Not verified at runtime.

## docker/ and setup scripts

Every path these files use still exists after the restructure:

- `compiler/qbnex.bas`
- `crates/qb-runtime/os/{linux,macos}/setup_build.*`
- `internal/c/libqb/os/{lnx,osx}`
- `internal/c/makeline_{lnx,osx}.txt`
- `RuntimeOsDir$` in `compiler/utilities/linker.bas`

Files checked: `scripts/setup_lnx.sh`, `scripts/setup_osx.command`, `docker/Dockerfile`, `docker/Dockerfile.dev`, `docker/docker-compose.yml`, `.dockerignore`. Nothing needed changing.

Notes, not fixed:

- The runtime stage of `docker/Dockerfile` has no g++, so the image cannot compile BASIC programs through the C++ path.
- It also lacks `libxkbcommon-x11-0` and `libx11-xcb1`, which winit loads with dlopen for windowed programs.
- `scripts/setup_lnx.sh` refuses to run as root. A Docker build runs as root, so `docker/Dockerfile`, which calls it, would exit 1 at that check. Suggested fix: skip the check when `/.dockerenv` exists, or build as a non-root user.
- Audio link flags (`-lasound` on Linux, the CoreAudio / AudioUnit / AudioToolbox frameworks on macOS) are added by `compiler/utilities/dependencies.bas`. That is correct.

## WSL (real Linux build)

WSL Debian 13, x86_64, root user, Rust 1.99.1.

- The first attempt failed: `Wsl/Service/CreateInstance/CreateVm/HCS/0x800705aa` (insufficient resources).
- - The second attempt got as far as qb-gpu, then the VM was killed (exit 253, host memory pressure).
- **Third attempt** (`CARGO_BUILD_JOBS=1`, no debuginfo): `cargo build --workspace --all-targets --locked` **passed** (8m34s).
- `cargo test -p qb-runtime --lib`: 4 tests failed on Linux. All four assert Windows-only behaviour. The non-Windows code matches legacy libqb.cpp (GFS_C / `remove()`). The tests were fixed with `cfg!(windows)`, and they still pass on Windows:

  | Test | Linux result | Cause |
  |---|---|---|
  | gfs.rs `open_missing_file_for_input_is_not_found` | -5, expected -6 | Legacy GFS_C only returns -5 (file not found) and -11 (bad file name), never "path not found". |
  | fileio.rs `open_errors_and_modes` | 64, expected 76 | Same cause (-11 maps to 64). |
  | fsops.rs `mkdir_rmdir_name_kill_exists` | 53, expected 0 | Legacy KILL accepts wildcards only on Windows (FindFirstFile). Unix uses `remove()` on the literal name. |
  | shell.rs `exit_codes` | `cmd: not found` | The test ran `cmd /c`. Unix now uses `sh -c`. |

- **`robustness_tests::image_decoders_never_panic_on_mutated_files` kills the WSL VM** when run without a memory limit. The decoders (stb.rs, pcx.rs, codecs.rs) count on `try_reserve_exact` failing for a huge size, the way stb's malloc fails on Windows. Linux overcommits, so the reservation succeeds, the decoder writes into it, and the OOM killer takes down the VM. Under `ulimit -v 8000000`, all 337 qb-runtime lib tests pass (441 s).
  - This is a real risk for Linux user programs: `_LOADIMAGE` of a corrupt file can OOM the process instead of returning -1.
  - Suggested fix (qb-runtime image, owner's call because it changes legacy behaviour): cap `width*height` before reserving, for example at 2^28 pixels or a size based on available memory. Or keep the test, but run it under a memory limit in Linux CI.
- The full `cargo test --workspace` in WSL was stopped on request (memory needed by R11a). The other crates' tests have not run on Linux yet.
- Scripts: `wsl-build.sh` (copy to `~/xplat` and build), `wsl-rt.sh <ulimitKB> [args]` (qb-runtime lib tests), `wsl-test.sh` (whole workspace under ulimit). They are kept in the session scratchpad and can be copied into `scripts/` on request.
## macOS code review (read-only, plus cargo check)

Checked by reading the code. After the changes below, `cargo check` passes on aarch64-apple-darwin, x86_64-apple-darwin and x86_64-unknown-linux-gnu.

Fixed:

- **keymap.rs `translate()`**: Backspace and Delete were swapped on macOS. The legacy swap (`glut_ascii(.., macos=true)`) was there because GLUT on macOS reported the backspace key as 0x7F. winit already reports the physical keys (`Key::Backspace` → 8, `Key::Delete` → 127), so `translate` no longer swaps on any OS. The C ABI `qb_keymap_glut_ascii(.., macos)` is unchanged.
- **screen.rs (`_SCREENPRINT` / `_SCREENCLICK` on macOS)**: `CFRelease(NULL)` crashes, so a CGEvent that could not be created is now skipped. The CoreGraphics prototypes match the SDK: `CGEventSourceCreate(i32)`, `CGEventCreateKeyboardEvent(src, u16, bool)`, `CGEventCreateMouseEvent(src, u32, CGPoint, u32)`, `CGEventPost(u32, e)`.
- **qb-ffi loader**: on macOS 11+, system dylibs (`/usr/lib/libz.dylib` and similar) exist only in the dyld shared cache, not on disk. `open_dynamic` skipped any candidate that is not a file, so it found only bare names through the fallback. It now also tries `dlopen` on absolute paths on Apple.

Checked, OK:

- `console/stdio.rs` uses `__stdoutp` / `__stdinp` (the correct libc symbols on macOS).
- qb-ffi `RTLD_NOW=2`, `RTLD_GLOBAL=8`, `RTLD_DEFAULT=-2` are right for Apple. The `thunk_name!` underscore prefix for Mach-O is right.
- `window.rs` runs the winit `EventLoop` on the main thread (`qb_window_run`), as macOS requires. The program runs on another thread.
- qb-link `flush_icache` uses `sys_icache_invalidate` on arm64 macOS.

Open (not fixed):

- `_SCREENIMAGE` is not implemented outside Windows. `qb_screenimage` returns NULL, so the program gets a blank image. Legacy macOS captured the screen. A port would use `CGDisplayCreateImage`, which is deprecated in macOS 15 (ScreenCaptureKit replaces it), or a crate.
- Posting CGEvents needs the Accessibility permission. Without it, macOS drops the events silently, the same as legacy.
- `crates/qb-link/src/exe/mod.rs` declares `pub mod macho;`, but `macho.rs` does not exist yet. That is fine for now, because `exe` is WIP (commit c0d6d80) and not included from `lib.rs`. The Mach-O writer has not been reviewed. It will need an ad-hoc code signature (arm64 kernel requirement), 16 KiB segment alignment on arm64, and `LC_BUILD_VERSION`.
- See "Issues to act on" above for the qb-link JIT under the hardened runtime and for qb-ffi variadic calls on arm64.