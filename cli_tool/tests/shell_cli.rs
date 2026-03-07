use std::process::Command;

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
