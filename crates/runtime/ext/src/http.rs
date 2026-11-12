//! HTTP/HTTPS client (ureq + rustls with the Mozilla root store; no
//! OpenSSL). Non-2xx statuses are normal results, not errors.

use std::time::Duration;

/// A finished request.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    /// "Name: value" lines joined with CR LF, in the order received.
    pub headers: String,
    pub body: Vec<u8>,
}

/// Upper bound for a response body (larger bodies are an error).
pub const MAX_BODY: u64 = 512 * 1024 * 1024;

/// Sends one request. `headers` holds "Name: value" lines (CR/LF or LF
/// separated, blank lines ignored). `timeout_ms` <= 0 means 30 seconds; it
/// bounds the whole request (connect, send, receive). Redirects (up to 10)
/// are followed.
#[cfg(target_arch = "wasm32")]
pub fn request(_method: &str, _url: &str, _headers: &str, _body: &[u8], _timeout_ms: i64) -> Result<Response, String> {
    let _ = Duration::ZERO;
    Err("HTTP is not available in a browser build".into())
}

/// TLS through rustls with the RustCrypto provider (pure Rust: no ring or
/// aws-lc, so building QBNex needs no C compiler) and the Mozilla roots.
#[cfg(not(target_arch = "wasm32"))]
fn tls_config(extra_roots: Option<&[u8]>) -> ureq::tls::TlsConfig {
    use std::sync::{Arc, OnceLock};
    static PROVIDER: OnceLock<Arc<rustls::crypto::CryptoProvider>> = OnceLock::new();
    let provider = PROVIDER.get_or_init(|| Arc::new(rustls_rustcrypto::provider()));
    ureq::tls::TlsConfig::builder()
        .provider(ureq::tls::TlsProvider::Rustls)
        .root_certs(match extra_roots {
            // Tests: trust exactly the given PEM certificate(s).
            Some(pem) => ureq::tls::RootCerts::new_with_certs(
                &ureq::tls::parse_pem(pem)
                    .filter_map(|i| match i {
                        Ok(ureq::tls::PemItem::Certificate(c)) => Some(c),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            None => ureq::tls::RootCerts::WebPki,
        })
        .unversioned_rustls_crypto_provider(provider.clone())
        .build()
}

/// See the browser variant above for the signature contract.
#[cfg(not(target_arch = "wasm32"))]
pub fn request(method: &str, url: &str, headers: &str, body: &[u8], timeout_ms: i64) -> Result<Response, String> {
    request_with_roots(method, url, headers, body, timeout_ms, None)
}

/// [`request`] that trusts only the PEM certificates in `extra_roots`
/// instead of the Mozilla roots (`None`: the Mozilla roots). For tests.
#[cfg(not(target_arch = "wasm32"))]
pub fn request_with_roots(
    method: &str,
    url: &str,
    headers: &str,
    body: &[u8],
    timeout_ms: i64,
    extra_roots: Option<&[u8]>,
) -> Result<Response, String> {
    let timeout = Duration::from_millis(if timeout_ms <= 0 { 30_000 } else { timeout_ms as u64 });
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .max_redirects(10)
        .user_agent("QBNex")
        .allow_non_standard_methods(true)
        .tls_config(tls_config(extra_roots))
        .build()
        .into();
    let mut b = ureq::http::Request::builder().method(method.to_ascii_uppercase().as_str()).uri(url);
    for line in headers.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            return Err(format!("bad header line: {line}"));
        };
        b = b.header(k.trim(), v.trim());
    }
    let mut resp = if body.is_empty() && !matches!(method.to_ascii_uppercase().as_str(), "POST" | "PUT" | "PATCH") {
        let req = b.body(()).map_err(|e| e.to_string())?;
        agent.run(req)
    } else {
        let req = b.body(body.to_vec()).map_err(|e| e.to_string())?;
        agent.run(req)
    }
    .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let mut head = String::new();
    for (k, v) in resp.headers() {
        if !head.is_empty() {
            head.push_str("\r\n");
        }
        head.push_str(k.as_str());
        head.push_str(": ");
        head.push_str(&String::from_utf8_lossy(v.as_bytes()));
    }
    let body = resp.body_mut().with_config().limit(MAX_BODY).read_to_vec().map_err(|e| e.to_string())?;
    Ok(Response { status, headers: head, body })
}

#[doc(hidden)]
pub mod testserver {
    //! A tiny in-process HTTP/1.1 server for tests (no internet needed),
    //! also reachable from BASIC through `qb_ext_test_http_server`.
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    /// Serves `count` requests on 127.0.0.1 in a thread; each response
    /// echoes the method, path, a request header and the body. Returns the
    /// base URL ("http://127.0.0.1:port").
    /// An empty string when no port can be bound. Malformed requests are
    /// dropped: the server thread never panics (with panic=abort that
    /// would end the BASIC program).
    pub fn start(count: usize) -> String {
        let Ok(l) = TcpListener::bind("127.0.0.1:0") else { return String::new() };
        let Ok(port) = l.local_addr().map(|a| a.port()) else { return String::new() };
        std::thread::spawn(move || {
            for _ in 0..count {
                let Ok((s, _)) = l.accept() else { return };
                let Ok(clone) = s.try_clone() else { continue };
                let mut r = BufReader::new(clone);
                let mut line = String::new();
                if r.read_line(&mut line).is_err() {
                    continue;
                }
                let mut parts = line.split_whitespace();
                let method = parts.next().unwrap_or("").to_string();
                let path = parts.next().unwrap_or("").to_string();
                let (mut len, mut xtest) = (0usize, String::new());
                let mut bad = false;
                loop {
                    let mut h = String::new();
                    if r.read_line(&mut h).is_err() {
                        bad = true;
                        break;
                    }
                    let h = h.trim_end();
                    if h.is_empty() {
                        break;
                    }
                    let (k, v) = h.split_once(':').unwrap_or((h, ""));
                    if k.eq_ignore_ascii_case("content-length") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                    if k.eq_ignore_ascii_case("x-test") {
                        xtest = v.trim().to_string();
                    }
                }
                if bad {
                    continue;
                }
                let mut body = vec![0; len.min(64 * 1024 * 1024)];
                if r.read_exact(&mut body).is_err() {
                    continue;
                }
                let mut s = s;
                if path == "/redirect" {
                    let _ = s.write_all(b"HTTP/1.1 302 Found\r\nLocation: /target\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    continue;
                }
                if path == "/slow" {
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                }
                let status = if path == "/missing" { "404 Not Found" } else { "200 OK" };
                let mut payload = format!("{method} {path} x-test={xtest} body=").into_bytes();
                payload.extend_from_slice(&body);
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nX-Server: qb-test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    payload.len()
                );
                let _ = s.write_all(head.as_bytes());
                let _ = s.write_all(&payload);
            }
        });
        format!("http://127.0.0.1:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_post_status_headers_redirect() {
        let base = testserver::start(5);
        let r = request("GET", &format!("{base}/hello?a=1"), "X-Test: yes\n", b"", 5000).unwrap();
        assert_eq!(r.status, 200);
        assert_eq!(r.body, b"GET /hello?a=1 x-test=yes body=");
        assert!(r.headers.contains("x-server: qb-test"));
        let r = request("post", &format!("{base}/p"), "Content-Type: text/plain\r\n", b"data\x00\xff", 5000).unwrap();
        assert_eq!(r.body, b"POST /p x-test= body=data\x00\xff");
        let r = request("GET", &format!("{base}/missing"), "", b"", 5000).unwrap();
        assert_eq!(r.status, 404);
        let r = request("GET", &format!("{base}/redirect"), "", b"", 5000).unwrap();
        assert_eq!((r.status, r.body.as_slice()), (200, &b"GET /target x-test= body="[..]));
        assert!(request("GET", &format!("{base}/x"), "no colon", b"", 5000).unwrap_err().contains("bad header"));
    }

    /// The test server is reachable from BASIC; a malformed request
    /// (non-UTF-8 request line, early close) must not kill it (with
    /// panic=abort a panic in its thread aborts the whole program).
    #[test]
    fn test_server_survives_malformed_requests() {
        use std::io::Write;
        let base = testserver::start(3);
        let addr = base.trim_start_matches("http://").to_string();
        let mut s = std::net::TcpStream::connect(&addr).unwrap();
        s.write_all(b"\xff\xfe GET / HTTP/1.1\r\n\r\n").unwrap();
        drop(s);
        let s = std::net::TcpStream::connect(&addr).unwrap();
        drop(s); // connect and close without a request
        let r = request("GET", &format!("{base}/after"), "", b"", 5000).unwrap();
        assert_eq!(r.body, b"GET /after x-test= body=");
    }

    #[test]
    fn timeout_and_refused() {
        let base = testserver::start(1);
        let e = request("GET", &format!("{base}/slow"), "", b"", 300).unwrap_err();
        assert!(e.to_lowercase().contains("timeout") || e.to_lowercase().contains("timed out"), "{e}");
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        drop(l);
        assert!(request("GET", &format!("http://127.0.0.1:{port}/"), "", b"", 2000).is_err());
        assert!(request("GET", "not a url", "", b"", 2000).is_err());
    }

    /// HTTPS end to end with the pure-Rust RustCrypto provider on both sides
    /// (self-signed P-256 certificate for 127.0.0.1 in tests_data/).
    #[test]
    fn https_with_rustcrypto_provider() {
        use rustls::pki_types::pem::PemObject;
        use rustls::pki_types::{CertificateDer, PrivateKeyDer};
        use std::io::{Read, Write};
        use std::sync::Arc;

        let cert_pem = include_bytes!("../tests_data/cert.pem");
        let key_pem = include_bytes!("../tests_data/key.pem");
        let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(cert_pem).map(Result::unwrap).collect();
        let key = PrivateKeyDer::from_pem_slice(key_pem).unwrap();
        let provider = Arc::new(rustls_rustcrypto::provider());
        let config = Arc::new(
            rustls::ServerConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(certs, key)
                .unwrap(),
        );
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for _ in 0..2 {
                let Ok((tcp, _)) = l.accept() else { return };
                let Ok(conn) = rustls::ServerConnection::new(config.clone()) else { return };
                let mut tls = rustls::StreamOwned::new(conn, tcp);
                let mut req = Vec::new();
                let mut buf = [0u8; 1024];
                while !req.windows(4).any(|w| w == b"\r\n\r\n") {
                    match tls.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => req.extend_from_slice(&buf[..n]),
                    }
                }
                let body = b"secure hello";
                let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = tls.write_all(head.as_bytes());
                let _ = tls.write_all(body);
                tls.conn.send_close_notify();
                let _ = tls.flush();
            }
        });
        let url = format!("https://127.0.0.1:{port}/");
        let r = request_with_roots("GET", &url, "", b"", 10_000, Some(cert_pem)).unwrap();
        assert_eq!((r.status, r.body.as_slice()), (200, &b"secure hello"[..]));
        // With the public roots the self-signed server is rejected.
        let e = request("GET", &url, "", b"", 10_000).unwrap_err();
        assert!(e.to_lowercase().contains("certificate") || e.to_lowercase().contains("issuer"), "{e}");
    }

    /// Real certificate chain against the Mozilla roots (needs internet;
    /// run by hand: cargo test -p qb-ext public_https -- --ignored).
    #[test]
    #[ignore]
    fn public_https() {
        let r = request("GET", "https://example.com/", "", b"", 15_000).unwrap();
        assert_eq!(r.status, 200);
        assert!(String::from_utf8_lossy(&r.body).contains("Example Domain"));
    }
}