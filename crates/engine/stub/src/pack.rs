//! `qb-pack <runner> --stub <qb-stub> -o <out>`: writes the launcher followed by
//! the runner, filtered and compressed with brotli (layout: crate `qb-payload`).
//! The compiler copies `<out>` in front of every program it builds.

use std::process::exit;

fn opt(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn read(path: &str) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("qb-pack: cannot read {path}: {e}");
        exit(2)
    })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(runner), Some(stub), Some(out)) = (args.first().filter(|a| !a.starts_with('-')), opt(&args, "--stub"), opt(&args, "-o")) else {
        eprintln!("usage: qb-pack <qb-exec> --stub <qb-stub> -o <packed runner>");
        exit(2)
    };
    let runner = read(runner);
    let stub = read(&stub);
    if qb_payload::program(&runner).is_some() {
        eprintln!("qb-pack: the runner already carries a program");
        exit(2)
    }
    let filter = qb_payload::Filter::for_executable(&runner);
    let id = qb_payload::runner_id(&runner);
    let mut filtered = runner.clone();
    filter.apply(&mut filtered, true);
    // the largest window and quality: packing runs once, unpacking stays fast
    let params = brotli::enc::BrotliEncoderParams { quality: 11, lgwin: 24, size_hint: filtered.len(), ..Default::default() };
    let mut packed = Vec::with_capacity(filtered.len() / 2);
    if let Err(e) = brotli::BrotliCompress(&mut &filtered[..], &mut packed, &params) {
        eprintln!("qb-pack: cannot compress: {e}");
        exit(1)
    }
    // the launcher unpacks exactly this: check it before anything uses the file
    let mut back = Vec::with_capacity(runner.len());
    let unpacked = brotli_decompressor::BrotliDecompress(&mut &packed[..], &mut back).is_ok();
    filter.apply(&mut back, false);
    if !unpacked || back != runner {
        eprintln!("qb-pack: the packed runner does not unpack to the runner");
        exit(1)
    }
    let mut file = stub;
    file.extend_from_slice(&packed);
    file.extend_from_slice(&qb_payload::runner_trailer(packed.len() as u64, runner.len() as u64, id, filter));
    if let Err(e) = std::fs::write(&out, &file) {
        eprintln!("qb-pack: cannot write {out}: {e}");
        exit(2)
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o755));
    }
    println!("qb-pack: {} -> {} bytes ({:.1}%, {filter:?} filter)", runner.len(), file.len(), file.len() as f64 * 100.0 / runner.len() as f64);
}
