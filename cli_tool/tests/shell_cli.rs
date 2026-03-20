use std::process::Command;

fn normalize_test_all_output(output: &str) -> String {
    output
        .replace('\r', "")
        .lines()
        .map(|line| {
            if line.starts_with("RND (สุ่มตัวเลข): ") {
                "RND (สุ่มตัวเลข): <dynamic>".to_string()
            } else if line.starts_with("DATE$: ") {
                "DATE$: <dynamic>".to_string()
            } else if line.starts_with("TIMER (วินาทีตั้งแต่เที่ยงคืน): ") {
                "TIMER (วินาทีตั้งแต่เที่ยงคืน): <dynamic>".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
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
    std::fs::write(
        &source_path,
        "DEF FNTWICE(X) = X * 2\nPRINT FNTWICE(5)\n",
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
    assert!(interpreter_stdout.starts_with("10\n"), "unexpected stdout: {interpreter_stdout}");

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
    assert!(native_stdout.starts_with("10\n"), "unexpected stdout: {native_stdout}");

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
    assert_eq!(std::fs::read_to_string(&out_path).unwrap().replace('\r', ""), expected);
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
    assert_eq!(std::fs::read_to_string(&out_path).unwrap().replace('\r', ""), expected);

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
    let temp_dir =
        std::env::temp_dir().join(format!("qbnex_cli_concat_{}", std::process::id()));
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
