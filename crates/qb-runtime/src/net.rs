//! TCP/IP for `_OPENHOST`, `_OPENCLIENT`, `_OPENCONNECTION`, `_CONNECTED`,
//! `_CONNECTIONADDRESS`, `PUT`/`GET` on network handles and `CLOSE`.
//!
//! Replaces the socket layer of libqb.cpp (`tcp_host_open`, `tcp_client_open`,
//! `tcp_connection_open`, `tcp_close`, `tcp_out`, the `recv` in
//! `stream_update`, `tcp_connected`), the info-string parsing of
//! `connection_new`, the address text of `func__connectionaddress` and
//! `myip.cpp` (`WHATISMYIP`). The handle tables (`special_handles`, streams)
//! stay in libqb.cpp because file I/O shares them.
//!
//! Semantics kept from legacy:
//! - IPv4 only; hosts listen on `0.0.0.0`; client connects block, everything
//!   after that is non-blocking.
//! - `connected` starts at -1 and drops to 0 on any send error (including
//!   "would block"), on a receive error other than "would block", and on EOF.
//! - The remote port of an accepted connection is reported with its bytes
//!   swapped on little-endian machines (legacy read the network-order field as
//!   a native `uint16`).

use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

enum Sock {
    Listener(TcpListener),
    Stream(TcpStream),
}

/// One open TCP endpoint (legacy `struct tcp_connection`).
pub struct Conn {
    sock: Sock,
    /// Listener: the requested port. Client: the port it connected to.
    /// Accepted connection: the remote port as legacy reported it.
    pub port: i32,
    /// Accepted connection: remote IPv4 address.
    pub ip4: [u8; 4],
    /// Client: host name as given (up to the first NUL).
    pub hostname: Vec<u8>,
    /// -1 while usable, 0 after a failure or remote close.
    pub connected: i32,
}

impl Conn {
    fn new(sock: Sock, port: i32) -> Box<Conn> {
        Box::new(Conn { sock, port, ip4: [0; 4], hostname: Vec::new(), connected: -1 })
    }

    /// Sends all of `data`; any error marks the connection as lost.
    pub fn send(&mut self, data: &[u8]) {
        let Sock::Stream(stream) = &mut self.sock else { return };
        let mut sent = 0;
        while sent < data.len() {
            match stream.write(&data[sent..]) {
                // Legacy treated every send() < 0 as fatal, "would block" included.
                Err(_) => {
                    self.connected = 0;
                    return;
                }
                Ok(0) => return,
                Ok(n) => sent += n,
            }
        }
    }

    /// Reads what is available (non-blocking). Returns the byte count; 0 means
    /// nothing arrived (or the connection was lost, see `connected`).
    pub fn recv(&mut self, buf: &mut [u8]) -> usize {
        let Sock::Stream(stream) = &mut self.sock else { return 0 };
        match stream.read(buf) {
            Ok(0) => {
                self.connected = 0; // graceful shutdown by the peer
                0
            }
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::WouldBlock => 0,
            Err(_) => {
                self.connected = 0;
                0
            }
        }
    }

    /// Shuts both directions down and closes the socket.
    pub fn close(self) {
        if let Sock::Stream(s) = &self.sock {
            let _ = s.shutdown(Shutdown::Both);
        }
    }
}

fn valid_port(port: i64) -> Option<u16> {
    u16::try_from(port).ok()
}

/// Opens a listening socket on every local IPv4 address.
pub fn host_open(port: i64) -> Option<Box<Conn>> {
    let port16 = valid_port(port)?;
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port16)).ok()?;
    listener.set_nonblocking(true).ok()?;
    Some(Conn::new(Sock::Listener(listener), port as i32))
}

/// Connects to `host` (name or dotted address; text up to the first NUL).
pub fn client_open(host: &[u8], port: i64) -> Option<Box<Conn>> {
    let port16 = valid_port(port)?;
    let host = &host[..host.iter().position(|&b| b == 0).unwrap_or(host.len())];
    let name = std::str::from_utf8(host).ok()?;
    let addrs = (name, port16).to_socket_addrs().ok()?;
    let stream = addrs.filter(SocketAddr::is_ipv4).find_map(|a| TcpStream::connect(a).ok())?;
    stream.set_nonblocking(true).ok()?;
    let mut c = Conn::new(Sock::Stream(stream), port as i32);
    c.hostname = host.to_vec();
    Some(c)
}

