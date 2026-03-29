#![allow(unused_assignments)]

use anyhow::{Context, Result};
use core_types::QError;
use miette::{Diagnostic, GraphicalReportHandler, NamedSource, SourceSpan};
use native_codegen::NativeBackendKind;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use syntax_tree::{parse_with_frontend, FrontendKind};
use thiserror::Error;

mod feature_check;
use feature_check::has_graphics_or_sound;

// ──────────────────────────────────────────────
//  Argument parsing
// ──────────────────────────────────────────────

#[derive(Default)]
struct Args {
    file: Option<String>,
    program_args: Vec<String>,
    compile: bool,
    output: Option<String>,
    run: bool,
    quiet: bool,
    explicit: bool,
    help: bool,
    version: bool,
    frontend: Option<String>,
    native_backend: Option<String>,
    allow_preview: bool,
    validate_release: bool,
    validate_pipeline: bool,
    list_pipelines: bool,
    explain_pipeline: bool,
}

fn parse_args() -> Args {
    let raw: Vec<String> = env::args().skip(1).collect();
    let mut a = Args::default();
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--" => {
                a.program_args.extend(raw[i + 1..].iter().cloned());
                break;
            }
            "-h" | "--help" => a.help = true,
            "-v" | "--version" => a.version = true,
            "--validate-release" => a.validate_release = true,
            "--validate-pipeline" => a.validate_pipeline = true,
            "--list-pipelines" => a.list_pipelines = true,
            "--explain-pipeline" => a.explain_pipeline = true,
            "--allow-preview" => a.allow_preview = true,
            "-c" => a.compile = true,
            "-x" => a.run = true,
            "-q" => a.quiet = true,
            "-e" => a.explicit = true,
            "-w" | "-m" => {}
            "-o" => {
                i += 1;
                if i < raw.len() {
                    a.output = Some(raw[i].clone());
                }
            }
            "--frontend" => {
                i += 1;
                if i < raw.len() {
                    a.frontend = Some(raw[i].clone());
                }
            }
            "--native-backend" => {
                i += 1;
                if i < raw.len() {
                    a.native_backend = Some(raw[i].clone());
                }
            }
            s if !s.starts_with('-') => a.file = Some(s.to_string()),
            _ => {}
        }
        i += 1;
    }
    a
}

// ──────────────────────────────────────────────
//  Entry point
// ──────────────────────────────────────────────

fn main() {
    if let Err(err) = try_main() {
        if let Some(exit_error) = err.downcast_ref::<ProgramExitError>() {
            std::process::exit(exit_error.code);
        }
        if let Some(diagnostic) = err.downcast_ref::<SourceDiagnostic>() {
            if render_diagnostic(diagnostic) {
                std::process::exit(1);
            }
        }
        eprintln!("{:#}", err);
        std::process::exit(1);
    }
}

fn render_diagnostic(diagnostic: &dyn Diagnostic) -> bool {
    let mut rendered = String::new();
    if GraphicalReportHandler::new()
        .render_report(&mut rendered, diagnostic)
        .is_ok()
    {
        eprintln!("{rendered}");
        true
    } else {
        false
    }
}

fn try_main() -> Result<()> {
    let args = parse_args();
    if args.validate_release && args.validate_pipeline {
        return Err(anyhow::anyhow!(
            "--validate-release and --validate-pipeline cannot be used together"
        ));
    }

    let frontend = args
        .frontend
        .as_deref()
        .map(str::parse::<FrontendKind>)
        .transpose()?
        .unwrap_or_else(syntax_tree::production_frontend);
    let native_backend = args
        .native_backend
        .as_deref()
        .map(str::parse::<NativeBackendKind>)
        .transpose()?;

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.list_pipelines {
        print_pipeline_status()?;
        return Ok(());
    }

    if args.explain_pipeline {
        let file = args.file.as_deref().ok_or_else(|| {
            anyhow::anyhow!("--explain-pipeline requires an input BASIC source file")
        })?;
        explain_pipeline_for_file(file, frontend, native_backend, args.allow_preview)?;
        return Ok(());
    }

    if args.validate_release {
        validate_release_surface(args.quiet)?;
        return Ok(());
    }

    if args.validate_pipeline {
        validate_pipeline_surface(frontend, native_backend, args.allow_preview, args.quiet)?;
        return Ok(());
    }

    if args.explicit {
        unsafe {
            std::env::set_var("QBNEX_EXPLICIT", "1");
        }
    }

    enforce_stability(frontend, native_backend, args.allow_preview)?;

    // Show help when: -h flag OR no file AND no action flag
    if args.help || args.file.is_none() {
        print_help();
        return Ok(());
    }

    if args.compile {
        match &args.file {
            Some(f) => retry_with_fragment_root(f, args.quiet, |entry, logical_input| {
                build_file(
                    entry,
                    logical_input,
                    args.output.as_deref(),
                    args.quiet,
                    frontend,
                    native_backend,
                )
            })?,
            None => {
                eprintln!("error: no input file specified for compilation");
                eprintln!("       usage: qb -c <file.bas>");
                std::process::exit(1);
            }
        }
    } else if args.run {
        match &args.file {
            Some(f) => retry_with_fragment_root(f, args.quiet, |entry, logical_input| {
                run_file(
                    entry,
                    logical_input,
                    args.output.as_deref(),
                    &args.program_args,
                    args.quiet,
                    frontend,
                    native_backend,
                )
            })?,
            None => {
                eprintln!("error: no input file specified");
                eprintln!("       usage: qb -x <file.bas>");
                std::process::exit(1);
            }
        }
    } else if args.output.is_some() {
        match &args.file {
            Some(f) => retry_with_fragment_root(f, args.quiet, |entry, logical_input| {
                build_file(
                    entry,
                    logical_input,
                    args.output.as_deref(),
                    args.quiet,
                    frontend,
                    native_backend,
                )
            })?,
            None => {
                eprintln!("error: no input file specified for compilation");
                eprintln!("       usage: qb -c <file.bas>");
                std::process::exit(1);
            }
        }
    } else {
        // Default mode: Compile and Run (like QB64)
        match &args.file {
            Some(f) => retry_with_fragment_root(f, args.quiet, |entry, logical_input| {
                compile_and_run(
                    entry,
                    logical_input,
                    args.output.as_deref(),
                    &args.program_args,
                    args.quiet,
                    frontend,
                    native_backend,
                )
            })?,
            None => unreachable!(),
        }
    }

    Ok(())
}

// ──────────────────────────────────────────────
//  Help (plain text)
// ──────────────────────────────────────────────

