//! GFS — the generic file system layer under OPEN/CLOSE/GET/PUT/PRINT #/INPUT #.
//!
//! QB64's `gfs_new` ... `gfs_unlock` and `gfs_open_com_syntax`, with the
//! table of open files owned here.
//!
//! Return codes are the legacy GFS codes:
//! -1 non-specific, -2 invalid handle, -3 bad file mode, -4 illegal function
//! call, -5 file not found, -6 path not found, -7 permission denied,
//! -8 device unavailable, -9 path/file access error, -10 read past EOF,
//! -11 bad file name, -13 disk full (-12 is `fileio`'s bad file mode on OPEN).
//!
//! Windows uses the same Win32 calls as legacy (`GFS_WINDOWS`). Other systems
//! follow the legacy `GFS_C` fstream behaviour with `std::fs` (not verified on
//! those systems yet).

use std::sync::Mutex;

/// One entry of the file table (`gfs_file_struct` without the fstream pointers).
#[repr(C)]
#[derive(Debug)]
pub struct GfsFile {
    pub id: i64,
    pub open: u8,
    pub read: u8,
    pub write: u8,
    pub lock_read: u8,
    pub lock_write: u8,
    /// Current byte position (base 0).
    pub pos: i64,
    pub eof_reached: u8,
    pub eof_passed: u8,
    pub fileno: i32,
    /// 1 RANDOM, 2 BINARY, 3 INPUT, 4 OUTPUT/APPEND.
    pub type_: u8,
    pub record_length: i64,
    /// Column for PRINT # tabbing (base 1 once opened).
    pub column: i64,
    pub com_port: u8,
    pub com_baud_rate: i32,
    pub com_parity: i8,
    pub com_data_bits_per_byte: i8,
    pub com_stop_bits: i8,
    pub com_bin_asc: i8,
    pub com_asc_lf: i8,
    pub com_rs: i8,
    pub com_cd_x: i32,
    pub com_cs_x: i32,
    pub com_ds_x: i32,
    pub com_op_x: i32,
    /// Opened as "SCRN:".
    pub scrn: u8,
    /// Opened FOR APPEND (`type_` is 4, as for OUTPUT).
    pub append: u8,
}

impl GfsFile {
    const fn zeroed() -> Self {
        GfsFile {
            id: 0,
            open: 0,
            read: 0,
            write: 0,
            lock_read: 0,
            lock_write: 0,
            pos: 0,
            eof_reached: 0,
            eof_passed: 0,
            fileno: 0,
            type_: 0,
            record_length: 0,
            column: 0,
            com_port: 0,
            com_baud_rate: 0,
            com_parity: 0,
            com_data_bits_per_byte: 0,
            com_stop_bits: 0,
            com_bin_asc: 0,
            com_asc_lf: 0,
            com_rs: 0,
            com_cd_x: 0,
            com_cs_x: 0,
            com_ds_x: 0,
            com_op_x: 0,
            scrn: 0,
            append: 0,
        }
    }
}

struct Slot {
    /// Boxed so its address stays the same while the table grows.
    file: Box<GfsFile>,
    os: os::Handle,
    /// A regular file opened for reading: INPUT # and LINE INPUT # read it
    /// through [`ReadAhead`] (a device, pipe or COM port is read byte by byte,
    /// a block read could wait for bytes that are not there yet).
    buffered: bool,
    ra: ReadAhead,
    /// The file itself (device and inode, or volume and file index) for a
    /// regular file: a write through one file number drops what other file
    /// numbers of the same file read ahead.
    ident: Option<(u64, u64)>,
    /// Linux and macOS: byte ranges (start, length; 0 = to the end of any file)
    /// this file number holds with LOCK, so that UNLOCK must name one of them
    /// exactly and LOCK may not overlap one, as on Windows.
    #[cfg_attr(windows, allow(dead_code))]
    locks: Vec<(u64, u64)>,
}

/// Bytes read ahead of the position for the byte-at-a-time readers (INPUT #,
/// LINE INPUT #, INPUT$ of an INPUT file): one system call per block instead
/// of one per byte (and one more per step back).
///
/// While `data` is not empty the OS file offset is at `start + data.len()` and
/// `GfsFile::pos` (the program's position) lies inside the block. Every other
/// operation puts the OS offset back first ([`Table::sync`]). The block is only
/// used while `epoch` equals [`Table::epoch`], which every OPEN, LOCK/UNLOCK and
/// [`invalidate_read_ahead`] change, and a write through any file number of
/// the same file drops it: bytes written through any file number (or by a
/// SHELL command) are never read from an older block.
#[derive(Default)]
struct ReadAhead {
    data: Vec<u8>,
    start: i64,
    epoch: u64,
    /// Size of the next block: it starts small (a program that opens a file to
    /// read one line reads little more than that line) and doubles while the
    /// program goes on reading.
    size: usize,
}

/// Smallest and largest [`ReadAhead`] block.
const READ_AHEAD_MIN: usize = 4 * 1024;
const READ_AHEAD_MAX: usize = 64 * 1024;

/// Highest usable file number; larger numbers are "Bad file name or number".
pub(crate) const MAX_FILENO: i32 = 32767;

pub(crate) struct Table {
    slots: Vec<Slot>,
    freed: Vec<i32>,
    nextid: i64,
    /// `gfs_fileno[f]` = table index or -1; index 0 unused.
    fileno: Vec<i32>,
    read_bytes: i64,
    /// Changes whenever file contents may have changed (see [`ReadAhead`]).
    epoch: u64,
}

// SAFETY: raw pointers inside point into data owned by the same table.
unsafe impl Send for Table {}

static TABLE: Mutex<Table> =
    Mutex::new(Table { slots: Vec::new(), freed: Vec::new(), nextid: 1, fileno: Vec::new(), read_bytes: 0, epoch: 0 });

