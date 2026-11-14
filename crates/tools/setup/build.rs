//! Gives the Windows installer the QBNex icon (Explorer, the taskbar, the
//! download list) and its version information (the Details tab of the file's
//! properties: description, versions, product name, copyright, language).
//! Both go into a Windows resource file (`.res`), written here in Rust, which
//! the linker takes as an input: MSVC `link.exe` and `lld-link` read `.res`
//! files, so no resource compiler is needed.

use std::path::Path;

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;
/// MOVEABLE | DISCARDABLE (| PURE for the group), as `rc.exe` writes them.
const FLAGS_ICON: u16 = 0x1010;
const FLAGS_GROUP: u16 = 0x1030;
/// MOVEABLE | PURE.
const FLAGS_VERSION: u16 = 0x0030;
/// English (United States), the language `rc.exe` uses by default.
const LANG: u16 = 0x0409;
/// The code page of the strings: UTF-16 (1200).
const CODEPAGE: u16 = 1200;

const PRODUCT: &str = "QBNex";
const DESCRIPTION: &str = "QBNex Setup";
/// As in NOTICE.txt.
const COPYRIGHT: &str = "Copyright (C) 2026 Thirawat Sinlapasomsak";

fn main() {
    let ico_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/icons/windows/QBNex.ico");
    println!("cargo:rerun-if-changed={}", ico_path.display());
    println!("cargo:rerun-if-changed=build.rs");
    // the GNU linker does not read .res files; the releases are MSVC builds
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") || std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let ico = std::fs::read(&ico_path).unwrap_or_else(|e| panic!("cannot read {}: {e}", ico_path.display()));
    let mut res = res_from_ico(&ico).unwrap_or_else(|e| panic!("{}: {e}", ico_path.display()));
    let version = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION");
    entry(&mut res, RT_VERSION, 1, FLAGS_VERSION, &version_info(&version));
    let out = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("qbnex-setup.res");
    std::fs::write(&out, res).unwrap_or_else(|e| panic!("cannot write {}: {e}", out.display()));
    println!("cargo:rustc-link-arg-bins={}", out.display());
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// One resource entry: header (numeric type and name), then the data padded to 4 bytes.
fn entry(out: &mut Vec<u8>, kind: u16, id: u16, flags: u16, data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_le_bytes()); // DataSize
    out.extend_from_slice(&32u32.to_le_bytes()); // HeaderSize
    out.extend_from_slice(&[0xFF, 0xFF]);
    out.extend_from_slice(&kind.to_le_bytes()); // TYPE
    out.extend_from_slice(&[0xFF, 0xFF]);
    out.extend_from_slice(&id.to_le_bytes()); // NAME
    out.extend_from_slice(&0u32.to_le_bytes()); // DataVersion
    out.extend_from_slice(&flags.to_le_bytes()); // MemoryFlags
    out.extend_from_slice(&LANG.to_le_bytes()); // LanguageId
    out.extend_from_slice(&0u32.to_le_bytes()); // Version
    out.extend_from_slice(&0u32.to_le_bytes()); // Characteristics
    out.extend_from_slice(data);
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}

/// A `.res` file holding every image of the `.ico` (RT_ICON 1..n) and the
/// icon group 1 that lists them (RT_GROUP_ICON), the first icon Explorer shows.
fn res_from_ico(ico: &[u8]) -> Result<Vec<u8>, String> {
    let bad = || "not an .ico file".to_string();
    if u16_at(ico, 0) != Some(0) || u16_at(ico, 2) != Some(1) {
        return Err(bad());
    }
    let count = u16_at(ico, 4).ok_or_else(bad)?;
    if count == 0 {
        return Err("the .ico file has no images".into());
    }
    let mut out = Vec::new();
    // the empty first entry that marks a 32-bit resource file
    out.extend_from_slice(&[0, 0, 0, 0, 32, 0, 0, 0, 0xFF, 0xFF, 0, 0, 0xFF, 0xFF, 0, 0]);
    out.extend_from_slice(&[0; 16]);
    let mut group = Vec::new();
    group.extend_from_slice(&0u16.to_le_bytes());
    group.extend_from_slice(&1u16.to_le_bytes());
    group.extend_from_slice(&count.to_le_bytes());
    for i in 0..count {
        let d = 6 + 16 * usize::from(i);
        let head = ico.get(d..d + 8).ok_or_else(bad)?; // width, height, colours, reserved, planes, bit count
        let size = u32_at(ico, d + 8).ok_or_else(bad)? as usize;
        let offset = u32_at(ico, d + 12).ok_or_else(bad)? as usize;
        let image = ico.get(offset..offset.checked_add(size).ok_or_else(bad)?).ok_or_else(bad)?;
        let id = i + 1;
        entry(&mut out, RT_ICON, id, FLAGS_ICON, image);
        group.extend_from_slice(head);
        group.extend_from_slice(&(size as u32).to_le_bytes());
        group.extend_from_slice(&id.to_le_bytes());
    }
    entry(&mut out, RT_GROUP_ICON, 1, FLAGS_GROUP, &group);
    Ok(out)
}