fn print_help() {
    println!(
        r#"QBNex - Modern QBasic Compiler

USAGE:
    qb [OPTIONS] [FILE]

ARGUMENTS:
    FILE                   Source file (.bas) to compile and run

OPTIONS:
    -c                     Compile FILE to .exe only (do not run)
    -o <OUTPUT>            Set output filename  (default: <FILE>.exe)
    -x                     Build a VM-backed executable and run it
    --frontend <NAME>      Select parser frontend: classic, chumsky
    --native-backend <NAME> Select native backend: rust, llvm-ir, cranelift-jit
    --allow-preview        Allow preview frontend/backend paths explicitly
    --list-pipelines       Show production/preview pipeline status and exit
    --explain-pipeline     Explain the selected pipeline for FILE and exit
    --validate-release     Run production-surface validation checks and exit
    --validate-pipeline    Run the release fixtures against the selected pipeline
    -e                     Enable OPTION _EXPLICIT (force variable declaration)
    -w                     Show warnings
    -q                     Quiet mode (suppress non-error output)
    -m                     Monochrome output
    -h, --help             Show this help message
    -v, --version          Show version

EXAMPLES:
    qb hello.bas           Compile & run hello.bas (default)
    qb -x hello.bas        Build and run hello.bas via the VM runner
    qb --frontend chumsky -x hello.bas
                           Parse with the preview Chumsky frontend (requires --allow-preview)
    qb --native-backend cranelift-jit -x hello.bas
                           Build and run via the preview Cranelift JIT runner (requires --allow-preview)
    qb -x hello.bas -- a b Run via the VM runner with COMMAND$ = "a b"
    qb -x -o app.exe a.bas Build, run, and keep the VM runner as app.exe
    qb -c hello.bas        Compile  -->  hello.exe
    qb -c -o out.exe a.bas Compile  -->  out.exe
    qb -e main.bas         Compile & run with forced variable declaration
    qb --explain-pipeline main.bas
                           Explain which runtime/backend path QBNex will use
    qb --list-pipelines    Show frontend/backend stability levels
    qb --validate-release  Validate the production compiler surface
    qb --validate-pipeline --frontend classic
                           Validate the release fixtures with the selected pipeline"#
    );
}

fn print_pipeline_status() -> Result<()> {
    let fixtures = release_fixture_specs();
    let summary = summarize_release_fixtures(&fixtures)?;

    println!("Frontends:");
    for frontend in FrontendKind::ALL {
        println!(
            "  {} [{}] - {}",
            frontend.name(),
            frontend.stability().label(),
            frontend.description()
        );
    }

    println!("Native backends:");
    for backend in NativeBackendKind::ALL {
        println!(
            "  {} [{}] - {}",
            backend.name(),
            backend.stability().label(),
            backend.description()
        );
    }

    println!();
    println!("Validation:");
    println!("  qb --validate-release");
    println!("  qb --validate-pipeline --frontend classic");
    println!("  qb --validate-pipeline --frontend classic --native-backend rust");
    println!(
        "  qb --validate-pipeline --frontend classic --native-backend cranelift-jit --allow-preview"
    );
    println!(
        "  fixtures={} graphics={} runtime-output={} vm-fallback={} native-codegen={}",
        summary.total,
        summary.graphics,
        summary.runtime_output,
        summary.vm_fallback,
        summary.native_codegen
    );
    println!("Validation fixtures:");
    for fixture in fixtures {
        println!("  {}", format_release_fixture_descriptor(fixture)?);
    }

    Ok(())
}

fn enforce_stability(
    frontend: FrontendKind,
    native_backend: Option<NativeBackendKind>,
    allow_preview: bool,
) -> Result<()> {
    if !allow_preview && !frontend.is_production_ready() {
        return Err(anyhow::anyhow!(
            "frontend '{}' is {}-only right now; rerun with --allow-preview to use it",
            frontend.name(),
            frontend.stability().label()
        ));
    }

    if let Some(native_backend) = native_backend {
        if !allow_preview && !native_backend.is_production_ready() {
            return Err(anyhow::anyhow!(
                "native backend '{}' is {}-only right now; rerun with --allow-preview to use it",
                native_backend.name(),
                native_backend.stability().label()
            ));
        }
    }

    Ok(())
}

fn explain_pipeline_for_file(
    input: &str,
    frontend: FrontendKind,
    native_backend: Option<NativeBackendKind>,
    allow_preview: bool,
) -> Result<()> {
    let stability_status = enforce_stability(frontend, native_backend, allow_preview).err();
    let loaded = load_program(input, frontend)?;
    let native_gaps =
        syntax_tree::unsupported_statements(&loaded.program, syntax_tree::Backend::Native);
    let vm_gaps = syntax_tree::unsupported_statements(&loaded.program, syntax_tree::Backend::Vm);

    println!("input: {}", clean_path_display(Path::new(input)));
    println!(
        "frontend: {} [{}]",
        frontend.name(),
        frontend.stability().label()
    );
    match native_backend {
        Some(backend) => println!(
            "requested native backend: {} [{}]",
            backend.name(),
            backend.stability().label()
        ),
        None => println!("requested native backend: auto"),
    }
    println!("graphics-or-sound: {}", loaded.has_graphics);

    if let Some(error) = stability_status {
        println!("preview-gate: blocked");
        println!("gate-reason: {:#}", error);
    } else {
        println!("preview-gate: allowed");
    }

    match native_backend {
        Some(NativeBackendKind::CraneliftJit) => {
            println!("selected execution path: preview cranelift-jit runner");
            match native_codegen::supports_cranelift_jit(&loaded.program) {
                Ok(()) => println!("preview backend support: supported"),
                Err(error) => println!(
                    "preview backend support: not supported\npreview backend note: {error}"
                ),
            }
        }
        Some(NativeBackendKind::LlvmIr) => {
            println!("selected execution path: llvm-ir text backend only");
        }
        Some(NativeBackendKind::Rust) | None => match select_runtime_backend(&loaded.program) {
            Ok(selected) => println!("selected execution path: {}", selected.label()),
            Err(error) => println!(
                "selected execution path: unavailable\nselection note: {:#}",
                error
            ),
        },
    }

    println!(
        "native gaps: {}",
        if native_gaps.is_empty() {
            "none".to_string()
        } else {
            native_gaps.join(", ")
        }
    );
    println!(
        "vm gaps: {}",
        if vm_gaps.is_empty() {
            "none".to_string()
        } else {
            vm_gaps.join(", ")
        }
    );

    Ok(())
}

#[derive(Clone, Copy)]
struct ReleaseFixtureSpec {
    name: &'static str,
    category: &'static str,
    expect_graphics: bool,
    expected_runtime_backend: ExecutionBackend,
    require_native_codegen: bool,
}

fn release_fixture_specs() -> [ReleaseFixtureSpec; 4] {
    [
        ReleaseFixtureSpec {
            name: "release_validation_text.bas",
            category: "text",
            expect_graphics: false,
            expected_runtime_backend: ExecutionBackend::Native,
            require_native_codegen: true,
        },
        ReleaseFixtureSpec {
            name: "release_validation_graphics.bas",
            category: "graphics",
            expect_graphics: true,
            expected_runtime_backend: ExecutionBackend::Native,
            require_native_codegen: true,
        },
        ReleaseFixtureSpec {
            name: "release_validation_file_io.bas",
            category: "file-io",
            expect_graphics: false,
            expected_runtime_backend: ExecutionBackend::Native,
            require_native_codegen: true,
        },
        ReleaseFixtureSpec {
            name: "release_validation_vm_fallback.bas",
            category: "vm-fallback",
            expect_graphics: false,
            expected_runtime_backend: ExecutionBackend::Vm,
            require_native_codegen: false,
        },
    ]
}