/// Locks the file table. Not re-entrant: helpers below take `&mut Table`.
pub(crate) fn table() -> std::sync::MutexGuard<'static, Table> {
    TABLE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Files may have been changed outside the file table (SHELL, BSAVE, KILL,
/// NAME): bytes read ahead are read again from the file.
pub fn invalidate_read_ahead() {
    table().epoch += 1;
}

impl Table {
    fn fileno_n(&self) -> i32 {
        (self.fileno.len() as i32 - 1).max(0)
    }

    pub(crate) fn f(&mut self, i: i32) -> &mut GfsFile {
        &mut self.slots[i as usize].file
    }

    /// `gfs_new`.
    pub(crate) fn new_entry(&mut self) -> i32 {
        let i = if let Some(i) = self.freed.pop() {
            i
        } else {
            self.slots.push(Slot {
                file: Box::new(GfsFile::zeroed()),
                os: os::Handle::NONE,
                buffered: false,
                ra: ReadAhead::default(),
                ident: None,
                locks: Vec::new(),
            });
            self.slots.len() as i32 - 1
        };
        let id = self.nextid;
        self.nextid += 1;
        let s = &mut self.slots[i as usize];
        *s.file = GfsFile::zeroed();
        s.os = os::Handle::NONE;
        s.buffered = false;
        s.ra = ReadAhead::default();
        s.ident = None;
        s.locks.clear();
        s.file.id = id;
        i
    }

    /// Puts the OS file offset back at the program's position when a
    /// [`ReadAhead`] block moved it, and drops the block.
    fn sync(&mut self, i: i32) -> i32 {
        let s = &mut self.slots[i as usize];
        if s.ra.data.is_empty() {
            return 0;
        }
        s.ra.data.clear();
        let pos = s.file.pos;
        os::setpos(s, pos)
    }

    /// `gfs_validhandle`.
    pub(crate) fn valid(&self, i: i32) -> bool {
        if i < 0 || i as usize >= self.slots.len() {
            return false;
        }
        let f = &self.slots[i as usize].file;
        f.scrn != 0 || f.open != 0
    }

    /// `gfs_fileno_valid`: -2 invalid number, 1 in use, 0 unused.
    /// A query never grows the table (legacy grew it to `f`, so `EOF(2000000000)`
    /// allocated gigabytes); numbers above [`MAX_FILENO`] are invalid (error 52).
    pub(crate) fn fileno_valid(&mut self, f: i32) -> i32 {
        if f <= 0 || f > MAX_FILENO {
            return -2;
        }
        if f <= self.fileno_n() {
            return if self.fileno[f as usize] == -1 { 0 } else { 1 };
        }
        0
    }

    /// Table index of file number `f`, -1 when it is not in use.
    pub(crate) fn fileno_index(&self, f: i32) -> i32 {
        usize::try_from(f).ok().and_then(|f| self.fileno.get(f)).copied().unwrap_or(-1)
    }

    /// `gfs_fileno_freefile`: lowest unused file number.
    pub(crate) fn freefile(&self) -> i32 {
        let n = self.fileno_n();
        (1..=n).find(|&x| self.fileno[x as usize] == -1).unwrap_or(n + 1)
    }

    /// `gfs_fileno_use` (handles assumed valid). The table grows here, only
    /// for a number [`Table::fileno_valid`] accepts.
    pub(crate) fn fileno_use(&mut self, f: i32, i: i32) {
        if f <= 0 || f > MAX_FILENO {
            return;
        }
        if f as usize >= self.fileno.len() {
            self.fileno.resize(f as usize + 1, -1);
        }
        self.fileno[f as usize] = i;
        self.f(i).fileno = f;
    }

    /// Returns an entry from [`Table::new_entry`] that never became open to
    /// the free list (`free` only takes open entries, so a failed OPEN leaked
    /// its slot).
    fn release(&mut self, i: i32) {
        let s = &mut self.slots[i as usize];
        s.file.open = 0;
        s.os = os::Handle::NONE;
        self.freed.push(i);
    }

    /// `gfs_free`.
    fn free(&mut self, i: i32) -> i32 {
        if !self.valid(i) {
            return -2;
        }
        let f = self.f(i);
        f.open = 0;
        let fileno = f.fileno;
        if fileno != 0 {
            self.fileno[fileno as usize] = -1;
        }
        self.freed.push(i);
        0
    }

    /// `gfs_close`.
    pub(crate) fn close(&mut self, i: i32) -> i32 {
        let x = self.free(i);
        if x != 0 {
            return x;
        }
        let s = &mut self.slots[i as usize];
        s.ra = ReadAhead::default();
        s.buffered = false;
        s.ident = None;
        // (closing the file also releases its LOCKs, as on Windows)
        s.locks.clear();
        if s.file.scrn != 0 {
            return 0;
        }
        s.os.close();
        0
    }

    /// `gfs_lof`.
    pub(crate) fn lof(&mut self, i: i32) -> i64 {
        if !self.valid(i) {
            return -2;
        }
        let s = &mut self.slots[i as usize];
        if s.file.scrn != 0 {
            return -4;
        }
        os::lof(s)
    }

    /// `gfs_setpos`.
    pub(crate) fn setpos(&mut self, i: i32, position: i64) -> i32 {
        if !self.valid(i) {
            return -2;
        }
        if position < 0 {
            return -4;
        }
        {
            // inside the bytes read ahead (INPUT # steps back one byte): no system
            // call, and no LOF either: the position is within what the file held
            let epoch = self.epoch;
            let s = &mut self.slots[i as usize];
            let end = s.ra.start + s.ra.data.len() as i64;
            if !s.ra.data.is_empty() && s.ra.epoch == epoch && (s.ra.start..=end).contains(&position) {
                s.file.pos = position;
                s.file.eof_passed = 0;
                s.file.eof_reached = 0;
                return 0;
            }
            s.ra.data.clear();
        }
        let e = os::setpos(&mut self.slots[i as usize], position);
        if e != 0 {
            return e;
        }
        self.f(i).pos = position;
        if position <= self.lof(i) {
            let f = self.f(i);
            f.eof_passed = 0;
            f.eof_reached = 0;
        }
        0
    }

    /// `gfs_getpos`.
    pub(crate) fn getpos(&mut self, i: i32) -> i64 {
        if !self.valid(i) {
            return -2;
        }
        self.f(i).pos
    }

    /// `gfs_write`; `position` -1 = current position.
    pub(crate) fn write(&mut self, i: i32, position: i64, data: &[u8]) -> i32 {
        if !self.valid(i) {
            return -2;
        }
        if self.f(i).write == 0 {
            return -3;
        }
        let x = self.sync(i);
        if x != 0 {
            return x;
        }
        // another file number of this file must not go on with bytes read before
        if let Some(id) = self.slots[i as usize].ident {
            for j in 0..self.slots.len() as i32 {
                let s = &self.slots[j as usize];
                if j != i && !s.ra.data.is_empty() && s.ident == Some(id) {
                    self.sync(j);
                }
            }
        }
        if position != -1 {
            let x = self.setpos(i, position);
            if x != 0 {
                return x;
            }
        }
        os::write(&mut self.slots[i as usize], data)
    }

    /// `gfs_read`; on a short read the rest of `data` is zeroed and -10 returned.
    pub(crate) fn read(&mut self, i: i32, position: i64, data: &mut [u8]) -> i32 {
        self.read_bytes = 0;
        if !self.valid(i) {
            return -2;
        }
        if self.f(i).read == 0 {
            return -3;
        }
        let x = self.sync(i);
        if x != 0 {
            return x;
        }
        if position != -1 {
            // as setpos, without its LOF: the read sets or clears eof_passed itself
            // (GET with a position: one system call less per record)
            if position < 0 {
                return -4;
            }
            let s = &mut self.slots[i as usize];
            let x = os::setpos(s, position);
            if x != 0 {
                return x;
            }
            s.file.pos = position;
            s.file.eof_reached = 0;
        }
        let (e, n) = os::read(&mut self.slots[i as usize], data);
        self.read_bytes = n;
        e
    }

    /// The next byte at the current position: [`Table::read`] of one byte, with
    /// the same results (`Err(-10)` at the end of the file, which sets
    /// `eof_passed`), served from a [`ReadAhead`] block for a regular file.
    pub(crate) fn read_byte(&mut self, i: i32) -> Result<u8, i32> {
        let mut c = [0u8; 1];
        let plain = |t: &mut Table, c: &mut [u8; 1]| match t.read(i, -1, c) {
            0 => Ok(c[0]),
            e => Err(e),
        };
        if !self.valid(i) || !self.slots[i as usize].buffered || self.f(i).read == 0 {
            return plain(self, &mut c);
        }
        self.read_bytes = 0;
        let epoch = self.epoch;
        let s = &mut self.slots[i as usize];
        let pos = s.file.pos;
        let end = s.ra.start + s.ra.data.len() as i64;
        if s.ra.epoch != epoch || pos < s.ra.start || pos >= end {
            // a new block from the position on; the OS offset is at the end of the old one
            let moved = !s.ra.data.is_empty() && pos != end;
            s.ra.data.clear();
            if moved && os::setpos(s, pos) != 0 {
                return plain(self, &mut c);
            }
            // sequential reading: a larger block each time
            let size = if pos == end { (s.ra.size * 2).clamp(READ_AHEAD_MIN, READ_AHEAD_MAX) } else { READ_AHEAD_MIN };
            s.ra.size = size;
            let mut block = std::mem::take(&mut s.ra.data);
            block.resize(size, 0);
            let got = os::read_some(s, &mut block);
            s.ra.data = block;
            match got {
                Ok(0) => {
                    s.ra.data.clear();
                    s.file.eof_passed = 1;
                    return Err(-10);
                }
                Ok(n) => {
                    s.ra.data.truncate(n);
                    s.ra.start = pos;
                    s.ra.epoch = epoch;
                }
                Err(_) => {
                    // (a byte range LOCKed by another program on Windows): the single byte
                    // as before, with its own result
                    s.ra.data.clear();
                    return plain(self, &mut c);
                }
            }
        }
        let b = s.ra.data[(pos - s.ra.start) as usize];
        s.file.pos += 1;
        s.file.eof_passed = 0;
        self.read_bytes = 1;
        Ok(b)
    }

    /// Bytes transferred by the last [`Table::read`].
    pub(crate) fn read_bytes(&self) -> i64 {
        self.read_bytes
    }

    /// `gfs_eof_reached`.
    pub(crate) fn eof_reached(&mut self, i: i32) -> i32 {
        if !self.valid(i) {
            return -2;
        }
        // a byte read ahead is a byte the file has: no LOF (EOF of an INPUT file
        // is asked once per line)
        let epoch = self.epoch;
        let s = &self.slots[i as usize];
        if s.ra.epoch == epoch && s.file.pos >= s.ra.start && s.file.pos < s.ra.start + s.ra.data.len() as i64 {
            return 0;
        }
        i32::from(self.getpos(i) >= self.lof(i))
    }

    /// `gfs_eof_passed`.
    pub(crate) fn eof_passed(&mut self, i: i32) -> i32 {
        if !self.valid(i) {
            return -2;
        }
        i32::from(self.f(i).eof_passed != 0)
    }

    /// `gfs_lock` / `gfs_unlock`: inclusive byte range, -1 = from start / to infinity.
    pub(crate) fn lock(&mut self, i: i32, start: i64, end: i64, lock: bool) -> i32 {
        if !self.valid(i) {
            return -2;
        }
        let start = if start == -1 { 0 } else { start };
        if start < 0 || end < -1 {
            return -4;
        }
        // another program may change the bytes once they are unlocked
        self.epoch += 1;
        let x = self.sync(i);
        if x != 0 {
            return x;
        }
        os::lock(&mut self.slots[i as usize], start, end, lock)
    }

    /// `gfs_open`: returns the table index or a negative GFS code.
    /// `access` 1 read / 2 write / 3 both; `restrictions` 1 deny read,
    /// 2 deny write; `how` 0 open existing, 1 create, 2 create + truncate,
    /// 3 create with whatever access is available.
    pub(crate) fn open(&mut self, filename: &[u8], access: i32, restrictions: i32, how: i32) -> i32 {
        let mut filenamez = filename.to_vec();
        filenamez.push(0);
        // OUTPUT truncates a file another file number may be reading
        self.epoch += 1;
        let i = self.new_entry();
        // "SCRN:" (legacy reads the first five bytes even past the end)
        let at = |k: usize| filename.get(k).copied().unwrap_or(0);
        if at(0) & 0xDF == b'S' && at(1) & 0xDF == b'C' && at(2) & 0xDF == b'R' && at(3) & 0xDF == b'N' && at(4) == b':'
        {
            self.f(i).scrn = 1;
            return i;
        }
        {
            let f = self.f(i);
            if access & 1 != 0 {
                f.read = 1;
            }
            if access & 2 != 0 {
                f.write = 1;
            }
            if restrictions & 1 != 0 {
                f.lock_read = 1;
            }
            if restrictions & 2 != 0 {
                f.lock_write = 1;
            }
            f.pos = 0;
            if open_com_syntax(&filenamez, f) == -1 {
                self.release(i);
                return -11;
            }
        }
        fixdir(&mut filenamez);
        let r = os::open(&mut self.slots[i as usize], &filenamez, access, restrictions, how);
        if r != 0 {
            self.release(i);
            return r;
        }
        let s = &mut self.slots[i as usize];
        s.ident = os::regular(s);
        s.buffered = s.ident.is_some() && s.file.read != 0;
        s.file.open = 1;
        i
    }


    /// Bytes waiting in a COM port's input buffer (LOC of a COM file).
    pub(crate) fn com_in_queue(&mut self, i: i32) -> i64 {
        os::com_in_queue(&self.slots[i as usize])
    }

    /// Highest file number seen so far (`gfs_fileno_n`).
    pub(crate) fn max_fileno(&self) -> i32 {
        self.fileno_n()
    }
}

/// The bytes of a program's file name as a path of this system. Linux and macOS take
/// the bytes as they are: read as UTF-8 text, every byte that is not UTF-8 became
/// U+FFFD, and two different names (CHR$(200), CHR$(201)) named one file.
pub fn os_path(name: &[u8]) -> std::path::PathBuf {
    // as OPEN reads it: on Windows a name that is not UTF-8 is in the ANSI code page
    // (KILL and _FILEEXISTS made it U+FFFD, so they did not find the file OPEN made)
    std::path::PathBuf::from(crate::osstr::os_string(name))
}

/// A path a program gave (`\` or `/`, any OS) as the system's path: what OPEN does
/// with a name, for the statements that load a file another way (_LOADIMAGE, BLOAD,
/// _SNDOPEN, ...), which skipped the separator change and failed with
/// "assets\hero.png" on Linux and macOS.
pub fn user_path(name: &[u8]) -> std::path::PathBuf {
    let mut v = name.to_vec();
    fixdir(&mut v);
    os_path(&v)
}

/// `fixdir`: converts path separators to the OS form.
pub fn fixdir(name: &mut [u8]) {
    for c in name.iter_mut() {
        if cfg!(windows) {
            if *c == b'/' {
                *c = b'\\';
            }
        } else if *c == b'\\' {
            *c = b'/';
        }
    }
}

/// `gfs_open_com_syntax`: parses "COMn:options" (`fstr` includes the NUL
/// terminator). Returns 0 not a COM name, -1 syntax error, else fills the
/// `com_*` fields (legacy returned an undefined value here, treated as "COM").
/// Legacy kept `x` in a function-static that is read before it is written, so
/// after one COM name every later one failed; each call starts at 0.
pub fn open_com_syntax(fstr: &[u8], f: &mut GfsFile) -> i32 {
    let mut x = 0;
    com_syntax(fstr, f, &mut x)
}

fn com_syntax(fstr: &[u8], f: &mut GfsFile, x: &mut i32) -> i32 {
    f.com_port = 0;
    let len = fstr.len() as i32;
    if len <= 3 {
        return 0;
    }
    if fstr[0] & 223 != 67 || fstr[1] & 223 != 79 || fstr[2] & 223 != 77 {
        return 0;
    }
    // legacy `v` is an int32: large numbers wrap instead of failing
    let mut v: i32 = -1;
    let mut i: i32 = 3;
    let mut found = false;
    while i < len - 1 {
        let c = fstr[i as usize];
        if c == 58 {
            found = true;
            break;
        }
        if !c.is_ascii_digit() {
            return 0;
        }
        if v == -1 {
            if c == 48 {
                return 0;
            }
            v = 0;
        }
        v = v.wrapping_mul(10).wrapping_add(i32::from(c - 48));
        i += 1;
    }
    if !found {
        return 0;
    }
    if *x >= 7 || v <= 0 || v > 255 {
        return -1;
    }
    f.com_port = v as u8;

    // upper-cased copy without the NUL terminator
    let s: Vec<u8> = fstr[..fstr.len() - 1].iter().map(|c| c.to_ascii_uppercase()).collect();
    let slen = s.len() as i32;
    f.com_baud_rate = -1;
    f.com_parity = -1;
    f.com_data_bits_per_byte = -1;
    f.com_stop_bits = -1;
    f.com_rs = -1;
    f.com_bin_asc = -1;
    f.com_asc_lf = -1;
    f.com_cd_x = -1;
    f.com_cs_x = -1;
    f.com_ds_x = -1;
    f.com_op_x = -1;

    let (mut com_rb_used, mut com_tb_used) = (false, false);
    let mut stage = 1;
    let mut str_or_num = 1;
    let mut strv: i64 = 0;
    v = -1;
    i += 1;
    while i < slen {
        let c = i64::from(s[i as usize]);
        if c != 44 {
            if c < 48 || (c > 57 && c < 65) || c > 90 {
                return -1;
            }
            if str_or_num == 2 && c >= 65 {
                return -1;
            }
            if c < 65 {
                str_or_num = 2;
            }
            if str_or_num == 1 || stage == 4 {
                if strv & 0xFF0000 != 0 {
                    strv |= c << 24;
                } else if strv & 0x00FF00 != 0 {
                    strv |= c << 16;
                } else if strv & 0x0000FF != 0 {
                    strv |= c << 8;
                } else {
                    strv = c;
                }
                if strv > 16_777_216 {
                    return -1;
                }
            } else {
                if c > 48 && c <= 57 {
                    if v == -2 {
                        return -1;
                    }
                    if v == -1 {
                        v = 0;
                    }
                    v = v.wrapping_mul(10).wrapping_add((c - 48) as i32);
                } else {
                    if v == -2 {
                        return -1;
                    }
                    if v == -1 {
                        v = -2;
                    }
                    if v > 0 {
                        v = v.wrapping_mul(10);
                    }
                }
            }
        }
        if c == 44 || i == slen - 1 {
            if v == -2 {
                v = 0;
            }
            let done = 'stage: {
                if stage == 1 {
                    if f.com_baud_rate != -1 || strv != 0 || v == 0 {
                        return -1;
                    }
                    if v == -1 {
                        v = 300;
                    }
                    f.com_baud_rate = v;
                    stage += 1;
                    break 'stage true;
                }
                if stage == 2 {
                    if f.com_parity != -1 || v != -1 {
                        return -1;
                    }
                    *x = match strv {
                        78 => 0,
                        0 | 69 => 1,
                        79 => 2,
                        83 => 3,
                        77 => 4,
                        17744 => 5,
                        _ => -1,
                    };
                    if *x == -1 {
                        return -1;
                    }
                    f.com_parity = *x as i8;
                    stage += 1;
                    break 'stage true;
                }
                if stage == 3 {
                    if f.com_data_bits_per_byte != -1 || strv != 0 {
                        return -1;
                    }
                    *x = match v {
                        -1 => 7,
                        5..=8 => v,
                        _ => -1,
                    };
                    if *x == -1 {
                        return -1;
                    }
                    f.com_data_bits_per_byte = *x as i8;
                    stage += 1;
                    break 'stage true;
                }
                if stage == 4 {
                    if f.com_stop_bits != -1 || v != -1 {
                        return -1;
                    }
                    *x = -1;
                    if strv == 0 {
                        *x = 10;
                        if f.com_baud_rate <= 110 {
                            *x = 20;
                            if f.com_data_bits_per_byte == 5 {
                                *x = 15;
                            }
                        }
                    }
                    match strv {
                        49 => *x = 10,
                        3_485_233 => *x = 15,
                        50 => *x = 20,
                        _ => {}
                    }
                    if *x == -1 {
                        return -1;
                    }
                    f.com_stop_bits = *x as i8;
                    stage += 1;
                    break 'stage true;
                }
                if strv == 0 {
                    return -1;
                }
                match strv {
                    21330 => {
                        if f.com_rs != -1 {
                            return -1;
                        }
                        f.com_rs = 1;
                    }
                    5_130_562 => {
                        if f.com_bin_asc != -1 {
                            return -1;
                        }
                        f.com_bin_asc = 0;
                    }
                    4_412_225 => {
                        if f.com_bin_asc != -1 {
                            return -1;
                        }
                        f.com_bin_asc = 1;
                    }
                    16980 => {
                        if com_tb_used {
                            return -1;
                        }
                        com_tb_used = true;
                    }
                    16978 => {
                        if com_rb_used {
                            return -1;
                        }
                        com_rb_used = true;
                    }
                    17996 => {
                        if f.com_asc_lf != -1 {
                            return -1;
                        }
                        f.com_asc_lf = 1;
                    }
                    17475 | 21315 | 21316 | 20559 => {
                        let (slot, default) = match strv {
                            17475 => (&mut f.com_cd_x, 0),
                            21315 => (&mut f.com_cs_x, 1000),
                            21316 => (&mut f.com_ds_x, 1000),
                            _ => (&mut f.com_op_x, 10000),
                        };
                        if *slot != -1 {
                            return -1;
                        }
                        if v == -1 {
                            v = default;
                        }
                        if v > 65535 {
                            return -1;
                        }
                        *slot = v;
                    }
                    _ => return -1,
                }
                true
            };
            if done {
                str_or_num = 1;
                strv = 0;
                v = -1;
            }
        }
        i += 1;
    }

    if f.com_baud_rate == -1 {
        f.com_baud_rate = 300;
    }
    if f.com_parity == -1 {
        f.com_parity = 1;
    }
    if f.com_data_bits_per_byte == -1 {
        f.com_data_bits_per_byte = 7;
    }
    if f.com_stop_bits == -1 {
        *x = 10;
        if f.com_baud_rate <= 110 {
            *x = 20;
            if f.com_data_bits_per_byte == 5 {
                *x = 15;
            }
        }
        f.com_stop_bits = *x as i8;
    }
    if f.com_bin_asc == -1 {
        f.com_bin_asc = 0;
    }
    if f.com_asc_lf == -1 {
        f.com_asc_lf = 0;
    }
    if f.com_asc_lf == 1 && f.com_bin_asc == 0 {
        f.com_asc_lf = 0;
    }
    if f.com_rs == -1 {
        f.com_rs = 0;
    }
    if f.com_cd_x == -1 {
        f.com_cd_x = 0;
    }
    if f.com_cs_x == -1 {
        f.com_cs_x = 1000;
    }
    if f.com_ds_x == -1 {
        f.com_ds_x = 1000;
    }
    if f.com_op_x == -1 {
        *x = f.com_cd_x * 10;
        let z = f.com_ds_x * 10;
        if z > *x {
            *x = z;
        }
        if *x > 65535 {
            *x = 65535;
        }
        f.com_op_x = *x;
    }
    1
}

