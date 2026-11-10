# R15 G4 — 3D API ระดับสูง (`_MESH` / `_CAMERA` / `_LIGHT`)

สถานะ (2026-11-11): ออกแบบ, ชั้น GPU (`qb_gpu::scene`), runtime (`qb_runtime::scene3d`, C ABI `qb_3d_*`), QIR (`qb_ir` table + `qb-exec` `rt/scene3d.rs`) เสร็จและมี test; keyword ใน compiler + shim C++ (`libqb/scene3d_bridge.cpp`) เขียนแล้วแต่ยังไม่ได้ build/ทดสอบกับ compiler จริง.
เกี่ยวข้อง: port-plan §9.6 (G4), progress.md R15-G1..G4, `crates/qb-gpu`.

## เป้าหมาย

- โปรแกรม BASIC วาดฉาก 3D (model, กล้อง, แสง) ได้โดยไม่ต้องเขียน `_MAPTRIANGLE` ทีละสามเหลี่ยม
- backward compatible: ไม่มี keyword เดิมเปลี่ยนความหมาย; ทุกชื่อใหม่ขึ้นต้น `_` แบบ QB64
- handle เป็น `LONG` แบบเดียวกับ `_LOADIMAGE` (ค่าลบ = ไม่ถูกต้อง, `_FREE*` คืนทรัพยากร)
- ไม่มี GPU: ทุกคำสั่งยังทำงานโดยไม่ error แต่ `_DRAW3D` ไม่วาดอะไร และ `_GPU3D` คืน 0 (โปรแกรมเช็คได้)
- ฉาก 3D วาดเป็น layer ใต้หรือบน SCREEN ตาม `_DISPLAYORDER` เดิม (`_HARDWARE` layer)

## API (BASIC)

```basic
' ---- model / mesh ----
m& = _LOADMODEL("ship.glb")                 ' glTF 2.0 (.gltf/.glb); -1 เมื่อโหลดไม่ได้
m& = _MESHCUBE(size!)                        ' primitive: กล่อง
m& = _MESHPLANE(w!, d!)                      ' primitive: พื้น (แกน XZ)
_MESHBEGIN                                   ' สร้าง mesh ทีละ vertex (scalar ล้วน: IR generic ใช้ได้)
_MESHVERTEX x!, y!, z! [, u!, v!]            ' vertex ลำดับ 0,1,2,... (normals คำนวณให้)
_MESHTRIANGLE a&, b&, c&                     ' หมายเลข vertex; error 5 ถ้ายังไม่มี vertex นั้น
m& = _MESHEND                                ' -1 ถ้าไม่มีสามเหลี่ยม
' (_MESH รับ array: ภายหลัง เมื่อ IR ส่ง array ให้ builtin แบบ generic ได้สะดวก)
_MESHCOLOR m&, _RGBA32(r, g, b, a)           ' base colour ทั้ง model
_MESHTEXTURE m&, img&                        ' texture จาก image handle (32-bit)
_FREEMESH m&

' ---- transform (ต่อการวาดหนึ่งครั้ง) ----
_DRAW3D m& [, x!, y!, z!] [, rx!, ry!, rz!] [, sx!, sy!, sz!]   ' มุมเป็นองศา, ลำดับ Y-X-Z

' ---- กล้อง ----
_CAMERA (x!, y!, z!) TO (tx!, ty!, tz!) [, fov!]       ' perspective (fov แนวตั้ง องศา 1..179, ค่าเริ่ม 60)
_CAMERAORTHO (x!, y!, z!) TO (tx!, ty!, tz!), height!  ' orthographic
_CAMERARANGE near!, far!                               ' ค่าเริ่ม 0.1, 1000

' ---- แสง ----
_AMBIENT _RGB32(r, g, b) [, intensity!]
l& = _LIGHT(1, dx!, dy!, dz!, colour~&, intensity!)            ' 1 = directional (ทิศที่แสงวิ่ง)
l& = _LIGHT(2, x!, y!, z!, colour~&, intensity! [, range!])     ' 2 = point; kind อื่น = error 5
_LIGHTON l&, 0|-1
_FREELIGHT l&

' ---- frame ----
_CLEAR3D [colour~&]          ' ล้าง colour + depth ของ layer 3D
_DISPLAY                     ' เดิม: ประกอบ SCREEN + layer 3D แล้วแสดง
f% = _GPU3D                  ' -1 เมื่อมี GPU และวาด 3D ได้
```

