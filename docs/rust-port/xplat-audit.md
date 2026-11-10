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
- The second attempt started. apt installed `libasound2-dev` and `libudev-dev`, the repo was copied to ext4, and `cargo build --workspace --all-targets --locked` compiled up to qb-gpu with no errors.
- Then the WSL process ended with exit 253 (the VM was killed, most likely from host memory pressure). The build did not finish and `cargo test` did not run.
- To rerun: `scratchpad/wsl-build.sh` (copy to `~/xplat`, build, then `QB_AUDIO_DEVICE=none cargo test --workspace`).
