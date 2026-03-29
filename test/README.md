# Test Layout

This workspace keeps file-based tests and shared BASIC fixtures under one top-level `test/` directory.

## Structure

- `test/integration/cli_tool/`
  CLI, compile, and end-to-end behavior tests
- `test/integration/syntax_tree/`
  Parser regression coverage for real QB64 source files
- `test/integration/vm_engine/`
  VM semantic regression tests
- `test/integration/tokenizer/`
  Tokenizer integration coverage
- `test/fixtures/basic/`
  Shared BASIC source fixtures used by smoke tests, compatibility tests, and manual validation

## Run Common Test Commands

```bash
cargo test
cargo test -p tokenizer
cargo test -p syntax_tree
cargo test -p vm_engine
cargo test -p cli_tool --test shell_cli
cargo test -p cli_tool --test compile_smoke_test
cargo test -p syntax_tree --test qb64_source_parse
cargo test -p cli_tool --test compile_smoke_test qb_compiles_supported_qb64_source_files -- --ignored
cargo test -p cli_tool --test compile_smoke_test qb_compiles_qb64_source_roots_discovered_from_include_graph -- --ignored
cargo test -p cli_tool --test compile_smoke_test qb_promotes_supported_qb64_source_fragments_through_their_unique_root -- --ignored
cargo test -p cli_tool --test shell_cli build_pipelines_clean_up_tagged_temp_workspaces -- --ignored
```

## Notes

- Unit tests that live inside Rust source modules remain colocated under `src/` because they depend on module-private APIs.
- New file-based regression tests should go in `test/integration/<crate>/`.
- New shared `.bas` fixtures should go in `test/fixtures/basic/`.
- CLI source loading now expands QB64-style `$INCLUDE` directives before parsing, so include-related regressions belong in `test/integration/cli_tool/`.
- QB64 source compile regression is intentionally focused on standalone/root programs; many files under `qb64/source/` are fragments that only make sense when included by a larger root file.
- The ignored `qb_compiles_supported_qb64_source_files` sweep now compiles every current `*.bas` file under `qb64/source/`, so new source files join the regression set automatically.
- Supported QB64 fragments with a single owning root now have their own ignored compile regression so fragment promotion stays covered without requiring a full source-tree sweep on every run.
- Nested fragment chains also have a non-ignored CLI regression, so root discovery keeps preferring the top-level owning program instead of stopping at an intermediate include parent.
- The CLI now reports a dedicated fragment hint when a non-standalone included module is compiled directly and another source file includes it.
- When a fragment is auto-promoted through its unique owning root, the generated output name still follows the fragment path the user invoked rather than the internal root path.
- Nested `-o` paths are covered in CLI regression tests so both native and VM-backed build paths keep creating missing parent directories automatically.
- Existing-directory `-o` targets are also covered so compile/run paths keep resolving to `<dir>/<source-stem>.exe` consistently.
- Temp workspace cleanup for release build pipelines is covered by an ignored CLI regression that tags child-process temp paths and verifies they are removed after both graphics/native and VM-backed builds.
- CLI syntax diagnostics now have a dedicated regression that checks for a highlighted source snippet on invalid BASIC input, so `miette`-based error rendering stays exercised.
- Experimental `--frontend chumsky` and `--native-backend cranelift-jit` paths also have CLI regressions, so alternate parser/backend wiring stays covered end to end.
- The CLI also has regressions for preview gating and `--validate-release`, so production-vs-preview behavior stays enforced.
- `--validate-release` is backed by shared BASIC fixtures under `test/fixtures/basic/`, so release health checks are exercised against real text-mode, file-I/O, VM-fallback, and graphics-mode source inputs rather than ad hoc inline strings.
- Fixtures with companion `.out` files are executed in-process through the VM during validation, so deterministic runtime output stays covered without paying the cost of rebuilding transient executables for every release-check fixture.
- `--validate-pipeline` reuses that same fixture set and the same `.out` runtime-output assertions for any selected frontend/backend combination, so preview coverage is measured against the production gate instead of an easier side suite.
- Validation success output now carries `[n/total]` per-fixture progress plus coverage counts for graphics, runtime-output, VM-fallback, and native-codegen expectations, and those UX details are locked by CLI regressions too.
- `--list-pipelines` also has a CLI regression, so the user-facing production/preview matrix stays synchronized with the actual stability markers in code.
- `--list-pipelines` now includes the release-fixture inventory and the same validation coverage counts, so operators can inspect the effective release gate without opening the source tree.
- `--explain-pipeline` has a CLI regression too, so the reported native-vs-VM fallback reasoning remains trustworthy when backend support changes.
