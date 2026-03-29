use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

static CONFORMANCE_COUNTER: AtomicU64 = AtomicU64::new(0);

const FIXTURE_STEMS: &[&str] = &[
    "arrays_and_bounds",
    "control_flow",
    "data_restore",
    "numeric_operators",
    "on_error",
    "print_using",
    "procedures_and_def_fn",
    "random_field_io",
    "sequential_file_io",
    "string_intrinsics",
    "user_defined_types",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("cli directory should live under the workspace root")
        .to_path_buf()
}

fn conformance_root() -> PathBuf {
    repo_root()
        .join("tests")
        .join("conformance")
        .join("non_dos_quickbasic")
}

fn fixture_source_path(stem: &str) -> PathBuf {
    conformance_root().join(format!("{stem}.bas"))
}

fn fixture_output_path(stem: &str) -> PathBuf {
    conformance_root().join(format!("{stem}.out"))
}

fn read_expected_output(stem: &str) -> String {
    fs::read_to_string(fixture_output_path(stem))
        .unwrap_or_else(|err| panic!("failed to read expected output for {stem}: {err}"))
        .replace("\r\n", "\n")
        .replace('\r', "")
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let counter = CONFORMANCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "qbnex_{prefix}_{}_{}_{}",
        std::process::id(),
        nanos,
        counter
    ))
}

fn retry_output_path(command: &Command) -> Option<PathBuf> {
    let current_dir = command
        .get_current_dir()
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())?;

    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    if let Some(index) = args.iter().position(|arg| arg == "-o") {
        let raw = args.get(index + 1)?;
        let path = PathBuf::from(raw);
        return Some(if path.is_absolute() {
            path
        } else {
            current_dir.join(path)
        });
    }

    let source = args
        .iter()
        .find(|arg| arg.to_ascii_lowercase().ends_with(".bas"))?;
    let source_path = Path::new(source);
    let stem = source_path.file_stem()?.to_string_lossy();
    let output_name = if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    };
    Some(current_dir.join(output_name))
}

#[cfg(windows)]
fn cleanup_retried_command_child(command: &Command) {
    let Some(output_path) = retry_output_path(command) else {
        return;
    };
    if !output_path.exists() {
        return;
    }

    let escaped = output_path.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$path = '{}'; Get-CimInstance Win32_Process | Where-Object {{ $_.ExecutablePath -eq $path }} | ForEach-Object {{ try {{ Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }} catch {{}} }}",
        escaped
    );
    let _ = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output();
}

#[cfg(not(windows))]
fn cleanup_retried_command_child(_command: &Command) {}

trait CommandTestExt {
    fn stable_output(&mut self) -> Output;
}

impl CommandTestExt for Command {
    fn stable_output(&mut self) -> Output {
        let mut last = self.output().unwrap();
        for attempt in 0..3 {
            if last.status.success() || !last.stdout.is_empty() || !last.stderr.is_empty() {
                return last;
            }

            if attempt == 2 {
                break;
            }

            cleanup_retried_command_child(self);
            std::thread::sleep(Duration::from_millis(250 * (attempt + 1) as u64));
            last = self.output().unwrap();
        }
        last
    }
}

fn normalize_output(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .replace("\r\n", "\n")
        .replace('\r', "")
}

fn copy_fixture_into_workspace(stem: &str, workspace: &Path) -> PathBuf {
    let source = fixture_source_path(stem);
    let destination = workspace.join(format!("{stem}.bas"));
    fs::copy(&source, &destination)
        .unwrap_or_else(|err| panic!("failed to copy {}: {err}", source.display()));
    destination
}

fn run_qb_command(command: &mut Command, label: &str) -> Result<String, String> {
    let output = command.stable_output();
    if !output.status.success() {
        return Err(format!(
            "{label} failed\nstdout:\n{}\nstderr:\n{}",
            normalize_output(&output.stdout),
            normalize_output(&output.stderr)
        ));
    }
    Ok(normalize_output(&output.stdout))
}

fn run_binary(binary: &Path, cwd: &Path, label: &str) -> Result<String, String> {
    let output = Command::new(binary).current_dir(cwd).stable_output();
    if !output.status.success() {
        return Err(format!(
            "{label} failed\nstdout:\n{}\nstderr:\n{}",
            normalize_output(&output.stdout),
            normalize_output(&output.stderr)
        ));
    }
    Ok(normalize_output(&output.stdout))
}

fn assert_fixture_output(stem: &str) -> Result<(), String> {
    let temp_dir = unique_temp_dir("qbasic_conformance");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).map_err(|err| err.to_string())?;

    let result = (|| {
        let source_path = copy_fixture_into_workspace(stem, &temp_dir);
        let expected = read_expected_output(stem);

        let default_stdout = run_qb_command(
            Command::new(env!("CARGO_BIN_EXE_qb"))
                .arg("-q")
                .arg(&source_path)
                .current_dir(&temp_dir),
            &format!("default run for {stem}"),
        )?;
        if default_stdout != expected {
            return Err(format!(
                "default run output mismatch for {stem}\nexpected:\n{expected}\nactual:\n{default_stdout}"
            ));
        }

        let vm_runner_path = if cfg!(windows) {
            temp_dir.join(format!("{stem}_vm.exe"))
        } else {
            temp_dir.join(format!("{stem}_vm"))
        };
        let vm_stdout = run_qb_command(
            Command::new(env!("CARGO_BIN_EXE_qb"))
                .arg("-q")
                .arg("-x")
                .arg(&source_path)
                .arg("-o")
                .arg(&vm_runner_path)
                .current_dir(&temp_dir),
            &format!("vm run for {stem}"),
        )?;
        if vm_stdout != expected {
            return Err(format!(
                "VM runner output mismatch for {stem}\nexpected:\n{expected}\nactual:\n{vm_stdout}"
            ));
        }

        let compiled_binary = if cfg!(windows) {
            temp_dir.join("build").join(format!("{stem}.exe"))
        } else {
            temp_dir.join("build").join(stem)
        };
        let compile_output = Command::new(env!("CARGO_BIN_EXE_qb"))
            .arg("-q")
            .arg("-c")
            .arg(&source_path)
            .arg("-o")
            .arg(&compiled_binary)
            .current_dir(&temp_dir)
            .stable_output();
        if !compile_output.status.success() {
            return Err(format!(
                "compile-only build failed for {stem}\nstdout:\n{}\nstderr:\n{}",
                normalize_output(&compile_output.stdout),
                normalize_output(&compile_output.stderr)
            ));
        }
        if !compiled_binary.exists() {
            return Err(format!(
                "compile-only build for {stem} did not produce {}",
                compiled_binary.display()
            ));
        }

        let compiled_stdout = run_binary(
            &compiled_binary,
            &temp_dir,
            &format!("compiled executable for {stem}"),
        )?;
        if compiled_stdout != expected {
            return Err(format!(
                "compiled executable output mismatch for {stem}\nexpected:\n{expected}\nactual:\n{compiled_stdout}"
            ));
        }

        Ok(())
    })();

    let _ = fs::remove_dir_all(&temp_dir);
    result
}

#[test]
fn non_dos_quickbasic_fixtures_match_expected_output_across_cli_execution_paths() {
    let mut failures = Vec::new();

    for stem in FIXTURE_STEMS {
        if let Err(err) = assert_fixture_output(stem) {
            failures.push(err);
        }
    }

    assert!(
        failures.is_empty(),
        "non-DOS QBasic/QuickBASIC conformance regressions:\n{}",
        failures.join("\n\n")
    );
}
