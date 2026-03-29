use std::{
    fs,
    path::Path,
    process::Command,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn normalize_test_all_output(output: &str) -> String {
    output
        .replace('\r', "")
        .lines()
        .map(|line| {
            if line.starts_with("RND (สุ่มตัวเลข): ") {
                "RND (สุ่มตัวเลข): <dynamic>".to_string()
            } else if line.starts_with("DATE$: ") {
                "DATE$: <dynamic>".to_string()
            } else if line.starts_with("TIMER (วินาทีตั้งแต่เที่ยงคืน): ")
            {
                "TIMER (วินาทีตั้งแต่เที่ยงคืน): <dynamic>".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn compile_with_qb(source_path: &Path, output_path: &Path, cwd: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-c")
        .arg(source_path)
        .arg("-o")
        .arg(output_path)
        .current_dir(cwd)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "native compile failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn tagged_temp_workspace_names(prefix: &str, tag: &str) -> Vec<String> {
    let start = format!("{prefix}_{tag}_");
    let mut names = fs::read_dir(std::env::temp_dir())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }
            let name = path.file_name()?.to_str()?;
            name.starts_with(&start).then(|| name.to_string())
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[test]
fn empty_shell_interpreter_run_is_quiet() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_shell_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("empty_shell.bas");
    std::fs::write(&source_path, "SHELL\nPRINT \"done\"\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "qb failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout).replace('\r', "");
    assert!(stdout.starts_with("done\n"), "unexpected stdout: {stdout}");
    assert!(
        !stdout.contains("[SHELL: Interactive shell not supported]"),
        "shell marker leaked into stdout: {stdout}"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn interpreter_run_builds_a_runnable_executable_in_the_working_directory() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_runner_build_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("runner.bas");
    std::fs::write(&source_path, "PRINT \"hello\"\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "qb -x failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout).replace('\r', "");
    assert_eq!(stdout, "hello\n");

    let built_binary = if cfg!(target_os = "windows") {
        temp_dir.join("runner.exe")
    } else {
        temp_dir.join("runner")
    };
    assert!(
        built_binary.exists(),
        "expected built runner at {}",
        built_binary.display()
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn compile_only_creates_missing_output_directories() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_compile_output_dirs_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("hello.bas");
    let output_path = if cfg!(target_os = "windows") {
        temp_dir.join("dist").join("native").join("hello.exe")
    } else {
        temp_dir.join("dist").join("native").join("hello")
    };
    std::fs::write(&source_path, "PRINT \"hello\"\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-c")
        .arg(&source_path)
        .arg("-o")
        .arg(&output_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "qb -c -o nested path failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output_path.exists(),
        "expected compiled executable at {}",
        output_path.display()
    );

    let compiled = Command::new(&output_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "compiled executable failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&compiled.stdout).replace('\r', ""),
        "hello\n"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn compile_only_output_directory_uses_the_source_stem() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_compile_output_dir_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("dist")).unwrap();

    let source_path = temp_dir.join("hello.bas");
    std::fs::write(&source_path, "PRINT \"hello\"\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-c")
        .arg(&source_path)
        .arg("-o")
        .arg(temp_dir.join("dist"))
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "qb -c -o <dir> failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let built_output = if cfg!(target_os = "windows") {
        temp_dir.join("dist").join("hello.exe")
    } else {
        temp_dir.join("dist").join("hello")
    };
    assert!(
        built_output.exists(),
        "expected compiled executable at {}",
        built_output.display()
    );

    let compiled = Command::new(&built_output)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "compiled executable failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&compiled.stdout).replace('\r', ""),
        "hello\n"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn interpreter_run_with_explicit_output_builds_and_runs_the_named_executable() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_runner_named_output_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("runner.bas");
    let named_output = if cfg!(target_os = "windows") {
        temp_dir.join("dist").join("vm").join("named_runner.exe")
    } else {
        temp_dir.join("dist").join("vm").join("named_runner")
    };
    std::fs::write(&source_path, "PRINT \"hello\"\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .arg("-o")
        .arg(&named_output)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "qb -x -o failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).replace('\r', ""),
        "hello\n"
    );
    assert!(
        named_output.exists(),
        "expected named runner at {}",
        named_output.display()
    );

    let default_output = if cfg!(target_os = "windows") {
        temp_dir.join("runner.exe")
    } else {
        temp_dir.join("runner")
    };
    assert!(
        !default_output.exists(),
        "explicit output should override the default runner name: {}",
        default_output.display()
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn commented_qb64_includes_are_expanded_recursively_for_all_cli_paths() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_include_expand_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("inc").join("nested")).unwrap();

    let source_path = temp_dir.join("include_root.bas");
    std::fs::write(
        temp_dir.join("inc").join("globals.bi"),
        "prefix$ = \"QB\"\n",
    )
    .unwrap();
    std::fs::write(
        temp_dir.join("inc").join("nested").join("suffix.bi"),
        "suffix$ = \"Nex\"\n",
    )
    .unwrap();
    std::fs::write(
        temp_dir.join("inc").join("helpers.bm"),
        "'$INCLUDE:'nested\\suffix.bi'\n",
    )
    .unwrap();
    std::fs::write(
        &source_path,
        "'$INCLUDE:'inc\\globals.bi'\n'$INCLUDE:'inc\\helpers.bm'\nPRINT prefix$ + suffix$\n",
    )
    .unwrap();

    let expected = "QBNex\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(
        interpreter.status.success(),
        "qb -x failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let interpreter_output_path = if cfg!(target_os = "windows") {
        temp_dir.join("include_root.exe")
    } else {
        temp_dir.join("include_root")
    };
    let _ = std::fs::remove_file(&interpreter_output_path);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(
        native.status.success(),
        "default run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let output_path = if cfg!(target_os = "windows") {
        temp_dir.join("include_root_compile.exe")
    } else {
        temp_dir.join("include_root_compile")
    };
    compile_with_qb(&source_path, &output_path, &temp_dir);

    let compiled = Command::new(&output_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "compiled executable failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
    let compiled_stdout = String::from_utf8_lossy(&compiled.stdout).replace('\r', "");
    assert_eq!(compiled_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn duplicate_qb64_includes_preserve_multiple_expansions() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_duplicate_includes_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("inc")).unwrap();

    let source_path = temp_dir.join("include_root.bas");
    std::fs::write(temp_dir.join("inc").join("emit.bi"), "PRINT \"QBNex\"\n").unwrap();
    std::fs::write(
        &source_path,
        "'$INCLUDE:'inc\\emit.bi'\n'$INCLUDE:'inc\\emit.bi'\n",
    )
    .unwrap();

    let expected = "QBNex\nQBNex\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(
        interpreter.status.success(),
        "qb -x failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&interpreter.stdout).replace('\r', ""),
        expected
    );

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(
        native.status.success(),
        "default run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&native.stdout).replace('\r', ""),
        expected
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn cyclic_qb64_includes_fail_with_a_clear_error() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_include_cycle_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("cycle.bas");
    std::fs::write(&source_path, "'$INCLUDE:'cycle.bi'\nPRINT 1\n").unwrap();
    std::fs::write(temp_dir.join("cycle.bi"), "'$INCLUDE:'cycle.bas'\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-c")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "cyclic include unexpectedly compiled\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr).replace('\r', "");
    assert!(
        stderr.contains("cyclic $INCLUDE detected"),
        "expected cycle error, got stderr:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn syntax_errors_render_with_a_highlighted_source_snippet() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_miette_diag_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("bad.bas");
    std::fs::write(&source_path, "PRINT @\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-c")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "invalid program unexpectedly compiled\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr).replace('\r', "");
    assert!(
        stderr.contains("Unexpected character: @"),
        "expected the syntax message in stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("bad.bas"),
        "expected the source file name in stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("PRINT @"),
        "expected the rendered source snippet in stderr:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn chumsky_frontend_runs_simple_programs_through_the_cli() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_chumsky_frontend_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("simple.bas");
    std::fs::write(&source_path, "LET total = 40 + 2\nPRINT total\nEND\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg("--frontend")
        .arg("chumsky")
        .arg("--allow-preview")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "qb with chumsky frontend failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).replace('\r', ""),
        "42\n"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn cranelift_jit_backend_builds_and_runs_simple_programs() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_cranelift_backend_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("simple.bas");
    std::fs::write(
        &source_path,
        "value = 40 + 2\nPRINT value\nPRINT \"OK\"\nEND\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg("--frontend")
        .arg("chumsky")
        .arg("--allow-preview")
        .arg("--native-backend")
        .arg("cranelift-jit")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "qb with cranelift-jit backend failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).replace('\r', ""),
        "42\nOK\n"
    );

    let built_binary = if cfg!(target_os = "windows") {
        temp_dir.join("simple.exe")
    } else {
        temp_dir.join("simple")
    };
    assert!(
        built_binary.exists(),
        "expected cranelift-backed runner at {}",
        built_binary.display()
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn cranelift_preview_failures_include_a_production_fallback_hint() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_cranelift_hint_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("unsupported.bas");
    std::fs::write(&source_path, "IF 1 THEN PRINT 1\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg("--native-backend")
        .arg("cranelift-jit")
        .arg("--allow-preview")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "unsupported preview program unexpectedly ran\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr).replace('\r', "");
    assert!(
        stderr.contains("rerun without --native-backend"),
        "expected production fallback hint, got stderr:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn preview_frontends_are_blocked_without_explicit_opt_in() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_preview_gate_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("simple.bas");
    std::fs::write(&source_path, "PRINT 42\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg("--frontend")
        .arg("chumsky")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "preview frontend unexpectedly ran without opt-in\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr).replace('\r', "");
    assert!(
        stderr.contains("rerun with --allow-preview"),
        "expected preview opt-in guidance, got stderr:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn release_validation_command_succeeds_for_the_production_surface() {
    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("--validate-release")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "release validation failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout).replace('\r', "");
    assert!(
        stdout.contains("release validation passed"),
        "expected validation summary, got stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("fixtures=4"),
        "expected fixture count in validation summary, got stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("[1/4] validated fixture: release_validation_text.bas [kind=text"),
        "expected per-fixture progress line, got stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("runtime-output=3") && stdout.contains("vm-fallback=1"),
        "expected validation capability summary, got stdout:\n{stdout}"
    );
}

#[test]
fn validate_pipeline_command_succeeds_for_the_production_pipeline() {
    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("--validate-pipeline")
        .arg("--frontend")
        .arg("classic")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "pipeline validation failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout).replace('\r', "");
    assert!(
        stdout.contains("pipeline validation passed"),
        "expected pipeline validation summary, got stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("frontend=classic") && stdout.contains("native-backend=auto"),
        "expected pipeline details in validation summary, got stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("[3/4] validated fixture: release_validation_file_io.bas [kind=file-io"),
        "expected fixture inventory progress in pipeline validation, got stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("graphics=1") && stdout.contains("runtime-output=3"),
        "expected pipeline validation summary counts, got stdout:\n{stdout}"
    );
}

#[test]
fn validate_pipeline_reports_preview_fixture_gaps_clearly() {
    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("--validate-pipeline")
        .arg("--frontend")
        .arg("classic")
        .arg("--native-backend")
        .arg("cranelift-jit")
        .arg("--allow-preview")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "preview pipeline unexpectedly passed the release fixtures\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr).replace('\r', "");
    assert!(
        stderr.contains("release_validation_text.bas"),
        "expected failing fixture name in stderr, got:\n{stderr}"
    );
    assert!(
        stderr.contains("cranelift-jit"),
        "expected failing backend name in stderr, got:\n{stderr}"
    );
    assert!(
        stderr.contains("rerun without --native-backend"),
        "expected fallback guidance in stderr, got:\n{stderr}"
    );
}

#[test]
fn validation_modes_are_mutually_exclusive() {
    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("--validate-release")
        .arg("--validate-pipeline")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "combined validation modes unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr).replace('\r', "");
    assert!(
        stderr.contains("--validate-release and --validate-pipeline cannot be used together"),
        "expected mutual exclusion error, got stderr:\n{stderr}"
    );
}

#[test]
fn explain_pipeline_reports_vm_fallback_reasons() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_explain_pipeline_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("vm_fallback.bas");
    std::fs::write(
        &source_path,
        "IF -1 THEN\nFallbackTarget:\n    PRINT \"ok\"\nEND IF\nGOTO FallbackTarget\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("--explain-pipeline")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "explaining pipeline failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout).replace('\r', "");
    assert!(
        stdout.contains("selected execution path: vm-fallback"),
        "expected vm-fallback selection, got stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("native gaps:") && stdout.contains("GOTO"),
        "expected native gap explanation, got stdout:\n{stdout}"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn preview_native_backends_are_blocked_without_explicit_opt_in() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_preview_backend_gate_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("simple.bas");
    std::fs::write(&source_path, "PRINT 42\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg("--native-backend")
        .arg("cranelift-jit")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "preview backend unexpectedly ran without opt-in\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr).replace('\r', "");
    assert!(
        stderr.contains("rerun with --allow-preview"),
        "expected preview opt-in guidance, got stderr:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn list_pipelines_reports_production_and_preview_statuses() {
    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("--list-pipelines")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "listing pipelines failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout).replace('\r', "");
    assert!(
        stdout.contains("Frontends:"),
        "missing frontend section:\n{stdout}"
    );
    assert!(
        stdout.contains("classic [production]"),
        "missing production frontend status:\n{stdout}"
    );
    assert!(
        stdout.contains("chumsky [preview]"),
        "missing preview frontend status:\n{stdout}"
    );
    assert!(
        stdout.contains("rust [production]"),
        "missing production backend status:\n{stdout}"
    );
    assert!(
        stdout.contains("cranelift-jit [preview]"),
        "missing preview backend status:\n{stdout}"
    );
    assert!(
        stdout.contains("--validate-pipeline"),
        "missing validation guidance:\n{stdout}"
    );
    assert!(
        stdout.contains("Validation fixtures:"),
        "missing validation fixture inventory section:\n{stdout}"
    );
    assert!(
        stdout.contains("release_validation_vm_fallback.bas [kind=vm-fallback"),
        "missing fixture descriptor details:\n{stdout}"
    );
    assert!(
        stdout.contains("fixtures=4")
            && stdout.contains("runtime-output=3")
            && stdout.contains("vm-fallback=1"),
        "missing validation summary counts:\n{stdout}"
    );
}

#[test]
fn unique_root_fragments_are_retried_via_the_owning_program() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_fragment_retry_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    std::fs::write(
        temp_dir.join("root.bas"),
        "DECLARE SUB Helper\n'$INCLUDE:'fragment.bas'\nEND\nSUB Helper\nPRINT \"root\"\nEND SUB\n",
    )
    .unwrap();
    let source_path = temp_dir.join("fragment.bas");
    std::fs::write(&source_path, "CALL Helper\n").unwrap();
    let output_path = if cfg!(target_os = "windows") {
        temp_dir.join("promoted.exe")
    } else {
        temp_dir.join("promoted")
    };

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-c")
        .arg(&source_path)
        .arg("-o")
        .arg(&output_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "fragment compile did not retry through root\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).trim().is_empty(),
        "successful fragment promotion should not leak initial diagnostics:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let compiled = Command::new(&output_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "compiled executable failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
    assert!(
        String::from_utf8_lossy(&compiled.stdout).replace('\r', "") == "root\n",
        "unexpected program output:\n{}",
        String::from_utf8_lossy(&compiled.stdout)
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn nested_fragment_chains_retry_via_the_top_level_root() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_fragment_nested_root_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("mid")).unwrap();

    std::fs::write(
        temp_dir.join("root.bas"),
        "'$INCLUDE:'mid\\mid.bas'\ndone:\nPRINT \"top-root\"\n",
    )
    .unwrap();
    std::fs::write(
        temp_dir.join("mid").join("mid.bas"),
        "'$INCLUDE:'leaf.bas'\n",
    )
    .unwrap();
    let source_path = temp_dir.join("mid").join("leaf.bas");
    std::fs::write(&source_path, "GOTO done\nPRINT \"mid-root\"\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-c")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "nested fragment compile did not retry through the top-level root\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let built_binary = if cfg!(target_os = "windows") {
        temp_dir.join("leaf.exe")
    } else {
        temp_dir.join("leaf")
    };
    let compiled = Command::new(&built_binary)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "compiled nested fragment runner failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&compiled.stdout).replace('\r', ""),
        "top-root\n"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn nested_fragment_chains_default_run_via_the_top_level_root() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_fragment_nested_default_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("mid")).unwrap();

    std::fs::write(
        temp_dir.join("root.bas"),
        "'$INCLUDE:'mid\\mid.bas'\ndone:\nPRINT \"top-root\"\n",
    )
    .unwrap();
    std::fs::write(
        temp_dir.join("mid").join("mid.bas"),
        "'$INCLUDE:'leaf.bas'\n",
    )
    .unwrap();
    let source_path = temp_dir.join("mid").join("leaf.bas");
    std::fs::write(&source_path, "GOTO done\nPRINT \"mid-root\"\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "nested fragment default run did not retry through the top-level root\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).replace('\r', ""),
        "top-root\n"
    );

    let built_binary = if cfg!(target_os = "windows") {
        temp_dir.join("leaf.exe")
    } else {
        temp_dir.join("leaf")
    };
    assert!(
        built_binary.exists(),
        "expected nested default run to leave the promoted executable at {}",
        built_binary.display()
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn unique_root_fragment_compile_preserves_the_requested_output_stem() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_fragment_stem_compile_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    std::fs::write(
        temp_dir.join("root.bas"),
        "DECLARE SUB Helper\n'$INCLUDE:'fragment.bas'\nEND\nSUB Helper\nPRINT \"root\"\nEND SUB\n",
    )
    .unwrap();
    let source_path = temp_dir.join("fragment.bas");
    std::fs::write(&source_path, "CALL Helper\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-c")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "fragment compile did not retry through root\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let promoted_output = if cfg!(target_os = "windows") {
        temp_dir.join("fragment.exe")
    } else {
        temp_dir.join("fragment")
    };
    let unexpected_root_output = if cfg!(target_os = "windows") {
        temp_dir.join("root.exe")
    } else {
        temp_dir.join("root")
    };

    assert!(
        promoted_output.exists(),
        "expected promoted fragment output at {}",
        promoted_output.display()
    );
    assert!(
        !unexpected_root_output.exists(),
        "fragment promotion should not use the root stem: {}",
        unexpected_root_output.display()
    );

    let compiled = Command::new(&promoted_output)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "compiled executable failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&compiled.stdout).replace('\r', ""),
        "root\n"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn unique_root_fragment_vm_run_builds_a_runner_using_the_fragment_stem() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_fragment_stem_run_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    std::fs::write(
        temp_dir.join("root.bas"),
        "DECLARE SUB Helper\n'$INCLUDE:'fragment.bas'\nEND\nSUB Helper\nPRINT \"root\"\nEND SUB\n",
    )
    .unwrap();
    let source_path = temp_dir.join("fragment.bas");
    std::fs::write(&source_path, "CALL Helper\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "fragment run did not retry through root\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).replace('\r', ""),
        "root\n"
    );

    let promoted_output = if cfg!(target_os = "windows") {
        temp_dir.join("fragment.exe")
    } else {
        temp_dir.join("fragment")
    };
    let unexpected_root_output = if cfg!(target_os = "windows") {
        temp_dir.join("root.exe")
    } else {
        temp_dir.join("root")
    };

    assert!(
        promoted_output.exists(),
        "expected promoted fragment runner at {}",
        promoted_output.display()
    );
    assert!(
        !unexpected_root_output.exists(),
        "fragment VM run should not use the root stem: {}",
        unexpected_root_output.display()
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn unique_root_fragment_vm_run_honors_explicit_output_path() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_fragment_named_output_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    std::fs::write(
        temp_dir.join("root.bas"),
        "DECLARE SUB Helper\n'$INCLUDE:'fragment.bas'\nEND\nSUB Helper\nPRINT \"root\"\nEND SUB\n",
    )
    .unwrap();
    let source_path = temp_dir.join("fragment.bas");
    std::fs::write(&source_path, "CALL Helper\n").unwrap();
    let named_output = if cfg!(target_os = "windows") {
        temp_dir
            .join("dist")
            .join("fragment")
            .join("custom_runner.exe")
    } else {
        temp_dir.join("dist").join("fragment").join("custom_runner")
    };

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .arg("-o")
        .arg(&named_output)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "fragment run with explicit output did not retry through root\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).replace('\r', ""),
        "root\n"
    );
    assert!(
        named_output.exists(),
        "expected named runner at {}",
        named_output.display()
    );

    let default_fragment_output = if cfg!(target_os = "windows") {
        temp_dir.join("fragment.exe")
    } else {
        temp_dir.join("fragment")
    };
    let default_root_output = if cfg!(target_os = "windows") {
        temp_dir.join("root.exe")
    } else {
        temp_dir.join("root")
    };
    assert!(
        !default_fragment_output.exists() && !default_root_output.exists(),
        "explicit output should override both fragment and root defaults"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn unique_root_fragment_vm_run_output_directory_uses_the_fragment_stem() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_fragment_output_dir_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("dist")).unwrap();

    std::fs::write(
        temp_dir.join("root.bas"),
        "DECLARE SUB Helper\n'$INCLUDE:'fragment.bas'\nEND\nSUB Helper\nPRINT \"root\"\nEND SUB\n",
    )
    .unwrap();
    let source_path = temp_dir.join("fragment.bas");
    std::fs::write(&source_path, "CALL Helper\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .arg("-o")
        .arg(temp_dir.join("dist"))
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "fragment run with output directory did not retry through root\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).replace('\r', ""),
        "root\n"
    );

    let built_output = if cfg!(target_os = "windows") {
        temp_dir.join("dist").join("fragment.exe")
    } else {
        temp_dir.join("dist").join("fragment")
    };
    let unexpected_root_output = if cfg!(target_os = "windows") {
        temp_dir.join("dist").join("root.exe")
    } else {
        temp_dir.join("dist").join("root")
    };

    assert!(
        built_output.exists(),
        "expected fragment runner at {}",
        built_output.display()
    );
    assert!(
        !unexpected_root_output.exists(),
        "output directory should still use the fragment stem: {}",
        unexpected_root_output.display()
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn fragment_root_discovery_ignores_build_artifact_directories() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_fragment_ignore_build_dirs_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("target")).unwrap();

    std::fs::write(
        temp_dir.join("root.bas"),
        "DECLARE SUB Helper\n'$INCLUDE:'fragment.bas'\nEND\nSUB Helper\nPRINT \"root\"\nEND SUB\n",
    )
    .unwrap();
    let source_path = temp_dir.join("fragment.bas");
    std::fs::write(&source_path, "CALL Helper\n").unwrap();

    std::fs::write(
        temp_dir.join("target").join("noise.bas"),
        "DECLARE SUB Noise\n'$INCLUDE:'..\\fragment.bas'\nEND\nSUB Noise\nPRINT \"noise\"\nEND SUB\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-c")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "fragment compile should ignore build artifact directories\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let promoted_output = if cfg!(target_os = "windows") {
        temp_dir.join("fragment.exe")
    } else {
        temp_dir.join("fragment")
    };
    assert!(
        promoted_output.exists(),
        "expected promoted fragment output at {}",
        promoted_output.display()
    );

    let compiled = Command::new(&promoted_output)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "compiled executable failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&compiled.stdout).replace('\r', ""),
        "root\n"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
#[ignore = "temp workspace cleanup regression uses release build paths"]
fn build_pipelines_clean_up_tagged_temp_workspaces() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_temp_workspace_cleanup_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let gfx_source = temp_dir.join("gfx.bas");
    std::fs::write(&gfx_source, "SCREEN 12\nPSET (1, 1), 15\nPRINT \"ok\"\n").unwrap();
    let gfx_output = if cfg!(target_os = "windows") {
        temp_dir.join("gfx.exe")
    } else {
        temp_dir.join("gfx")
    };

    let vm_source = temp_dir.join("vm.bas");
    std::fs::write(&vm_source, "PRINT \"ok\"\n").unwrap();
    let vm_output = if cfg!(target_os = "windows") {
        temp_dir.join("vm_runner.exe")
    } else {
        temp_dir.join("vm_runner")
    };

    let temp_tag = format!(
        "cleanup_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let gfx = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-c")
        .arg(&gfx_source)
        .arg("-o")
        .arg(&gfx_output)
        .env("QBNEX_TEMP_TAG", &temp_tag)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(
        gfx.status.success(),
        "graphics compile failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&gfx.stdout),
        String::from_utf8_lossy(&gfx.stderr)
    );
    assert!(
        tagged_temp_workspace_names("qbnex_build", &temp_tag).is_empty(),
        "graphics compile leaked tagged temp workspaces: {:?}",
        tagged_temp_workspace_names("qbnex_build", &temp_tag)
    );

    let vm = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&vm_source)
        .arg("-o")
        .arg(&vm_output)
        .env("QBNEX_TEMP_TAG", &temp_tag)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(
        vm.status.success(),
        "vm runner build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&vm.stdout),
        String::from_utf8_lossy(&vm.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&vm.stdout).replace('\r', ""),
        "ok\n"
    );
    assert!(
        tagged_temp_workspace_names("qbnex_vm_build", &temp_tag).is_empty(),
        "vm runner build leaked tagged temp workspaces: {:?}",
        tagged_temp_workspace_names("qbnex_vm_build", &temp_tag)
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn ambiguous_project_fragments_report_a_clear_standalone_compile_hint() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_fragment_hint_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    std::fs::write(
        temp_dir.join("root1.bas"),
        "DECLARE SUB HelperOne\n'$INCLUDE:'fragment.bas'\nEND\nSUB HelperOne\nPRINT \"one\"\nEND SUB\n",
    )
    .unwrap();
    std::fs::write(
        temp_dir.join("root2.bas"),
        "DECLARE SUB HelperTwo\n'$INCLUDE:'fragment.bas'\nEND\nSUB HelperTwo\nPRINT \"two\"\nEND SUB\n",
    )
    .unwrap();
    let source_path = temp_dir.join("fragment.bas");
    std::fs::write(&source_path, "CALL HelperOne\nCALL HelperTwo\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-c")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "ambiguous fragment unexpectedly compiled\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr).replace('\r', "");
    assert!(
        stderr.contains("appears to be a project fragment owned by"),
        "expected fragment hint, got stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("root1.bas") && stderr.contains("root2.bas"),
        "expected both roots in stderr:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn qb64_style_dim_metacommand_line_continuation_and_binary_array_io_match_between_backends() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_qb64_compile_parity_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("qb64_style.bas");
    std::fs::write(
        &source_path,
        r#"$CONSOLE
DIM AS INTEGER a(1 TO 2, 1 TO 2), b(1 TO 2, 1 TO 2)
message$ = "QB" + _
           "64"
OPEN "data.bin" FOR BINARY AS #1
a(2, 1) = 321
a(1, 2) = 654
PUT #1, , a(2, 1)
PUT #1, , a(1, 2)
CLOSE #1
OPEN "data.bin" FOR BINARY AS #1
GET #1, , b(2, 1)
GET #1, , b(1, 2)
CLOSE #1
PRINT message$
PRINT b(2, 1)
PRINT b(1, 2)
"#,
    )
    .unwrap();

    let expected = "QB64\n321\n654\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn asc_assignment_and_large_integer_literals_match_between_backends() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_qb64_asc_assign_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("asc_assign.bas");
    std::fs::write(
        &source_path,
        r#"a$ = "A/B"
FOR x = 1 TO LEN(a$)
    IF ASC(a$, x) = 47 THEN ASC(a$, x) = 92
NEXT
IF 9999999999 > 1 THEN PRINT "BIG"
PRINT a$
"#,
    )
    .unwrap();

    let expected = "BIG\nA\\B\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn qb64_cv_type_conversions_match_between_backends() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_qb64_cv_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("qb64_cv.bas");
    std::fs::write(
        &source_path,
        concat!(
            "i$ = MKI$(12345)\n",
            "l$ = MKL$(123456789)\n",
            "f$ = MKS$(12.5)\n",
            "d$ = MKD$(123456.5)\n",
            "u16$ = CHR$(255) + CHR$(255)\n",
            "u32$ = CHR$(255) + CHR$(255) + CHR$(255) + CHR$(255)\n",
            "i64$ = CHR$(0) + CHR$(0) + CHR$(0) + CHR$(0) + CHR$(1) + CHR$(0) + CHR$(0) + CHR$(0)\n",
            "s8$ = CHR$(254)\n",
            "PRINT _CV(INTEGER, i$)\n",
            "PRINT _CV(LONG, l$)\n",
            "PRINT _CV(SINGLE, f$)\n",
            "PRINT _CV(_FLOAT, d$)\n",
            "PRINT _CV(_BYTE, s8$)\n",
            "PRINT _CV(_UNSIGNED INTEGER, u16$)\n",
            "PRINT _CV(_UNSIGNED LONG, u32$)\n",
            "PRINT _CV(_INTEGER64, i64$)\n",
            "PRINT _CV(_UNSIGNED _INTEGER64, i64$)\n",
        ),
    )
    .unwrap();

    let expected =
        "12345\n123456789\n12.5\n123456.5\n-2\n65535\n4294967295\n4294967296\n4294967296\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn qb64_file_exists_dir_exists_and_trim_match_between_backends() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_qb64_fs_builtins_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("fs_builtins.bas");
    std::fs::write(
        &source_path,
        concat!(
            "OPEN \"sample.txt\" FOR OUTPUT AS #1\n",
            "PRINT #1, \"data\"\n",
            "CLOSE #1\n",
            "IF _FILEEXISTS(\"sample.txt\") THEN PRINT _TRIM$(\"  ok  \")\n",
            "IF _DIREXISTS(\".\") THEN PRINT \"DIR\"\n",
        ),
    )
    .unwrap();

    let expected = "ok\nDIR\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn graphics_interpreter_run_uses_runtime_results_without_debug_markers() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_graphics_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("graphics.bas");
    std::fs::write(
        &source_path,
        r#"screenMode = 13
SCREEN screenMode
x = 5
y = 7
c = 11
PSET (x, y), c
LINE (x, y)-(x + 2, y), c
CIRCLE (x + 10, y + 10), 3, c
cmd$ = "BM30,20 C13 R4"
DRAW cmd$
PRINT POINT(x, y)
PRINT POINT(x + 2, y)
PRINT POINT(x + 13, y + 10)
PRINT POINT(34, 20)
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "qb failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout).replace('\r', "");
    assert!(
        stdout.starts_with("11\n11\n11\n13\n"),
        "unexpected stdout: {stdout}"
    );
    assert!(
        !stdout.contains("[SCREEN"),
        "screen marker leaked: {stdout}"
    );
    assert!(!stdout.contains("[PSET"), "pset marker leaked: {stdout}");
    assert!(
        !stdout.contains("[CIRCLE"),
        "circle marker leaked: {stdout}"
    );
    assert!(!stdout.contains("[DRAW"), "draw marker leaked: {stdout}");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn write_console_formats_values_identically_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_write_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("write_console.bas");
    std::fs::write(
        &source_path,
        "WRITE \"Hello\", 123, 45.5\nWRITE \"World\", 456\n",
    )
    .unwrap();

    let expected = "\"Hello\",123,45.5\n\"World\",456\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn print_using_reference_numeric_formats_match_quickbasic_examples_in_interpreter_and_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_print_using_ref_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("print_using_ref.bas");
    std::fs::write(
        &source_path,
        concat!(
            "PRINT USING \"##.##\"; .78\n",
            "PRINT USING \"##.##-\"; -68.95\n",
            "PRINT USING \"**#.#\"; 12.39\n",
            "PRINT USING \"$$###.##\"; 456.78\n",
            "PRINT USING \"**$##.##\"; 2.34\n",
            "PRINT USING \"**$##.##\"; -2.34\n",
            "PRINT USING \"####,.##\"; 1234.5\n",
            "PRINT USING \"##.##^^^^\"; 234.56\n",
            "PRINT USING \".####^^^^-\"; -888888\n",
            "PRINT USING \"+.##^^^^\"; 123\n",
            "PRINT USING \"+.##^^^^^\"; 123\n",
            "PRINT USING \"_!##.##_!\"; 12.34\n",
            "PRINT USING \"##.##\"; 111.22\n",
            "PRINT USING \".##\"; .999\n",
        ),
    )
    .unwrap();

    let expected = concat!(
        " 0.78\n",
        "68.95-\n",
        "*12.4\n",
        " $456.78\n",
        "***$2.34\n",
        "**-$2.34\n",
        "1,234.50\n",
        " 2.35E+02\n",
        ".8889E+06-\n",
        "+.12E+03\n",
        "+.12E+003\n",
        "!12.34!\n",
        "%111.22\n",
        "%1.00\n",
    );

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn print_using_cycles_across_multiple_fields_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_print_using_cycle_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("print_using_cycle.bas");
    std::fs::write(
        &source_path,
        "PRINT USING \"Item: ! Qty: ## \"; \"A\"; 1; \"B\"; 2\n",
    )
    .unwrap();

    let expected = "Item: A Qty:  1 Item: B Qty:  2 \n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn print_using_cycles_across_adjacent_mixed_fields_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_print_using_adjacent_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("print_using_adjacent.bas");
    std::fs::write(&source_path, "PRINT USING \"!##\"; \"A\"; 1; \"B\"; 2\n").unwrap();

    let expected = "A 1B 2\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn print_using_preserves_literal_commas_around_numeric_fields_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_print_using_commas_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("print_using_commas.bas");
    std::fs::write(&source_path, "PRINT USING \"Total, ###,\"; 12\n").unwrap();

    let expected = "Total,  12,\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn print_using_literal_only_formats_print_literal_text_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_print_using_literal_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("print_using_literal.bas");
    std::fs::write(
        &source_path,
        concat!("PRINT USING \"Header:\";\n", "PRINT USING \"Total:\"; 12\n",),
    )
    .unwrap();

    let expected = "Header:\nTotal:\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn on_timer_breaks_out_of_do_while_loop_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_on_timer_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("on_timer.bas");
    std::fs::write(
        &source_path,
        concat!(
            "COUNT = 0\n",
            "ON TIMER(0) GOSUB Tick\n",
            "TIMER ON\n",
            "DO WHILE COUNT = 0\n",
            "LOOP\n",
            "PRINT COUNT\n",
            "END\n",
            "Tick:\n",
            "COUNT = COUNT + 1\n",
            "TIMER OFF\n",
            "RETURN\n",
        ),
    )
    .unwrap();

    let expected = "1\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn do_loop_conditions_match_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_do_loop_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("do_loop.bas");
    std::fs::write(
        &source_path,
        concat!(
            "DECLARE SUB RunLoops ()\n",
            "CALL RunLoops\n",
            "A = 0\n",
            "DO WHILE A < 3\n",
            "A = A + 1\n",
            "LOOP\n",
            "PRINT \"TOP_PRE_WHILE\"; A\n",
            "B = 0\n",
            "DO\n",
            "B = B + 1\n",
            "LOOP WHILE B < 3\n",
            "PRINT \"TOP_POST_WHILE\"; B\n",
            "END\n",
            "SUB RunLoops\n",
            "C = 0\n",
            "DO UNTIL C = 3\n",
            "C = C + 1\n",
            "LOOP\n",
            "PRINT \"SUB_PRE_UNTIL\"; C\n",
            "D = 0\n",
            "DO\n",
            "D = D + 1\n",
            "LOOP UNTIL D = 3\n",
            "PRINT \"SUB_POST_UNTIL\"; D\n",
            "END SUB\n",
        ),
    )
    .unwrap();

    let expected = "SUB_PRE_UNTIL3\nSUB_POST_UNTIL3\nTOP_PRE_WHILE3\nTOP_POST_WHILE3\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn beep_sound_and_play_have_consistent_blocking_terminal_semantics() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_sound_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("sound.bas");
    std::fs::write(
        &source_path,
        concat!(
            "PRINT \"A\"\n",
            "BEEP\n",
            "PRINT \"B\"\n",
            "SOUND 0, 2\n",
            "PRINT \"C\"\n",
            "PLAY \"T240L8C\"\n",
            "PRINT \"D\"\n",
        ),
    )
    .unwrap();

    let expected = format!("A\n{bell}B\nC\n{bell}D\n", bell = '\u{7}');

    let interpreter_start = Instant::now();
    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    let interpreter_elapsed = interpreter_start.elapsed();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);
    assert!(
        interpreter_elapsed >= Duration::from_millis(300),
        "interpreter returned too quickly: {:?}",
        interpreter_elapsed
    );

    let native_start = Instant::now();
    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    let native_elapsed = native_start.elapsed();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);
    assert!(
        native_elapsed >= Duration::from_millis(300),
        "native returned too quickly: {:?}",
        native_elapsed
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn play_mml_extensions_block_consistently_in_interpreter_and_native_binary() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_play_mml_ext_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("play_mml_ext.bas");
    let output_path = temp_dir.join("play_mml_ext.exe");
    std::fs::write(
        &source_path,
        concat!("PRINT \"A\"\n", "PLAY \"MFT120P8N1.\"\n", "PRINT \"B\"\n",),
    )
    .unwrap();

    let expected = format!("A\n{bell}B\n", bell = '\u{7}');

    let interpreter_start = Instant::now();
    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    let interpreter_elapsed = interpreter_start.elapsed();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);
    assert!(
        interpreter_elapsed >= Duration::from_millis(900),
        "interpreter returned too quickly for PLAY extensions: {:?}",
        interpreter_elapsed
    );

    compile_with_qb(&source_path, &output_path, &temp_dir);

    let native_start = Instant::now();
    let native = Command::new(&output_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    let native_elapsed = native_start.elapsed();

    assert!(
        native.status.success(),
        "native binary failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);
    assert!(
        native_elapsed >= Duration::from_millis(900),
        "native binary returned too quickly for PLAY extensions: {:?}",
        native_elapsed
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn on_play_event_and_play_function_work_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_on_play_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("on_play.bas");
    std::fs::write(
        &source_path,
        concat!(
            "FIRED = -1\n",
            "ON PLAY(2) GOSUB Hit\n",
            "PLAY ON\n",
            "PLAY \"MBT120MLL8CCCR\"\n",
            "DO WHILE PLAY(0) > 0 OR FIRED < 0\n",
            "LOOP\n",
            "PRINT FIRED\n",
            "PRINT PLAY(0)\n",
            "END\n",
            "Hit:\n",
            "FIRED = PLAY(0)\n",
            "PLAY OFF\n",
            "RETURN\n",
        ),
    )
    .unwrap();

    let expected = format!("{bell}1\n0\n", bell = '\u{7}');

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn command_func_uses_program_arguments_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_command_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("command.bas");
    std::fs::write(&source_path, "PRINT COMMAND$\n").unwrap();

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .arg("--")
        .arg("alpha")
        .arg("beta")
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, "alpha beta\n");

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .arg("--")
        .arg("alpha")
        .arg("beta")
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, "alpha beta\n");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn environ_index_iteration_is_sorted_consistently_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_environ_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("environ.bas");
    std::fs::write(
        &source_path,
        concat!(
            "FOR I = 1 TO 4096\n",
            "    E$ = ENVIRON$(I)\n",
            "    IF LEFT$(E$, 12) = \"QBNEX_ORDER_\" THEN PRINT E$\n",
            "NEXT I\n",
            "PRINT ENVIRON$(\"QBNEX_ORDER_Z\")\n",
        ),
    )
    .unwrap();

    let expected = "QBNEX_ORDER_A=first\nQBNEX_ORDER_Z=last\nlast\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .env("QBNEX_ORDER_Z", "last")
        .env("QBNEX_ORDER_A", "first")
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .env("QBNEX_ORDER_Z", "last")
        .env("QBNEX_ORDER_A", "first")
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn tron_troff_and_erl_track_line_numbers_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_tron_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("tron.bas");
    std::fs::write(
        &source_path,
        concat!(
            "10 TRON\n",
            "20 ON ERROR GOTO Trap\n",
            "30 ERROR 5\n",
            "40 END\n",
            "Trap:\n",
            "50 TROFF\n",
            "60 PRINT ERR\n",
            "70 PRINT ERL\n",
        ),
    )
    .unwrap();

    let expected = "[20]\n[30]\n[50]\n5\n30\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn key_statements_store_and_list_assignments_consistently_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_key_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("key.bas");
    std::fs::write(
        &source_path,
        concat!(
            "KEY 1, \"LIST\" + CHR$(13)\n",
            "KEY 15, \"TEST\" + CHR$(13)\n",
            "KEY ON\n",
            "KEY LIST\n",
            "KEY OFF\n",
            "KEY LIST\n",
        ),
    )
    .unwrap();

    let expected = "F1 LIST<CR>\nF15 TEST<CR>\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn on_error_goto_line_number_works_in_interpreter_and_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_on_error_line_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("on_error_line.bas");
    std::fs::write(
        &source_path,
        concat!(
            "10 ON ERROR GOTO 40\n",
            "20 ERROR 5\n",
            "30 END\n",
            "40 PRINT ERR\n",
            "50 PRINT ERL\n",
        ),
    )
    .unwrap();

    let expected = "5\n20\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn lprint_and_lpos_use_printer_state_without_console_leakage_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_lprint_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("lprint.bas");
    std::fs::write(
        &source_path,
        concat!(
            "LPRINT \"X\"\n",
            "PRINT LPOS(0)\n",
            "LPRINT \"AB\";\n",
            "PRINT LPOS(0)\n",
        ),
    )
    .unwrap();

    let expected = "1\n3\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn lprint_using_comma_updates_printer_state_without_console_leakage_in_interpreter_and_native_run()
{
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_lprint_using_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("lprint_using.bas");
    std::fs::write(
        &source_path,
        concat!("LPRINT USING \"##\"; 8, 9;\n", "PRINT LPOS(0)\n",),
    )
    .unwrap();

    let expected = "17\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn print_file_using_formats_and_respects_commas_in_interpreter_and_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_print_file_using_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("print_file_using.bas");
    let output_path = temp_dir.join("using.txt");
    std::fs::write(
        &source_path,
        concat!(
            "OPEN \"using.txt\" FOR OUTPUT AS #1\n",
            "PRINT #1, USING \"##\"; 1, 2\n",
            "CLOSE #1\n",
        ),
    )
    .unwrap();

    let expected_file = format!(" 1{} 2\n", " ".repeat(12));

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&interpreter.stdout).replace('\r', ""),
        ""
    );
    let interpreter_file = std::fs::read_to_string(&output_path)
        .unwrap()
        .replace('\r', "");
    assert_eq!(interpreter_file, expected_file);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&native.stdout).replace('\r', ""),
        ""
    );
    let native_file = std::fs::read_to_string(&output_path)
        .unwrap()
        .replace('\r', "");
    assert_eq!(native_file, expected_file);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn print_file_using_cycles_across_multiple_fields_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_print_file_using_cycle_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("print_file_using_cycle.bas");
    let output_path = temp_dir.join("cycle.txt");
    std::fs::write(
        &source_path,
        concat!(
            "OPEN \"cycle.txt\" FOR OUTPUT AS #1\n",
            "PRINT #1, USING \"Item: ! Qty: ## \"; \"A\"; 1; \"B\"; 2\n",
            "CLOSE #1\n",
        ),
    )
    .unwrap();

    let expected_file = "Item: A Qty:  1 Item: B Qty:  2 \n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&interpreter.stdout).replace('\r', ""),
        ""
    );
    let interpreter_file = std::fs::read_to_string(&output_path)
        .unwrap()
        .replace('\r', "");
    assert_eq!(interpreter_file, expected_file);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&native.stdout).replace('\r', ""),
        ""
    );
    let native_file = std::fs::read_to_string(&output_path)
        .unwrap()
        .replace('\r', "");
    assert_eq!(native_file, expected_file);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn print_file_using_preserves_literal_commas_around_numeric_fields_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_print_file_using_commas_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("print_file_using_commas.bas");
    let output_path = temp_dir.join("commas.txt");
    std::fs::write(
        &source_path,
        concat!(
            "OPEN \"commas.txt\" FOR OUTPUT AS #1\n",
            "PRINT #1, USING \"Total, ###,\"; 12\n",
            "CLOSE #1\n",
        ),
    )
    .unwrap();

    let expected_file = "Total,  12,\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&interpreter.stdout).replace('\r', ""),
        ""
    );
    let interpreter_file = std::fs::read_to_string(&output_path)
        .unwrap()
        .replace('\r', "");
    assert_eq!(interpreter_file, expected_file);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&native.stdout).replace('\r', ""),
        ""
    );
    let native_file = std::fs::read_to_string(&output_path)
        .unwrap()
        .replace('\r', "");
    assert_eq!(native_file, expected_file);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn print_file_using_literal_only_formats_print_literal_text_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_print_file_using_literal_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("print_file_using_literal.bas");
    let output_path = temp_dir.join("literal.txt");
    std::fs::write(
        &source_path,
        concat!(
            "OPEN \"literal.txt\" FOR OUTPUT AS #1\n",
            "PRINT #1, USING \"Header:\";\n",
            "PRINT #1, USING \"Total:\"; 12\n",
            "CLOSE #1\n",
        ),
    )
    .unwrap();

    let expected_file = "Header:\nTotal:\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&interpreter.stdout).replace('\r', ""),
        ""
    );
    let interpreter_file = std::fs::read_to_string(&output_path)
        .unwrap()
        .replace('\r', "");
    assert_eq!(interpreter_file, expected_file);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&native.stdout).replace('\r', ""),
        ""
    );
    let native_file = std::fs::read_to_string(&output_path)
        .unwrap()
        .replace('\r', "");
    assert_eq!(native_file, expected_file);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn width_auto_wrap_matches_between_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_width_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("width.bas");
    std::fs::write(
        &source_path,
        concat!(
            "WIDTH 10\n",
            "PRINT STRING$(10, \"A\");\n",
            "WRITE CSRLIN, POS(0)\n",
        ),
    )
    .unwrap();

    let expected = "AAAAAAAAAA\n2,1\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn view_print_text_region_row_state_matches_between_interpreter_and_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_view_print_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("view_print.bas");
    std::fs::write(
        &source_path,
        concat!(
            "VIEW PRINT 3 TO 4\n",
            "a = CSRLIN\n",
            "PRINT\n",
            "b = CSRLIN\n",
            "PRINT\n",
            "c = CSRLIN\n",
            "WRITE a, b, c\n",
        ),
    )
    .unwrap();

    let expected = "\n\n3,4,4\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn screen_zero_restores_default_text_geometry_and_viewport_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_screen0_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("screen0.bas");
    std::fs::write(
        &source_path,
        concat!(
            "WIDTH 10\n",
            "VIEW PRINT 3 TO 4\n",
            "SCREEN 0\n",
            "WRITE CSRLIN, POS(0)\n",
            "PRINT STRING$(80, \"A\");\n",
            "WRITE CSRLIN, POS(0)\n",
        ),
    )
    .unwrap();

    let expected = "1,1\nAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n3,1\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn cls_modes_respect_text_viewport_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_cls_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("cls.bas");
    std::fs::write(
        &source_path,
        concat!(
            "VIEW PRINT 3 TO 4\n",
            "LOCATE 4, 5\n",
            "CLS 2\n",
            "WRITE CSRLIN, POS(0)\n",
            "LOCATE 4, 5\n",
            "CLS 0\n",
            "WRITE CSRLIN, POS(0)\n",
        ),
    )
    .unwrap();

    let expected = "\u{1b}[4;5H3,1\n\u{1b}[4;5H1,1\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn locate_omitted_row_and_col_preserve_cursor_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_locate_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("locate.bas");
    std::fs::write(
        &source_path,
        concat!(
            "LOCATE 5, 7\n",
            "LOCATE , 9\n",
            "a = CSRLIN\n",
            "b = POS(0)\n",
            "LOCATE 6\n",
            "c = CSRLIN\n",
            "d = POS(0)\n",
            "WRITE a, b, c, d\n",
        ),
    )
    .unwrap();

    let expected = "\u{1b}[5;7H\u{1b}[5;9H\u{1b}[6;9H5,9,6,9\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn locate_cursor_visibility_matches_between_interpreter_and_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_locate_cursor_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("locate_cursor.bas");
    std::fs::write(
        &source_path,
        concat!(
            "PRINT \"A\";\n",
            "LOCATE , , 0, 5, 6\n",
            "PRINT \"B\";\n",
            "LOCATE , , 1\n",
            "PRINT \"C\"\n",
        ),
    )
    .unwrap();

    let expected = "A\u{1b}[?25lB\u{1b}[?25hC\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn screen_function_reads_text_buffer_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_screen_fn_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("screen_fn.bas");
    std::fs::write(
        &source_path,
        concat!("PRINT \"A\";\n", "WRITE SCREEN(1, 1), SCREEN(1, 1, 1)\n",),
    )
    .unwrap();

    let expected = "A65,7\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn view_print_defers_scroll_until_next_output_in_interpreter_and_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_view_scroll_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("view_scroll_deferred.bas");
    std::fs::write(
        &source_path,
        concat!(
            "VIEW PRINT 2 TO 3\n",
            "PRINT \"A\"\n",
            "PRINT \"B\"\n",
            "WRITE SCREEN(2, 1), SCREEN(3, 1), CSRLIN\n",
        ),
    )
    .unwrap();

    let expected = "A\nB\n65,66,3\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn view_print_scrolls_when_next_output_arrives_in_interpreter_and_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_view_scroll_next_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("view_scroll_next.bas");
    std::fs::write(
        &source_path,
        concat!(
            "VIEW PRINT 2 TO 3\n",
            "PRINT \"A\"\n",
            "PRINT \"B\"\n",
            "PRINT \"C\"\n",
            "WRITE SCREEN(2, 1), SCREEN(3, 1), CSRLIN\n",
        ),
    )
    .unwrap();

    let expected = "A\nB\nC\n66,67,3\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn view_print_blank_line_triggers_deferred_scroll_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_view_scroll_blank_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("view_scroll_blank.bas");
    std::fs::write(
        &source_path,
        concat!(
            "VIEW PRINT 2 TO 3\n",
            "PRINT \"A\"\n",
            "PRINT \"B\"\n",
            "PRINT\n",
            "WRITE SCREEN(2, 1), SCREEN(3, 1), CSRLIN\n",
        ),
    )
    .unwrap();

    let expected = "A\nB\n\n66,32,3\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn locate_cancels_pending_view_print_scroll_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_view_scroll_locate_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("view_scroll_locate.bas");
    std::fs::write(
        &source_path,
        concat!(
            "VIEW PRINT 2 TO 3\n",
            "PRINT \"A\"\n",
            "PRINT \"B\"\n",
            "LOCATE 2, 5\n",
            "PRINT \"C\"\n",
            "WRITE SCREEN(2, 1), SCREEN(2, 5), SCREEN(3, 1), CSRLIN\n",
        ),
    )
    .unwrap();

    let expected = "A\nB\n\u{1b}[2;5HC\n65,67,66,3\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn full_screen_scrolls_when_next_output_arrives_in_interpreter_and_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_full_scroll_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("full_scroll_next.bas");
    std::fs::write(
        &source_path,
        concat!(
            "WIDTH 10, 2\n",
            "PRINT STRING$(10, \"A\");\n",
            "PRINT STRING$(10, \"B\");\n",
            "PRINT STRING$(10, \"C\");\n",
            "WRITE SCREEN(1, 1), SCREEN(2, 1), CSRLIN\n",
        ),
    )
    .unwrap();

    let expected = "AAAAAAAAAA\nBBBBBBBBBB\nCCCCCCCCCC\n66,67,2\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn write_console_scrolls_in_view_print_region_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_write_view_scroll_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("write_view_scroll.bas");
    std::fs::write(
        &source_path,
        concat!(
            "VIEW PRINT 2 TO 3\n",
            "WRITE 1\n",
            "WRITE 2\n",
            "WRITE 3\n",
            "WRITE SCREEN(2, 1), SCREEN(3, 1), CSRLIN\n",
        ),
    )
    .unwrap();

    let expected = "1\n2\n3\n50,51,3\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn write_console_scrolls_in_full_screen_text_mode_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_write_full_scroll_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("write_full_scroll.bas");
    std::fs::write(
        &source_path,
        concat!(
            "WIDTH 10, 2\n",
            "WRITE 1\n",
            "WRITE 2\n",
            "WRITE 3\n",
            "WRITE SCREEN(1, 1), SCREEN(2, 1), CSRLIN\n",
        ),
    )
    .unwrap();

    let expected = "1\n2\n3\n50,51,2\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn print_comma_scrolls_when_zone_wrap_hits_view_print_bottom_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_print_comma_scroll_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("print_comma_scroll.bas");
    std::fs::write(
        &source_path,
        concat!(
            "WIDTH 10, 3\n",
            "VIEW PRINT 2 TO 3\n",
            "PRINT \"A\"\n",
            "PRINT 1, 2\n",
            "WRITE SCREEN(2, 1), SCREEN(3, 1), CSRLIN\n",
        ),
    )
    .unwrap();

    let expected = "A\n1\n2\n49,50,3\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn print_using_comma_scrolls_when_zone_wrap_hits_view_print_bottom_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_print_using_scroll_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("print_using_scroll.bas");
    std::fs::write(
        &source_path,
        concat!(
            "WIDTH 10, 3\n",
            "VIEW PRINT 2 TO 3\n",
            "PRINT USING \"##\"; 1\n",
            "PRINT USING \"##\"; 2, 3\n",
            "WRITE SCREEN(2, 2), SCREEN(3, 2), CSRLIN\n",
        ),
    )
    .unwrap();

    let expected = " 1\n 2\n 3\n50,51,3\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn print_tab_scrolls_when_wrap_hits_full_screen_bottom_in_interpreter_and_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_print_tab_scroll_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("print_tab_scroll.bas");
    std::fs::write(
        &source_path,
        concat!(
            "WIDTH 10, 2\n",
            "PRINT STRING$(10, \"A\")\n",
            "PRINT \"1\"; TAB(1); \"2\"\n",
            "WRITE SCREEN(1, 1), SCREEN(2, 1), CSRLIN\n",
        ),
    )
    .unwrap();

    let expected = "AAAAAAAAAA\n\n1\n2\n49,50,2\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_all_example_matches_between_interpreter_and_native_after_normalizing_dynamic_lines() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let source_path = workspace_root.join("examples").join("test_all.bas");

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&workspace_root)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&workspace_root)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );

    let interpreter_stdout =
        normalize_test_all_output(&String::from_utf8_lossy(&interpreter.stdout));
    let native_stdout = normalize_test_all_output(&String::from_utf8_lossy(&native.stdout));
    assert_eq!(interpreter_stdout, native_stdout);
}

