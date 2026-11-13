//! 3D builtins (`_LOADMODEL`, `_MESH*`, `_DRAW3D`, `_CAMERA*`,
//! `_AMBIENT`, `_LIGHT*`, `_CLEAR3D`, `_GPU3D`), routed to the
//! `qb_runtime::scene3d` C ABI — the same functions libqb's shims call.
//! Arguments follow the registry callnames (trailing `passed` flags where the
//! keyword has optional parts). `_DISPLAY` needs nothing here: the frame
//! capture hook moves the queued draws to the shown layer.

use super::builtins::Host;
use super::{Stop, R, V};
use qb_runtime::gfx::registry as reg;
use qb_runtime::scene3d as s3;

fn raise(err: i32) -> R<Option<V>> {
    if err != 0 {
        return Err(Stop::Err(err));
    }
    Ok(None)
}

/// Handles `name` if it is a 3D builtin; `None` otherwise.
pub(crate) fn call<'a>(vm: &mut dyn Host<'a>, name: &str, a: &[V]) -> Option<R<Option<V>>> {
    let known = matches!(
        name,
        "func__loadmodel"
            | "func__meshcube"
            | "func__meshplane"
            | "sub__meshbegin"
            | "sub__meshvertex"
            | "sub__meshtriangle"
            | "func__meshend"
            | "sub__meshcolor"
            | "sub__meshtexture"
            | "sub__freemesh"
            | "sub__draw3d"
            | "sub__camera"
            | "sub__cameraortho"
            | "sub__camerarange"
            | "sub__ambient"
            | "func__light"
            | "sub__lighton"
            | "sub__freelight"
            | "sub__clear3d"
            | "func__gpu3d"
            | "func__shader"
            | "func__shadererror"
            | "sub__shaderparam"
            | "sub__meshshader"
            | "sub__freeshader"
    );
    if !known {
        return None;
    }
    let i = |k: usize| a[k].i() as i32;
    let u = |k: usize| a[k].i() as u32;
    let f = |k: usize| a[k].f() as f32;
    let int = |v: i32| Some(Ok(Some(V::I(v.into()))));
    Some(match name {
        "func__loadmodel" => {
            return int(s3::qb_3d_load_model(a[0].s()));

        }
        "func__meshcube" => return int(s3::qb_3d_mesh_cube(f(0))),
        "func__meshplane" => return int(s3::qb_3d_mesh_plane(f(0), f(1))),
        "sub__meshbegin" => {
            s3::qb_3d_mesh_begin();
            Ok(None)
        }
        "sub__meshvertex" => {
            let uv = i(5) & 1 != 0;
            let (tu, tv) = if uv { (f(3), f(4)) } else { (0.0, 0.0) };
            raise(s3::qb_3d_mesh_vertex(f(0), f(1), f(2), tu, tv))
        }
        "sub__meshtriangle" => raise(s3::qb_3d_mesh_triangle(i(0), i(1), i(2))),
        "func__meshend" => return int(s3::qb_3d_mesh_end()),
        "sub__meshcolor" => raise(s3::qb_3d_mesh_color(i(0), u(1))),
        "sub__meshtexture" => {
            let d = &mut vm.rt().display;
            let Some(x) = d.resolve(i(1)) else { return Some(Err(Stop::Err(258))) };
            // SAFETY: a resolved registry entry.
            let im = unsafe { &*reg::img.add(x as usize) };
            if im.bytes_per_pixel != 4 || im.text != 0 {
                return Some(Err(Stop::Err(5)));
            }
            // SAFETY: a 32-bit surface owns width*height pixels.
            let px = unsafe { std::slice::from_raw_parts(im.offset as *const u32, usize::from(im.width) * usize::from(im.height)) };
            raise(s3::qb_3d_mesh_texture(i(0), px, i32::from(im.width), i32::from(im.height)))
        }
        "sub__freemesh" => raise(s3::qb_3d_free_mesh(i(0))),
        "sub__draw3d" => {
            let p = i(10);
            let g = |k: usize, bit: i32, def: f32| if p & bit != 0 { f(k) } else { def };
            raise(s3::qb_3d_draw(
                i(0),
                g(1, 1, 0.0),
                g(2, 1, 0.0),
                g(3, 1, 0.0),
                g(4, 2, 0.0),
                g(5, 2, 0.0),
                g(6, 2, 0.0),
                g(7, 4, 1.0),
                g(8, 4, 1.0),
                g(9, 4, 1.0),
            ))
        }
        "sub__camera" => {
            let fov = if i(7) & 1 != 0 { f(6) } else { 60.0 };
            raise(s3::qb_3d_camera(f(0), f(1), f(2), f(3), f(4), f(5), fov, 0))
        }
        "sub__cameraortho" => raise(s3::qb_3d_camera(f(0), f(1), f(2), f(3), f(4), f(5), f(6), 1)),
        "sub__camerarange" => raise(s3::qb_3d_camera_range(f(0), f(1))),
        "sub__ambient" => {
            let k = if i(2) & 1 != 0 { f(1) } else { 1.0 };
            raise(s3::qb_3d_ambient(u(0), k))
        }
        "func__light" => {
            let range = if i(7) & 1 != 0 { f(6) } else { 0.0 };
            let h = s3::qb_3d_light(i(0), f(1), f(2), f(3), u(4), f(5), range);
            if h < 0 {
                return Some(Err(Stop::Err(5)));
            }
            return int(h);
        }
        "sub__lighton" => raise(s3::qb_3d_light_on(i(0), i(1))),
        "sub__freelight" => raise(s3::qb_3d_free_light(i(0))),
        "sub__clear3d" => {
            s3::qb_3d_clear(u(0), i(1));
            Ok(None)
        }
        "func__gpu3d" => return int(s3::qb_3d_available()),
        "func__shader" => {
            return int(s3::qb_3d_shader(a[0].s()));

        }
        "func__shadererror" => return Some(Ok(Some(V::S(s3::shader_error().into_bytes())))),
        "sub__shaderparam" => raise(s3::qb_3d_shader_param(i(0), i(1), f(2))),
        "sub__meshshader" => raise(s3::qb_3d_mesh_shader(i(0), i(1))),
        "sub__freeshader" => raise(s3::qb_3d_free_shader(i(0))),
        _ => return None,
    })
}
