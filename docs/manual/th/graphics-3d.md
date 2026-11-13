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

## 3 มิติระดับสูง: model, กล้อง, แสง (ทดลอง)

คอมไพเลอร์รู้จักคำสั่งและฟังก์ชันเหล่านี้แล้ว และจะวาดได้เมื่อ runtime ถูก
build พร้อมการรองรับ GPU (wgpu) ถ้าไม่มี GPU ทุกคำสั่งยังรันได้โดยไม่เกิด
error แต่ `_DRAW3D` จะไม่วาดอะไร และ `_GPU3D` คืน 0 เอกสารออกแบบอยู่ที่
`docs/dev/3d-api.md`

```basic
SCREEN _NEWIMAGE(800, 600, 32)
IF _GPU3D = 0 THEN PRINT "no GPU: 3D statements do nothing"

box& = _MESHCUBE(1)
floor& = _MESHPLANE(20, 20)
_MESHCOLOR floor&, _RGBA32(80, 120, 80, 255)
tex& = _NEWIMAGE(64, 64, 32)
_MESHTEXTURE box&, tex&

' a mesh from vertices (numbered from 0 in the order given)
_MESHBEGIN
_MESHVERTEX 0, 1, 0, 0.5, 0
_MESHVERTEX -1, 0, 0, 0, 1
_MESHVERTEX 1, 0, 0, 1, 1
_MESHTRIANGLE 0, 1, 2
tri& = _MESHEND

_CAMERA (0, 3, 8) TO (0, 0, 0), 60         ' eye TO target, vertical field of view
_CAMERARANGE 0.1, 1000
_AMBIENT _RGB32(255, 255, 255), 0.2
sun& = _LIGHT(1, -1, -2, -1, _RGB32(255, 250, 240), 1)       ' 1 = directional
lamp& = _LIGHT(2, 2, 2, 0, _RGB32(255, 200, 120), 2, 6)       ' 2 = point, range 6

FOR frame = 1 TO 3
    a! = a! + 1
    _CLEAR3D _RGB32(20, 20, 40)
    _DRAW3D floor&
    _DRAW3D box&, 0, 0.5, 0, 0, a!, 0                 ' position, rotation (degrees)
    _DRAW3D tri&, 3, 1, 0, 0, -a!, 0, 0.5, 0.5, 0.5   ' ... and scale
    _DISPLAY
NEXT
_FREEMESH box&: _FREELIGHT lamp&
```

| คำสั่ง / ฟังก์ชัน | ความหมาย |
| --- | --- |
| `_LOADMODEL(file$)` | โหลด glTF 2.0 (.gltf/.glb) คืน -1 ถ้าโหลดไม่ได้ |
| `_MESHCUBE(size!)`, `_MESHPLANE(w!, d!)` | รูปทรงพื้นฐาน |
| `_MESHBEGIN` · `_MESHVERTEX x!, y!, z! [, u!, v!]` · `_MESHTRIANGLE a&, b&, c&` · `_MESHEND` | สร้าง mesh เอง `_MESHEND` คืน handle (-1 ถ้าไม่มีสามเหลี่ยม) จุดยอดนับจาก 0 |
| `_MESHCOLOR m&, colour~&` · `_MESHTEXTURE m&, img&` · `_FREEMESH m&` | วัสดุ (texture จากภาพ 32-bit), คืนหน่วยความจำ |
| `_DRAW3D m& [, x, y, z] [, rx, ry, rz] [, sx, sy, sz]` | เข้าคิววาด (มุมเป็นองศา ลำดับ Y-X-Z) แล้ววาดจริงตอน `_DISPLAY` |
| `_CAMERA (x, y, z) TO (tx, ty, tz) [, fov!]` | กล้องแบบ perspective (มุมมองแนวตั้งเป็นองศา ค่าเริ่มต้น 60) |
| `_CAMERAORTHO (x, y, z) TO (tx, ty, tz), height!` | กล้องแบบ orthographic |
| `_CAMERARANGE near!, far!` | ระยะตัดภาพ (ค่าเริ่มต้น 0.1, 1000) |
| `_AMBIENT colour~& [, intensity!]` | แสงโดยรอบ |
| `_LIGHT(kind&, x, y, z, colour~&, intensity! [, range!])` | แหล่งแสง: kind 1 = ทิศทาง (x, y, z คือทิศ), 2 = จุด (x, y, z คือตำแหน่ง) |
| `_LIGHTON l&, on&` · `_FREELIGHT l&` | เปิดปิด (เปิดพร้อมกันได้สูงสุด 8 ดวง), คืนหน่วยความจำ |
| `_CLEAR3D [colour~&]` | ล้างสีและ depth ของชั้น 3 มิติ |
| `_GPU3D` | คืน -1 เมื่อวาด 3 มิติได้ |
| `_SHADER(wgsl$)` · `_SHADERERROR$` | คอมไพล์ฟังก์ชัน WGSL `fn shade(i: ShadeIn) -> vec4<f32>` คืน -1 เมื่อคอมไพล์ไม่ผ่าน และ `_SHADERERROR$` บอกบรรทัด คอลัมน์ และเหตุผล |
| `_SHADERPARAM sh&, index&, value!` · `_MESHSHADER m&, sh&` · `_FREESHADER sh&` | ตั้งพารามิเตอร์ 1 ใน 16 ตัว, วาด mesh ด้วย shader นั้น (0 = แสงปกติ), คืนหน่วยความจำ |
