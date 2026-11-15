//! Where `DECLARE DYNAMIC LIBRARY "name"` looks for a library: the program's
//! folder and the system folders, never the current directory; an explicit
//! relative path still means the current directory. One test in its own
//! binary, because it changes the current directory of the process.

use qb_ffi::{Library, program_dir, set_program_dir};
use std::path::Path;

fn build_lib(out: &Path, name: &str) -> std::path::PathBuf {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("testlib.rs");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let status = std::process::Command::new(rustc)
        .args(["--crate-type", "cdylib", "--crate-name", name, "-O", "--edition", "2024"])
        .arg(&src)
        .arg("--out-dir")
        .arg(out)
        .args(std::env::var("QB_FFI_TEST_RUSTC_ARGS").unwrap_or_default().split_whitespace())
        .status()
        .expect("run rustc");
    assert!(status.success());
    out.join(format!("{}{name}{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX))
}

#[test]
fn bare_names_skip_the_current_directory() {
    let root = std::env::temp_dir().join(format!("qb_ffi_order_{}", std::process::id()));
    let (cwd, prog, sub) = (root.join("cwd"), root.join("prog"), root.join("cwd").join("sub"));
    for d in [&cwd, &prog, &sub] {
        std::fs::create_dir_all(d).unwrap();
    }
    let built = build_lib(&cwd, "qbffiplant");
    let file = built.file_name().unwrap().to_owned();
    std::fs::copy(&built, sub.join(&file)).unwrap();
    std::env::set_current_dir(&cwd).unwrap();

    // a file of that name in the current directory is not loaded
    set_program_dir(Some(&prog));
    let e = Library::open_dynamic("qbffiplant", None).expect_err("must not load from the current directory");
    assert!(!e.tried.iter().any(|t| Path::new(t).parent().is_none_or(|p| p.as_os_str().is_empty())), "{e}");

    // the program's own folder is
    std::fs::copy(&built, prog.join(&file)).unwrap();
    let lib = Library::open_dynamic("qbffiplant", None).expect("load from the program folder");
    assert!(Path::new(lib.path()).starts_with(&prog), "{}", lib.path());
    assert!(lib.symbol("qb_test_neg_i16").is_some());
    drop(lib);

    // an explicit relative path is relative to the current directory, as before
    let lib = Library::open_dynamic("sub/qbffiplant", None).expect("explicit relative path");
    assert!(lib.path().starts_with("sub"), "{}", lib.path());
    drop(lib);

    // by default the program's folder is the executable's
    set_program_dir(None);
    assert_eq!(program_dir(), std::env::current_exe().unwrap().parent().map(Path::to_path_buf));

    std::env::set_current_dir(std::env::temp_dir()).unwrap();
    let _ = std::fs::remove_dir_all(&root);
}
