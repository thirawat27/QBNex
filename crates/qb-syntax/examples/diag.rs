fn main() {
    let mut total = 0;
    for a in std::env::args().skip(1) {
        let b = std::fs::read(&a).unwrap();
        let (p, _) = qb_syntax::parse_bytes(&b);
        for d in &p.diagnostics { total += 1; println!("{a}: {} @{}", d.message, d.range.start); }
    }
    println!("total {total}");
}
