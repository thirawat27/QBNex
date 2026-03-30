# Test Layout

This workspace keeps file-based regressions, conformance suites, and shared BASIC fixtures under one top-level `tests/` directory.

## Structure

- `tests/integration/cli/`
  CLI, compile, and end-to-end behavior regressions
- `tests/integration/frontend/`
  Parser regression coverage for real QB64 source files
- `tests/integration/runtime/`
  VM semantic regression tests
- `tests/integration/lexer/`
  Lexer/tokenizer integration coverage
- `tests/conformance/non_dos_quickbasic/`
  Centralized non-DOS QBasic/QuickBASIC conformance fixtures and CLI-driven parity checks
- `tests/corpora/qb64/`
  Shipped QB64 compatibility corpus used when the external `qb64/source/` tree is not present
- `tests/fixtures/basic/`
  Shared BASIC source fixtures used by smoke tests, validation, and manual checks
- `tests/runners/`
  Helper scripts for long-running or sharded regression execution

## Run Common Test Commands

```bash
cargo test
cargo test -p tokenizer
cargo test -p syntax_tree
cargo test -p vm_engine
cargo test -p cli_tool --test shell_cli
cargo test -p cli_tool --test compile_smoke_test
cargo test -p cli_tool --test qbasic_conformance
cargo test -p syntax_tree --test qb64_source_parse
powershell -ExecutionPolicy Bypass -File tests/runners/run-cli-regression-suite.ps1 -Workspace D:\QBNex
powershell -ExecutionPolicy Bypass -File tests/runners/run-cli-regression-suite.ps1 -Workspace D:\QBNex -Filter play
python tests/runners/run_cli_regression_suite.py --workspace D:\QBNex
python tests/runners/run_cli_regression_suite.py --workspace D:\QBNex --filter graphics_
cargo test -p cli_tool --test compile_smoke_test qb_compiles_supported_qb64_source_files -- --ignored
cargo test -p cli_tool --test compile_smoke_test qb_compiles_qb64_source_roots_discovered_from_include_graph -- --ignored
cargo test -p cli_tool --test compile_smoke_test qb_promotes_supported_qb64_source_fragments_through_their_unique_root -- --ignored
cargo test -p cli_tool --test shell_cli build_pipelines_clean_up_tagged_temp_workspaces -- --ignored
```

## Notes