/// Accepts one pending connection, or `None` when nobody is waiting.
pub fn accept(host: &Conn) -> Option<Box<Conn>> {
    let Sock::Listener(listener) = &host.sock else { return None };
    let (stream, peer) = listener.accept().ok()?;
    stream.set_nonblocking(true).ok()?;
    let mut c = Conn::new(Sock::Stream(stream), 0);
    if let SocketAddr::V4(v4) = peer {
        // IPv4 sockaddr bytes: port (network order), then the address.
        c.port = i32::from(u16::from_ne_bytes(v4.port().to_be_bytes()));
        c.ip4 = v4.ip().octets();
    }
    Some(c)
}

/// Result of parsing an `_OPENHOST`/`_OPENCLIENT` info string.
#[derive(Debug, PartialEq, Eq)]
pub struct Info {
    pub port: i32,
    /// Byte range of the host name inside the info string (`_OPENCLIENT`).
    pub host: Option<std::ops::Range<usize>>,
    /// Error raised by `VAL` on the port text (legacy raised it and went on
    /// with port 0).
    pub val_error: i32,
}

pub const METHOD_CLIENT: i32 = 0;
pub const METHOD_HOST: i32 = 1;

/// Parses `"TCP/IP:port"` (host) or `"TCP/IP:port:address"` (client).
/// `Err(val_error)` is legacy's -1 ("invalid arguments", error 5); `val_error`
/// is the error `VAL` raised on the port before the rejection (0 if none).
/// `"QBNEXIDE"` is accepted instead of `TCP/IP` only when `ide_allowed`
/// (the `$DEBUG` connection, `vwatch == -1`).
pub fn parse_info(info: &[u8], method: i32, ide_allowed: bool) -> Result<Info, i32> {
    let mut parts = Vec::new();
    let mut start = 0;
    for (i, &b) in info.iter().enumerate() {
        if b == b':' {
            parts.push(start..i);
            start = i + 1;
        }
    }
    parts.push(start..info.len());
    if parts.len() < 2 {
        return Err(0);
    }
    let kind = info[parts[0].clone()].to_ascii_uppercase();
    if kind != b"TCP/IP" && (kind != b"QBNEXIDE" || !ide_allowed) {
        return Err(0);
    }
    let (value, val_error) = match crate::val::val(&info[parts[1].clone()]) {
        Ok(v) => (crate::val::f64_from_f80(v), 0),
        Err(e) => (0.0, e),
    };
    let port = crate::round::fistp_i32(value);
    let host = match (method, parts.len()) {
        (METHOD_CLIENT, 3) => Some(parts[2].clone()),
        (METHOD_HOST, 2) => None,
        _ => return Err(val_error),
    };
    Ok(Info { port, host, val_error })
}

/// `_CONNECTIONADDRESS` text. `listener_ip` is the public address for a host
/// listener (empty: unknown).
pub fn address(conn: &Conn, conn_type: i32, listener_ip: &[u8]) -> Vec<u8> {
    format_address(conn_type, conn.port, conn.ip4, &conn.hostname, listener_ip)
}

/// `"TCP/IP:" + LTRIM$(STR$(port)) + ":" + address` as `func__connectionaddress`
/// built it: the public IP (or 127.0.0.1) for a listener, the dotted remote
/// address for an accepted connection, the host name for a client.
pub fn format_address(conn_type: i32, port: i32, ip4: [u8; 4], hostname: &[u8], listener_ip: &[u8]) -> Vec<u8> {
    let mut s = b"TCP/IP:".to_vec();
    s.extend_from_slice(crate::numfmt::str_i64(i64::from(port)).trim_ascii_start());
    s.push(b':');
    match conn_type {
        2 if listener_ip.is_empty() => s.extend_from_slice(b"127.0.0.1"),
        2 => s.extend_from_slice(listener_ip),
        3 => {
            let ip = ip4.map(|o| o.to_string()).join(".");
            s.extend_from_slice(ip.as_bytes());
        }
        _ => s.extend_from_slice(hostname),
    }
    s
}

