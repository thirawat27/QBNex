' =============================================================================
' QBNex Math Library — 4×4 Matrix — matrix.bas
' =============================================================================
'
' Row-major 4×4 floating-point matrix for 3D transforms.
' Compatible with OpenGL column-major order via Mat4_Transpose.
'
' Usage:
'
'   '$INCLUDE:'stdlib/math/matrix.bas'
'
'   DIM m AS Mat4, v AS Vec3
'   Mat4_Identity m
'   Mat4_Translate m, 1.0, 2.0, 3.0
'
'   DIM rot AS Mat4
'   Mat4_RotateY rot, 45.0   ' degrees
'
'   DIM result AS Mat4
'   Mat4_Mul result, m, rot
'
' =============================================================================

'$INCLUDE:'stdlib/math/vector.bas'

TYPE Mat4
    m(15) AS SINGLE   ' 4×4 stored row-major: m(row*4 + col)
END TYPE

CONST QBNEX_DEG2RAD = 0.01745329251994329576923690768489#

' m(row, col) helper macros — use inline for performance
' M(mat, r, c) = mat.m(r * 4 + c)

SUB Mat4_Zero (mat AS Mat4)
    DIM i AS INTEGER
    FOR i = 0 TO 15: mat.m(i) = 0: NEXT i
END SUB

SUB Mat4_Identity (mat AS Mat4)
    Mat4_Zero mat
    mat.m(0) = 1: mat.m(5) = 1: mat.m(10) = 1: mat.m(15) = 1
END SUB

SUB Mat4_Copy (dest AS Mat4, src AS Mat4)
    DIM i AS INTEGER
    FOR i = 0 TO 15: dest.m(i) = src.m(i): NEXT i
END SUB

' Matrix multiplication: out = a * b
SUB Mat4_Mul (out AS Mat4, a AS Mat4, b AS Mat4)
    DIM r AS INTEGER, c AS INTEGER, k AS INTEGER, sum AS SINGLE
    Mat4_Zero out
    FOR r = 0 TO 3
        FOR c = 0 TO 3
            sum = 0
            FOR k = 0 TO 3
                sum = sum + a.m(r * 4 + k) * b.m(k * 4 + c)
            NEXT k
            out.m(r * 4 + c) = sum
        NEXT c
    NEXT r
END SUB

' Transpose in place
SUB Mat4_Transpose (mat AS Mat4)
    DIM tmp AS SINGLE, r AS INTEGER, c AS INTEGER
    FOR r = 0 TO 3
        FOR c = r + 1 TO 3
            tmp          = mat.m(r * 4 + c)
            mat.m(r * 4 + c) = mat.m(c * 4 + r)
            mat.m(c * 4 + r) = tmp
        NEXT c
    NEXT r
END SUB

' Apply translation to an existing matrix
SUB Mat4_Translate (mat AS Mat4, tx AS SINGLE, ty AS SINGLE, tz AS SINGLE)
    DIM t AS Mat4
    Mat4_Identity t
    t.m(3)  = tx
    t.m(7)  = ty
    t.m(11) = tz
    DIM tmp AS Mat4
    Mat4_Mul tmp, mat, t
    Mat4_Copy mat, tmp
END SUB

' Apply uniform scale
SUB Mat4_Scale (mat AS Mat4, sx AS SINGLE, sy AS SINGLE, sz AS SINGLE)
    DIM s AS Mat4
    Mat4_Identity s
    s.m(0) = sx: s.m(5) = sy: s.m(10) = sz
    DIM tmp AS Mat4
    Mat4_Mul tmp, mat, s
    Mat4_Copy mat, tmp
END SUB

' Rotation around X axis (degrees)
SUB Mat4_RotateX (mat AS Mat4, deg AS SINGLE)
    DIM r AS Mat4, rad AS SINGLE, c AS SINGLE, s AS SINGLE
    rad = deg * QBNEX_DEG2RAD
    c = COS(rad): s = SIN(rad)
    Mat4_Identity r
    r.m(5) = c:  r.m(6)  = -s
    r.m(9) = s:  r.m(10) = c
    DIM tmp AS Mat4
    Mat4_Mul tmp, mat, r
    Mat4_Copy mat, tmp
