//! R14 hardening: parsers and decoders that BASIC programs feed with arbitrary
//! data must never panic (a panic aborts the whole program instead of raising
//! a BASIC error). Deterministic random and mutated inputs; each test reports
//! every input that panicked. (A unit-test module: the crate needs libqb host
//! symbols that only the test build provides.)

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
    /// Bit flips, byte overwrites, truncation, insertion, duplication.
    fn mutate(&mut self, src: &[u8]) -> Vec<u8> {
        let mut v = src.to_vec();
        for _ in 0..1 + self.below(4) {
            match self.below(6) {
                0 if !v.is_empty() => {
                    let i = self.below(v.len());
                    v[i] ^= 1 << self.below(8);
                }
                1 if !v.is_empty() => {
                    let i = self.below(v.len());
                    v[i] = [0, 0xFF, 0x7F, 0x80, self.next() as u8][self.below(5)];
                }
                2 => {
                    let n = self.below(v.len() + 1);
                    v.truncate(n);
                }
                3 => {
                    let i = self.below(v.len() + 1);
                    let k = 1 + self.below(8);
                    let ins = self.bytes(k);
                    v.splice(i..i, ins);
                }
                4 if v.len() > 4 => {
                    // Large 32-bit field (sizes, counts).
                    let i = self.below(v.len() - 3);
                    v[i..i + 4].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0x7F]);
                }
                _ => {}
            }
        }
        v
    }
}

fn check<F: FnMut()>(what: &str, failures: &mut Vec<String>, mut f: F) {
    if catch_unwind(AssertUnwindSafe(&mut f)).is_err() {
        failures.push(what.to_string());
    }
}

fn report(failures: Vec<String>) {
    assert!(failures.is_empty(), "{} panics, first: {:#?}", failures.len(), &failures[..failures.len().min(10)]);
}

fn hexs(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn corpus() -> Vec<(String, Vec<u8>)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/diff/data/images");
    let mut v = Vec::new();
    for e in std::fs::read_dir(dir).expect("image corpus") {
        let p = e.expect("entry").path();
        if p.extension().is_some_and(|x| x == "py") {
            continue;
        }
        v.push((p.file_name().unwrap().to_string_lossy().into_owned(), std::fs::read(&p).expect("read")));
    }
    v
}

#[test]
fn image_decoders_never_panic_on_mutated_files() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut rng = Rng(0x1234_5678_9ABC_DEF1);
    let mut failures = Vec::new();
    let files = corpus();
    for (name, data) in &files {
        for k in 0..400 {
            let m = rng.mutate(data);
            check(&format!("{name} mutation {k} len {}", m.len()), &mut failures, || {
                let _ = crate::image::decode(&m);
            });
            check(&format!("pcx {name} mutation {k}"), &mut failures, || {
                let _ = crate::image::pcx::decode(&m);
            });
        }
    }
    for k in 0..2000 {
        let len = rng.below(600);
        let m = rng.bytes(len);
        check(&format!("random {k}: {}", hexs(&m[..m.len().min(32)])), &mut failures, || {
            let _ = crate::image::decode(&m);
            let _ = crate::image::pcx::decode(&m);
        });
    }
    let _ = std::panic::take_hook();
    report(failures);
}

#[test]
fn inflate_never_panics_and_deflate_round_trips() {
    let mut rng = Rng(0x0BAD_C0DE_1234_5678);
    let mut failures = Vec::new();
    for k in 0..3000 {
        let n = rng.below(3000);
        let src: Vec<u8> = if k % 2 == 0 { rng.bytes(n) } else { (0..n).map(|i| (i % (1 + k % 7)) as u8).collect() };
        let z = crate::compress::deflate(&src);
        let mut out = vec![0u8; src.len()];
        let (code, used) = crate::compress::inflate(&z, &mut out);
        if code != 0 || used != src.len() || out != src {
            failures.push(format!("round trip {k} len {n}: code {code} used {used}"));
        }
        let bad = rng.mutate(&z);
        let mut out = vec![0u8; rng.below(4000)];
        check(&format!("inflate mutated {k}"), &mut failures, || {
            let _ = crate::compress::inflate(&bad, &mut out);
        });
    }
    report(failures);
}

#[test]
fn val_never_panics() {
    let mut rng = Rng(42);
    let mut failures = Vec::new();
    let alphabet = b"0123456789+-.eEdD&HhOoBb ,\tFfAaxX%!#";
    for k in 0..200_000 {
        let n = rng.below(40);
        let s: Vec<u8> = (0..n).map(|_| if rng.below(8) == 0 { rng.next() as u8 } else { alphabet[rng.below(alphabet.len())] }).collect();
        check(&format!("VAL {k} {:?}", String::from_utf8_lossy(&s)), &mut failures, || {
            let _ = crate::val::val(&s);
        });
    }
    let long: Vec<u8> = std::iter::repeat_n(b'9', 5000).chain(*b"E-99999").collect();
    check("long VAL", &mut failures, || {
        let _ = crate::val::val(&long);
    });
    report(failures);
}

#[test]
fn data_and_input_parsing_never_panics() {
    use crate::numparse::{read_float, read_int64, read_string, read_uint64};
    let mut rng = Rng(7);
    let mut failures = Vec::new();
    let alphabet = b"0123456789+-.eEdD&HhOoBb ,\"\0\tFf";
    for k in 0..100_000 {
        let n = rng.below(30);
        let data: Vec<u8> = (0..n).map(|_| alphabet[rng.below(alphabet.len())]).collect();
        for typ in [0, 1, 2, 3, 32, 64, 256, -1] {
            let mut off = rng.below(n + 2);
            check(&format!("read_float {k} typ {typ} {:?}", String::from_utf8_lossy(&data)), &mut failures, || {
                let _ = read_float(&data, &mut off, typ);
            });
        }
        let mut off = rng.below(n + 2);
        check(&format!("read_int64 {k}"), &mut failures, || {
            let _ = read_int64(&data, &mut off);
        });
        let mut off = rng.below(n + 2);
        check(&format!("read_uint64 {k}"), &mut failures, || {
            let _ = read_uint64(&data, &mut off);
        });
        let mut off = rng.below(n + 2);
        check(&format!("read_string {k}"), &mut failures, || {
            let _ = read_string(&data, &mut off);
        });
    }
    report(failures);
}

#[test]
fn radix_and_number_formatting_never_panic() {
    let mut rng = Rng(99);
    let mut failures = Vec::new();
    for k in 0..100_000 {
        let v = rng.next() as i64 >> rng.below(64);
        let bits = [-1, 0, 8, 16, 32, 64, 65, 1000, i32::MIN, i32::MAX][rng.below(10)];
        check(&format!("radix {k} {v} {bits}"), &mut failures, || {
            let _ = crate::mathfn::bin(v, bits);
            let _ = crate::mathfn::oct(v, bits);
            let _ = crate::mathfn::hex(v, bits);
        });
        let d = f64::from_bits(rng.next());
        let f = f32::from_bits(rng.next() as u32);
        check(&format!("str {k} {d:e} {f:e}"), &mut failures, || {
            let _ = crate::numfmt::str_f64(d);
            let _ = crate::numfmt::str_f32(f);
        });
    }
    report(failures);
}