ความหมาย:

- พิกัดโลกแบบ right-handed, +Y ขึ้น, กล้องมองไป −Z เมื่อไม่ได้ตั้ง (ตรงกับ `_MAPTRIANGLE` 3D)
- `_DRAW3D` เข้าคิว (เหมือนคำสั่ง hardware เดิม) และวาดตอน `_DISPLAY`; ลำดับในคิว = ลำดับที่วาด
- แสงสูงสุด 8 ดวงที่เปิดอยู่ (`MAX_LIGHTS`); เกินนั้นดวงหลังถูกข้าม
- shading: Lambert diffuse + ambient (ไม่มี specular ใน G4; PBR เต็มรอ G5 shader)
  - `colour = base × texture × (ambient + Σ max(n·l, 0) × light × intensity × atten)`
  - point light: `atten = 1 / (1 + (d/range)²)` (range ≤ 0 = 1/d²) ไม่มีจุดตัดคม
- alpha: base/texture alpha < 1 ผสมแบบ alpha blend; ไม่ sort ตามระยะ (ผู้ใช้วาดของทึบก่อน)
- glTF: อ่าน node tree ของ scene เริ่มต้น (transform สะสม), primitive แบบ triangles, POSITION/NORMAL/TEXCOORD_0, index, `baseColorFactor` + `baseColorTexture`; ข้าม skin/animation/morph/KHR extension (G6+)

## ชั้น implementation

| ชั้น | ที่อยู่ | งาน |
| --- | --- | --- |
| GPU | `crates/qb-gpu/src/scene.rs` | `Model` (glTF/array/primitive), `Material`, `Light`, `Lighting`, `SceneRenderer::render` (depth, Lambert) — **เสร็จ** |
| runtime | `crates/qb-runtime` (feature `gpu`) | ตาราง handle model/light, คิว `_DRAW3D`, render ลง layer 3D ตอนประกอบ frame; ไม่มี GPU = no-op |
| compiler | `compiler/subs_functions` + qb-syntax builtins | keyword ใหม่, ตรวจ argument |
| docs/tests | conformance `graphics/3d_*` | ภาพ golden render offscreen (`QBNEX_HEADLESS` + GPU), skip เมื่อไม่มี adapter |

ข้อควรระวัง:

- layer 3D ต้องประกอบบน GPU ถ้าอยากเร็ว; ระยะแรกอ่านกลับ (`read_pixels`) แล้วผสมด้วย CPU เพื่อใช้เส้นทางประกอบ frame เดิม (`window_bridge.cpp`) — ช้าแต่ถูก, ตัดทิ้งเมื่อการประกอบทั้ง frame ย้ายไป GPU (G6)
- feature `gpu` ของ qb-runtime ยังไม่ได้ตรวจการ link แบบ staticlib กับ MinGW (wgpu ใช้ `windows` crate)
- golden image ต่างกันได้ระหว่าง GPU vendor ที่ขอบสามเหลี่ยม/การปัดเศษ: test เทียบแบบมี tolerance หรือเฉพาะจุดด้านใน

## Test (qb-gpu, headless)

`crates/qb-gpu/tests/scene.rs`: ความสว่างหน้ากล่องตามมุมแสง directional, point light ใกล้ = สว่างกว่า,
ambient อย่างเดียว, depth ระหว่างสอง model, โหลด `.glb` ที่สร้างในหน่วยความจำ (node transform + baseColorFactor
+ texture), normals ที่คำนวณให้เมื่อไฟล์ไม่มี. ทุก test skip เมื่อไม่มี adapter.

## สถานะ implementation (2026-11-11)

