//! The bundle carried by an installer: a zip appended to the executable, then the
//! zip's length (u64 little endian) and [`MAGIC`].

use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const MAGIC: &[u8; 8] = b"QBNXSETP";
const TRAILER: usize = 16;

/// Splits an installer image into (the program, the zip). None when there is no bundle.
pub fn split(image: &[u8]) -> Option<(&[u8], &[u8])> {
    let n = image.len();
    if n < TRAILER || &image[n - 8..] != MAGIC {
        return None;
    }
    let len_bytes: [u8; 8] = image[n - TRAILER..n - 8].try_into().ok()?;
    let len = usize::try_from(u64::from_le_bytes(len_bytes)).ok()?;
    let start = (n - TRAILER).checked_sub(len)?;
    Some((&image[..start], &image[start..n - TRAILER]))
}

/// Joins a relative archive or manifest path to `dir`; None for absolute paths and `..`.
pub fn safe_join(dir: &Path, rel: &str) -> Option<PathBuf> {
    let rel = Path::new(rel);
    if rel.components().all(|c| matches!(c, Component::Normal(_))) && rel.components().next().is_some() {
        Some(dir.join(rel))
    } else {
        None
    }
}

/// Size in bytes of the files in the bundle once extracted.
pub fn unpacked_size(zip: &[u8]) -> Result<u64, String> {
    let mut archive = ZipArchive::new(Cursor::new(zip)).map_err(|e| format!("damaged installer: {e}"))?;
    let mut total = 0u64;
    for i in 0..archive.len() {
        total = total.saturating_add(archive.by_index_raw(i).map_err(|e| format!("damaged installer: {e}"))?.size());
    }
    Ok(total)
}

/// The files [`extract`] would write (relative, `/`-separated), without writing them.
pub fn names(zip: &[u8]) -> Result<Vec<String>, String> {
    let mut archive = ZipArchive::new(Cursor::new(zip)).map_err(|e| format!("damaged installer: {e}"))?;
    let mut out = Vec::new();
    for i in 0..archive.len() {
        let f = archive.by_index_raw(i).map_err(|e| format!("damaged installer: {e}"))?;
        if f.is_dir() {
            continue;
        }
        let Some(rel) = f.enclosed_name() else {
            let name = f.name().map(|n| n.into_owned()).unwrap_or_default();
            return Err(format!("damaged installer: unsafe path {name}"));
        };
        out.push(rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"));
    }
    Ok(out)
}

/// What stops [`extract`] when `on_file` returns false.
pub const CANCELLED: &str = "cancelled";

/// Extracts the bundle into `dir`. Every file is added to `files` (relative,
/// `/`-separated) before it is written, so after an error the list names all that
/// may exist. `on_file` gets each name and the bytes extracted before it; returning
/// false stops with [`CANCELLED`].
pub fn extract(zip: &[u8], dir: &Path, files: &mut Vec<String>, on_file: &mut dyn FnMut(&str, u64) -> bool) -> Result<(), String> {
    let mut archive = ZipArchive::new(Cursor::new(zip)).map_err(|e| format!("damaged installer: {e}"))?;
    let mut done = 0u64;
    for i in 0..archive.len() {
        let mut f = archive.by_index(i).map_err(|e| format!("damaged installer: {e}"))?;
        let Some(rel) = f.enclosed_name() else {
            let name = f.name().map(|n| n.into_owned()).unwrap_or_default();
            return Err(format!("damaged installer: unsafe path {name}"));
        };
        let path = dir.join(&rel);
        if f.is_dir() {
            std::fs::create_dir_all(&path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
        let name = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/");
        if !on_file(&name, done) {
            return Err(CANCELLED.into());
        }
        files.push(name);
        let mut out = std::fs::File::create(&path).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        done += std::io::copy(&mut f, &mut out).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        #[cfg(unix)]
        if let Some(mode) = f.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode & 0o777));
        }
    }
    Ok(())
}