- Unit tests that live inside Rust source modules remain colocated under `src/` because they depend on module-private APIs.
- New file-based regression tests should go in `tests/integration/<area>/`.
- New shared `.bas` fixtures should go in `tests/fixtures/basic/`.
- New QBasic/QuickBASIC language-coverage fixtures should go in `tests/conformance/non_dos_quickbasic/`.
- Conformance stdin payloads and expected stdout are centralized in [fixture_io_catalog.rs](D:\QBNex\tests\fixtures\fixture_io_catalog.rs), so console `INPUT` / `LINE INPUT` coverage and output assertions stay in one catalog without `.in` / `.out` sidecars.
- Multi-file conformance fixtures are supported by naming helper files with the same stem prefix as the main `.bas`, so `$INCLUDE`-driven project behavior can be covered in the same canonical harness.
- Catalog-backed expected outputs may use `<BEL>` for the ASCII bell character so sound/event fixtures stay readable in source control.
- `QBNEX_CONFORMANCE_FILTER=fixture_a,fixture_b cargo test -p cli_tool --test qbasic_conformance -- --nocapture` runs only selected conformance fixtures when you want to iterate on a narrower semantic area.
- Large real-world BASIC source corpora that are useful for parser/compile regressions but are not general-purpose fixtures should go in `tests/corpora/`.
- CLI source loading expands QB64-style `$INCLUDE` directives before parsing, so include-related regressions belong in `tests/integration/cli/`.
- QB64 source compile regression is intentionally focused on standalone/root programs; many files under `qb64/source/` are fragments that only make sense when included by a larger root file.
- The ignored `qb_compiles_supported_qb64_source_files` sweep compiles every current `*.bas` file under `qb64/source/`, so new source files join the regression set automatically.
- Supported QB64 fragments with a single owning root have their own ignored compile regression so fragment promotion stays covered without requiring a full source-tree sweep on every run.
- Nested fragment chains also have a non-ignored CLI regression, so root discovery keeps preferring the top-level owning program instead of stopping at an intermediate include parent.
- The CLI reports a dedicated fragment hint when a non-standalone included module is compiled directly and another source file includes it.
- When a fragment is auto-promoted through its unique owning root, the generated output name still follows the fragment path the user invoked rather than the internal root path.
- Nested `-o` paths are covered in CLI regression tests so both native and VM-backed build paths keep creating missing parent directories automatically.
- Existing-directory `-o` targets are also covered so compile/run paths keep resolving to a platform-appropriate executable name such as `<dir>/<source-stem>.exe` on Windows or `<dir>/<source-stem>` on Unix.
- Temp workspace cleanup for release build pipelines is covered by an ignored CLI regression that tags child-process temp paths and verifies they are removed after both graphics/native and VM-backed builds.
- CLI syntax diagnostics have a dedicated regression that checks for a highlighted source snippet on invalid BASIC input, so `miette`-based error rendering stays exercised.
- The public CLI is production-only now, and `shell_cli` keeps regressions for removed legacy flags such as `--frontend`, `--native-backend`, `--allow-preview`, `--validate-pipeline`, and `--list-pipelines` so release builds do not accidentally re-expose them.
- Windows-only retry/cleanup helpers inside the CLI and conformance harness are now guarded with `#[cfg(windows)]`, so cross-platform `clippy -D warnings` stays clean instead of carrying dead-code exceptions on Linux or macOS.
- `tests/runners/run-cli-regression-suite.ps1` is the Windows-oriented shard runner for long `shell_cli` sweeps.
- `tests/runners/run_cli_regression_suite.py` provides the same one-test-at-a-time sharding flow on Windows, Linux, and macOS without depending on PowerShell.
- `--validate-release` is backed by shared BASIC fixtures under `tests/fixtures/basic/`, so release health checks are exercised against real text-mode, file-I/O, VM-fallback, and graphics-mode source inputs rather than ad hoc inline strings.
- Release/runtime-output expectations are read from [fixture_io_catalog.rs](D:\QBNex\tests\fixtures\fixture_io_catalog.rs) and executed in-process through the VM during validation, so deterministic runtime output stays covered without paying the cost of rebuilding transient executables for every release-check fixture.
- `--validate-release` is the single release gate for the shipped compiler surface, and it reuses the same catalog-backed runtime-output assertions that back the canonical conformance coverage.
- The centralized non-DOS QBasic/QuickBASIC conformance suite runs the CLI in default, `-x`, and compile-only modes against the same fixture inventory, so language-coverage claims can be checked against one canonical set of expected outputs.
- The current non-DOS conformance suite covers arrays and bounds, array lifecycle with `ERASE`/`REDIM`, `BYVAL`/`BYREF`, `CLEAR` with `FREEFILE` recovery, `COMMON SHARED`, `COMMON SHARED` through `$INCLUDE`, computed branching with `ON GOTO`/`ON GOSUB`, console `INPUT`/`LINE INPUT`, constants and `DEF FN`, control flow, `DATA`/`READ`/`RESTORE`, `DEFINT`/`DEFLNG` default-type coercion, fixed-length string `LSET`/`RSET` on UDT fields, logical/comparison operators, loop controls, `MID$` assignment, numeric operators, `ON PLAY`, `ON TIMER`, `ON ERROR`, `PRINT USING`, procedures/`DEF FN`, random record I/O with `FIELD`/`LSET`/`GET`/`PUT`, text-screen state through `WIDTH`/`TAB`/`SCREEN`/`CSRLIN`, sequential file I/O, advanced `SELECT CASE`, `DIM SHARED`, `STATIC`, string intrinsics, `SWAP`, `CLEAR`, built-in type conversions, and user-defined types.
- The `-x` VM-runner path now rebuilds through Cargo's shared target cache on each invocation instead of trusting a stale cached executable, so conformance runs exercise the current runtime semantics after dependency changes.