#[cfg(windows)]
mod os {
    use super::{GfsFile, Slot};
    use crate::win32::*;
    use std::ptr::null_mut;

    #[derive(Clone, Copy)]
    pub struct Handle(pub HANDLE);
    impl Handle {
        pub const NONE: Handle = Handle(null_mut());
        pub fn close(&mut self) {
            // SAFETY: handle came from CreateFileA or CreateFileW.
            unsafe { CloseHandle(self.0) };
        }
    }

    fn map_open_error(e: DWORD) -> i32 {
        match e {
            3 => -6,
            4 | 29 | 30 => -9,
            5 | 19 | 33 | 32 => -7,
            15 | 21 => -8,
            _ => -5,
        }
    }

    pub fn open(s: &mut Slot, namez: &[u8], access: i32, restrictions: i32, mut how: i32) -> i32 {
        let f: &mut GfsFile = &mut s.file;
        let mut x: DWORD = 0;
        if access & 1 != 0 {
            x |= GENERIC_READ;
        }
        if access & 2 != 0 {
            x |= GENERIC_WRITE;
        }
        let mut x2 = FILE_SHARE_READ | FILE_SHARE_WRITE;
        if restrictions & 1 != 0 {
            x2 ^= FILE_SHARE_READ;
        }
        if restrictions & 2 != 0 {
            x2 ^= FILE_SHARE_WRITE;
        }
        // SAFETY: Win32 calls with valid NUL-terminated names and owned structs.
        unsafe {
            if f.com_port != 0 {
                // \\.\COMn: the DOS names COM1-COM9 are the only ones Win32 knows
                let portname = format!("\\\\.\\COM{}\0", f.com_port);
                let h = CreateFileA(portname.as_ptr(), x, 0, null_mut(), OPEN_EXISTING, 0, null_mut());
                if h == INVALID_HANDLE_VALUE {
                    return -8;
                }
                s.os = Handle(h);
                let mut cs = DCB { DCBlength: std::mem::size_of::<DCB>() as u32, ..DCB::default() };
                if GetCommState(h, &mut cs) == 0 {
                    CloseHandle(h);
                    return -8;
                }
                cs.BaudRate = f.com_baud_rate as u32;
                let mut stop: u8 = 0; // legacy x2 kept its share-mode value when no case matched
                stop = match f.com_stop_bits {
                    10 => 0,
                    15 => 1,
                    20 => 2,
                    _ => stop,
                };
                cs.StopBits = stop;
                cs.ByteSize = f.com_data_bits_per_byte as u8;
                let parity: u8 = match f.com_parity {
                    0 => 0,
                    1 => 2,
                    2 => 1,
                    3 => 4,
                    4 => 3,
                    _ => stop,
                };
                cs.Parity = parity;
                if parity == 0 {
                    cs.bits &= !2;
                } else {
                    cs.bits |= 2;
                }
                if f.com_rs != 0 {
                    cs.bits &= !(3 << 12); // RTS_CONTROL_DISABLE
                }
                if f.com_bin_asc == 0 {
                    cs.bits |= 1;
                } else {
                    cs.bits &= !1;
                }
                cs.EofChar = 26;
                if SetCommState(h, &mut cs) == 0 {
                    CloseHandle(h);
                    return -8;
                }
                let mut ct = COMMTIMEOUTS::default();
                if f.com_ds_x == 0 {
                    ct.ReadIntervalTimeout = MAXDWORD;
                } else {
                    ct.ReadTotalTimeoutConstant = f.com_ds_x as u32;
                }
                ct.WriteTotalTimeoutConstant = f.com_cs_x as u32;
                if SetCommTimeouts(h, &mut ct) == 0 {
                    CloseHandle(h);
                    return -8;
                }
                return 0;
            }

            // a UTF-8 name (ไทย.bas) through CreateFileW, as _FILEEXISTS and KILL read it
            let wide = utf16z(namez);
            let create = |access: DWORD, share: DWORD, disposition: DWORD| match &wide {
                Some(w) => CreateFileW(w.as_ptr(), access, share, null_mut(), disposition, FILE_ATTRIBUTE_NORMAL, null_mut()),
                None => CreateFileA(namez.as_ptr(), access, share, null_mut(), disposition, FILE_ATTRIBUTE_NORMAL, null_mut()),
            };
            let x3 = if how != 0 { OPEN_ALWAYS } else { OPEN_EXISTING };
            let mut h;
            loop {
                h = create(x, x2, x3);
                if h != INVALID_HANDLE_VALUE {
                    break;
                }
                if how == 3 {
                    x = GENERIC_READ;
                    f.read = 1;
                    f.write = 0;
                    how += 1;
                    continue;
                }
                if how == 4 {
                    x = GENERIC_WRITE;
                    f.read = 0;
                    f.write = 1;
                    how += 1;
                    continue;
                }
                return map_open_error(GetLastError());
            }
            if how == 2 {
                let mut hi: DWORD = 0;
                let lo = GetFileSize(h, &mut hi);
                if lo != 0 || hi != 0 {
                    CloseHandle(h);
                    h = create(x, x2, TRUNCATE_EXISTING);
                    if h == INVALID_HANDLE_VALUE {
                        return map_open_error(GetLastError());
                    }
                }
            }
            s.os = Handle(h);
        }
        0
    }