struct ReleaseFixtureSummary {
    total: usize,
    graphics: usize,
    runtime_output: usize,
    vm_fallback: usize,
    native_codegen: usize,
}

fn summarize_release_fixtures(fixtures: &[ReleaseFixtureSpec]) -> Result<ReleaseFixtureSummary> {
    let mut summary = ReleaseFixtureSummary {
        total: fixtures.len(),
        graphics: 0,
        runtime_output: 0,
        vm_fallback: 0,
        native_codegen: 0,
    };

    for fixture in fixtures {
        if fixture.expect_graphics {
            summary.graphics += 1;
        }
        if fixture.expected_runtime_backend == ExecutionBackend::Vm {
            summary.vm_fallback += 1;
        }
        if fixture.require_native_codegen {
            summary.native_codegen += 1;
        }
        if release_fixture_expected_stdout(fixture.name)?.is_some() {
            summary.runtime_output += 1;
        }
    }

    Ok(summary)
}

fn format_release_fixture_descriptor(fixture: ReleaseFixtureSpec) -> Result<String> {
    let runtime_output = if release_fixture_expected_stdout(fixture.name)?.is_some() {
        "checked"
    } else {
        "none"
    };

    Ok(format!(
        "{} [kind={} backend={} graphics={} native-codegen={} runtime-output={}]",
        fixture.name,
        fixture.category,
        fixture.expected_runtime_backend.label(),
        if fixture.expect_graphics { "yes" } else { "no" },
        if fixture.require_native_codegen {
            "required"
        } else {
            "not-required"
        },
        runtime_output
    ))
}

fn format_release_fixture_progress(
    index: usize,
    total: usize,
    fixture: ReleaseFixtureSpec,
) -> Result<String> {
    Ok(format!(
        "[{}/{}] validated fixture: {}",
        index + 1,
        total,
        format_release_fixture_descriptor(fixture)?
    ))
}

fn validate_release_surface(quiet: bool) -> Result<()> {
    let frontend = syntax_tree::production_frontend();
    let native_backend = NativeBackendKind::Rust;

    if !frontend.is_production_ready() {
        return Err(anyhow::anyhow!(
            "production frontend '{}' is not marked production-ready",
            frontend.name()
        ));
    }
    if !native_backend.is_production_ready() {
        return Err(anyhow::anyhow!(
            "production native backend '{}' is not marked production-ready",
            native_backend.name()
        ));
    }

    let fixtures = release_fixture_specs();
    let summary = summarize_release_fixtures(&fixtures)?;

    for (index, fixture) in fixtures.into_iter().enumerate() {
        validate_release_fixture(frontend, native_backend, fixture)?;
        if !quiet {
            println!(
                "{}",
                format_release_fixture_progress(index, summary.total, fixture)?
            );
        }
    }

    if !quiet {
        println!(
            "release validation passed: frontend={} native-backend={} fixtures={} graphics={} runtime-output={} vm-fallback={} native-codegen={}",
            frontend.name(),
            native_backend.name(),
            summary.total,
            summary.graphics,
            summary.runtime_output,
            summary.vm_fallback,
            summary.native_codegen
        );
    }

    Ok(())
}

fn validate_pipeline_surface(
    frontend: FrontendKind,
    native_backend: Option<NativeBackendKind>,
    allow_preview: bool,
    quiet: bool,
) -> Result<()> {
    enforce_stability(frontend, native_backend, allow_preview)?;

    let fixtures = release_fixture_specs();
    let summary = summarize_release_fixtures(&fixtures)?;
    for (index, fixture) in fixtures.into_iter().enumerate() {
        validate_pipeline_fixture(frontend, native_backend, fixture)?;
        if !quiet {
            println!(
                "{}",
                format_release_fixture_progress(index, summary.total, fixture)?
            );
        }
    }

    if !quiet {
        println!(
            "pipeline validation passed: frontend={} native-backend={} fixtures={} graphics={} runtime-output={} vm-fallback={} native-codegen={}",
            frontend.name(),
            native_backend
                .map(NativeBackendKind::name)
                .unwrap_or("auto"),
            summary.total,
            summary.graphics,
            summary.runtime_output,
            summary.vm_fallback,
            summary.native_codegen
        );
    }

    Ok(())
}

fn release_fixture_path(fixture_name: &str) -> Result<PathBuf> {
    Ok(workspace_root_path()?
        .join("test")
        .join("fixtures")
        .join("basic")
        .join(fixture_name))
}

