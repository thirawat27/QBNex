# กราฟิกและ 3 มิติ

[สารบัญคู่มือ](../README.md) · [English](../en/graphics-3d.md)

## กราฟิก 2 มิติ (ใช้ได้แล้ว)

กราฟิกทั้งหมดของ QBasic และ QB64 รันบน runtime ภาษา Rust ได้แก่
`SCREEN` 0-13 และหน้า 32-bit ของ `_NEWIMAGE`, `PSET`, `LINE`, `CIRCLE`,
`PAINT`, `DRAW`, `VIEW`/`WINDOW`, `GET`/`PUT`, `PCOPY`, `_PUTIMAGE`,
`_LOADIMAGE` (รูปแบบไฟล์ที่ QB64 อ่านได้: PNG, JPEG, BMP, PCX ...),
ฟอนต์ TrueType ด้วย `_LOADFONT`, palette, alpha blending, `_DISPLAY`,
`_LIMIT`, คีย์บอร์ด เมาส์ และจอยเกม หน้าต่างใช้ winit + softbuffer
ไม่ต้องมี OpenGL, FreeGLUT หรือ FreeType ผลลัพธ์ตรงกับ runtime C++ เดิม
ทุกพิกเซลในเทสต์ conformance

## สามเหลี่ยม 3 มิติ: `_MAPTRIANGLE` (ใช้ได้แล้ว)

`_MAPTRIANGLE` แบบ 3 มิติของ QB64 ใช้ได้

```basic
SCREEN _NEWIMAGE(640, 480, 32)
tex& = _LOADIMAGE("brick.png", 32)
_DEPTHBUFFER ON
_MAPTRIANGLE _CLOCKWISE (0, 0)-(63, 0)-(0, 63), tex& TO(-1, 1, -3)-(1, 1, -3)-(-1, -1, -3), , _SMOOTH
_DISPLAY
```

การฉายภาพ, depth buffer (`_DEPTHBUFFER ON|LOCK|OFF`), การตัดด้านหลัง
(`_CLOCKWISE`/`_ANTICLOCKWISE`), การกรองแบบ `_SMOOTH*` และ alpha ทำงาน
เหมือน QB64 ตัววาดค่าเริ่มต้นเป็นแบบซอฟต์แวร์

**ทดลอง:** runtime ที่ build พร้อมฟีเจอร์ `gpu` จะแสดงผลเฟรมและวาด
สามเหลี่ยม 3 มิติด้วย wgpu (Vulkan, DX12, Metal) ได้ ถ้าไม่มีการ์ดจอหรือ
ตั้ง `QB_GPU=off` จะกลับไปใช้ทางซอฟต์แวร์ ข้อแตกต่างที่รู้แล้วคือบน GPU
ระนาบไกล (10000) จะตัดภาพทิ้ง เหมือนที่ OpenGL ของ QB64 เดิมทำ

## 3 มิติระดับสูง: model, กล้อง, แสง (วางแผนไว้)

ฝั่ง GPU ทำเสร็จและมีเทสต์แล้ว (`crates/qb-gpu`: model glTF 2.0,
รูปทรงพื้นฐาน, แสงแบบ Lambert สูงสุด 8 ดวง, depth) แต่คำสั่ง BASIC
ด้านล่างยังไม่ได้ต่อเข้ากับคอมไพเลอร์ เอกสารออกแบบอยู่ที่
`docs/rust-port/r15-g4-3d-api.md`

```basic
SCREEN _NEWIMAGE(800, 600, 32)
IF NOT _GPU3D THEN PRINT "no GPU: 3D statements do nothing"

ship& = _LOADMODEL("ship.glb")        ' glTF 2.0 (.gltf/.glb); -1 ถ้าโหลดไม่ได้
box& = _MESHCUBE(1)
floor& = _MESHPLANE(20, 20)
_MESHCOLOR floor&, _RGBA32(80, 120, 80, 255)
_MESHTEXTURE box&, _LOADIMAGE("crate.png", 32)

_CAMERA 0, 3, 8 TO 0, 0, 0, 60        ' ตำแหน่งตา TO จุดที่มอง, มุมมองแนวตั้ง
_CAMERARANGE 0.1, 1000
_AMBIENT _RGB32(255, 255, 255), 0.2
sun& = _LIGHT(_DIRECTIONAL, -1, -2, -1, _RGB32(255, 250, 240), 1)
lamp& = _LIGHT(_POINT, 2, 2, 0, _RGB32(255, 200, 120), 2, 6)

DO
    _LIMIT 60
    a! = a! + 1
    _CLEAR3D _RGB32(20, 20, 40)
    _DRAW3D floor&
    _DRAW3D box&, 0, 0.5, 0, 0, a!, 0            ' ตำแหน่ง, การหมุน (องศา)
    _DRAW3D ship&, 3, 1, 0, 0, -a!, 0, 0.5, 0.5, 0.5
    _DISPLAY
LOOP UNTIL _KEYHIT = 27
_FREEMESH box&: _FREELIGHT lamp&
```

| คำสั่ง / ฟังก์ชัน | ความหมาย |
| --- | --- |
| `_LOADMODEL(file$)` | โหลด glTF 2.0 |
| `_MESHCUBE(size!)`, `_MESHPLANE(w!, d!)` | รูปทรงพื้นฐาน |
| `_MESH(px!(), py!(), pz!(), nx!(), ny!(), nz!(), u!(), v!(), idx&())` | model จาก array ถ้า normal ว่างจะคำนวณให้ |
| `_MESHCOLOR m&, colour~&` · `_MESHTEXTURE m&, img&` · `_FREEMESH m&` | วัสดุ, คืนหน่วยความจำ |
| `_DRAW3D m& [, x, y, z] [, rx, ry, rz] [, sx, sy, sz]` | เข้าคิววาด แล้ววาดจริงตอน `_DISPLAY` |
| `_CAMERA x, y, z TO tx, ty, tz [, fov!]` · `..., _ORTHO height!` | กล้องแบบ perspective หรือ orthographic |
| `_CAMERARANGE near!, far!` | ระยะตัดภาพ (ค่าเริ่มต้น 0.1, 1000) |
| `_AMBIENT colour~& [, intensity!]` | แสงโดยรอบ |
| `_LIGHT(_DIRECTIONAL|_POINT, ...)` · `_LIGHTON l&, on` · `_FREELIGHT l&` | แหล่งแสง (เปิดได้พร้อมกันสูงสุด 8 ดวง) |
| `_CLEAR3D [colour~&]` | ล้างสีและ depth ของชั้น 3 มิติ |
| `_GPU3D` | คืน -1 เมื่อวาด 3 มิติได้ |

ถ้าไม่มี GPU ทุกคำสั่งยังรันได้โดยไม่เกิด error แต่ `_DRAW3D` จะไม่วาดอะไร
และ `_GPU3D` คืน 0