    pub fn lof(s: &mut Slot) -> i64 {
        let mut hi: DWORD = 0;
        // SAFETY: open handle.
        unsafe {
            let lo = GetFileSize(s.os.0, &mut hi);
            if lo == 0xFFFF_FFFF && GetLastError() != NO_ERROR {
                return -3;
            }
            (i64::from(hi) << 32) | i64::from(lo)
        }
    }

    pub fn setpos(s: &mut Slot, position: i64) -> i32 {
        let mut hi = (position >> 32) as i32;
        // SAFETY: open handle.
        unsafe {
            if SetFilePointer(s.os.0, position as i32, &mut hi, FILE_BEGIN) == 0xFFFF_FFFF
                && GetLastError() != NO_ERROR
            {
                return -3;
            }
        }
        0
    }

    pub fn write(s: &mut Slot, mut data: &[u8]) -> i32 {
        while !data.is_empty() {
            let n = data.len().min(0xFFFF_FFFF) as DWORD;
            let mut written: DWORD = 0;
            // SAFETY: open handle, buffer valid for n bytes.
            unsafe {
                if WriteFile(s.os.0, data.as_ptr(), n, &mut written, null_mut()) == 0 {
                    let e = GetLastError();
                    return if e == 5 || e == 33 { -7 } else { -9 };
                }
            }
            data = &data[written as usize..];
            s.file.pos += i64::from(written);
            if written != n {
                return -1;
            }
        }
        0
    }