fn release_fixture_expected_stdout(fixture_name: &str) -> Result<Option<String>> {
    let expected_path = release_fixture_path(fixture_name)?.with_extension("out");
    match fs::read_to_string(&expected_path) {
        Ok(output) => Ok(Some(output.replace("\r\n", "\n"))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(anyhow::anyhow!(
            "failed to read expected output fixture '{}': {}",
            clean_path_display(&expected_path),
            error
        )),
    }
}

fn prepare_release_fixture(
    frontend: FrontendKind,
    fixture: ReleaseFixtureSpec,
) -> Result<(syntax_tree::Program, bool, Vec<vm_engine::OpCode>)> {
    let fixture_name = fixture.name;
    let source = fs::read_to_string(release_fixture_path(fixture_name)?)
        .with_context(|| format!("failed to read validation fixture '{fixture_name}'"))?;

    let program = parse_with_frontend(frontend, source)
        .map_err(|error| anyhow::anyhow!("fixture parse failed for {fixture_name}: {error}"))?;

    let has_graphics = has_graphics_or_sound(&program);
    if has_graphics != fixture.expect_graphics {
        return Err(anyhow::anyhow!(
            "validation fixture '{}' expected graphics={} but detected graphics={}",
            fixture_name,
            fixture.expect_graphics,
            has_graphics
        ));
    }

    let symbol_table = analyzer::scope::analyze_program(&program)
        .map_err(|error| anyhow::anyhow!("fixture analysis failed for {fixture_name}: {error}"))?;
    let mut type_checker = analyzer::TypeChecker::new(symbol_table);
    type_checker.check_program(&program).map_err(|error| {
        anyhow::anyhow!("fixture type checking failed for {fixture_name}: {error}")
    })?;

    let bytecode = vm_engine::BytecodeCompiler::new(program.clone())
        .compile()
        .map_err(|error| {
            anyhow::anyhow!("fixture VM compile failed for {fixture_name}: {error}")
        })?;

    Ok((program, has_graphics, bytecode))
}

fn validate_release_fixture(
    frontend: FrontendKind,
    native_backend: NativeBackendKind,
    fixture: ReleaseFixtureSpec,
) -> Result<()> {
    let (program, has_graphics, bytecode) = prepare_release_fixture(frontend, fixture)?;
    validate_release_fixture_behavior(&program, has_graphics, native_backend, fixture)?;
    validate_fixture_runtime_output(&bytecode, fixture)
}

fn validate_release_fixture_behavior(
    program: &syntax_tree::Program,
    has_graphics: bool,
    native_backend: NativeBackendKind,
    fixture: ReleaseFixtureSpec,
) -> Result<()> {
    let fixture_name = fixture.name;
    let selected_backend = select_runtime_backend(&program).map_err(|error| {
        anyhow::anyhow!("release validation backend selection failed for {fixture_name}: {error:#}")
    })?;
    if selected_backend != fixture.expected_runtime_backend {
        return Err(anyhow::anyhow!(
            "release validation fixture '{}' expected runtime backend '{}' but selected '{}'",
            fixture_name,
            fixture.expected_runtime_backend.label(),
            selected_backend.label()
        ));
    }

    if fixture.require_native_codegen {
        native_codegen::generate_with_backend(
            program,
            native_backend,
            native_codegen::NativeBackendOptions {
                graphics: has_graphics,
            },
        )
        .map_err(|error| {
            anyhow::anyhow!("release validation native codegen failed for {fixture_name}: {error}")
        })?;
    } else if syntax_tree::unsupported_statements(&program, syntax_tree::Backend::Native).is_empty()
    {
        return Err(anyhow::anyhow!(
            "release validation fixture '{}' was expected to require VM fallback, but native support is currently reported as complete",
            fixture_name
        ));
    }

    Ok(())
}

fn validate_pipeline_fixture(
    frontend: FrontendKind,
    native_backend: Option<NativeBackendKind>,
    fixture: ReleaseFixtureSpec,
) -> Result<()> {
    let fixture_name = fixture.name;
    let (program, has_graphics, bytecode) = prepare_release_fixture(frontend, fixture)?;

    match native_backend {
        Some(NativeBackendKind::CraneliftJit) => {
            ensure_cranelift_preview_support(&program).map_err(|error| {
                anyhow::anyhow!(
                    "pipeline validation preview backend failed for {}: {:#}",
                    fixture_name,
                    error
                )
            })?;
        }
        Some(NativeBackendKind::LlvmIr) => {
            native_codegen::generate_with_backend(
                &program,
                NativeBackendKind::LlvmIr,
                native_codegen::NativeBackendOptions {
                    graphics: has_graphics,
                },
            )
            .map_err(|error| {
                anyhow::anyhow!(
                    "pipeline validation native codegen failed for {}: {}",
                    fixture_name,
                    error
                )
            })?;
        }
        Some(NativeBackendKind::Rust) | None => validate_release_fixture_behavior(
            &program,
            has_graphics,
            NativeBackendKind::Rust,
            fixture,
        )?,
    }

    validate_fixture_runtime_output(&bytecode, fixture)?;

    Ok(())
}

fn validate_fixture_runtime_output(
    bytecode: &[vm_engine::OpCode],
    fixture: ReleaseFixtureSpec,
) -> Result<()> {
    let Some(expected_stdout) = release_fixture_expected_stdout(fixture.name)? else {
        return Ok(());
    };

    let temp_dir = unique_temp_path("release_fixture_runtime");
    fs::create_dir_all(&temp_dir)?;

    let result = (|| -> Result<()> {
        let _cwd_guard = CurrentDirGuard::change_to(&temp_dir).with_context(|| {
            format!(
                "failed to switch runtime validation working directory to '{}'",
                clean_path_display(&temp_dir)
            )
        })?;

        let mut vm = vm_engine::VM::new(bytecode.to_vec());
        vm.enable_stdout_capture();
        vm.run().map_err(|error| {
            anyhow::anyhow!(
                "runtime validation VM execution failed for {}: {}",
                fixture.name,
                error
            )
        })?;

        let stdout = normalize_runtime_output(&vm.take_captured_stdout());
        if stdout != expected_stdout {
            return Err(anyhow::anyhow!(
                "runtime validation output mismatch for {}\nexpected:\n{}\nactual:\n{}",
                fixture.name,
                expected_stdout,
                stdout
            ));
        }

        Ok(())
    })();

    let _ = fs::remove_dir_all(&temp_dir);
    result
}

fn command_failure(action: &str, output: &std::process::Output) -> anyhow::Error {
    let mut message = action.to_string();

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stdout.is_empty() {
        message.push_str("\nstdout:\n");
        message.push_str(&stdout);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !stderr.is_empty() {
        message.push_str("\nstderr:\n");
        message.push_str(&stderr);
    }

    anyhow::anyhow!(message)
}

fn normalize_runtime_output(stdout: &str) -> String {
    stdout.replace("\r\n", "\n").replace('\r', "")
}

struct CurrentDirGuard {
    previous: PathBuf,
}

impl CurrentDirGuard {
    fn change_to(path: &Path) -> Result<Self> {
        let previous = env::current_dir().context("failed to capture current working directory")?;
        env::set_current_dir(path).with_context(|| {
            format!(
                "failed to enter working directory '{}'",
                clean_path_display(path)
            )
        })?;
        Ok(Self { previous })
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        let _ = env::set_current_dir(&self.previous);
    }
}

fn unique_temp_path(prefix: &str) -> PathBuf {
    let tag = env::var("QBNEX_TEMP_TAG")
        .ok()
        .map(|value| {
            value
                .chars()
                .map(|ch| {
                    if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                        ch
                    } else {
                        '_'
                    }
                })
                .collect::<String>()
        })
        .filter(|value| !value.is_empty());
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = TEMP_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
    let name = if let Some(tag) = tag {
        format!(
            "{prefix}_{tag}_{}_{}_{}",
            std::process::id(),
            nonce,
            counter
        )
    } else {
        format!("{prefix}_{}_{}_{}", std::process::id(), nonce, counter)
    };
    std::env::temp_dir().join(name)
}

struct TempWorkspace {
    path: PathBuf,
}

impl TempWorkspace {
    fn new(prefix: &str) -> Result<Self> {
        let path = unique_temp_path(prefix);
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Debug)]
struct ProgramExitError {
    code: i32,
}

impl std::fmt::Display for ProgramExitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "compiled program exited with status {}", self.code)
    }
}

impl std::error::Error for ProgramExitError {}

#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
#[diagnostic(code(qbnex::source))]
#[allow(unused_assignments)]
struct SourceDiagnostic {
    message: String,
    #[source_code]
    source_code: NamedSource<String>,
    #[label("here")]
    span: Option<SourceSpan>,
    help: Option<String>,
}

impl SourceDiagnostic {
    fn from_qerror(path: &Path, source: String, error: QError) -> Self {
        Self {
            message: error.message().into_owned(),
            source_code: NamedSource::new(path.display().to_string(), source),
            span: error.source_span().map(|(offset, len)| (offset, len).into()),
            help: match error {
                QError::Syntax(_) | QError::SyntaxAt { .. } => Some(
                    "Check the highlighted source span. If this file is a QB64 fragment, compile the owning root source file instead.".to_string(),
                ),
                _ => None,
            },
        }
    }
}

struct LoadedProgram {
    source: String,
    program: syntax_tree::Program,
    has_graphics: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExecutionBackend {
    Vm,
    Native,
}

impl ExecutionBackend {
    fn label(self) -> &'static str {
        match self {
            ExecutionBackend::Vm => "vm-fallback",
            ExecutionBackend::Native => "native",
        }
    }
}

static TEMP_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