// --- WHATISMYIP (was myip.cpp, generated from BASIC) --------------------------

pub const MYIP_HOST: &str = "api.ipify.org";

/// The request legacy sent (its URL handling dropped the leading `/`).
pub fn myip_request(host: &str) -> Vec<u8> {
    format!("GET {host} HTTP/1.1\r\nHost: {host}\r\n\r\n").into_bytes()
}

/// BASIC `INSTR(start, hay, needle)`, 1-based; 0 = not found.
fn instr(start: usize, hay: &[u8], needle: &[u8]) -> usize {
    let from = start.max(1) - 1;
    if from >= hay.len() {
        return 0;
    }
    hay[from..].windows(needle.len()).position(|w| w == needle).map_or(0, |p| from + p + 1)
}

/// BASIC `MID$(s, start, len)`; `len = None` means "to the end".
fn mid(s: &[u8], start: usize, len: Option<i64>) -> &[u8] {
    let from = (start.max(1) - 1).min(s.len());
    let rest = &s[from..];
    match len {
        None => rest,
        Some(l) => &rest[..usize::try_from(l).unwrap_or(0).min(rest.len())],
    }
}

fn val_f32(s: &[u8]) -> f32 {
    crate::val::val(s).map_or(0.0, |v| crate::val::f64_from_f80(v) as f32)
}

/// Reply parser: one call per poll, with all bytes received since the last
/// one. Mirrors the BASIC loop body of `whatismyip$` (Content-Length or a
/// chunk size line).
#[derive(Default)]
pub struct IpReply {
    acc: Vec<u8>,
    len: f32,
}

impl IpReply {
    pub fn feed(&mut self, data: &[u8]) -> Option<Vec<u8>> {
        const E: &[u8] = b"\r\n";
        self.acc.extend_from_slice(data);
        let a = &self.acc;
        if self.len == 0.0 {
            let i = instr(1, a, b"\r\n\r\n");
            if i != 0 {
                let x = instr(1, a, b"Content-Length: ");
                if x != 0 && x < i {
                    let i2 = instr(x, a, E);
                    if i2 != 0 {
                        self.len = val_f32(mid(a, x + 16, Some(i2 as i64 - x as i64 - 16)));
                        self.acc = mid(a, i + 4, None).to_vec();
                    }
                } else {
                    let i2 = instr(i + 4, a, E);
                    if i2 != 0 {
                        let mut hex = b"&H".to_vec();
                        hex.extend_from_slice(mid(a, i + 4, Some(i2 as i64 - i as i64 - 2)));
                        self.len = val_f32(&hex);
                        self.acc = mid(a, i2 + 2, None).to_vec();
                    }
                }
            }
            None
        } else if self.acc.len() as f32 >= self.len {
            let n = crate::round::qbr(crate::round::f80_from_f64(f64::from(self.len)));
            Some(self.acc[..usize::try_from(n).unwrap_or(0).min(self.acc.len())].to_vec())
        } else {
            None
        }
    }
}

/// Asks `host:port` for this machine's public address. Returns empty text
/// when the server cannot be reached or does not answer within `timeout`.
pub fn fetch_ip(host: &str, port: u16, timeout: Duration) -> Vec<u8> {
    let Some(mut c) = client_open(host.as_bytes(), i64::from(port)) else { return Vec::new() };
    c.send(&myip_request(host));
    let start = Instant::now();
    let mut reply = IpReply::default();
    let mut buf = vec![0u8; 4096];
    loop {
        std::thread::sleep(Duration::from_millis(50));
        let mut got = Vec::new();
        loop {
            let n = c.recv(&mut buf);
            if n == 0 {
                break;
            }
            got.extend_from_slice(&buf[..n]);
        }
        if let Some(ip) = reply.feed(&got) {
            c.close();
            return ip;
        }
        if start.elapsed() > timeout {
            c.close();
            return Vec::new();
        }
    }
}

/// `WHATISMYIP()`: public IPv4 address from api.ipify.org, or empty.
pub fn whatismyip() -> Vec<u8> {
    fetch_ip(MYIP_HOST, 80, Duration::from_secs(5))
}

// --- C ABI (include/qb_runtime.h) ---------------------------------------------------

