use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

mod fixture_io_catalog {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/fixture_io_catalog.rs"
    ));
}

static CONFORMANCE_COUNTER: AtomicU64 = AtomicU64::new(0);

const FIXTURE_STEMS: &[&str] = &[
    "arrays_and_bounds",
    "byval_byref",
    "clear_freefile",
    "common_shared",
    "common_shared_include",
    "computed_branching",
    "console_input",
    "const_and_def_fn",
    "control_flow",
    "data_restore",
    "def_type_coercion",
    "erase_redim",
    "fixed_length_lset_rset",
    "logical_comparisons",
    "loop_controls",
    "mid_assignment",
    "numeric_operators",
    "on_play_event",
    "on_timer_event",
    "on_error",
    "print_using",
    "procedures_and_def_fn",
    "random_field_io",
    "screen_text_state",
    "sequential_file_io",
    "select_case_advanced",
    "shared_globals",
    "static_state",
    "string_intrinsics",
    "swap_and_clear",
    "type_conversions",
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

fn read_expected_output(stem: &str) -> String {
    fixture_io_catalog::conformance_expected_output(stem)
        .unwrap_or_else(|| panic!("missing expected output fixture for {stem}"))
        .replace("\r\n", "\n")
        .replace('\r', "")
        .replace("<BEL>", "\u{7}")
}

fn read_fixture_input(stem: &str) -> Option<Vec<u8>> {
    fixture_io_catalog::conformance_input(stem).map(|input| input.as_bytes().to_vec())
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
    fn stable_output_with_input(&mut self, input: &[u8]) -> Output;
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

    fn stable_output_with_input(&mut self, input: &[u8]) -> Output {
        let mut last = command_output_with_input(self, input);
        for attempt in 0..3 {
            if last.status.success() || !last.stdout.is_empty() || !last.stderr.is_empty() {
                return last;
            }

            if attempt == 2 {
                break;
            }

            cleanup_retried_command_child(self);
            std::thread::sleep(Duration::from_millis(250 * (attempt + 1) as u64));
            last = command_output_with_input(self, input);
        }
        last
    }
}

fn command_output_with_input(command: &mut Command, input: &[u8]) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(input)
        .unwrap();
    child.wait_with_output().unwrap()
}

fn normalize_output(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .replace("\r\n", "\n")
        .replace('\r', "")
}

fn copy_fixture_into_workspace(stem: &str, workspace: &Path) -> PathBuf {
    let fixture_file_name = format!("{stem}.bas");
    let fixture_dot_prefix = format!("{stem}.");
    let fixture_helper_prefix = format!("{stem}_");
    let main_source = fixture_source_path(stem);

    for entry in fs::read_dir(conformance_root())
        .unwrap_or_else(|err| panic!("failed to enumerate conformance fixtures for {stem}: {err}"))
    {
        let entry =
            entry.unwrap_or_else(|err| panic!("failed to read fixture entry for {stem}: {err}"));
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if !file_name.eq_ignore_ascii_case(&fixture_file_name)
            && !file_name.starts_with(&fixture_dot_prefix)
            && !file_name.starts_with(&fixture_helper_prefix)
        {
            continue;
        }

        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default();
        if extension.eq_ignore_ascii_case("out") || extension.eq_ignore_ascii_case("in") {
            continue;
        }

        let destination = workspace.join(file_name);
        fs::copy(&path, &destination)
            .unwrap_or_else(|err| panic!("failed to copy {}: {err}", path.display()));
    }

    let destination = workspace.join(format!("{stem}.bas"));
    assert!(
        destination.exists(),
        "main BASIC fixture {} was not copied",
        main_source.display()
    );
    destination
}

fn run_qb_command(
    command: &mut Command,
    label: &str,
    input: Option<&[u8]>,
) -> Result<String, String> {
    let output = if let Some(input) = input {
        command.stable_output_with_input(input)
    } else {
        command.stable_output()
    };
    if !output.status.success() {
        return Err(format!(
            "{label} failed\nstdout:\n{}\nstderr:\n{}",
            normalize_output(&output.stdout),
            normalize_output(&output.stderr)
        ));
    }
    Ok(normalize_output(&output.stdout))
}

fn run_binary(
    binary: &Path,
    cwd: &Path,
    label: &str,
    input: Option<&[u8]>,
) -> Result<String, String> {
    let mut command = Command::new(binary);
    command.current_dir(cwd);
    let output = if let Some(input) = input {
        command.stable_output_with_input(input)
    } else {
        command.stable_output()
    };
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
        let input = read_fixture_input(stem);

        let default_stdout = run_qb_command(
            Command::new(env!("CARGO_BIN_EXE_qb"))
                .arg("-q")
                .arg(&source_path)
                .current_dir(&temp_dir),
            &format!("default run for {stem}"),
            input.as_deref(),
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
            input.as_deref(),
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
            input.as_deref(),
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

fn selected_fixture_stems() -> Vec<&'static str> {
    let Some(filter) = std::env::var("QBNEX_CONFORMANCE_FILTER")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return FIXTURE_STEMS.to_vec();
    };

    let selected = filter
        .split(',')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    FIXTURE_STEMS
        .iter()
        .copied()
        .filter(|stem| selected.iter().any(|item| item.eq_ignore_ascii_case(stem)))
        .collect()
}

#[test]
fn non_dos_quickbasic_fixtures_match_expected_output_across_cli_execution_paths() {
    let mut failures = Vec::new();
    let stems = selected_fixture_stems();

    for stem in stems {
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