fn parse_include_path(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let directive = trimmed.strip_prefix('\'').unwrap_or(trimmed).trim_start();

    if !directive
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("$INCLUDE"))
    {
        return None;
    }

    let rest = directive
        .get(8..)?
        .trim_start()
        .strip_prefix(':')?
        .trim_start();
    let quote = rest.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }

    let after_quote = rest.get(quote.len_utf8()..)?;
    let end = after_quote.find(quote)?;
    let include_path = after_quote[..end].trim();
    if include_path.is_empty() {
        None
    } else {
        Some(include_path.to_string())
    }
}

fn normalized_include_path(include_path: &str) -> PathBuf {
    include_path
        .split(['\\', '/'])
        .filter(|segment| !segment.is_empty())
        .fold(PathBuf::new(), |mut path, segment| {
            path.push(segment);
            path
        })
}

fn clean_path_display(path: &Path) -> String {
    let raw = path.display().to_string();
    raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string()
}

struct SourceExpander {
    cache: HashMap<PathBuf, String>,
    active_stack: Vec<PathBuf>,
}

impl SourceExpander {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            active_stack: Vec::new(),
        }
    }

    fn expand(&mut self, path: &Path) -> Result<String> {
        let canonical = fs::canonicalize(path).with_context(|| {
            format!("cannot resolve source file '{}'", clean_path_display(path))
        })?;

        if let Some(cycle_start) = self
            .active_stack
            .iter()
            .position(|entry| entry == &canonical)
        {
            let mut cycle = self.active_stack[cycle_start..]
                .iter()
                .map(|entry| clean_path_display(entry))
                .collect::<Vec<_>>();
            cycle.push(clean_path_display(&canonical));
            return Err(anyhow::anyhow!(
                "cyclic $INCLUDE detected: {}",
                cycle.join(" -> ")
            ));
        }

        if let Some(expanded) = self.cache.get(&canonical) {
            return Ok(expanded.clone());
        }

        let source = fs::read_to_string(&canonical).with_context(|| {
            format!(
                "cannot read source file '{}'",
                clean_path_display(&canonical)
            )
        })?;
        let parent = canonical
            .parent()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "source file '{}' has no parent directory",
                    clean_path_display(&canonical)
                )
            })?
            .to_path_buf();

        self.active_stack.push(canonical.clone());
        let expanded_result: Result<String> = (|| {
            let mut expanded = String::new();
            for line in source.split_inclusive('\n') {
                if let Some(include_path) = parse_include_path(line) {
                    let resolved_include = parent.join(normalized_include_path(&include_path));
                    let included_source = self.expand(&resolved_include).with_context(|| {
                        format!(
                            "while expanding $INCLUDE '{}' from '{}'",
                            include_path,
                            clean_path_display(&canonical)
                        )
                    })?;
                    expanded.push_str(&included_source);
                    if !included_source.ends_with('\n') {
                        expanded.push('\n');
                    }
                } else {
                    expanded.push_str(line);
                }
            }

            if !source.ends_with('\n') && !expanded.ends_with('\n') {
                expanded.push('\n');
            }

            Ok(expanded)
        })();
        self.active_stack.pop();

        let expanded = expanded_result?;
        self.cache.insert(canonical, expanded.clone());
        Ok(expanded)
    }
}

fn load_source(input: &str) -> Result<String> {
    SourceExpander::new().expand(Path::new(input))
}

fn is_basic_source_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        ext.eq_ignore_ascii_case("bas")
            || ext.eq_ignore_ascii_case("bi")
            || ext.eq_ignore_ascii_case("bm")
    })
}

fn should_skip_source_scan_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                ".git" | ".hg" | ".svn" | ".idea" | ".vscode" | "target" | "node_modules"
            )
        })
}

fn collect_basic_source_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in
        fs::read_dir(root).with_context(|| format!("cannot read directory '{}'", root.display()))?
    {
        let entry =
            entry.with_context(|| format!("cannot read entry under '{}'", root.display()))?;
        let path = entry.path();
        if path.is_dir() {
            if !should_skip_source_scan_dir(&path) {
                collect_basic_source_files(&path, files)?;
            }
        } else if is_basic_source_file(&path) {
            files.push(path);
        }
    }

    Ok(())
}

fn display_path(path: &Path) -> String {
    std::env::current_dir()
        .ok()
        .and_then(|cwd| {
            path.strip_prefix(&cwd)
                .ok()
                .map(clean_path_display)
                .or_else(|| {
                    fs::canonicalize(path).ok().and_then(|canonical_path| {
                        fs::canonicalize(&cwd).ok().and_then(|canonical_cwd| {
                            canonical_path
                                .strip_prefix(&canonical_cwd)
                                .ok()
                                .map(clean_path_display)
                        })
                    })
                })
        })
        .unwrap_or_else(|| clean_path_display(path))
}

fn build_reverse_include_map(search_root: &Path) -> Result<HashMap<PathBuf, Vec<PathBuf>>> {
    let canonical_root = fs::canonicalize(search_root).with_context(|| {
        format!(
            "cannot resolve source tree '{}'",
            clean_path_display(search_root)
        )
    })?;
    let mut sources = Vec::new();
    collect_basic_source_files(&canonical_root, &mut sources)?;

    let mut reverse_includes = HashMap::new();
    for source in sources {
        let canonical_source = match fs::canonicalize(&source) {
            Ok(path) => path,
            Err(_) => continue,
        };
        let parent = match canonical_source.parent() {
            Some(parent) => parent,
            None => continue,
        };
        let contents = match fs::read_to_string(&canonical_source) {
            Ok(contents) => contents,
            Err(_) => continue,
        };

        for include in contents.lines().filter_map(parse_include_path) {
            let resolved = parent.join(normalized_include_path(&include));
            let canonical_target = match fs::canonicalize(&resolved) {
                Ok(path) => path,
                Err(_) => continue,
            };

            if canonical_target.starts_with(&canonical_root) {
                reverse_includes
                    .entry(canonical_target)
                    .or_insert_with(Vec::new)
                    .push(canonical_source.clone());
            }
        }
    }

    for parents in reverse_includes.values_mut() {
        parents.sort();
        parents.dedup();
    }

    Ok(reverse_includes)
}

fn collect_owning_roots(
    start: &Path,
    reverse_includes: &HashMap<PathBuf, Vec<PathBuf>>,
) -> Vec<PathBuf> {
    let mut visited = HashSet::new();
    let mut stack = vec![start.to_path_buf()];
    let mut roots = Vec::new();

    while let Some(node) = stack.pop() {
        if !visited.insert(node.clone()) {
            continue;
        }

        match reverse_includes.get(&node) {
            Some(parents) if !parents.is_empty() => stack.extend(parents.iter().cloned()),
            _ if node
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("bas")) =>
            {
                roots.push(node);
            }
            _ => {}
        }
    }

    roots.sort();
    roots.dedup();
    roots
}

