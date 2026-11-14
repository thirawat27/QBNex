//! TCP/IP handles of the runtime: `_OPENHOST`, `_OPENCLIENT`,
//! `_OPENCONNECTION`, `_CONNECTED`, `_CONNECTIONADDRESS$` and GET/PUT/CLOSE
//! on negative handles. Sockets, the info parser and addresses are
//! `qb_runtime::net`; this is the handle table.

use super::builtins::Host;
use super::{e, Stop, R, V};
use qb_runtime::net::{self as qnet, Conn};

pub(crate) struct Handle {
    conn: Box<Conn>,
    /// 1 client, 2 listening host, 3 host's connection from a client
    ctype: i32,
    inbuf: Vec<u8>,
    /// the last GET of a fixed size could not be filled (EOF(handle) is -1)
    eof: bool,
}

#[derive(Default)]
pub(crate) struct Net {
    /// handle i (1-based) is slot i-1; the BASIC value is -1-i
    handles: Vec<Option<Handle>>,
}

impl Drop for Net {
    fn drop(&mut self) {
        for h in self.handles.drain(..).flatten() {
            Conn::close(*h.conn);
        }
    }
}

impl Net {
    fn register(&mut self, conn: Box<Conn>, ctype: i32) -> i32 {
        let h = Handle { conn, ctype, inbuf: Vec::new(), eof: false };
        if let Some(k) = self.handles.iter().position(|x| x.is_none()) {
            self.handles[k] = Some(h);
            return k as i32 + 1;
        }
        self.handles.push(Some(h));
        self.handles.len() as i32
    }
    fn get(&mut self, v: i64) -> Option<&mut Handle> {
        if v >= 0 {
            return None;
        }
        let i = (-(v + 1)) as usize;
        self.handles.get_mut(i.checked_sub(1)?)?.as_mut()
    }
}

/// `stream_update`: everything the socket has received so far.
fn update(h: &mut Handle) {
    let mut buf = vec![0u8; 65536];
    loop {
        let n = h.conn.recv(&mut buf);
        if n == 0 {
            break;
        }
        h.inbuf.extend_from_slice(&buf[..n]);
        if n < buf.len() {
            break;
        }
    }
}

fn open<'a>(vm: &mut dyn Host<'a>, method: i32, info: &[u8]) -> R<i64> {
    let Ok(parsed) = qnet::parse_info(info, method, false) else { return e(5) };
    let conn = if method == 0 {
        qnet::client_open(&info[parsed.host.unwrap_or_default()], i64::from(parsed.port))
    } else {
        qnet::host_open(i64::from(parsed.port))
    };
    if parsed.val_error != 0 {
        // as in QB64: VAL raised the error and the open went on
        vm.rt().net_pending_error = parsed.val_error;
    }
    let Some(conn) = conn else { return Ok(0) };
    let i = vm.rt().net.register(conn, if method == 0 { 1 } else { 2 });
    Ok(i64::from(-1 - i))
}

/// Handles `name` for TCP/IP; `None` if it is not a network call (or not a negative handle).
pub(crate) fn call<'a>(vm: &mut dyn Host<'a>, name: &str, a: &[V]) -> Option<R<Option<V>>> {
    let h0 = a.first().map_or(0, |v| v.i());
    let r = match name {
        "_OPENHOST" | "_OPENCLIENT" => {
            let m = if name == "_OPENCLIENT" { 0 } else { 1 };
            open(vm, m, a[0].s()).map(|v| Some(V::I(v)))
        }
        "_OPENCONNECTION" => (|| {
            let Some(h) = vm.rt().net.get(h0) else { return e(258) };
            if h.ctype != 2 {
                return e(258);
            }
            let Some(c) = qnet::accept(&h.conn) else { return Ok(Some(V::I(0))) };
            let i = vm.rt().net.register(c, 3);
            Ok(Some(V::I(i64::from(-1 - i))))
        })(),
        "_CONNECTED" => match vm.rt().net.get(h0) {
            Some(h) if h.ctype == 2 => Ok(Some(V::I(-1))),
            Some(h) => {
                h.conn.flush();
                Ok(Some(V::I(h.conn.connected.into())))
            }
            None => e(52),
        },
        "_CONNECTIONADDRESS$" => match vm.rt().net.get(h0) {
            Some(h) => {
                // a listener reports the public address (asked from api.ipify.org)
                let ip = if h.ctype == 2 { qnet::whatismyip() } else { Vec::new() };
                Ok(Some(V::S(qnet::address(&h.conn, h.ctype, &ip))))
            }
            None => e(52),
        },
        "CLOSE" if h0 < 0 => {
            if let Some(slot) = (-(h0 + 1) as usize).checked_sub(1).and_then(|i| vm.rt().net.handles.get_mut(i))
                && let Some(h) = slot.take() {
                    Conn::close(*h.conn);
                }
            Ok(None)
        }
        "PUTSTR" | "PUTREC" if h0 < 0 => (|| {
            let data: Vec<u8> = if name == "PUTSTR" { a[2].s().to_vec() } else { vm.bytes(a[2].p(), a[3].i() as usize)?.to_vec() };
            let Some(h) = vm.rt().net.get(h0) else { return e(52) };
            if h.ctype == 2 {
                return e(52);
            }
            h.conn.send(&data);
            Ok(None)
        })(),
        "GETSTR" if h0 < 0 => (|| {
            let Some(h) = vm.rt().net.get(h0) else { return e(52) };
            if h.ctype == 2 {
                return e(52);
            }
            update(h);
            h.eof = h.inbuf.is_empty();
            Ok(Some(V::S(std::mem::take(&mut h.inbuf))))
        })(),
        "GETREC" if h0 < 0 => (|| {
            let n = a[3].i() as usize;
            let Some(h) = vm.rt().net.get(h0) else { return e(52) };
            if h.ctype == 2 {
                return e(52);
            }
            update(h);
            h.eof = h.inbuf.len() < n;
            if h.eof {
                return Ok(None);
            }
            let got: Vec<u8> = h.inbuf.drain(..n).collect();
            vm.bytes(a[2].p(), n)?.copy_from_slice(&got);
            Ok(None)
        })(),
        // a whole array as one block of bytes, like a record
        "PUTARR" | "GETARR" if h0 < 0 => (|| {
            let (base, total) = vm.array_extent(a[2].p())?;
            if total == 0 {
                return e(9);
            }
            let n = total as usize;
            let data = if name == "PUTARR" { vm.bytes(base, n)?.to_vec() } else { Vec::new() };
            let Some(h) = vm.rt().net.get(h0) else { return e(52) };
            if h.ctype == 2 {
                return e(52);
            }
            if name == "PUTARR" {
                h.conn.send(&data);
                return Ok(None);
            }
            update(h);
            h.eof = h.inbuf.len() < n;
            if h.eof {
                return Ok(None);
            }
            let got: Vec<u8> = h.inbuf.drain(..n).collect();
            vm.bytes(base, n)?.copy_from_slice(&got);
            Ok(None)
        })(),
        // EOF(handle): whether the last GET of a value went without data (as in QB64)
        "EOF" if h0 < 0 => match vm.rt().net.get(h0) {
            Some(h) if h.ctype != 2 => Ok(Some(V::I(if h.eof { -1 } else { 0 }))),
            _ => e(52),
        },
        _ => return None,
    };
    let r = match (r, std::mem::take(&mut vm.rt().net_pending_error)) {
        (Ok(v), 0) => Ok(v),
        (Ok(_), err) => Err(Stop::Err(err)),
        (x, _) => x,
    };
    Some(r)
}