/// Parses an info string. Returns 0 and fills the outputs, or -1 for invalid
/// arguments (only `*val_error` is written then). `*host_off`/`*host_len` are
/// -1/0 when there is no host part.
///
/// # Safety
/// `info` must be readable for `len` bytes; the outputs must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_net_parse_info(
    info: *const u8,
    len: i32,
    method: i32,
    ide_allowed: i32,
    port: *mut i32,
    host_off: *mut i32,
    host_len: *mut i32,
    val_error: *mut i32,
) -> i32 {
    // SAFETY: caller guarantees `info` covers `len` bytes.
    let info = unsafe { bytes(info, len) };
    match parse_info(info, method, ide_allowed != 0) {
        Err(e) => {
            // SAFETY: caller guarantees the outputs are writable.
            unsafe { val_error.write_unaligned(e) };
            -1
        }
        Ok(r) => {
            let (off, l) = r.host.map_or((-1, 0), |h| (h.start as i32, h.len() as i32));
            // SAFETY: caller guarantees the outputs are writable.
            unsafe {
                port.write_unaligned(r.port);
                host_off.write_unaligned(off);
                host_len.write_unaligned(l);
                val_error.write_unaligned(r.val_error);
            }
            0
        }
    }
}

unsafe fn bytes<'a>(p: *const u8, len: impl TryInto<usize>) -> &'a [u8] {
    let len = len.try_into().unwrap_or(0);
    if p.is_null() || len == 0 {
        &[]
    } else {
        // SAFETY: caller guarantees `p` covers `len` bytes.
        unsafe { std::slice::from_raw_parts(p, len) }
    }
}

fn into_raw(c: Option<Box<Conn>>) -> *mut Conn {
    c.map_or(std::ptr::null_mut(), Box::into_raw)
}

/// # Safety
/// Always safe; returns NULL on failure.
#[unsafe(no_mangle)]
pub extern "C" fn qb_tcp_host_open(port: i64) -> *mut Conn {
    into_raw(host_open(port))
}

/// # Safety
/// `host` must be readable for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_tcp_client_open(host: *const u8, len: i32, port: i64) -> *mut Conn {
    // SAFETY: caller guarantees `host` covers `len` bytes.
    into_raw(client_open(unsafe { bytes(host, len) }, port))
}

/// # Safety
/// `host` must come from `qb_tcp_host_open` and not be closed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_tcp_accept(host: *mut Conn) -> *mut Conn {
    // SAFETY: caller guarantees a live connection.
    into_raw(accept(unsafe { &*host }))
}

/// # Safety
/// `c` must come from this module and not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_tcp_close(c: *mut Conn) {
    if !c.is_null() {
        // SAFETY: ownership returns to Rust exactly once.
        unsafe { Box::from_raw(c) }.close();
    }
}

/// # Safety
/// `c` must be live; `data` readable for `bytes` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_tcp_send(c: *mut Conn, data: *const u8, bytes_: i64) {
    // SAFETY: caller guarantees both.
    unsafe { (*c).send(bytes(data, bytes_)) }
}

/// # Safety
/// `c` must be live; `buf` writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_tcp_recv(c: *mut Conn, buf: *mut u8, cap: i64) -> i64 {
    let cap = usize::try_from(cap).unwrap_or(0);
    if buf.is_null() || cap == 0 {
        return 0;
    }
    // SAFETY: caller guarantees both.
    unsafe { (*c).recv(std::slice::from_raw_parts_mut(buf, cap)) as i64 }
}

/// # Safety
/// `c` must be live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_tcp_connected(c: *mut Conn) -> i32 {
    // SAFETY: caller guarantees a live connection.
    unsafe { (*c).connected }
}

/// Writes the `_CONNECTIONADDRESS` text into `out` (at most `cap` bytes) and
/// returns its full length. `conn_type`: 1 client, 2 host listener, 3
/// accepted connection. A listener asks api.ipify.org for the public address.
///
/// # Safety
/// `c` must be live; `out` writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_tcp_address(c: *mut Conn, conn_type: i32, out: *mut u8, cap: i32) -> i32 {
    // SAFETY: caller guarantees a live connection.
    let conn = unsafe { &*c };
    let ip = if conn_type == 2 { whatismyip() } else { Vec::new() };
    let text = address(conn, conn_type, &ip);
    let n = text.len().min(usize::try_from(cap).unwrap_or(0));
    // SAFETY: caller guarantees `out` covers `cap` bytes.
    unsafe { std::ptr::copy_nonoverlapping(text.as_ptr(), out, n) };
    text.len() as i32
}

