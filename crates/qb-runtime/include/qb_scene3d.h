// R15 G4 3D API: C ABI of crates/qb-runtime/src/scene3d.rs (handles, camera,
// lights, the _DRAW3D queue). Errors: 0 ok, 5 illegal function call,
// 258 invalid handle. Functions creating handles return -1 on failure.
#ifndef QB_SCENE3D_H
#define QB_SCENE3D_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

int32_t qb_3d_available(void);
int32_t qb_3d_load_model(const uint8_t *path, int32_t len);
int32_t qb_3d_mesh_cube(float size);
int32_t qb_3d_mesh_plane(float w, float d);
void qb_3d_mesh_begin(void);
int32_t qb_3d_mesh_vertex(float x, float y, float z, float u, float v);
int32_t qb_3d_mesh_triangle(int32_t a, int32_t b, int32_t c);
int32_t qb_3d_mesh_end(void);
int32_t qb_3d_mesh_color(int32_t h, uint32_t color);
int32_t qb_3d_mesh_texture(int32_t h, const uint32_t *px, int32_t w, int32_t hh);
int32_t qb_3d_free_mesh(int32_t h);
int32_t qb_3d_draw(int32_t h, float x, float y, float z, float rx, float ry, float rz, float sx, float sy,
                   float sz);
int32_t qb_3d_camera(float ex, float ey, float ez, float tx, float ty, float tz, float fov_or_height,
                     int32_t ortho);
int32_t qb_3d_camera_range(float near_, float far_);
int32_t qb_3d_ambient(uint32_t color, float intensity);
int32_t qb_3d_light(int32_t kind, float x, float y, float z, uint32_t color, float intensity, float range);
int32_t qb_3d_light_on(int32_t h, int32_t on);
int32_t qb_3d_free_light(int32_t h);
void qb_3d_clear(uint32_t color, int32_t passed);
// window presenter: blends the shown 3D layer over the composed frame
void qb_3d_present(uint32_t *px, int32_t w, int32_t h);

#ifdef __cplusplus
}
#endif

#endif // QB_SCENE3D_H
