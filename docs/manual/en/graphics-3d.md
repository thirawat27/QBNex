# Graphics and 3D

[Manual index](../README.md) · [ภาษาไทย](../th/graphics-3d.md)

## 2D graphics (Available)

All QBasic and QB64 graphics run on the Rust runtime: `SCREEN` 0-13 and
`_NEWIMAGE` 32-bit pages, `PSET`, `LINE`, `CIRCLE`, `PAINT`, `DRAW`,
`VIEW`/`WINDOW`, `GET`/`PUT`, `PCOPY`, `_PUTIMAGE`, `_LOADIMAGE` (the formats
QB64 reads: PNG, JPEG, BMP, PCX, ...), TrueType fonts with `_LOADFONT`, palettes, alpha
blending, `_DISPLAY`, `_LIMIT`, keyboard, mouse and gamepads. The window
uses winit + softbuffer; no OpenGL, FreeGLUT or FreeType is needed. The
results are pixel-identical to the old C++ runtime in the conformance
tests.

## 3D triangles: `_MAPTRIANGLE` (Available)

QB64's 3D form of `_MAPTRIANGLE` works:

```basic
SCREEN _NEWIMAGE(640, 480, 32)
tex& = _LOADIMAGE("brick.png", 32)
_DEPTHBUFFER ON
_MAPTRIANGLE _CLOCKWISE (0, 0)-(63, 0)-(0, 63), tex& TO(-1, 1, -3)-(1, 1, -3)-(-1, -1, -3), , _SMOOTH
_DISPLAY
```

Projection, depth buffer (`_DEPTHBUFFER ON|LOCK|OFF`), culling
(`_CLOCKWISE`/`_ANTICLOCKWISE`), `_SMOOTH*` filtering and alpha follow
QB64. The default renderer is the software one.

**Preview:** a runtime built with the `gpu` feature presents frames and can
draw 3D triangles with wgpu (Vulkan, DX12, Metal). It falls back to the
software path when there is no adapter or `QB_GPU=off`. One known
difference: on the GPU the far plane (10000) clips, as legacy OpenGL did.

## High-level 3D: models, camera, lights (Preview)

The compiler knows these statements and functions; they draw on a runtime
built with GPU support (wgpu). Without a GPU every statement still runs
without an error, `_DRAW3D` draws nothing and `_GPU3D` returns 0. Design:
`docs/rust-port/r15-g4-3d-api.md`.

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

| Statement / function | Meaning |
| --- | --- |
| `_LOADMODEL(file$)` | Load glTF 2.0 (.gltf/.glb); -1 when it cannot be loaded. |
| `_MESHCUBE(size!)`, `_MESHPLANE(w!, d!)` | Primitives. |
| `_MESHBEGIN` · `_MESHVERTEX x!, y!, z! [, u!, v!]` · `_MESHTRIANGLE a&, b&, c&` · `_MESHEND` | Build a mesh; `_MESHEND` returns its handle (-1 without triangles). Vertices are numbered from 0. |
| `_MESHCOLOR m&, colour~&` · `_MESHTEXTURE m&, img&` · `_FREEMESH m&` | Material (texture from a 32-bit image), free. |
| `_DRAW3D m& [, x, y, z] [, rx, ry, rz] [, sx, sy, sz]` | Queue a draw (angles in degrees, order Y-X-Z); drawn at `_DISPLAY`. |
| `_CAMERA (x, y, z) TO (tx, ty, tz) [, fov!]` | Perspective camera (vertical field of view in degrees, default 60). |
| `_CAMERAORTHO (x, y, z) TO (tx, ty, tz), height!` | Orthographic camera. |
| `_CAMERARANGE near!, far!` | Clipping range (default 0.1, 1000). |
| `_AMBIENT colour~& [, intensity!]` | Ambient light. |
| `_LIGHT(kind&, x, y, z, colour~&, intensity! [, range!])` | Light: kind 1 = directional (x, y, z is the direction), 2 = point (x, y, z is the position). |
| `_LIGHTON l&, on&` · `_FREELIGHT l&` | Switch (at most 8 lights on), free. |
| `_CLEAR3D [colour~&]` | Clear colour and depth of the 3D layer. |
| `_GPU3D` | -1 when 3D can be drawn. |
