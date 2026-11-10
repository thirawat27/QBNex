//! R15 G4: the 3D builtins are callable from QIR (`rt func__meshcube ...`)
//! and reach `qb_runtime::scene3d` (handles, errors). Rendering itself is
//! tested in qb-runtime / qb-gpu.

fn run(body: &str) -> String {
    let src = format!("QIR 1\nfunc main\n{body}\nend\nendfunc\n");
    let mut out = Vec::new();
    match qb_exec::run_ir(&src, &mut out) {
        Ok(()) => String::from_utf8_lossy(&out).into_owned(),
        Err(e) => format!("{}ERR {} line {}", String::from_utf8_lossy(&out), e.code, e.line),
    }
}

#[test]
fn mesh_light_and_draw_builtins_from_qir() {
    // the 3D state is global: one test, parts in sequence
    qb_runtime::scene3d::reset();
    let mut b = String::from("const.i32 %90 0\nconst.i32 %0 0\n");
    let p = |reg: u32| format!("cvt.i64.i32 %{} %{reg}\nrt PRINTN.i64 %90 %{}\n", 100 + reg, 100 + reg);
    // cube, plane
    b += "const.f32 %1 2.0\nrt %2 func__meshcube %1\n";
    b += &p(2);
    b += "const.f32 %3 0.0\nrt %4 func__meshplane %1 %3\n"; // zero depth: -1
    b += &p(4);
    // built mesh: three vertices (uv given for one), one triangle
    b += "rt sub__meshbegin\nconst.f32 %5 1.0\nconst.i32 %6 1\nrt sub__meshvertex %3 %3 %3 %3 %3 %0\nrt sub__meshvertex %5 %3 %3 %5 %3 %6\nrt sub__meshvertex %3 %5 %3 %3 %3 %0\n";
    b += "const.i32 %7 2\nrt sub__meshtriangle %0 %6 %7\nrt %8 func__meshend\n";
    b += &p(8);
    // colour, draw with position only (passed = 1), a point light with range, camera
    b += "const.u32 %10 4294901760\nrt sub__meshcolor %2 %10\n";
    b += "rt sub__draw3d %2 %5 %5 %5 %3 %3 %3 %3 %3 %3 %6\n";
    b += "const.i32 %11 2\nconst.f32 %12 4.0\nrt %13 func__light %11 %3 %5 %3 %10 %5 %12 %6\n";
    b += &p(13);
    b += "const.f32 %14 5.0\nconst.f32 %15 45.0\nrt sub__camera %3 %3 %14 %3 %3 %3 %15 %6\n";
    b += "rt sub__lighton %13 %0\nrt %16 func__gpu3d\nrt PRINTNL %90\n";
    // freed mesh: error 258
    b += "rt sub__freemesh %2\nline 9\nrt sub__meshcolor %2 %10";
    let o = run(&b);
    assert!(o.starts_with(" 1 -1  2  1 \n"), "{o}");
    assert!(o.ends_with("ERR 258 line 9"), "{o}");
    let (shown, queued) = (qb_runtime::scene3d::shown(), qb_runtime::scene3d::queued());
    // the draw of the freed mesh is gone
    assert!(shown.is_empty() && queued.is_empty(), "{shown:?} {queued:?}");
    let c = qb_runtime::scene3d::camera();
    assert_eq!((c.eye, c.fov_or_height, c.ortho), ([0.0, 0.0, 5.0], 45.0, false));
    let l = qb_runtime::scene3d::light(1).expect("light");
    assert_eq!((l.kind, l.v, l.range, l.on), (2, [0.0, 1.0, 0.0], 4.0, false));

    qb_runtime::scene3d::reset();
    // draw with nothing optional: position 0, rotation 0, scale 1
    let o = run("const.i32 %0 0\nconst.f32 %1 1.0\nrt %2 func__meshcube %1\nconst.f32 %3 9.0\nrt sub__draw3d %2 %3 %3 %3 %3 %3 %3 %3 %3 %3 %0\nline 4\nconst.i32 %4 7\nconst.u32 %6 0\nrt %5 func__light %4 %1 %1 %1 %6 %1 %1 %0");
    assert!(o.ends_with("ERR 5 line 4"), "{o}");
    let q = qb_runtime::scene3d::queued();
    assert!(q.iter().any(|d| d.t == [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]), "{q:?}");
}