    pub fn read(s: &mut Slot, data: &mut [u8]) -> (i32, i64) {
        let mut done = 0usize;
        while done < data.len() {
            let n = (data.len() - done).min(0xFFFF_FFFF) as DWORD;
            let mut got: DWORD = 0;
            // SAFETY: open handle, buffer valid for n bytes.
            let ok = unsafe { ReadFile(s.os.0, data[done..].as_mut_ptr(), n, &mut got, null_mut()) };
            if ok == 0 {
                // SAFETY: plain call.
                let e = unsafe { GetLastError() };
                return (if e == 5 || e == 33 { -7 } else { -9 }, done as i64);
            }
            done += got as usize;
            s.file.pos += i64::from(got);
            if got != n {
                data[done..].fill(0);
                s.file.eof_passed = 1;
                return (-10, done as i64);
            }
        }
        s.file.eof_passed = 0;
        (0, done as i64)
    }

    pub fn lock(s: &mut Slot, start: i64, end: i64, lock: bool) -> i32 {
        let mut bytes = end as u64;
        if bytes != u64::MAX {
            bytes = bytes.wrapping_sub(start as u64).wrapping_add(1);
        }
        let (slo, shi) = (start as u64 as DWORD, ((start as u64) >> 32) as DWORD);
        let (blo, bhi) = (bytes as DWORD, (bytes >> 32) as DWORD);
        // SAFETY: open handle.
        unsafe {
            let ok =
                if lock { LockFile(s.os.0, slo, shi, blo, bhi) } else { UnlockFile(s.os.0, slo, shi, blo, bhi) };
            if ok == 0 {
                let e = GetLastError();
                if e == 5 || e == 33 || (!lock && e == 158) {
                    return -7;
                }
                return -9;
            }
        }
        0
    }

    /// Up to `buf.len()` bytes at the file offset; `Ok(0)` at the end of the file.
    pub fn read_some(s: &mut Slot, buf: &mut [u8]) -> Result<usize, ()> {
        let n = buf.len().min(0xFFFF_FFFF) as DWORD;
        let mut got: DWORD = 0;
        // SAFETY: open handle, buffer valid for n bytes.
        if unsafe { ReadFile(s.os.0, buf.as_mut_ptr(), n, &mut got, null_mut()) } == 0 {
            return Err(());
        }
        Ok(got as usize)
    }

    /// Volume serial number and file index of a disk file; `None` for a
    /// device, pipe or COM port.
    pub fn regular(s: &Slot) -> Option<(u64, u64)> {
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, FILE_TYPE_DISK, GetFileInformationByHandle, GetFileType,
        };
        // SAFETY: open handle, owned struct.
        unsafe {
            if GetFileType(s.os.0) != FILE_TYPE_DISK {
                return None;
            }
            let mut info: BY_HANDLE_FILE_INFORMATION = std::mem::zeroed();
            if GetFileInformationByHandle(s.os.0, &mut info) == 0 {
                return None;
            }
            Some((
                u64::from(info.dwVolumeSerialNumber),
                (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
            ))
        }
    }

    pub fn com_in_queue(s: &Slot) -> i64 {
        let mut st = COMSTAT::default();
        let mut ignore: DWORD = 0;
        // SAFETY: open COM handle.
        if unsafe { ClearCommError(s.os.0, &mut ignore, &mut st) } == 0 {
            return 0;
        }
        i64::from(st.cbInQue)
    }
}

