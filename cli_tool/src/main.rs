use anyhow::Result;
use std::env;
use std::fs;
use std::path::Path;

mod feature_check;
use feature_check::has_graphics_or_sound;

// ──────────────────────────────────────────────
//  Argument parsing
// ──────────────────────────────────────────────

#[derive(Default)]
struct Args {
    file: Option<String>,
    compile: bool,
    output: Option<String>,
    run: bool,
    quiet: bool,
    explicit: bool,
    help: bool,
    version: bool,
}

fn parse_args() -> Args {
    let raw: Vec<String> = env::args().skip(1).collect();
    let mut a = Args::default();
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "-h" | "--help" => a.help = true,
            "-v" | "--version" => a.version = true,
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

fn main() -> Result<()> {
    let args = parse_args();

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.explicit {
        unsafe {
            std::env::set_var("QBNEX_EXPLICIT", "1");
        }
    }

    // Show help when: -h flag OR no file AND no action flag
    if args.help || args.file.is_none() {
        print_help();
        return Ok(());
    }

    if args.compile || args.output.is_some() {
        match &args.file {
            Some(f) => build_file(f, args.output.as_deref(), args.quiet)?,
            None => {
                eprintln!("error: no input file specified for compilation");
                eprintln!("       usage: qb -c <file.bas>");
                std::process::exit(1);
            }
        }
    } else if args.run {
        match &args.file {
            Some(f) => run_file(f, args.quiet)?,
            None => {
                eprintln!("error: no input file specified");
                eprintln!("       usage: qb -x <file.bas>");
                std::process::exit(1);
            }
        }
    } else {
        // Default mode: Compile and Run (like QB64)
        match &args.file {
            Some(f) => compile_and_run(f, args.output.as_deref(), args.quiet)?,
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
    -x                     Run FILE through the interpreter (no compile)
    -e                     Enable OPTION _EXPLICIT (force variable declaration)
    -w                     Show warnings
    -q                     Quiet mode (suppress non-error output)
    -m                     Monochrome output
    -h, --help             Show this help message
    -v, --version          Show version

EXAMPLES:
    qb hello.bas           Compile & run hello.bas (default)
    qb -x hello.bas        Run hello.bas via interpreter
    qb -c hello.bas        Compile  -->  hello.exe
    qb -c -o out.exe a.bas Compile  -->  out.exe
    qb -e main.bas         Compile & run with forced variable declaration"#
    );
}

fn print_command_output_on_error(output: &std::process::Output) {
    use std::io::{self, Write};

    let mut stderr = io::stderr();

    if !output.stdout.is_empty() {
        let _ = stderr.write_all(&output.stdout);
        if !output.stdout.ends_with(b"\n") {
            let _ = stderr.write_all(b"\n");
        }
    }

    if !output.stderr.is_empty() {
        let _ = stderr.write_all(&output.stderr);
        if !output.stderr.ends_with(b"\n") {
            let _ = stderr.write_all(b"\n");
        }
    }
}

// ──────────────────────────────────────────────
//  Compile and Run (default mode like QB64)
// ──────────────────────────────────────────────

fn compile_and_run(input: &str, output: Option<&str>, quiet: bool) -> Result<()> {
    use std::time::Instant;

    let output_path = output
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_output_name(input));

    let start = Instant::now();

    let source =
        fs::read_to_string(input).map_err(|e| anyhow::anyhow!("cannot read file: {}", e))?;

    // Fast parsing
    let mut parser = syntax_tree::Parser::new(source)?;
    let program = parser.parse()?;

    if has_graphics_or_sound(&program) {
        return compile_with_cargo(input, output, quiet, true);
    }

    // Generate Rust code
    let mut codegen = native_codegen::CodeGenerator::new();
    let rust_code = codegen.generate(&program)?;

    // Write temporary Rust file
    let temp_dir = std::env::temp_dir();
    let rust_file = temp_dir.join(format!("qb_{}.rs", std::process::id()));
    fs::write(&rust_file, rust_code)
        .map_err(|e| anyhow::anyhow!("cannot write temp file: {}", e))?;

    // Compile with rustc
    let mut cmd = std::process::Command::new("rustc");
    cmd.args([
        "-O", // Optimize
        "--edition",
        "2021",
    ]);

    if cfg!(target_os = "windows") {
        cmd.args(["-C", "link-arg=/DEBUG:NONE"]); // No PDB file on Windows
    } else {
        cmd.args(["-C", "strip=symbols"]); // Strip symbols on Linux/macOS
    }

    let output = cmd
        .args(["-o", &output_path, rust_file.to_str().unwrap()])
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run rustc: {}. Make sure Rust is installed.", e))?;

    // Clean up temp file
    let _ = fs::remove_file(&rust_file);

    if !output.status.success() {
        print_command_output_on_error(&output);
        return Err(anyhow::anyhow!("rustc compilation failed"));
    }

    // Get absolute path for the executable
    let abs_path = std::env::current_dir()?.join(&output_path);

    // Run the compiled executable
    let run_status = std::process::Command::new(&abs_path)
        .status()
        .map_err(|e| {
            anyhow::anyhow!(
                "failed to run compiled program '{}': {}",
                abs_path.display(),
                e
            )
        })?;

    if !quiet {
        println!("\nprogram finished in {:.2?}.", start.elapsed());
    }

    // Propagate exit code
    if !run_status.success() {
        std::process::exit(run_status.code().unwrap_or(1));
    }

    Ok(())
}

// ──────────────────────────────────────────────
//  Run (interpreter) - OPTIMIZED with timeout
// ──────────────────────────────────────────────

fn run_file(filename: &str, quiet: bool) -> Result<()> {
    use std::time::Instant;

    let start = Instant::now();

    let source =
        fs::read_to_string(filename).map_err(|e| anyhow::anyhow!("cannot read file: {}", e))?;

    // Skip tokenizer for faster parsing (integrated in parser)
    let mut parser = syntax_tree::Parser::new(source)?;
    let program = parser.parse()?;

    if has_graphics_or_sound(&program) {
        return compile_with_cargo(filename, None, quiet, true);
    }

    let symbol_table = analyzer::scope::analyze_program(&program)?;
    let mut type_checker = analyzer::TypeChecker::new(symbol_table);
    type_checker.check_program(&program)?;

    let mut compiler = vm_engine::BytecodeCompiler::new(program);
    let bytecode = compiler.compile()?;

    let mut vm = vm_engine::VM::new(bytecode);
    vm.run()?;

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

fn build_file(input: &str, output: Option<&str>, quiet: bool) -> Result<()> {
    let output_path = output
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_output_name(input));

    let source =
        fs::read_to_string(input).map_err(|e| anyhow::anyhow!("cannot read file: {}", e))?;

    // Fast parsing
    let mut parser = syntax_tree::Parser::new(source)?;
    let program = parser.parse()?;

    if has_graphics_or_sound(&program) {
        return compile_with_cargo(input, output, quiet, false);
    }

    // Generate Rust code
    let mut codegen = native_codegen::CodeGenerator::new();
    let rust_code = codegen.generate(&program)?;

    // Write temporary Rust file
    let temp_dir = std::env::temp_dir();
    let rust_file = temp_dir.join(format!("qb_{}.rs", std::process::id()));
    fs::write(&rust_file, rust_code)
        .map_err(|e| anyhow::anyhow!("cannot write temp file: {}", e))?;

    // Compile with rustc (no PDB)
    let mut cmd = std::process::Command::new("rustc");
    cmd.args([
        "-O", // Optimize
        "--edition",
        "2021",
    ]);

    if cfg!(target_os = "windows") {
        cmd.args(["-C", "link-arg=/DEBUG:NONE"]); // No PDB file on Windows
    } else {
        cmd.args(["-C", "strip=symbols"]); // Strip symbols on Linux/macOS
    }

    let output = cmd
        .args(["-o", &output_path, rust_file.to_str().unwrap()])
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run rustc: {}. Make sure Rust is installed.", e))?;

    // Clean up temp file
    let _ = fs::remove_file(&rust_file);

    if !output.status.success() {
        print_command_output_on_error(&output);
        return Err(anyhow::anyhow!("rustc compilation failed"));
    }
    Ok(())
}

fn compile_with_cargo(input: &str, output: Option<&str>, _quiet: bool, run: bool) -> Result<()> {
    let output_path = output
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_output_name(input));
    let source =
        fs::read_to_string(input).map_err(|e| anyhow::anyhow!("cannot read file: {}", e))?;

    let mut parser = syntax_tree::Parser::new(source)?;
    let program = parser.parse()?;

    // Generate Rust code with graphics enabled
    let mut codegen = native_codegen::CodeGenerator::new();
    codegen.enable_graphics(); // Enable graphics
    let rust_code = codegen.generate(&program)?;

    // Create temp directory for cargo project
    let temp_dir = std::env::temp_dir().join(format!("qbnex_build_{}", std::process::id()));
    fs::create_dir_all(&temp_dir)?;

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow::anyhow!("failed to locate workspace root"))?;
    let hal_layer_path = repo_root
        .join("hal_layer")
        .to_string_lossy()
        .replace('\\', "/");
    let core_types_path = repo_root
        .join("core_types")
        .to_string_lossy()
        .replace('\\', "/");

    // Create Cargo.toml
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

    // Create .cargo/config.toml for static linking on Windows (standalone executable)
    if cfg!(target_os = "windows") {
        let dot_cargo = temp_dir.join(".cargo");
        fs::create_dir_all(&dot_cargo)?;
        let cargo_config = r#"[target.'cfg(all(windows, target_env = "msvc"))']
rustflags = ["-C", "target-feature=+crt-static"]
"#;
        fs::write(dot_cargo.join("config.toml"), cargo_config)?;
    }

    // Create src directory
    let src_dir = temp_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(src_dir.join("main.rs"), rust_code)?;

    // Run cargo build --release
    let output = std::process::Command::new("cargo")
        .args(["build", "--release", "--quiet"])
        .current_dir(&temp_dir)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run cargo: {}", e))?;

    if !output.status.success() {
        print_command_output_on_error(&output);
        return Err(anyhow::anyhow!("cargo compilation failed"));
    }

    // Copy executable to output path
    let target_bin = temp_dir.join("target/release/qbnex_app.exe");
    let dest = std::env::current_dir()?.join(&output_path);
    if target_bin.exists() {
        fs::copy(&target_bin, &dest)?;
    } else {
        // Try without extension (linux/mac)
        let target_bin = temp_dir.join("target/release/qbnex_app");
        if target_bin.exists() {
            fs::copy(&target_bin, &dest)?;
        } else {
            return Err(anyhow::anyhow!("compiled binary not found"));
        }
    }

    if run {
        let run_status = std::process::Command::new(&dest).status()?;

        if !run_status.success() {
            // Cleanup before exit
            let _ = fs::remove_dir_all(&temp_dir);
            std::process::exit(run_status.code().unwrap_or(1));
        }
    }

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}
