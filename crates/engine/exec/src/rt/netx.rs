//! TCP/IP handles of the runtime: `_OPENHOST`, `_OPENCLIENT`,
//! `_OPENCONNECTION`, `_CONNECTED`, `_CONNECTIONADDRESS$` and GET/PUT/CLOSE
//! on negative handles. Sockets, the info parser and addresses are
//! `qb_runtime::net`; this is the handle table
//! (`connection_new`, `connection_register`, `stream_update`, `sub_get2`...).

use super::builtins::Host;
use super::{e, Stop, R, V};
use qb_runtime::net::{self as qnet, Conn};

pub(crate) struct Handle {
    conn: *mut Conn,
    /// 1 client, 2 listening host, 3 host's connection from a client
    ctype: i32,
    inbuf: Vec<u8>,
}

#[derive(Default)]
pub(crate) struct Net {
    /// handle i (1-based) is slot i-1; the BASIC value is -1-i
    handles: Vec<Option<Handle>>,
}

impl Drop for Net {
    fn drop(&mut self) {
        for h in self.handles.iter_mut().flatten() {
            // SAFETY: owned connection.
            unsafe { qnet::qb_tcp_close(h.conn) };
        }
    }
}

impl Net {
    fn register(&mut self, conn: *mut Conn, ctype: i32) -> i32 {
        let h = Handle { conn, ctype, inbuf: Vec::new() };
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
        // SAFETY: open connection, buffer of buf.len() bytes.
        let n = unsafe { qnet::qb_tcp_recv(h.conn, buf.as_mut_ptr(), buf.len() as i64) };
        if n <= 0 {
            break;
        }
        h.inbuf.extend_from_slice(&buf[..n as usize]);
        if (n as usize) < buf.len() {
            break;
        }
    }
}

fn open<'a>(vm: &mut dyn Host<'a>, method: i32, info: &[u8]) -> R<i64> {
    let (mut port, mut off, mut len, mut verr) = (0i32, 0i32, 0i32, 0i32);
    // SAFETY: info is a live slice.
    let r = unsafe { qnet::qb_net_parse_info(info.as_ptr(), info.len() as i32, method, 0, &mut port, &mut off, &mut len, &mut verr) };
    if r != 0 {
        return e(5);
    }
    let conn = if method == 0 {
        // SAFETY: host text inside info.
        unsafe { qnet::qb_tcp_client_open(info.as_ptr().add(off as usize), len, i64::from(port)) }
    } else {
        qnet::qb_tcp_host_open(i64::from(port))
    };
    if verr != 0 {
        // VAL raises the error and the open goes on (as the old runtime did)
        vm.rt().net_pending_error = verr;
    }
    if conn.is_null() {
        return Ok(0);
    }
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
            // SAFETY: listening connection.
            let c = unsafe { qnet::qb_tcp_accept(h.conn) };
            if c.is_null() {
                return Ok(Some(V::I(0)));
            }
            let i = vm.rt().net.register(c, 3);
            Ok(Some(V::I(i64::from(-1 - i))))
        })(),
        "_CONNECTED" => match vm.rt().net.get(h0) {
            Some(h) if h.ctype == 2 => Ok(Some(V::I(-1))),
            // SAFETY: open connection.
            Some(h) => Ok(Some(V::I(unsafe { qnet::qb_tcp_connected(h.conn) }.into()))),
            None => e(52),
        },
        "_CONNECTIONADDRESS$" => match vm.rt().net.get(h0) {
            Some(h) => {
                let t = if h.ctype == 2 { 2 } else { h.ctype };
                let mut buf = vec![0u8; 512];
                // SAFETY: buffer of 512 bytes.
                let n = unsafe { qnet::qb_tcp_address(h.conn, t, buf.as_mut_ptr(), buf.len() as i32) };
                if n as usize > buf.len() {
                    buf.resize(n as usize, 0);
                    // SAFETY: resized buffer.
                    unsafe { qnet::qb_tcp_address(h.conn, t, buf.as_mut_ptr(), n) };
                }
                buf.truncate(n.max(0) as usize);
                Ok(Some(V::S(buf)))
            }
            None => e(52),
        },
        "CLOSE" if h0 < 0 => {
            if let Some(slot) = (-(h0 + 1) as usize).checked_sub(1).and_then(|i| vm.rt().net.handles.get_mut(i))
                && let Some(h) = slot.take() {
                    // SAFETY: owned connection.
                    unsafe { qnet::qb_tcp_close(h.conn) };
                }
            Ok(None)
        }
        "PUTSTR" | "PUTREC" if h0 < 0 => (|| {
            let data: Vec<u8> = if name == "PUTSTR" { a[2].s().to_vec() } else { vm.bytes(a[2].p(), a[3].i() as usize)?.to_vec() };
            let Some(h) = vm.rt().net.get(h0) else { return e(52) };
            if h.ctype == 2 {
                return e(52);
            }
            // SAFETY: open connection, live data.
            unsafe { qnet::qb_tcp_send(h.conn, data.as_ptr(), data.len() as i64) };
            Ok(None)
        })(),
        "GETSTR" if h0 < 0 => (|| {
            let Some(h) = vm.rt().net.get(h0) else { return e(52) };
            if h.ctype == 2 {
                return e(52);
            }
            update(h);
            Ok(Some(V::S(std::mem::take(&mut h.inbuf))))
        })(),
        "GETREC" if h0 < 0 => (|| {
            let n = a[3].i() as usize;
            let Some(h) = vm.rt().net.get(h0) else { return e(52) };
            if h.ctype == 2 {
                return e(52);
            }
            update(h);
            if h.inbuf.len() < n {
                return Ok(None);
            }
            let got: Vec<u8> = h.inbuf.drain(..n).collect();
            vm.bytes(a[2].p(), n)?.copy_from_slice(&got);
            Ok(None)
        })(),
        _ => return None,
    };
    let r = match (r, std::mem::take(&mut vm.rt().net_pending_error)) {
        (Ok(v), 0) => Ok(v),
        (Ok(_), err) => Err(Stop::Err(err)),
        (x, _) => x,
    };
    Some(r)
}