fn fragment_search_roots(canonical: &Path) -> Vec<PathBuf> {
    let canonical_cwd = env::current_dir()
        .ok()
        .and_then(|cwd| fs::canonicalize(cwd).ok());

    let mut search_roots = Vec::new();
    for ancestor in canonical.ancestors().skip(1) {
        if let Some(cwd) = canonical_cwd.as_ref() {
            if canonical.starts_with(cwd) {
                if ancestor.starts_with(cwd) {
                    search_roots.push(ancestor.to_path_buf());
                    if ancestor == cwd {
                        break;
                    }
                }
                continue;
            }
        }

        search_roots.push(ancestor.to_path_buf());
        if search_roots.len() >= 6 {
            break;
        }
    }

    search_roots
}

fn find_fragment_roots(path: &Path) -> Vec<PathBuf> {
    let canonical = match fs::canonicalize(path) {
        Ok(path) => path,
        Err(_) => return Vec::new(),
    };

    let mut discovered_roots = Vec::new();
    for search_root in fragment_search_roots(&canonical) {
        let reverse_includes = match build_reverse_include_map(&search_root) {
            Ok(map) => map,
            Err(_) => continue,
        };

        if !reverse_includes.contains_key(&canonical) {
            continue;
        }

        let roots = collect_owning_roots(&canonical, &reverse_includes);
        if !roots.is_empty() {
            discovered_roots = roots;
        }
    }

    discovered_roots
}

fn fragment_context_note(input: &str) -> Option<String> {
    let roots = find_fragment_roots(Path::new(input));
    if roots.is_empty() {
        return None;
    }

    let shown = roots
        .iter()
        .take(3)
        .map(|path| display_path(path))
        .collect::<Vec<_>>();

    let mut note = format!(
        "'{}' appears to be a project fragment owned by {}",
        display_path(Path::new(input)),
        shown.join(", ")
    );
    if roots.len() > shown.len() {
        note.push_str(&format!(
            " and {} more root source file(s)",
            roots.len() - shown.len()
        ));
    }
    note.push_str(". Compile the owning root source file instead of the fragment directly.");
    Some(note)
}

fn retry_with_fragment_root<T, F>(input: &str, quiet: bool, mut action: F) -> Result<T>
where
    F: FnMut(&str, &str) -> Result<T>,
{
    match action(input, input) {
        Ok(value) => Ok(value),
        Err(original_error) => {
            let roots = find_fragment_roots(Path::new(input));
            if roots.len() == 1 {
                let root = &roots[0];
                if fs::canonicalize(input)
                    .ok()
                    .is_some_and(|path| path != *root)
                {
                    if !quiet {
                        eprintln!(
                            "note: '{}' is a project fragment; retrying with root source '{}'",
                            display_path(Path::new(input)),
                            display_path(root)
                        );
                    }

                    let root_string = root.to_string_lossy().into_owned();
                    return action(&root_string, input).map_err(|retry_error| {
                        retry_error.context(format!(
                            "retrying via owning root '{}' also failed",
                            display_path(root)
                        ))
                    });
                }
            }

            if let Some(note) = fragment_context_note(input) {
                Err(original_error.context(note))
            } else {
                Err(original_error)
            }
        }
    }
}

fn load_program(input: &str, frontend: FrontendKind) -> Result<LoadedProgram> {
    let source = load_source(input)?;
    let path = Path::new(input);
    let program = parse_with_frontend(frontend, source.clone()).map_err(|error| {
        anyhow::Error::new(SourceDiagnostic::from_qerror(path, source.clone(), error))
    })?;
    let has_graphics = has_graphics_or_sound(&program);
    Ok(LoadedProgram {
        source,
        program,
        has_graphics,
    })
}

fn format_backend_gaps(gaps: &[&str]) -> String {
    if gaps.is_empty() {
        "supported".to_string()
    } else {
        gaps.join(", ")
    }
}

fn unsupported_backend_error(program: &syntax_tree::Program) -> anyhow::Error {
    let native = syntax_tree::unsupported_statements(program, syntax_tree::Backend::Native);
    let vm = syntax_tree::unsupported_statements(program, syntax_tree::Backend::Vm);
    anyhow::anyhow!(
        "program uses unsupported statements\n  native compiler: {}\n  interpreter: {}",
        format_backend_gaps(&native),
        format_backend_gaps(&vm)
    )
}

fn ensure_cranelift_preview_support(program: &syntax_tree::Program) -> Result<()> {
    native_codegen::supports_cranelift_jit(program).map_err(|error| {
        anyhow::anyhow!(
            "preview backend 'cranelift-jit' cannot compile this program yet: {}\nrerun without --native-backend for the production auto-selected path, or keep --allow-preview and simplify the program to the current preview subset",
            error
        )
    })
}

fn select_runtime_backend(program: &syntax_tree::Program) -> Result<ExecutionBackend> {
    let native = syntax_tree::unsupported_statements(program, syntax_tree::Backend::Native);
    if native.is_empty() {
        return Ok(ExecutionBackend::Native);
    }

    let vm = syntax_tree::unsupported_statements(program, syntax_tree::Backend::Vm);
    if vm.is_empty() {
        return Ok(ExecutionBackend::Vm);
    }

    Err(unsupported_backend_error(program))
}

const COMMAND_LINE_ENV: &str = "QBNEX_COMMAND_LINE";

fn command_tail(program_args: &[String]) -> String {
    program_args.join(" ")
}

fn run_compiled_binary(
    binary_path: &Path,
    program_args: &[String],
) -> Result<std::process::ExitStatus> {
    let mut command = std::process::Command::new(binary_path);
    command.args(program_args);
    command.env(COMMAND_LINE_ENV, command_tail(program_args));
    command.status().map_err(|e| {
        anyhow::anyhow!(
            "failed to run compiled program '{}': {}",
            binary_path.display(),
            e
        )
    })
}

fn ensure_run_success(run_status: std::process::ExitStatus) -> Result<()> {
    if run_status.success() {
        return Ok(());
    }

    Err(ProgramExitError {
        code: run_status.code().unwrap_or(1),
    }
    .into())
}

fn output_spec_looks_like_directory(output_path: &str) -> bool {
    output_path.ends_with('\\') || output_path.ends_with('/')
}

fn resolve_output_path(logical_input: &str, output_path: Option<&str>) -> Result<PathBuf> {
    let default_name = default_output_name(logical_input);
    let raw_output = output_path.unwrap_or(&default_name);
    let mut resolved = std::env::current_dir()?.join(raw_output);

    if output_path.is_some() && (output_spec_looks_like_directory(raw_output) || resolved.is_dir())
    {
        resolved = resolved.join(default_name);
    }

    if let Some(parent) = resolved.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "cannot create output directory '{}'",
                    clean_path_display(parent)
                )
            })?;
        }
    }
    Ok(resolved)
}

fn copy_built_binary(
    temp_dir: &Path,
    output_path: &Path,
    run: bool,
    program_args: &[String],
) -> Result<()> {
    let target_bin = temp_dir.join("target/release/qbnex_app.exe");
    let dest = output_path.to_path_buf();
    if target_bin.exists() {
        fs::copy(&target_bin, &dest)?;
    } else {
        let target_bin = temp_dir.join("target/release/qbnex_app");
        if target_bin.exists() {
            fs::copy(&target_bin, &dest)?;
        } else {
            return Err(anyhow::anyhow!("compiled binary not found"));
        }
    }

    if run {
        let run_status = run_compiled_binary(&dest, program_args)?;
        let run_result = ensure_run_success(run_status);
        if run_result.is_err() {
            let _ = fs::remove_dir_all(temp_dir);
        }
        run_result?;
    }

    Ok(())
}