- `_DISPLAY` (ทั้งสองเส้นทางเรียก hook `qb_fb_capture`) ย้ายคิว `_DRAW3D` ไปเป็น layer ที่แสดง; หน้าต่างผสม layer ใน `window_bridge.cpp` ผ่าน `qb_3d_present`; โหมด `_AUTODISPLAY` แสดงคิวเมื่อยังไม่มี layer
- ไม่มี feature `gpu` หรือไม่มี adapter: handle/error ทำงานครบ, ไม่วาด, `_GPU3D` = 0
- error: handle ผิด = 258, argument ผิด = 5 (`_LIGHT` kind ผิดก็ 5)
- ต้องตรวจตอน build compiler จริง: bit ของ `passed` ต่อกลุ่ม `[...]` ใน specialformat ของ `_DRAW3D` (ตั้งใจ 1 ตำแหน่ง, 2 การหมุน, 4 scale), `{To}` ใน `_CAMERA`, และรายการ keyword ใน `crates/qb-syntax` (LSP/formatter) ยังไม่ได้เพิ่ม

## G5 — `_SHADER` (WGSL ของผู้ใช้)

```basic
sh& = _SHADER(wgsl$)            ' -1 เมื่อ compile ไม่ผ่าน; _SHADERERROR$ บอกเหตุผล
e$ = _SHADERERROR$              ' "line 2, column 21: ..." (บรรทัดในข้อความของผู้ใช้) หรือ "" หลังสำเร็จ
_SHADERPARAM sh&, index&, v!    ' index 0..15 -> i.params[index \ 4][index MOD 4]
_MESHSHADER mesh&, sh&          ' 0 = แสงปกติ
_FREESHADER sh&                 ' mesh ที่ใช้อยู่กลับไปใช้แสงปกติ
```

ผู้ใช้เขียนเฉพาะ `fn shade(i: ShadeIn) -> vec4<f32>` (ฟังก์ชันช่วยอื่นได้) — runtime ห่อด้วย prelude
(binding, vertex stage, `ShadeIn`, `lighting()`, `shade_lit(i)` = หน้าตาปกติ) และ fragment entry ที่เรียก `shade`:

```wgsl
struct ShadeIn {
    world: vec3<f32>, normal: vec3<f32>, uv: vec2<f32>,
    base: vec4<f32>,   // _MESHCOLOR x texture
    light: vec3<f32>,  // ambient + แสง (Lambert) ที่ pixel นี้
    params: array<vec4<f32>, 4>,
};
```

ตรวจด้วย naga (parse + validate) ตอน `_SHADER` จึงได้ข้อความ error ที่อ่านง่ายพร้อมบรรทัด/คอลัมน์ของผู้ใช้
ไม่ใช่ panic ของ wgpu; ปัญหาที่ไม่ได้อยู่ในข้อความผู้ใช้ (เช่น `shade` คืนชนิดผิด) แจ้งเป็น line 0 พร้อมคำแนะนำ.
ไม่มี feature `gpu`: เก็บ handle โดยไม่ตรวจ. Keyword มีเฉพาะเส้นทาง QIR (ไม่มี shim C++ เพราะ libqb กำลังจะถูกลบ).

## G6 — ประสิทธิภาพ

`SceneRenderer` batch ต่อ frame: uniform ของทุก draw อยู่ใน buffer เดียว (dynamic offset), bind group ของ texture
หนึ่งอันต่อ texture, เปลี่ยน pipeline/vertex buffer เฉพาะเมื่อต่างจาก draw ก่อนหน้า.
`cargo run --release -p qb-gpu --example bench_scene` (512x512, รอ GPU ทุก frame):

| ฉาก (10k สามเหลี่ยม) | ก่อน batch | หลัง batch | WARP (software) |
| --- | --- | --- | --- |
| กล่อง 834 instance (834 draw) | 8.34 ms (120 fps) | 0.91 ms (~1100 fps) | 70.5 ms (14 fps) |
| mesh เดียว 10 000 สามเหลี่ยม (1 draw) | 0.22 ms | 0.41 ms | 6.4 ms (156 fps) |

เครื่องทดสอบ AMD Radeon 780M: ผ่าน exit gate ≥ 60 fps. ยังไม่ได้ทำ: instancing จริง (instance buffer) สำหรับ
model เดียวกันหลายพันชิ้น, ซึ่งจะช่วย WARP/GPU อ่อน.