/// Writes an installer to `out`: this program (without any bundle it carries), a zip
/// of `src`, and the trailer. Returns the number of files packed.
pub fn pack(src: &Path, out: &Path) -> Result<usize, String> {
    if !src.is_dir() {
        return Err(format!("{} is not a folder", src.display()));
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let image = std::fs::read(&exe).map_err(|e| format!("cannot read {}: {e}", exe.display()))?;
    let stub = split(&image).map_or(&image[..], |(s, _)| s);

    let mut paths = Vec::new();
    walk(src, &mut paths).map_err(|e| format!("cannot read {}: {e}", src.display()))?;
    paths.sort();
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    for p in &paths {
        let rel = p.strip_prefix(src).map_err(|e| e.to_string())?;
        let name = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/");
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated).unix_permissions(mode_of(p));
        w.start_file(name, opts).map_err(|e| e.to_string())?;
        let mut data = Vec::new();
        std::fs::File::open(p).and_then(|mut f| f.read_to_end(&mut data)).map_err(|e| format!("cannot read {}: {e}", p.display()))?;
        w.write_all(&data).map_err(|e| e.to_string())?;
    }
    let zip = w.finish().map_err(|e| e.to_string())?.into_inner();

    let mut bytes = Vec::with_capacity(stub.len() + zip.len() + TRAILER);
    bytes.extend_from_slice(stub);
    bytes.extend_from_slice(&zip);
    bytes.extend_from_slice(&(zip.len() as u64).to_le_bytes());
    bytes.extend_from_slice(MAGIC);
    std::fs::write(out, &bytes).map_err(|e| format!("cannot write {}: {e}", out.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(out, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    Ok(paths.len())
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let t = e.file_type()?;
        if t.is_dir() {
            walk(&e.path(), out)?;
        } else if t.is_file() {
            out.push(e.path());
        }
    }
    Ok(())
}

#[cfg(unix)]
fn mode_of(p: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p).map(|m| m.permissions().mode() & 0o777).unwrap_or(0o644)
}

#[cfg(not(unix))]
fn mode_of(_: &Path) -> u32 {
    0o644
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_finds_the_bundle() {
        let mut img = b"PROGRAM".to_vec();
        img.extend_from_slice(b"ZIPDATA");
        img.extend_from_slice(&7u64.to_le_bytes());
        img.extend_from_slice(MAGIC);
        assert_eq!(split(&img), Some((&b"PROGRAM"[..], &b"ZIPDATA"[..])));
        assert_eq!(split(b"PROGRAM"), None);
        // a length larger than the file is rejected, not a panic
        let mut bad = b"X".to_vec();
        bad.extend_from_slice(&u64::MAX.to_le_bytes());
        bad.extend_from_slice(MAGIC);
        assert_eq!(split(&bad), None);
    }

    #[test]
    fn safe_join_rejects_escapes() {
        let d = Path::new("base");
        assert!(safe_join(d, "a/b.txt").is_some());
        assert!(safe_join(d, "../x").is_none());
        assert!(safe_join(d, "").is_none());
        assert!(safe_join(d, if cfg!(windows) { "C:\\x" } else { "/x" }).is_none());
    }

    #[test]
    fn extract_round_trip() {
        let mut w = ZipWriter::new(Cursor::new(Vec::new()));
        let o = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        w.start_file("compiler/stdlib/a.bas", o).unwrap();
        w.write_all(b"PRINT 1").unwrap();
        let zip = w.finish().unwrap().into_inner();
        let dir = std::env::temp_dir().join(format!("qbnex-setup-x-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(unpacked_size(&zip), Ok(7));
        let mut files = Vec::new();
        extract(&zip, &dir, &mut files, &mut |_, _| true).unwrap();
        assert_eq!(files, vec!["compiler/stdlib/a.bas".to_string()]);
        // a refusal stops before the file is written or listed
        let mut none = Vec::new();
        assert_eq!(extract(&zip, &dir.join("no"), &mut none, &mut |_, _| false), Err(CANCELLED.to_string()));
        assert!(none.is_empty() && !dir.join("no/compiler/stdlib/a.bas").exists());
        assert_eq!(std::fs::read(dir.join("compiler/stdlib/a.bas")).unwrap(), b"PRINT 1");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