fn write_windows_static_config(temp_dir: &Path) -> Result<()> {
    if cfg!(target_os = "windows") {
        let dot_cargo = temp_dir.join(".cargo");
        fs::create_dir_all(&dot_cargo)?;
        let cargo_config = r#"[target.'cfg(all(windows, target_env = "msvc"))']
rustflags = ["-C", "target-feature=+crt-static"]
"#;
        fs::write(dot_cargo.join("config.toml"), cargo_config)?;
    }
    Ok(())
}

fn workspace_root_path() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow::anyhow!("failed to locate workspace root"))
}

fn workspace_paths() -> Result<(String, String, String, String, String)> {
    let repo_root = workspace_root_path()?;

    let normalize = |name: &str| repo_root.join(name).to_string_lossy().replace('\\', "/");

    Ok((
        normalize("core_types"),
        normalize("hal_layer"),
        normalize("syntax_tree"),
        normalize("analyzer"),
        normalize("vm_engine"),
    ))
}

fn build_with_rustc(
    program: &syntax_tree::Program,
    logical_input: &str,
    output_path: Option<&str>,
    run: bool,
    program_args: &[String],
) -> Result<()> {
    let rust_code = native_codegen::generate_with_backend(
        program,
        native_codegen::NativeBackendKind::Rust,
        native_codegen::NativeBackendOptions::default(),
    )?;
    let resolved_output = resolve_output_path(logical_input, output_path)?;

    let rust_file = unique_temp_path("qb").with_extension("rs");
    fs::write(&rust_file, rust_code)
        .map_err(|e| anyhow::anyhow!("cannot write temp file: {}", e))?;

    let mut cmd = std::process::Command::new("rustc");
    cmd.args(["-O", "--edition", "2021"]);

    if cfg!(target_os = "windows") {
        cmd.args(["-C", "link-arg=/DEBUG:NONE"]);
    } else {
        cmd.args(["-C", "strip=symbols"]);
    }

    let output = cmd
        .arg("-o")
        .arg(&resolved_output)
        .arg(&rust_file)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run rustc: {}. Make sure Rust is installed.", e))?;

    let _ = fs::remove_file(&rust_file);

    if !output.status.success() {
        return Err(command_failure("rustc compilation failed", &output));
    }

    if run {
        let run_status = run_compiled_binary(&resolved_output, program_args)?;
        ensure_run_success(run_status)?;
    }

    Ok(())
}

fn build_with_cargo(
    logical_input: &str,
    program: &syntax_tree::Program,
    output: Option<&str>,
    run: bool,
    program_args: &[String],
) -> Result<()> {
    let resolved_output = resolve_output_path(logical_input, output)?;

    let rust_code = native_codegen::generate_with_backend(
        program,
        native_codegen::NativeBackendKind::Rust,
        native_codegen::NativeBackendOptions { graphics: true },
    )?;

    let temp_workspace = TempWorkspace::new("qbnex_build")?;
    let temp_dir = temp_workspace.path();

    let (core_types_path, hal_layer_path, _, _, _) = workspace_paths()?;

    let cargo_toml = format!(
        r#"[package]
name = "qbnex_app"
version = "0.1.0"
edition = "2021"

[dependencies]
core_types = {{ path = "{}" }}
hal_layer = {{ path = "{}" }}
minifb = "0.28.0"

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"
"#,
        core_types_path, hal_layer_path
    );
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml)?;

    write_windows_static_config(temp_dir)?;

    let src_dir = temp_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(src_dir.join("main.rs"), rust_code)?;

    let output = std::process::Command::new("cargo")
        .args(["build", "--release", "--quiet"])
        .current_dir(temp_dir)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run cargo: {}", e))?;

    if !output.status.success() {
        return Err(command_failure("cargo compilation failed", &output));
    }

    copy_built_binary(temp_dir, &resolved_output, run, program_args)?;

    Ok(())
}

fn build_with_vm_bundle(
    logical_input: &str,
    source: &str,
    output: Option<&str>,
    run: bool,
    program_args: &[String],
    frontend: FrontendKind,
) -> Result<()> {
    let resolved_output = resolve_output_path(logical_input, output)?;

    let temp_workspace = TempWorkspace::new("qbnex_vm_build")?;
    let temp_dir = temp_workspace.path();

    let (core_types_path, hal_layer_path, syntax_tree_path, analyzer_path, vm_engine_path) =
        workspace_paths()?;

    let cargo_toml = format!(
        r#"[package]
name = "qbnex_app"
version = "0.1.0"
edition = "2021"

[dependencies]
core_types = {{ path = "{}" }}
hal_layer = {{ path = "{}" }}
syntax_tree = {{ path = "{}" }}
analyzer = {{ path = "{}" }}
vm_engine = {{ path = "{}" }}

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"
"#,
        core_types_path, hal_layer_path, syntax_tree_path, analyzer_path, vm_engine_path
    );
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml)?;
    write_windows_static_config(temp_dir)?;

    let main_rs = format!(
        r#"use analyzer::scope::analyze_program;
use syntax_tree::parse_with_frontend;
use vm_engine::{{BytecodeCompiler, VM}};

fn main() {{
    if let Err(err) = run() {{
        eprintln!("{{}}", err);
        std::process::exit(1);
    }}
}}

fn run() -> Result<(), Box<dyn std::error::Error>> {{
    let source = {source:?};
    let frontend: syntax_tree::FrontendKind = {frontend_name:?}.parse()?;
    let program = parse_with_frontend(frontend, source.to_string())?;
    let symbol_table = analyze_program(&program)?;
    let mut type_checker = analyzer::TypeChecker::new(symbol_table);
    type_checker.check_program(&program)?;
    let mut compiler = BytecodeCompiler::new(program);
    let bytecode = compiler.compile()?;
    let mut vm = VM::new(bytecode);
    vm.run()?;
    Ok(())
}}
"#,
        frontend_name = frontend.name()
    );

    let src_dir = temp_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(src_dir.join("main.rs"), main_rs)?;

    let output = std::process::Command::new("cargo")
        .args(["build", "--release", "--quiet"])
        .current_dir(temp_dir)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run cargo: {}", e))?;

    if !output.status.success() {
        return Err(command_failure("cargo compilation failed", &output));
    }

    copy_built_binary(temp_dir, &resolved_output, run, program_args)?;

    Ok(())
}

