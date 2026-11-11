//! The browser runner's logic on the host (same code as the wasm build,
//! minus the page imports): text screen, INPUT echo, SCREEN 13.

fn fixture(n: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/fixtures/{n}.qir", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

#[test]
fn hello_draws_on_the_text_screen() {
    let o = qb_web::run(&fixture("hello"), b"");
    assert_eq!(o.status, 0, "{o:?}");
    let f = o.screen.expect("screen");
    assert_eq!((f.width, f.height), (640, 400));
    assert_eq!(f.text, "Hello, QBNex\n 1  2  3\ndone\n");
}
