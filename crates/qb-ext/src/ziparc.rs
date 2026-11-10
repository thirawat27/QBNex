//! Zip archives: list, read one entry, extract all, create.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// One archive member.
#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub size: u64,
    pub compressed_size: u64,
    pub is_dir: bool,
}

fn open(path: &Path) -> Result<ZipArchive<File>, String> {
    let f = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    ZipArchive::new(f).map_err(|e| e.to_string())
}

/// Lists the members in archive order.
pub fn list(path: &Path) -> Result<Vec<Entry>, String> {
    let mut z = open(path)?;
    let mut out = Vec::with_capacity(z.len());
    for i in 0..z.len() {
        let f = z.by_index(i).map_err(|e| e.to_string())?;
        let name = f.name().map(|n| n.into_owned()).map_err(|e| e.to_string())?;
        out.push(Entry { name, size: f.size(), compressed_size: f.compressed_size(), is_dir: f.is_dir() });
    }
    Ok(out)
}

/// Reads one member into memory.
pub fn read(path: &Path, name: &str) -> Result<Vec<u8>, String> {
    let mut z = open(path)?;
    let mut f = z.by_name(name).map_err(|e| format!("{name}: {e}"))?;
    let mut buf = Vec::with_capacity(usize::try_from(f.size()).unwrap_or(0));
    f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

/// Extracts every member below `dest` (created if needed). Members whose
/// names would leave `dest` (`../x`, absolute paths) are rejected.
/// Returns the number of files written.
pub fn extract(path: &Path, dest: &Path) -> Result<usize, String> {
    let mut z = open(path)?;
    fs::create_dir_all(dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    let mut files = 0;
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel) = f.enclosed_name() else {
            return Err(format!("unsafe member name: {}", f.name().map(|n| n.into_owned()).unwrap_or_default()));
        };
        let target: PathBuf = dest.join(rel);
        if f.is_dir() {
            fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = File::create(&target).map_err(|e| format!("{}: {e}", target.display()))?;
        std::io::copy(&mut f, &mut out).map_err(|e| e.to_string())?;
        files += 1;
    }
    Ok(files)
}

/// An archive being written.
pub struct Builder {
    w: ZipWriter<File>,
}

fn options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated).large_file(false)
}

impl Builder {
    /// Creates (or truncates) `path`.
    pub fn create(path: &Path) -> Result<Self, String> {
        let f = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Builder { w: ZipWriter::new(f) })
    }

    /// Adds a member with the given bytes. `/` separates folders.
    pub fn add_data(&mut self, name: &str, data: &[u8]) -> Result<(), String> {
        let opts = if data.len() > u32::MAX as usize { options().large_file(true) } else { options() };
        self.w.start_file(name, opts).map_err(|e| e.to_string())?;
        self.w.write_all(data).map_err(|e| e.to_string())
    }

    /// Adds a file from disk under `name`.
    pub fn add_file(&mut self, name: &str, source: &Path) -> Result<(), String> {
        let data = fs::read(source).map_err(|e| format!("{}: {e}", source.display()))?;
        self.add_data(name, &data)
    }

    /// Adds an (empty) folder member.
    pub fn add_dir(&mut self, name: &str) -> Result<(), String> {
        self.w.add_directory(name, options()).map_err(|e| e.to_string())
    }

    /// Writes the central directory and closes the file.
    pub fn finish(self) -> Result<(), String> {
        self.w.finish().map(|_| ()).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("qb_ext_zip_{}_{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn create_list_read_extract() {
        let d = tmp("round");
        let src = d.join("src.txt");
        fs::write(&src, b"from disk").unwrap();
        let zp = d.join("a.zip");
        let mut b = Builder::create(&zp).unwrap();
        b.add_data("hello.txt", b"hello hello hello hello").unwrap();
        b.add_dir("sub/").unwrap();
        b.add_file("sub/disk.txt", &src).unwrap();
        b.add_data("empty.bin", b"").unwrap();
        b.finish().unwrap();

        let l = list(&zp).unwrap();
        let names: Vec<&str> = l.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["hello.txt", "sub/", "sub/disk.txt", "empty.bin"]);
        assert_eq!(l[0].size, 23);
        assert!(l[1].is_dir);
        assert_eq!(read(&zp, "sub/disk.txt").unwrap(), b"from disk");
        assert!(read(&zp, "nope").is_err());

        let out = d.join("out");
        assert_eq!(extract(&zp, &out).unwrap(), 3);
        assert_eq!(fs::read(out.join("hello.txt")).unwrap(), b"hello hello hello hello");
        assert_eq!(fs::read(out.join("sub").join("disk.txt")).unwrap(), b"from disk");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn rejects_path_traversal_and_bad_files() {
        let d = tmp("evil");
        let zp = d.join("evil.zip");
        let mut b = Builder::create(&zp).unwrap();
        b.add_data("../escape.txt", b"x").unwrap();
        b.finish().unwrap();
        assert!(extract(&zp, &d.join("out")).unwrap_err().contains("unsafe"));
        assert!(!d.join("escape.txt").exists());
        fs::write(d.join("not.zip"), b"not a zip").unwrap();
        assert!(list(&d.join("not.zip")).is_err());
        assert!(list(&d.join("missing.zip")).is_err());
        let _ = fs::remove_dir_all(&d);
    }
}