fn build_with_cranelift_bundle(
    logical_input: &str,
    source: &str,
    output: Option<&str>,
    run: bool,
    program_args: &[String],
    frontend: FrontendKind,
) -> Result<()> {
    let resolved_output = resolve_output_path(logical_input, output)?;

    let temp_workspace = TempWorkspace::new("qbnex_cranelift_build")?;
    let temp_dir = temp_workspace.path();

    let (core_types_path, _, syntax_tree_path, _, _) = workspace_paths()?;
    let native_codegen_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow::anyhow!("failed to locate workspace root"))?
        .join("native_codegen")
        .to_string_lossy()
        .replace('\\', "/");

    let cargo_toml = format!(
        r#"[package]
name = "qbnex_app"
version = "0.1.0"
edition = "2021"

[dependencies]
core_types = {{ path = "{}" }}
syntax_tree = {{ path = "{}" }}
native_codegen = {{ path = "{}" }}

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"
"#,
        core_types_path, syntax_tree_path, native_codegen_path
    );
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml)?;
    write_windows_static_config(temp_dir)?;

    let main_rs = format!(
        r#"use native_codegen::{{run_with_cranelift_jit, supports_cranelift_jit}};
use syntax_tree::parse_with_frontend;

fn main() {{
    if let Err(err) = run() {{
        eprintln!("{{}}", err);
        std::process::exit(1);
    }}
}}

fn run() -> Result<(), Box<dyn std::error::Error>> {{
    let source = {source:?};
    let frontend: syntax_tree::FrontendKind = {frontend_name:?}.parse()?;
    let program = parse_with_frontend(frontend, source.to_string())?;
    supports_cranelift_jit(&program)?;
    run_with_cranelift_jit(&program)?;
    Ok(())
}}
"#,
        frontend_name = frontend.name()
    );

    let src_dir = temp_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(src_dir.join("main.rs"), main_rs)?;

    let output = std::process::Command::new("cargo")
        .args(["build", "--release", "--quiet"])
        .current_dir(temp_dir)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run cargo: {}", e))?;

    if !output.status.success() {
        return Err(command_failure("cargo compilation failed", &output));
    }

    copy_built_binary(temp_dir, &resolved_output, run, program_args)?;
    Ok(())
}

// ──────────────────────────────────────────────
//  Compile and Run (default mode like QB64)
// ──────────────────────────────────────────────

fn compile_and_run(
    input: &str,
    logical_input: &str,
    output: Option<&str>,
    program_args: &[String],
    quiet: bool,
    frontend: FrontendKind,
    native_backend: Option<NativeBackendKind>,
) -> Result<()> {
    use std::time::Instant;

    let start = Instant::now();

    let loaded = load_program(input, frontend)?;
    match native_backend {
        Some(NativeBackendKind::CraneliftJit) => {
            ensure_cranelift_preview_support(&loaded.program)?;
            build_with_cranelift_bundle(
                logical_input,
                &loaded.source,
                output,
                true,
                program_args,
                frontend,
            )?;
        }
        Some(NativeBackendKind::LlvmIr) => {
            return Err(anyhow::anyhow!(
                "--native-backend llvm-ir is not executable yet; use rust or cranelift-jit"
            ));
        }
        Some(NativeBackendKind::Rust) | None => match select_runtime_backend(&loaded.program)? {
            ExecutionBackend::Vm => {
                build_with_vm_bundle(
                    logical_input,
                    &loaded.source,
                    output,
                    true,
                    program_args,
                    frontend,
                )?;
            }
            ExecutionBackend::Native => {
                if loaded.has_graphics {
                    build_with_cargo(logical_input, &loaded.program, output, true, program_args)?;
                } else {
                    build_with_rustc(&loaded.program, logical_input, output, true, program_args)?;
                }
            }
        },
    }

    if !quiet {
        println!("\nprogram finished in {:.2?}.", start.elapsed());
    }

    Ok(())
}

// ──────────────────────────────────────────────
//  Run (interpreter) - OPTIMIZED with timeout
// ──────────────────────────────────────────────

fn run_file(
    filename: &str,
    logical_input: &str,
    output: Option<&str>,
    program_args: &[String],
    quiet: bool,
    frontend: FrontendKind,
    native_backend: Option<NativeBackendKind>,
) -> Result<()> {
    use std::time::Instant;

    let start = Instant::now();

    let loaded = load_program(filename, frontend)?;
    match native_backend {
        Some(NativeBackendKind::CraneliftJit) => {
            ensure_cranelift_preview_support(&loaded.program)?;
            build_with_cranelift_bundle(
                logical_input,
                &loaded.source,
                output,
                true,
                program_args,
                frontend,
            )?;
        }
        Some(NativeBackendKind::LlvmIr) => {
            return Err(anyhow::anyhow!(
                "--native-backend llvm-ir is not executable yet; use rust or cranelift-jit"
            ));
        }
        Some(NativeBackendKind::Rust) | None => {
            let vm_gaps =
                syntax_tree::unsupported_statements(&loaded.program, syntax_tree::Backend::Vm);
            if vm_gaps.is_empty() {
                build_with_vm_bundle(
                    logical_input,
                    &loaded.source,
                    output,
                    true,
                    program_args,
                    frontend,
                )?;
            } else if syntax_tree::unsupported_statements(
                &loaded.program,
                syntax_tree::Backend::Native,
            )
            .is_empty()
            {
                if loaded.has_graphics {
                    build_with_cargo(logical_input, &loaded.program, output, true, program_args)?;
                } else {
                    build_with_rustc(&loaded.program, logical_input, output, true, program_args)?;
                }
            } else {
                return Err(unsupported_backend_error(&loaded.program));
            }
        }
    }

    let elapsed = start.elapsed();

    if !quiet {
        println!("\nprogram finished in {:.2?}.", elapsed);
    }
    Ok(())
}

// ──────────────────────────────────────────────
//  Compile - Build native .exe using Rust compiler
// ──────────────────────────────────────────────

fn default_output_name(input: &str) -> String {
    let stem = Path::new(input)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();

    if cfg!(target_os = "windows") {
        format!("{}.exe", stem)
    } else {
        stem.to_string()
    }
}

fn build_file(
    input: &str,
    logical_input: &str,
    output: Option<&str>,
    quiet: bool,
    frontend: FrontendKind,
    native_backend: Option<NativeBackendKind>,
) -> Result<()> {
    let _ = quiet;
    let loaded = load_program(input, frontend)?;
    match native_backend {
        Some(NativeBackendKind::CraneliftJit) => {
            ensure_cranelift_preview_support(&loaded.program)?;
            build_with_cranelift_bundle(logical_input, &loaded.source, output, false, &[], frontend)
        }
        Some(NativeBackendKind::LlvmIr) => Err(anyhow::anyhow!(
            "--native-backend llvm-ir does not emit standalone executables yet"
        )),
        Some(NativeBackendKind::Rust) | None => {
            let native_gaps =
                syntax_tree::unsupported_statements(&loaded.program, syntax_tree::Backend::Native);
            if native_gaps.is_empty() {
                if loaded.has_graphics {
                    build_with_cargo(logical_input, &loaded.program, output, false, &[])
                } else {
                    build_with_rustc(&loaded.program, logical_input, output, false, &[])
                }
            } else if syntax_tree::unsupported_statements(&loaded.program, syntax_tree::Backend::Vm)
                .is_empty()
            {
                build_with_vm_bundle(logical_input, &loaded.source, output, false, &[], frontend)
            } else {
                Err(unsupported_backend_error(&loaded.program))
            }
        }
    }
}