END SUB

' Rotation around Y axis (degrees)
SUB Mat4_RotateY (mat AS Mat4, deg AS SINGLE)
    DIM r AS Mat4, rad AS SINGLE, c AS SINGLE, s AS SINGLE
    rad = deg * QBNEX_DEG2RAD
    c = COS(rad): s = SIN(rad)
    Mat4_Identity r
    r.m(0) = c:   r.m(2)  = s
    r.m(8) = -s:  r.m(10) = c
    DIM tmp AS Mat4
    Mat4_Mul tmp, mat, r
    Mat4_Copy mat, tmp
END SUB

' Rotation around Z axis (degrees)
SUB Mat4_RotateZ (mat AS Mat4, deg AS SINGLE)
    DIM r AS Mat4, rad AS SINGLE, c AS SINGLE, s AS SINGLE
    rad = deg * QBNEX_DEG2RAD
    c = COS(rad): s = SIN(rad)
    Mat4_Identity r
    r.m(0) = c:  r.m(1) = -s
    r.m(4) = s:  r.m(5) = c
    DIM tmp AS Mat4
    Mat4_Mul tmp, mat, r
    Mat4_Copy mat, tmp
END SUB

' Transform Vec3 by Mat4 (homogeneous division)
SUB Mat4_TransformVec3 (out AS Vec3, mat AS Mat4, v AS Vec3)
    DIM x AS SINGLE, y AS SINGLE, z AS SINGLE, w AS SINGLE
    x = mat.m(0)  * v.x + mat.m(1)  * v.y + mat.m(2)  * v.z + mat.m(3)
    y = mat.m(4)  * v.x + mat.m(5)  * v.y + mat.m(6)  * v.z + mat.m(7)
    z = mat.m(8)  * v.x + mat.m(9)  * v.y + mat.m(10) * v.z + mat.m(11)
    w = mat.m(12) * v.x + mat.m(13) * v.y + mat.m(14) * v.z + mat.m(15)
    IF w = 0 THEN w = 1
    out.x = x / w: out.y = y / w: out.z = z / w
END SUB

' Build a perspective projection matrix
SUB Mat4_Perspective (mat AS Mat4, fovDeg AS SINGLE, aspect AS SINGLE, _
                      nearZ AS SINGLE, farZ AS SINGLE)
    DIM f AS SINGLE
    Mat4_Zero mat
    f = 1.0 / TAN(fovDeg * QBNEX_DEG2RAD / 2.0)
    mat.m(0)  = f / aspect
    mat.m(5)  = f
    mat.m(10) = (farZ + nearZ) / (nearZ - farZ)
    mat.m(11) = (2.0 * farZ * nearZ) / (nearZ - farZ)
    mat.m(14) = -1.0
END SUB

' Build a look-at view matrix
SUB Mat4_LookAt (mat AS Mat4, eye AS Vec3, center AS Vec3, up AS Vec3)
    DIM f AS Vec3, r AS Vec3, u2 AS Vec3
    DIM sub3 AS Vec3
    Vec3_Sub   sub3, center, eye
    Vec3_Normalize f, sub3
    Vec3_Cross r, f, up
    Vec3_Normalize r, r
    Vec3_Cross u2, r, f

    Mat4_Zero mat
    mat.m(0)  =  r.x:  mat.m(1)  =  r.y:  mat.m(2)  =  r.z:  mat.m(3)  = -Vec3_Dot(r, eye)
    mat.m(4)  =  u2.x: mat.m(5)  =  u2.y: mat.m(6)  =  u2.z: mat.m(7)  = -Vec3_Dot(u2, eye)
    mat.m(8)  = -f.x:  mat.m(9)  = -f.y:  mat.m(10) = -f.z:  mat.m(11) =  Vec3_Dot(f, eye)
    mat.m(15) = 1.0
END SUB