/// UTF-16LE with the terminating 0.
fn wide(s: &str) -> Vec<u8> {
    s.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect()
}

fn pad4(b: &mut Vec<u8>) {
    while !b.len().is_multiple_of(4) {
        b.push(0);
    }
}

/// One block of a version resource: length, value length, type (1 text,
/// 0 binary), key, the value and the child blocks, each on a 4-byte boundary.
fn block(key: &str, value: &[u8], value_len: u16, text: bool, children: &[Vec<u8>]) -> Vec<u8> {
    let mut b = vec![0, 0];
    b.extend_from_slice(&value_len.to_le_bytes());
    b.extend_from_slice(&u16::from(text).to_le_bytes());
    b.extend_from_slice(&wide(key));
    pad4(&mut b);
    b.extend_from_slice(value);
    for c in children {
        pad4(&mut b);
        b.extend_from_slice(c);
    }
    let len = b.len() as u16;
    b[..2].copy_from_slice(&len.to_le_bytes());
    b
}

fn string(key: &str, value: &str) -> Vec<u8> {
    let v = wide(value);
    block(key, &v, (v.len() / 2) as u16, true, &[])
}

/// VS_VERSIONINFO for `version` ("2.0.0-preview.2"): the numeric version is
/// 2.0.0.2 (the last number of a pre-release suffix, else 0), the text one
/// is the full version, and a pre-release is flagged as one.
fn version_info(version: &str) -> Vec<u8> {
    let (core, pre) = version.split_once('-').map_or((version, None), |(c, p)| (c, Some(p)));
    let mut n = core.split('.').map(|x| x.parse::<u16>().unwrap_or(0)).chain(std::iter::repeat(0));
    let (major, minor, patch) = (n.next().unwrap_or(0), n.next().unwrap_or(0), n.next().unwrap_or(0));
    let build = pre.and_then(|p| p.rsplit('.').next()).and_then(|x| x.parse::<u16>().ok()).unwrap_or(0);
    let ms = (u32::from(major) << 16) | u32::from(minor);
    let ls = (u32::from(patch) << 16) | u32::from(build);
    const VS_FF_PRERELEASE: u32 = 0x2;
    const VOS_NT_WINDOWS32: u32 = 0x0004_0004;
    const VFT_APP: u32 = 0x1;
    let fixed: Vec<u8> = [
        0xFEEF_04BD, // signature
        0x0001_0000, // structure version
        ms,
        ls,
        ms,
        ls,
        0x3F, // valid flags
        if pre.is_some() { VS_FF_PRERELEASE } else { 0 },
        VOS_NT_WINDOWS32,
        VFT_APP,
        0,
        0,
        0,
    ]
    .iter()
    .flat_map(|v: &u32| v.to_le_bytes())
    .collect();
    let numeric = format!("{major}.{minor}.{patch}.{build}");
    let strings = [
        string("CompanyName", "Thirawat Sinlapasomsak"),
        string("FileDescription", DESCRIPTION),
        string("FileVersion", &numeric),
        string("InternalName", "qbnex-setup"),
        string("LegalCopyright", COPYRIGHT),
        string("OriginalFilename", "qbnex-setup.exe"),
        string("ProductName", PRODUCT),
        string("ProductVersion", version),
    ];
    let table = block(&format!("{LANG:04X}{CODEPAGE:04X}"), &[], 0, true, &strings);
    let string_info = block("StringFileInfo", &[], 0, true, &[table]);
    let translation: Vec<u8> = [LANG, CODEPAGE].iter().flat_map(|v| v.to_le_bytes()).collect();
    let var = block("Translation", &translation, 4, false, &[]);
    let var_info = block("VarFileInfo", &[], 0, true, &[var]);
    block("VS_VERSION_INFO", &fixed, fixed.len() as u16, false, &[string_info, var_info])
}