/// `format_address` for the differential test (`ip4` 4 bytes).
///
/// # Safety
/// All pointers must cover their lengths; `out` writable for `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qb_net_format_address(
    conn_type: i32,
    port: i32,
    ip4: *const u8,
    host: *const u8,
    host_len: i32,
    listener_ip: *const u8,
    ip_len: i32,
    out: *mut u8,
    cap: i32,
) -> i32 {
    // SAFETY: caller guarantees the buffers.
    let text = unsafe {
        let ip: [u8; 4] = bytes(ip4, 4usize).try_into().unwrap_or([0; 4]);
        format_address(conn_type, port, ip, bytes(host, host_len), bytes(listener_ip, ip_len))
    };
    let n = text.len().min(usize::try_from(cap).unwrap_or(0));
    // SAFETY: caller guarantees `out` covers `cap` bytes.
    unsafe { std::ptr::copy_nonoverlapping(text.as_ptr(), out, n) };
    text.len() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    fn free_port() -> u16 {
        TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
    }

    /// Polls `f` for up to two seconds.
    fn poll<T>(mut f: impl FnMut() -> Option<T>) -> T {
        let start = Instant::now();
        loop {
            if let Some(v) = f() {
                return v;
            }
            assert!(start.elapsed() < Duration::from_secs(2), "timed out");
            sleep(Duration::from_millis(5));
        }
    }

    fn recv_exact(c: &mut Conn, n: usize) -> Vec<u8> {
        let mut got = Vec::new();
        poll(|| {
            let mut buf = [0u8; 4096];
            let k = c.recv(&mut buf);
            got.extend_from_slice(&buf[..k]);
            (got.len() >= n).then_some(())
        });
        got
    }

    #[test]
    fn loopback_echo_between_host_and_client() {
        let port = free_port();
        let host = host_open(i64::from(port)).expect("listen");
        assert_eq!(host.connected, -1);
        assert!(accept(&host).is_none(), "accept must not block");

        let mut client = client_open(b"localhost", i64::from(port)).expect("connect");
        assert_eq!(client.connected, -1);
        assert_eq!(client.hostname, b"localhost");
        let mut server = poll(|| accept(&host));
        assert_eq!(server.ip4, [127, 0, 0, 1]);

        let mut buf = [0u8; 16];
        assert_eq!(server.recv(&mut buf), 0, "recv must not block");
        assert_eq!(server.connected, -1);

        client.send(b"hello, world");
        assert_eq!(recv_exact(&mut server, 12), b"hello, world");
        server.send(&[0, 1, 2, 255]);
        assert_eq!(recv_exact(&mut client, 4), [0, 1, 2, 255]);

        let big: Vec<u8> = (0..50_000u32).map(|i| i as u8).collect();
        client.send(&big);
        assert_eq!(client.connected, -1);
        assert_eq!(recv_exact(&mut server, big.len()), big);

        client.close();
        poll(|| {
            server.recv(&mut buf);
            (server.connected == 0).then_some(())
        });
        server.close();
        host.close();
    }

    #[test]
    fn accepted_connection_reports_byte_swapped_remote_port() {
        let port = free_port();
        let host = host_open(i64::from(port)).unwrap();
        let client = client_open(b"127.0.0.1", i64::from(port)).unwrap();
        let server = poll(|| accept(&host));
        let local = match &client.sock {
            Sock::Stream(s) => s.local_addr().unwrap().port(),
            Sock::Listener(_) => unreachable!(),
        };
        assert_eq!(server.port, i32::from(u16::from_ne_bytes(local.to_be_bytes())));
        assert_eq!(address(&server, 3, b""), format!("TCP/IP:{}:127.0.0.1", server.port).into_bytes());
    }

    #[test]
    fn failures_return_none() {
        assert!(host_open(-1).is_none());
        assert!(host_open(65_536).is_none());
        assert!(client_open(b"localhost", -1).is_none());
        assert!(client_open(b"localhost", 70_000).is_none());
        assert!(client_open(b"no-such-host.invalid", 80).is_none());
        let port = free_port(); // nothing listens there
        assert!(client_open(b"127.0.0.1", i64::from(port)).is_none());
        let busy = TcpListener::bind("0.0.0.0:0").unwrap();
        assert!(host_open(i64::from(busy.local_addr().unwrap().port())).is_none() || cfg!(unix));
    }

    #[test]
    fn hostname_stops_at_nul() {
        let port = free_port();
        let host = host_open(i64::from(port)).unwrap();
        let c = client_open(b"127.0.0.1\0garbage", i64::from(port)).unwrap();
        assert_eq!(c.hostname, b"127.0.0.1");
        assert_eq!(address(&c, 1, b""), format!("TCP/IP:{port}:127.0.0.1").into_bytes());
        drop(host);
    }

    #[test]
    fn listener_address_uses_public_ip_or_localhost() {
        let host = host_open(0).unwrap();
        let mut h = host;
        h.port = 4567;
        assert_eq!(address(&h, 2, b""), b"TCP/IP:4567:127.0.0.1");
        assert_eq!(address(&h, 2, b"203.0.113.9"), b"TCP/IP:4567:203.0.113.9");
    }

    #[test]
    fn parse_host_and_client_info() {
        let ok = |p, h: Option<std::ops::Range<usize>>| Ok(Info { port: p, host: h, val_error: 0 });
        assert_eq!(parse_info(b"TCP/IP:7319", METHOD_HOST, false), ok(7319, None));
        assert_eq!(parse_info(b"tcp/ip:7319", METHOD_HOST, false), ok(7319, None));
        assert_eq!(parse_info(b"TCP/IP:7319:localhost", METHOD_CLIENT, false), ok(7319, Some(12..21)));
        assert_eq!(parse_info(b"TCP/IP:80:", METHOD_CLIENT, false), ok(80, Some(10..10)));
        assert_eq!(parse_info(b"TCP/IP:", METHOD_HOST, false), ok(0, None));
        assert_eq!(parse_info(b"TCP/IP: 12.5", METHOD_HOST, false), ok(12, None)); // VAL + round half even
        assert_eq!(parse_info(b"TCP/IP:13.5", METHOD_HOST, false), ok(14, None));
        assert_eq!(parse_info(b"TCP/IP:&H10", METHOD_HOST, false), ok(16, None));
        assert_eq!(parse_info(b"TCP/IP:-5", METHOD_HOST, false), ok(-5, None));
        assert_eq!(parse_info(b"TCP/IP:1E20", METHOD_HOST, false), ok(i32::MIN, None));
    }

    #[test]
    fn parse_rejects_bad_info() {
        for (info, method) in [
            (&b""[..], METHOD_HOST),
            (b"TCP/IP", METHOD_HOST),
            (b"UDP:80", METHOD_HOST),
            (b"TCP/IP:80:host", METHOD_HOST),
            (b"TCP/IP:80", METHOD_CLIENT),
            (b"TCP/IP:80:a:b", METHOD_CLIENT),
            (b"TCP/IP:1:2:3:4:5:6:7:8:9:10:11:12", METHOD_CLIENT),
            (b"QBNEXIDE:80", METHOD_HOST),
        ] {
            assert_eq!(parse_info(info, method, false), Err(0), "{}", String::from_utf8_lossy(info));
        }
        assert_eq!(parse_info(b"qbnexide:9000:localhost", METHOD_CLIENT, true).map(|i| i.port), Ok(9000));
    }

    #[test]
    fn parse_reports_val_errors() {
        // VAL("&H" + 17 hex digits) overflows: legacy raised the error and used port 0.
        let r = parse_info(b"TCP/IP:&H11111111111111111", METHOD_HOST, false).unwrap();
        assert_eq!(r.port, 0);
        assert_ne!(r.val_error, 0);
        // Rejected after VAL ran: the VAL error is still reported.
        assert_eq!(parse_info(b"TCP/IP:&H11111111111111111", METHOD_CLIENT, false), Err(r.val_error));
    }

    #[test]
    fn myip_request_matches_legacy_bytes() {
        assert_eq!(myip_request("api.ipify.org"), b"GET api.ipify.org HTTP/1.1\r\nHost: api.ipify.org\r\n\r\n");
    }

    #[test]
    fn myip_reply_with_content_length() {
        let mut r = IpReply::default();
        assert_eq!(r.feed(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n"), None);
        assert_eq!(r.feed(b"\r\n203.0."), None); // length found, body too short
        assert_eq!(r.feed(b"113.9"), Some(b"203.0.113.9".to_vec()));
        let mut r = IpReply::default();
        assert_eq!(r.feed(b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\n1.2.3.4"), None); // checked next poll
        assert_eq!(r.feed(b""), Some(b"1.2.3.4".to_vec()));
    }

    #[test]
    fn myip_reply_chunked() {
        let mut r = IpReply::default();
        assert_eq!(r.feed(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nb\r\n203.0.113.9\r\n0\r\n\r\n"), None);
        assert_eq!(r.feed(b""), Some(b"203.0.113.9".to_vec()));
        let mut r = IpReply::default();
        assert_eq!(r.feed(b"HTTP/1.1 200 OK\r\n\r\n"), None);
        assert_eq!(r.feed(b"7"), None);
        assert_eq!(r.feed(b"\r\n1.2.3.4\r\n"), None);
        assert_eq!(r.feed(b""), Some(b"1.2.3.4".to_vec()));
    }

    #[test]
    fn fetch_ip_from_a_local_server() {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = server.local_addr().unwrap().port();
        let t = std::thread::spawn(move || {
            let (mut s, _) = server.accept().unwrap();
            let mut req = vec![0u8; 256];
            let n = s.read(&mut req).unwrap();
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\n10.9.8.7\n").unwrap();
            req.truncate(n);
            req
        });
        assert_eq!(fetch_ip("127.0.0.1", port, Duration::from_secs(5)), b"10.9.8.7\n");
        assert_eq!(t.join().unwrap(), myip_request("127.0.0.1"));
        let closed = free_port();
        assert_eq!(fetch_ip("127.0.0.1", closed, Duration::from_secs(5)), b"");
    }

    #[test]
    fn fetch_ip_gives_up_after_timeout() {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = server.local_addr().unwrap().port();
        let start = Instant::now();
        assert_eq!(fetch_ip("127.0.0.1", port, Duration::from_millis(300)), b"");
        assert!(start.elapsed() >= Duration::from_millis(300));
        drop(server);
    }

    #[test]
    fn c_abi_round_trip() {
        let port = free_port();
        let info = format!("TCP/IP:{port}");
        let (mut p, mut off, mut len, mut ve) = (0, 0, 0, 0);
        let r = unsafe {
            qb_net_parse_info(info.as_ptr(), info.len() as i32, METHOD_HOST, 0, &mut p, &mut off, &mut len, &mut ve)
        };
        assert_eq!((r, p, off, len, ve), (0, i32::from(port), -1, 0, 0));
        let h = qb_tcp_host_open(i64::from(p));
        assert!(!h.is_null());
        let c = unsafe { qb_tcp_client_open(b"127.0.0.1".as_ptr(), 9, i64::from(p)) };
        assert!(!c.is_null());
        let s = poll(|| {
            let s = unsafe { qb_tcp_accept(h) };
            (!s.is_null()).then_some(s)
        });
        unsafe { qb_tcp_send(c, b"ping".as_ptr(), 4) };
        let mut buf = [0u8; 8];
        let n = poll(|| {
            let n = unsafe { qb_tcp_recv(s, buf.as_mut_ptr(), 8) };
            (n > 0).then_some(n)
        });
        assert_eq!(&buf[..n as usize], b"ping");
        assert_eq!(unsafe { qb_tcp_connected(s) }, -1);
        let mut out = [0u8; 64];
        let l = unsafe { qb_tcp_address(c, 1, out.as_mut_ptr(), 64) } as usize;
        assert_eq!(&out[..l], format!("TCP/IP:{port}:127.0.0.1").as_bytes());
        unsafe {
            qb_tcp_close(c);
            qb_tcp_close(s);
            qb_tcp_close(h);
            qb_tcp_close(std::ptr::null_mut());
        }
    }
}
