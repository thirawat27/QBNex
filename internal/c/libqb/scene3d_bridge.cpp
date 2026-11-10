//-----------------------------------------------------------------------------------------------------
// QBNex R15 G4 3D API: thin shims over the Rust runtime (crates/qb-runtime/src/scene3d.rs,
// declarations in qb_scene3d.h). #included into libqb.cpp. Keeps the signatures generated code
// calls (registry callnames in compiler/subs_functions/subs_functions.bas), converts qbs strings,
// resolves image handles and raises BASIC errors; everything else happens in Rust.
//-----------------------------------------------------------------------------------------------------

static void qb_3d_raise(int32 err) {
    if (err)
        error(err);
}

int32 func__loadmodel(qbs *f) {
    if (new_error)
        return -1;
    return qb_3d_load_model(f->chr, f->len);
}

int32 func__meshcube(float size) {
    if (new_error)
        return -1;
    return qb_3d_mesh_cube(size);
}

int32 func__meshplane(float w, float d) {
    if (new_error)
        return -1;
    return qb_3d_mesh_plane(w, d);
}

void sub__meshbegin() {
    if (new_error)
        return;
    qb_3d_mesh_begin();
}

// _MESHVERTEX x, y, z [, u, v]
void sub__meshvertex(float x, float y, float z, float u, float v, int32 passed) {
    if (new_error)
        return;
    if (!(passed & 1)) {
        u = 0;
        v = 0;
    }
    qb_3d_raise(qb_3d_mesh_vertex(x, y, z, u, v));
}

void sub__meshtriangle(int32 a, int32 b, int32 c) {
    if (new_error)
        return;
    qb_3d_raise(qb_3d_mesh_triangle(a, b, c));
}

int32 func__meshend() {
    if (new_error)
        return -1;
    return qb_3d_mesh_end();
}

void sub__meshcolor(int32 h, uint32 col) {
    if (new_error)
        return;
    qb_3d_raise(qb_3d_mesh_color(h, col));
}

// _MESHTEXTURE mesh&, image& (a 32-bit software image or page)
void sub__meshtexture(int32 h, int32 i) {
    if (new_error)
        return;
    if (!resolve_page_or_image_index(i, i))
        return; // error 258 raised
    img_struct *im = &img[i];
    if (im->text || im->bytes_per_pixel != 4) {
        error(5);
        return;
    }
    qb_3d_raise(qb_3d_mesh_texture(h, im->offset32, im->width, im->height));
}

void sub__freemesh(int32 h) {
    if (new_error)
        return;
    qb_3d_raise(qb_3d_free_mesh(h));
}

// _DRAW3D mesh& [, x, y, z] [, rx, ry, rz] [, sx, sy, sz]; passed: 1 position, 2 rotation, 4 scale
void sub__draw3d(int32 h, float x, float y, float z, float rx, float ry, float rz, float sx, float sy, float sz,
                 int32 passed) {
    if (new_error)
        return;
    if (!(passed & 1)) {
        x = 0;
        y = 0;
        z = 0;
    }
    if (!(passed & 2)) {
        rx = 0;
        ry = 0;
        rz = 0;
    }
    if (!(passed & 4)) {
        sx = 1;
        sy = 1;
        sz = 1;
    }
    qb_3d_raise(qb_3d_draw(h, x, y, z, rx, ry, rz, sx, sy, sz));
}

// _CAMERA (x, y, z) TO (tx, ty, tz) [, fov]
void sub__camera(float x, float y, float z, float tx, float ty, float tz, float fov, int32 passed) {
    if (new_error)
        return;
    if (!(passed & 1))
        fov = 60;
    qb_3d_raise(qb_3d_camera(x, y, z, tx, ty, tz, fov, 0));
}

// _CAMERAORTHO (x, y, z) TO (tx, ty, tz), height
void sub__cameraortho(float x, float y, float z, float tx, float ty, float tz, float height, int32 passed) {
    if (new_error)
        return;
    qb_3d_raise(qb_3d_camera(x, y, z, tx, ty, tz, height, 1));
}

void sub__camerarange(float n, float f) {
    if (new_error)
        return;
    qb_3d_raise(qb_3d_camera_range(n, f));
}

// _AMBIENT colour~& [, intensity]
void sub__ambient(uint32 col, float intensity, int32 passed) {
    if (new_error)
        return;
    if (!(passed & 1))
        intensity = 1;
    qb_3d_raise(qb_3d_ambient(col, intensity));
}

// _LIGHT(kind, x, y, z, colour~&, intensity [, range]); kind 1 directional, 2 point
int32 func__light(int32 kind, float x, float y, float z, uint32 col, float intensity, float range, int32 passed) {
    if (new_error)
        return -1;
    if (!(passed & 1))
        range = 0;
    int32 h = qb_3d_light(kind, x, y, z, col, intensity, range);
    if (h < 0)
        error(5);
    return h;
}

void sub__lighton(int32 h, int32 on) {
    if (new_error)
        return;
    qb_3d_raise(qb_3d_light_on(h, on));
}

void sub__freelight(int32 h) {
    if (new_error)
        return;
    qb_3d_raise(qb_3d_free_light(h));
}

// _CLEAR3D [colour~&]
void sub__clear3d(uint32 col, int32 passed) {
    if (new_error)
        return;
    qb_3d_clear(col, passed);
}

int32 func__gpu3d() { return qb_3d_available(); }