#[cfg(not(windows))]
mod os {
    //! Legacy GFS_C (fstream) behaviour on top of std::fs. Not verified yet.
    use super::Slot;
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Seek, SeekFrom, Write};

    pub struct Handle(pub Option<File>);
    impl Handle {
        pub const NONE: Handle = Handle(None);
        pub fn close(&mut self) {
            self.0 = None;
        }
    }

    /// Opens like `CreateFileA` with `OPEN_ALWAYS` (`how` != 0) or
    /// `OPEN_EXISTING`: a missing file is created, an existing one is never
    /// truncated except by `how` 2. Writes go to the current position (no
    /// O_APPEND: APPEND mode seeks to LOF itself, PUT/SEEK need real positions).
    /// The file, or whether the open was refused for lack of permission (as opposed
    /// to a missing file or path).
    fn try_open(name: &std::path::Path, access: i32, how: i32) -> Result<File, bool> {
        let mut o = OpenOptions::new();
        match access {
            1 => o.read(true),
            2 => o.write(true),
            _ => o.read(true).write(true),
        };
        let writes = access & 2 != 0;
        o.create(how != 0 && writes).truncate(how == 2 && writes);
        let denied = |e: std::io::Error| e.kind() == std::io::ErrorKind::PermissionDenied;
        match o.open(name) {
            Ok(f) => Ok(f),
            Err(e) if how != 0 && !writes && e.kind() == std::io::ErrorKind::NotFound => {
                // read-only OPEN_ALWAYS: create the missing file, leave an existing one alone
                let _ = OpenOptions::new().write(true).create_new(true).open(name);
                OpenOptions::new().read(true).open(name).map_err(denied)
            }
            Err(e) => Err(denied(e)),
        }
    }

    pub fn open(s: &mut Slot, namez: &[u8], access: i32, restrictions: i32, how: i32) -> i32 {
        // a serial port is opened by the Windows layer only: "Device unavailable",
        // not a file called COM1:9600,N,8,1 in the current folder
        if s.file.com_port != 0 {
            return -8;
        }
        let name = super::os_path(&namez[..namez.len() - 1]);
        let mut h = try_open(&name, access, how);
        let mut denied = matches!(h, Err(true));
        if how == 3 && h.is_err() {
            // like GFS_WINDOWS: read+write, then read-only, then write-only
            s.file.read = 1;
            s.file.write = 0;
            h = try_open(&name, 1, how);
            denied |= matches!(h, Err(true));
            if h.is_err() {
                s.file.read = 0;
                s.file.write = 1;
                h = try_open(&name, 2, how);
                denied |= matches!(h, Err(true));
            }
        }
        match h {
            Ok(f) => {
                use std::os::unix::fs::MetadataExt;
                // one fstat for both: the file's identity, and whether it is a regular file
                s.ident = f.metadata().ok().filter(std::fs::Metadata::is_file).map(|m| (m.dev(), m.ino()));
                if s.ident.is_some() && !share(&f, restrictions) {
                    s.ident = None;
                    return -7;
                }
                s.os = Handle(Some(f));
                0
            }
            // the file is there (or its folder is) but may not be opened this way:
            // "Permission denied", not "File not found" / "Bad file name"
            Err(_) if denied => -7,
            Err(_) if how != 0 => -11,
            Err(_) => -5,
        }
    }

    pub fn lof(s: &mut Slot) -> i64 {
        match s.os.0.as_ref().and_then(|f| f.metadata().ok()) {
            Some(m) if s.file.read != 0 || s.file.write != 0 => m.len() as i64,
            _ => -1,
        }
    }

    pub fn setpos(s: &mut Slot, position: i64) -> i32 {
        if let Some(f) = s.os.0.as_mut()
            && f.seek(SeekFrom::Start(position as u64)).is_err()
        {
            return -3; // as GFS_WINDOWS
        }
        0
    }

    pub fn write(s: &mut Slot, data: &[u8]) -> i32 {
        let Some(f) = s.os.0.as_mut() else { return -7 };
        // (write_all goes on after an interrupted call)
        if let Err(e) = f.write_all(data) {
            // what did get written moved the file position: read it back, so that
            // the next statement does not work from a position the file is not at
            if let Ok(p) = f.stream_position() {
                s.file.pos = p as i64;
            }
            // a full disk is "Disk full" (61), not "Permission denied"
            return if matches!(e.kind(), std::io::ErrorKind::StorageFull | std::io::ErrorKind::QuotaExceeded) { -13 } else { -7 };
        }
        s.file.pos += data.len() as i64;
        0
    }

    pub fn read(s: &mut Slot, data: &mut [u8]) -> (i32, i64) {
        let Some(f) = s.os.0.as_mut() else { return (-7, 0) };
        let mut done = 0;
        while done < data.len() {
            match f.read(&mut data[done..]) {
                Ok(0) => break,
                Ok(n) => done += n,
                // a signal arrived during the call: nothing was read, ask again
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => return (-7, done as i64),
            }
        }
        s.file.pos += done as i64;
        if done < data.len() {
            data[done..].fill(0);
            s.file.eof_passed = 1;
            return (-10, done as i64);
        }
        s.file.eof_passed = 0;
        (0, done as i64)
    }

    /// Up to `buf.len()` bytes at the file offset; `Ok(0)` at the end of the file.
    pub fn read_some(s: &mut Slot, buf: &mut [u8]) -> Result<usize, ()> {
        let Some(f) = s.os.0.as_mut() else { return Err(()) };
        loop {
            match f.read(buf) {
                Ok(n) => return Ok(n),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => return Err(()),
            }
        }
    }

    /// Device and inode of a regular file; `None` for a device, FIFO or socket
    /// (found by [`open`]).
    pub fn regular(s: &Slot) -> Option<(u64, u64)> {
        s.ident
    }

    /// `OPEN ... LOCK READ`, `LOCK WRITE` or `LOCK READ WRITE` (Windows share
    /// modes) on Linux: a whole-file `flock`, exclusive for a file opened with
    /// any LOCK clause and shared otherwise, so that one program's LOCK clause
    /// refuses the other programs' OPEN of the file and the other way round.
    /// Stricter than Windows: LOCK READ and LOCK WRITE refuse every other OPEN.
    /// False when another file number or program holds a lock that refuses
    /// this OPEN; a file system without `flock` opens as before. Regular files only.
    #[cfg(target_os = "linux")]
    fn share(f: &File, restrictions: i32) -> bool {
        use std::os::fd::AsRawFd;
        let op = if restrictions != 0 { libc::LOCK_EX } else { libc::LOCK_SH };
        // SAFETY: plain call on an open descriptor.
        if unsafe { libc::flock(f.as_raw_fd(), op | libc::LOCK_NB) } == 0 {
            return true;
        }
        std::io::Error::last_os_error().raw_os_error() != Some(libc::EWOULDBLOCK)
    }

    /// macOS and other systems: whole-file `flock` locks there may conflict
    /// with the program's own LOCK byte ranges (one lock list for both), so the
    /// LOCK clause of OPEN is not enforced.
    #[cfg(not(target_os = "linux"))]
    fn share(_f: &File, _restrictions: i32) -> bool {
        true
    }

    /// Linux: open file description locks (released only when this file number
    /// is closed, and seen by the program's other file numbers, as Windows
    /// LockFile); a kernel without them uses process locks.
    #[cfg(target_os = "linux")]
    fn setlk(fd: i32, fl: &mut libc::flock) -> i32 {
        use std::sync::atomic::{AtomicBool, Ordering};
        static NO_OFD: AtomicBool = AtomicBool::new(false);
        if !NO_OFD.load(Ordering::Relaxed) {
            // SAFETY: plain call, `fl` valid (l_pid 0 as F_OFD_SETLK requires).
            if unsafe { libc::fcntl(fd, libc::F_OFD_SETLK, fl as *mut libc::flock) } == 0 {
                return 0;
            }
            if std::io::Error::last_os_error().raw_os_error() != Some(libc::EINVAL) {
                return -1;
            }
            NO_OFD.store(true, Ordering::Relaxed);
        }
        // SAFETY: as above.
        unsafe { libc::fcntl(fd, libc::F_SETLK, fl as *mut libc::flock) }
    }

    /// Other systems: process locks (closing another file number of the same
    /// file in the program releases them; documented).
    #[cfg(not(target_os = "linux"))]
    fn setlk(fd: i32, fl: &mut libc::flock) -> i32 {
        // SAFETY: plain call, `fl` valid.
        unsafe { libc::fcntl(fd, libc::F_SETLK, fl as *mut libc::flock) }
    }

    /// LOCK / UNLOCK of the inclusive range `start..=end` (`end` -1 = to the end
    /// of any file length) with `fcntl` record locks, which other programs that
    /// lock the file see: a range another program holds is "Permission denied",
    /// as on Windows. A file opened for writing takes an exclusive lock, a file
    /// opened only for reading (INPUT, ACCESS READ) a shared one (`fcntl` only
    /// allows that much). A file system without record locks behaves as before
    /// (every LOCK succeeds).
    pub fn lock(s: &mut Slot, start: i64, end: i64, lock: bool) -> i32 {
        use std::os::fd::AsRawFd;
        if end != -1 && end < start {
            return -4;
        }
        let Some(f) = s.os.0.as_ref() else { return -9 };
        let start_u = start as u64;
        // fcntl length 0 = to the end of any file length
        let len: u64 = if end == -1 || (end - start) as u64 >= i64::MAX as u64 { 0 } else { (end - start) as u64 + 1 };
        let last = |a: u64, n: u64| if n == 0 { u64::MAX } else { a + (n - 1) };
        if lock {
            // LockFile refuses a range that overlaps one this file number holds
            // (fcntl would merge the two without a word)
            if s.locks.iter().any(|&(a, n)| a <= last(start_u, len) && start_u <= last(a, n)) {
                return -7;
            }
        } else if !s.locks.contains(&(start_u, len)) {
            // UnlockFile takes only a range that was locked, exactly
            return -7;
        }
        // SAFETY: an all-zero flock is a valid value.
        let mut fl: libc::flock = unsafe { std::mem::zeroed() };
        fl.l_type = (if !lock {
            libc::F_UNLCK
        } else if s.file.write != 0 {
            libc::F_WRLCK
        } else {
            libc::F_RDLCK
        }) as _;
        fl.l_whence = libc::SEEK_SET as _;
        fl.l_start = start as libc::off_t;
        fl.l_len = len as libc::off_t;
        if setlk(f.as_raw_fd(), &mut fl) != 0 {
            match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::EACCES | libc::EAGAIN) => return -7,
                // no record locks on this file system: as before
                Some(libc::ENOLCK | libc::EOPNOTSUPP | libc::ENOSYS) => {}
                _ => return -9,
            }
        }
        if lock {
            s.locks.push((start_u, len));
        } else {
            s.locks.retain(|&r| r != (start_u, len));
        }
        0
    }

    pub fn com_in_queue(_s: &Slot) -> i64 {
        0
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Fresh temp dir per test (the table itself is shared between tests).
    pub(crate) fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("qb_test_{}_{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    pub(crate) fn p(d: &std::path::Path, f: &str) -> Vec<u8> {
        d.join(f).to_string_lossy().into_owned().into_bytes()
    }

    #[test]
    fn open_missing_file_for_input_is_not_found() {
        let d = tmp("missing");
        let mut t = table();
        assert_eq!(t.open(&p(&d, "nope.txt"), 1, 0, 0), -5);
        // GFS_WINDOWS maps ERROR_PATH_NOT_FOUND to -6; legacy GFS_C (fstream)
        // only knows "file not found" (-5) and, when creating, "bad file name" (-11).
        let (missing_dir, missing_dir_create) = if cfg!(windows) { (-6, -6) } else { (-5, -11) };
        assert_eq!(t.open(&p(&d, "nodir/x.txt"), 1, 0, 0), missing_dir);
        assert_eq!(t.open(&p(&d, "nodir/x.txt"), 3, 0, 3), missing_dir_create);
    }

    #[test]
    fn opens_names_that_are_not_ascii() {
        // UTF-8 names, as _FILEEXISTS and KILL read them: on Windows OPEN used the
        // ANSI code page and did not find ไทย.bas, nor a file in a Thai folder
        let d = tmp("utf8").join("โฟลเดอร์");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("ไทย.bas"), b"PRINT 1").unwrap();
        let mut t = table();
        let i = t.open(&p(&d, "ไทย.bas"), 1, 0, 0);
        assert!(i >= 0, "{i}");
        let mut buf = [0u8; 7];
        assert_eq!(t.read(i, -1, &mut buf), 0);
        assert_eq!(&buf, b"PRINT 1");
        assert_eq!(t.close(i), 0);
        let i = t.open(&p(&d, "ใหม่.txt"), 3, 0, 3);
        assert!(i >= 0, "{i}");
        assert_eq!(t.close(i), 0);
        assert!(d.join("ใหม่.txt").exists());
    }

    #[test]
    fn write_read_lof_setpos_and_eof() {
        let d = tmp("rw");
        let mut t = table();
        let i = t.open(&p(&d, "a.bin"), 3, 0, 3);
        assert!(i >= 0);
        assert_eq!(t.write(i, -1, b"hello world"), 0);
        assert_eq!(t.getpos(i), 11);
        assert_eq!(t.lof(i), 11);
        let mut buf = [0xAAu8; 5];
        assert_eq!(t.read(i, 6, &mut buf), 0);
        assert_eq!(&buf, b"world");
        assert_eq!(t.eof_reached(i), 1);
        assert_eq!(t.eof_passed(i), 0);
        let mut buf = [0xAAu8; 4];
        assert_eq!(t.read(i, 9, &mut buf), -10);
        assert_eq!(&buf, b"ld\0\0");
        assert_eq!(t.read_bytes(), 2);
        assert_eq!(t.eof_passed(i), 1);
        assert_eq!(t.setpos(i, 3), 0);
        assert_eq!(t.eof_passed(i), 0);
        assert_eq!(t.setpos(i, -1), -4);
        assert_eq!(t.setpos(i, 100), 0);
        assert_eq!(t.getpos(i), 100);
        assert_eq!(t.close(i), 0);
        assert_eq!(t.close(i), -2);
        assert_eq!(t.lof(i), -2);
    }

    #[test]
    fn modes_and_truncate() {
        let d = tmp("modes");
        let mut t = table();
        let i = t.open(&p(&d, "m.txt"), 2, 0, 2);
        assert_eq!(t.write(i, -1, b"0123456789"), 0);
        let mut b = [0u8; 1];
        assert_eq!(t.read(i, -1, &mut b), -3);
        t.close(i);
        let i = t.open(&p(&d, "m.txt"), 1, 0, 0);
        assert_eq!(t.write(i, -1, b"x"), -3);
        assert_eq!(t.lof(i), 10);
        t.close(i);
        let i = t.open(&p(&d, "m.txt"), 2, 0, 2); // OUTPUT truncates
        assert_eq!(t.lof(i), 0);
        t.close(i);
    }

    #[test]
    fn scrn_device_and_fileno_bookkeeping() {
        let mut t = table();
        let i = t.open(b"scrn:", 2, 0, 2);
        assert!(i >= 0);
        assert_eq!(t.f(i).scrn, 1);
        assert_eq!(t.lof(i), -4);
        assert_eq!(t.fileno_valid(0), -2);
        assert_eq!(t.fileno_valid(-3), -2);
        let f = t.max_fileno() + 7;
        assert_eq!(t.fileno_valid(f), 0);
        t.fileno_use(f, i);
        assert_eq!(t.fileno_valid(f), 1);
        assert_eq!(t.fileno_index(f), i);
        assert_eq!(t.f(i).fileno, f);
        assert_eq!(t.close(i), 0);
        assert_eq!(t.fileno_valid(f), 0);
        // closed SCRN: entries stay "valid" (legacy keeps scrn set)
        assert!(t.valid(i) || t.f(i).scrn == 0);
    }

    #[test]
    fn com_syntax_parses_options_and_defaults() {
        let mut f = GfsFile::zeroed();
        let mut x = 0;
        assert_eq!(com_syntax(b"COM1:\0", &mut f, &mut x), 1);
        assert_eq!(
            (f.com_port, f.com_baud_rate, f.com_parity, f.com_data_bits_per_byte, f.com_stop_bits),
            (1, 300, 1, 7, 10)
        );
        assert_eq!((f.com_cs_x, f.com_ds_x, f.com_op_x), (1000, 1000, 10000));
        // a stale `x` (the OP timeout) makes the next name fail; open_com_syntax
        // starts every call at 0, so a second COM name works
        assert_eq!(x, 10000);
        assert_eq!(com_syntax(b"COM1:\0", &mut GfsFile::zeroed(), &mut x), -1);
        assert_eq!(open_com_syntax(b"COM1:\0", &mut GfsFile::zeroed()), 1);
        assert_eq!(open_com_syntax(b"COM2:\0", &mut GfsFile::zeroed()), 1);
        x = 0;
        let mut f = GfsFile::zeroed();
        assert_eq!(com_syntax(b"com2:9600,N,8,1,ASC,LF,CD0,DS500\0", &mut f, &mut x), 1);
        assert_eq!(
            (f.com_port, f.com_baud_rate, f.com_parity, f.com_data_bits_per_byte, f.com_stop_bits),
            (2, 9600, 0, 8, 10)
        );
        assert_eq!((f.com_bin_asc, f.com_asc_lf, f.com_cd_x, f.com_ds_x), (1, 1, 0, 500));
        assert_eq!(com_syntax(b"file.txt\0", &mut f, &mut x), 0);
        assert_eq!(f.com_port, 0);
        assert_eq!(com_syntax(b"COM0:\0", &mut f, &mut x), 0);
        x = 0;
        assert_eq!(com_syntax(b"COM300:\0", &mut f, &mut x), -1);
        assert_eq!(com_syntax(b"COM1:9600,Q\0", &mut f, &mut x), -1);
        assert_eq!(com_syntax(b"COM1\0", &mut f, &mut x), 0);
    }

    #[test]
    fn fixdir_uses_os_separator() {
        let mut v = b"a/b\\c".to_vec();
        fixdir(&mut v);
        if cfg!(windows) {
            assert_eq!(v, b"a\\b\\c");
        } else {
            assert_eq!(v, b"a/b/c");
        }
    }

    #[test]
    fn fileno_queries_do_not_grow_the_table() {
        let mut t = table();
        assert_eq!(t.fileno_valid(2_000_000_000), -2);
        assert_eq!(t.fileno_valid(MAX_FILENO + 1), -2);
        let n = t.max_fileno();
        assert!(n + 100 <= MAX_FILENO);
        assert_eq!(t.fileno_valid(n + 100), 0);
        assert_eq!(t.fileno_valid(MAX_FILENO), 0);
        assert_eq!(t.max_fileno(), n);
        assert_eq!(t.fileno_index(n + 100), -1);
    }

    #[test]
    fn failed_open_returns_its_slot() {
        let d = tmp("slotleak");
        let name = p(&d, "nope.txt");
        let mut t = table();
        let before = t.slots.len();
        for _ in 0..10 {
            assert_eq!(t.open(&name, 1, 0, 0), -5);
        }
        assert!(t.slots.len() <= before + 1);
    }

    #[cfg(unix)]
    #[test]
    fn unix_write_only_open_writes_at_position_and_keeps_data() {
        let d = tmp("wronly");
        let name = p(&d, "w.bin");
        let mut t = table();
        let i = t.open(&name, 3, 0, 3);
        assert!(i >= 0);
        assert_eq!(t.write(i, -1, b"0123456789"), 0);
        t.close(i);
        // BINARY/RANDOM ACCESS WRITE: write-only, create, no truncate
        let i = t.open(&name, 2, 0, 1);
        assert!(i >= 0);
        assert_eq!(t.lof(i), 10);
        assert_eq!(t.write(i, 2, b"XY"), 0);
        t.close(i);
        assert_eq!(std::fs::read(d.join("w.bin")).unwrap(), b"01XY456789");
        // read-only with create (OPEN_ALWAYS) makes the missing file
        let i = t.open(&p(&d, "r.bin"), 1, 0, 1);
        assert!(i >= 0);
        assert_eq!(t.lof(i), 0);
        t.close(i);
    }

    /// The bytes INPUT # reads one at a time come from a read-ahead block: the
    /// same bytes, positions and end of file as single reads, steps back
    /// included, and a write through another file number of the file is seen.
    #[test]
    fn read_ahead_matches_single_reads_and_sees_writes() {
        let d = tmp("readahead");
        let name = p(&d, "r.txt");
        let data: Vec<u8> = (0..20_000u32).map(|k| b'a' + (k % 26) as u8).collect();
        std::fs::write(d.join("r.txt"), &data).unwrap();
        let mut t = table();
        let i = t.open(&name, 1, 0, 0);
        assert!(i >= 0 && t.slots[i as usize].buffered);
        for (k, &b) in data.iter().enumerate().take(9000) {
            assert_eq!(t.read_byte(i), Ok(b));
            assert_eq!(t.getpos(i), k as i64 + 1);
            if k % 1000 == 999 {
                // INPUT # steps back one byte
                assert_eq!(t.setpos(i, k as i64), 0);
                assert_eq!(t.read_byte(i), Ok(b));
            }
        }
        assert_eq!(t.eof_reached(i), 0);
        // GET of an INPUT file mixed in: the real position
        let mut two = [0u8; 2];
        assert_eq!(t.read(i, -1, &mut two), 0);
        assert_eq!(&two, &data[9000..9002]);
        assert_eq!(t.read_byte(i), Ok(data[9002]));
        // a write through another file number, inside the block read ahead
        let w = t.open(&name, 3, 0, 1);
        assert!(w >= 0);
        assert_eq!(t.write(w, 9003, b"XYZ"), 0);
        assert_eq!(t.read_byte(i), Ok(b'X'));
        assert_eq!(t.read_byte(i), Ok(b'Y'));
        t.close(w);
        assert_eq!(t.setpos(i, 19_999), 0);
        assert_eq!(t.read_byte(i), Ok(*data.last().unwrap()));
        assert_eq!(t.eof_reached(i), 1);
        assert_eq!(t.read_byte(i), Err(-10));
        assert_eq!(t.eof_passed(i), 1);
        assert_eq!(t.getpos(i), 20_000);
        t.close(i);
    }

    /// LOCK and UNLOCK as on Windows: another file number (or program) that
    /// locks an overlapping range is refused, a range cannot be locked twice,
    /// and UNLOCK takes only a range that was locked.
    #[cfg(target_os = "linux")]
    #[test]
    fn linux_lock_ranges_conflict_between_file_numbers() {
        let d = tmp("locks");
        let name = p(&d, "l.dat");
        let mut t = table();
        let a = t.open(&name, 3, 0, 3);
        let b = t.open(&name, 3, 0, 3);
        assert!(a >= 0 && b >= 0);
        assert_eq!(t.write(a, -1, &[0u8; 100]), 0);
        assert_eq!(t.lock(a, 10, 19, true), 0);
        assert_eq!(t.lock(a, 15, 30, true), -7);
        assert_eq!(t.lock(b, 19, 19, true), -7);
        assert_eq!(t.lock(b, 20, 29, true), 0);
        assert_eq!(t.lock(a, 10, 18, false), -7);
        assert_eq!(t.lock(a, 10, 19, false), 0);
        assert_eq!(t.lock(b, 10, 19, true), 0);
        assert_eq!(t.lock(a, -1, -1, true), -7);
        t.close(b);
        assert_eq!(t.lock(a, -1, -1, true), 0);
        assert_eq!(t.lock(a, -1, -1, false), 0);
        t.close(a);
    }

    /// `OPEN ... LOCK READ WRITE` refuses every other OPEN of the file (and is
    /// refused while the file is open), until it is closed.
    #[cfg(target_os = "linux")]
    #[test]
    fn linux_open_lock_clause_refuses_other_opens() {
        let d = tmp("share");
        let name = p(&d, "s.dat");
        let mut t = table();
        let a = t.open(&name, 3, 3, 3);
        assert!(a >= 0);
        assert_eq!(t.open(&name, 1, 0, 0), -7);
        assert_eq!(t.open(&name, 3, 3, 3), -7);
        t.close(a);
        let a = t.open(&name, 1, 0, 0);
        let b = t.open(&name, 1, 0, 0);
        assert!(a >= 0 && b >= 0);
        assert_eq!(t.open(&name, 3, 2, 3), -7);
        t.close(a);
        t.close(b);
        let c = t.open(&name, 3, 2, 3);
        assert!(c >= 0);
        t.close(c);
    }

    #[cfg(unix)]
    #[test]
    fn unix_setpos_reports_seek_errors() {
        use std::os::fd::OwnedFd;
        let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
        let mut s = Slot {
            file: Box::new(GfsFile::zeroed()),
            os: os::Handle(Some(std::fs::File::from(OwnedFd::from(a)))),
            buffered: false,
            ra: ReadAhead::default(),
            ident: None,
            locks: Vec::new(),
        };
        assert_eq!(os::setpos(&mut s, 5), -3);
    }
}