#[test]
fn def_fn_works_in_interpreter_and_default_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_def_fn_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("def_fn.bas");
    std::fs::write(&source_path, "DEF FNTWICE(X) = X * 2\nPRINT FNTWICE(5)\n").unwrap();

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert!(
        interpreter_stdout.starts_with("10\n"),
        "unexpected stdout: {interpreter_stdout}"
    );

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert!(
        native_stdout.starts_with("10\n"),
        "unexpected stdout: {native_stdout}"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn option_base_and_multidimensional_arrays_match_in_interpreter_and_default_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_option_base_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("option_base.bas");
    std::fs::write(
        &source_path,
        concat!(
            "OPTION BASE 1\n",
            "DIM A(3)\n",
            "DIM M(1 TO 2, 3 TO 4)\n",
            "A(1) = 10\n",
            "A(3) = 30\n",
            "M(1, 3) = 11\n",
            "M(2, 4) = 22\n",
            "WRITE LBOUND(A), UBOUND(A)\n",
            "WRITE A(1), A(3)\n",
            "WRITE LBOUND(M, 1), UBOUND(M, 1)\n",
            "WRITE LBOUND(M, 2), UBOUND(M, 2)\n",
            "WRITE M(1, 3), M(2, 4)\n",
        ),
    )
    .unwrap();

    let expected = "1,3\n10,30\n1,2\n3,4\n11,22\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "default run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn redim_preserve_string_arrays_match_in_interpreter_and_default_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_redim_string_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("redim_string.bas");
    std::fs::write(
        &source_path,
        concat!(
            "OPTION BASE 1\n",
            "REDIM A$(2)\n",
            "A$(1) = \"ALPHA\"\n",
            "A$(2) = \"BETA\"\n",
            "REDIM PRESERVE A$(3)\n",
            "WRITE LBOUND(A$), UBOUND(A$)\n",
            "PRINT A$(1) + \"|\" + A$(2) + \"|\" + A$(3)\n",
        ),
    )
    .unwrap();

    let expected = "1,3\nALPHA|BETA|\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "default run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn const_values_flow_into_def_fn_and_binary_seek_in_interpreter_and_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_const_def_fn_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("const_def_fn.bas");
    std::fs::write(
        &source_path,
        concat!(
            "CONST MY_PI = 3.141592653589793\n",
            "CLEAR\n",
            "DEF FNAREA(R) = MY_PI * (R ^ 2)\n",
            "PRINT CSNG(MY_PI)\n",
            "PRINT FNAREA(5)\n",
            "OPEN \"seek.txt\" FOR OUTPUT AS #1\n",
            "PRINT #1, \"Hello\"\n",
            "CLOSE #1\n",
            "OPEN \"seek.txt\" FOR BINARY AS #1\n",
            "SEEK #1, 1\n",
            "PRINT INPUT$(5, #1)\n",
            "CLOSE #1\n",
        ),
    )
    .unwrap();

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    let interpreter_lines = interpreter_stdout.lines().collect::<Vec<_>>();
    assert!(
        interpreter_lines
            .first()
            .is_some_and(|line| line.starts_with("3.14159")),
        "unexpected stdout: {interpreter_stdout}"
    );
    assert!(
        interpreter_lines
            .get(1)
            .is_some_and(|line| line.starts_with("78.5398")),
        "unexpected stdout: {interpreter_stdout}"
    );
    assert_eq!(interpreter_lines.get(2).copied(), Some("Hello"));

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    let native_lines = native_stdout.lines().collect::<Vec<_>>();
    assert!(
        native_lines
            .first()
            .is_some_and(|line| line.starts_with("3.14159")),
        "unexpected stdout: {native_stdout}"
    );
    assert!(
        native_lines
            .get(1)
            .is_some_and(|line| line.starts_with("78.5398")),
        "unexpected stdout: {native_stdout}"
    );
    assert_eq!(native_lines.get(2).copied(), Some("Hello"));

    let _ = std::fs::remove_file(temp_dir.join("seek.txt"));
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn tab_and_spc_print_controls_match_in_interpreter_and_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_print_ctrl_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("print_ctrl.bas");
    std::fs::write(
        &source_path,
        "PRINT \"A\"; TAB(5); \"B\"\nPRINT \"X\"; SPC(3); \"Y\"\n",
    )
    .unwrap();

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, "A   B\nX   Y\n");

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, "A   B\nX   Y\n");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn print_comma_separators_match_qbasic_zones_in_interpreter_and_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_print_comma_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("print_comma.bas");
    std::fs::write(&source_path, "PRINT 1024, \"QBASIC\"\nPRINT 10, 20\n").unwrap();

    let expected = "1024          QBASIC\n10            20\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn print_file_commas_preserve_print_zones_in_interpreter_and_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_print_file_comma_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("print_file_comma.bas");
    let out_path = temp_dir.join("out.txt");
    std::fs::write(
        &source_path,
        "OPEN \"out.txt\" FOR OUTPUT AS #1\nPRINT #1, \"HELLO\", \"WORLD\"\nPRINT #1, \"A\", \"B\"\nCLOSE #1\n",
    )
    .unwrap();

    let expected = "HELLO         WORLD\nA             B\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(&out_path)
            .unwrap()
            .replace('\r', ""),
        expected
    );
    let _ = std::fs::remove_file(&out_path);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(&out_path)
            .unwrap()
            .replace('\r', ""),
        expected
    );

    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn lof_eof_and_line_input_work_with_dynamic_file_numbers_in_interpreter_and_native_run() {
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_file_funcs_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("file_funcs.bas");
    std::fs::write(
        &source_path,
        concat!(
            "f = FREEFILE\n",
            "OPEN \"temp1.txt\" FOR OUTPUT AS #f\n",
            "PRINT #f, \"Hello\", 123\n",
            "WRITE #f, \"World\", 456\n",
            "CLOSE #f\n",
            "OPEN \"temp1.txt\" FOR APPEND AS #1\n",
            "PRINT #1, \"Append Data\"\n",
            "CLOSE #1\n",
            "OPEN \"temp1.txt\" FOR INPUT AS #f\n",
            "PRINT LOF(f)\n",
            "WHILE NOT EOF(f)\n",
            "    LINE INPUT #f, l$\n",
            "    PRINT l$\n",
            "WEND\n",
            "CLOSE #f\n",
        ),
    )
    .unwrap();

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    let interpreter_lines = interpreter_stdout.lines().collect::<Vec<_>>();
    assert_eq!(interpreter_lines.first().copied(), Some("42"));
    assert_eq!(interpreter_lines.get(1).copied(), Some("Hello         123"));
    assert_eq!(interpreter_lines.get(2).copied(), Some("\"World\",456"));
    assert_eq!(interpreter_lines.get(3).copied(), Some("Append Data"));

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    let native_lines = native_stdout.lines().collect::<Vec<_>>();
    assert_eq!(native_lines.first().copied(), Some("42"));
    assert_eq!(native_lines.get(1).copied(), Some("Hello         123"));
    assert_eq!(native_lines.get(2).copied(), Some("\"World\",456"));
    assert_eq!(native_lines.get(3).copied(), Some("Append Data"));

    let _ = std::fs::remove_file(temp_dir.join("temp1.txt"));
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn string_concatenation_with_builtin_string_results_matches_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_concat_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("concat.bas");
    std::fs::write(&source_path, "PRINT \"[\" + SPACE$(2) + \"]\"\n").unwrap();

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, "[  ]\n");

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, "[  ]\n");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn clear_resets_string_state_and_closes_open_files_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_clear_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("clear_runner.bas");
    std::fs::write(
        &source_path,
        "A$ = \"X\"\nOPEN \"held.txt\" FOR OUTPUT AS #1\nCLEAR\nPRINT \"[\"; A$; \"]\"\nPRINT FREEFILE\n",
    )
    .unwrap();

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, "[]\n1\n");

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, "[]\n1\n");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn files_pattern_filters_directory_entries_in_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_files_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    std::fs::write(temp_dir.join("alpha.bas"), "PRINT 1\n").unwrap();
    std::fs::write(temp_dir.join("beta.txt"), "x\n").unwrap();
    let source_path = temp_dir.join("runner.bas");
    std::fs::write(&source_path, "FILES \"a*.BAS\"\n").unwrap();

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    let interpreter_lines: Vec<&str> = interpreter_stdout.lines().collect();
    assert!(interpreter_lines.contains(&"alpha.bas"));
    assert!(!interpreter_lines.contains(&"beta.txt"));
    assert!(!interpreter_lines.contains(&"runner.bas"));

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    let native_lines: Vec<&str> = native_stdout.lines().collect();
    assert!(native_lines.contains(&"alpha.bas"));
    assert!(!native_lines.contains(&"beta.txt"));
    assert!(!native_lines.contains(&"runner.bas"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn inkey_loop_completes_in_noninteractive_mode_for_interpreter_and_native_run() {
    let temp_dir = std::env::temp_dir().join(format!("qbnex_cli_inkey_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("inkey.bas");
    std::fs::write(
        &source_path,
        "DO\nK$ = INKEY$\nLOOP UNTIL K$ <> \"\"\nPRINT \"done\"\n",
    )
    .unwrap();

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, "done\n");

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, "done\n");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn declared_string_params_locals_and_byref_calls_match_between_backends() {
    let temp_dir = std::env::temp_dir().join(format!(
        "qbnex_cli_declared_string_semantics_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_path = temp_dir.join("declared_string_semantics.bas");
    std::fs::write(
        &source_path,
        concat!(
            "FUNCTION TFStringToBool% (s AS STRING)\n",
            "    SELECT CASE _TRIM$(UCASE$(s))\n",
            "        CASE \"TRUE\": TFStringToBool% = -1\n",
            "        CASE \"FALSE\": TFStringToBool% = 0\n",
            "        CASE ELSE: TFStringToBool% = -2\n",
            "    END SELECT\n",
            "END FUNCTION\n",
            "SUB AppendSuffix (text AS STRING, suffix AS STRING)\n",
            "    text = text + suffix\n",
            "END SUB\n",
            "FUNCTION CheckDeclaredStringLocal% ()\n",
            "    DIM value AS STRING\n",
            "    value = \"  true  \"\n",
            "    CheckDeclaredStringLocal% = TFStringToBool%(value)\n",
            "END FUNCTION\n",
            "DIM v AS STRING\n",
            "v = \"A\"\n",
            "AppendSuffix v, \"B\"\n",
            "PRINT CheckDeclaredStringLocal%()\n",
            "PRINT v\n",
        ),
    )
    .unwrap();

    let expected = "-1\nAB\n";

    let interpreter = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg("-x")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        interpreter.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&interpreter.stdout),
        String::from_utf8_lossy(&interpreter.stderr)
    );
    let interpreter_stdout = String::from_utf8_lossy(&interpreter.stdout).replace('\r', "");
    assert_eq!(interpreter_stdout, expected);

    let native = Command::new(env!("CARGO_BIN_EXE_qb"))
        .arg("-q")
        .arg(&source_path)
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    assert!(
        native.status.success(),
        "native run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native.stdout).replace('\r', "");
    assert_eq!(native_stdout, expected);

    let _ = std::fs::remove_dir_all(&temp_dir);
}
