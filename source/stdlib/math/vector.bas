' =============================================================================
' QBNex Math Library — 2D/3D Vector Mathematics — vector.bas
' =============================================================================
'
' Provides Vec2 and Vec3 TYPEs plus operator-style functions.
'
' Usage:
'
'   '$INCLUDE:'stdlib/math/vector.bas'
'
'   DIM a AS Vec3, b AS Vec3
'   Vec3_Set a, 1.0, 2.0, 3.0
'   Vec3_Set b, 4.0, 5.0, 6.0
'
'   DIM c AS Vec3
'   Vec3_Add c, a, b
'   PRINT Vec3_Str$(c)          ' (5.00, 7.00, 9.00)
'
'   PRINT Vec3_Dot(a, b)        ' 32.0
'   PRINT Vec3_Length(a)        ' 3.742...
'
'   DIM n AS Vec3
'   Vec3_Normalize n, a
'   PRINT Vec3_Length(n)        ' ~ 1.0
'
' =============================================================================

' ---------------------------------------------------------------------------
' Vec2 — 2-component vector
' ---------------------------------------------------------------------------
TYPE Vec2
    x AS SINGLE
    y AS SINGLE
END TYPE

SUB Vec2_Set (v AS Vec2, x AS SINGLE, y AS SINGLE)
    v.x = x: v.y = y
END SUB

SUB Vec2_Zero (v AS Vec2)
    v.x = 0: v.y = 0
END SUB

SUB Vec2_Add (out AS Vec2, a AS Vec2, b AS Vec2)
    out.x = a.x + b.x: out.y = a.y + b.y
END SUB

SUB Vec2_Sub (out AS Vec2, a AS Vec2, b AS Vec2)
    out.x = a.x - b.x: out.y = a.y - b.y
END SUB

SUB Vec2_Scale (out AS Vec2, v AS Vec2, s AS SINGLE)
    out.x = v.x * s: out.y = v.y * s
END SUB

FUNCTION Vec2_Dot (a AS Vec2, b AS Vec2)
    Vec2_Dot = a.x * b.x + a.y * b.y
END FUNCTION

FUNCTION Vec2_Length (v AS Vec2)
    Vec2_Length = SQR(v.x * v.x + v.y * v.y)
END FUNCTION

FUNCTION Vec2_LengthSq (v AS Vec2)
    Vec2_LengthSq = v.x * v.x + v.y * v.y
END FUNCTION

FUNCTION Vec2_Distance (a AS Vec2, b AS Vec2)
    DIM dx AS SINGLE, dy AS SINGLE
    dx = a.x - b.x: dy = a.y - b.y
    Vec2_Distance = SQR(dx * dx + dy * dy)
END FUNCTION

SUB Vec2_Normalize (out AS Vec2, v AS Vec2)
    DIM len AS SINGLE
    len = Vec2_Length(v)
    IF len = 0 THEN Vec2_Zero out: EXIT SUB
    out.x = v.x / len: out.y = v.y / len
END SUB

FUNCTION Vec2_Str$ (v AS Vec2)
    Vec2_Str$ = "(" + _TRIM$(STR$(v.x)) + ", " + _TRIM$(STR$(v.y)) + ")"
END FUNCTION

' ---------------------------------------------------------------------------
' Vec3 — 3-component vector
' ---------------------------------------------------------------------------
TYPE Vec3
    x AS SINGLE
    y AS SINGLE
    z AS SINGLE
END TYPE

SUB Vec3_Set (v AS Vec3, x AS SINGLE, y AS SINGLE, z AS SINGLE)
    v.x = x: v.y = y: v.z = z
END SUB

SUB Vec3_Zero (v AS Vec3)
    v.x = 0: v.y = 0: v.z = 0
END SUB

SUB Vec3_Copy (dest AS Vec3, src AS Vec3)
    dest.x = src.x: dest.y = src.y: dest.z = src.z
END SUB

SUB Vec3_Add (out AS Vec3, a AS Vec3, b AS Vec3)
    out.x = a.x + b.x: out.y = a.y + b.y: out.z = a.z + b.z
END SUB

SUB Vec3_Sub (out AS Vec3, a AS Vec3, b AS Vec3)
    out.x = a.x - b.x: out.y = a.y - b.y: out.z = a.z - b.z
END SUB

SUB Vec3_Scale (out AS Vec3, v AS Vec3, s AS SINGLE)
    out.x = v.x * s: out.y = v.y * s: out.z = v.z * s
END SUB

SUB Vec3_Negate (out AS Vec3, v AS Vec3)
    out.x = -v.x: out.y = -v.y: out.z = -v.z
END SUB

FUNCTION Vec3_Dot (a AS Vec3, b AS Vec3)
    Vec3_Dot = a.x * b.x + a.y * b.y + a.z * b.z
END FUNCTION

SUB Vec3_Cross (out AS Vec3, a AS Vec3, b AS Vec3)
    out.x = a.y * b.z - a.z * b.y
    out.y = a.z * b.x - a.x * b.z
    out.z = a.x * b.y - a.y * b.x
END SUB

FUNCTION Vec3_Length (v AS Vec3)
    Vec3_Length = SQR(v.x * v.x + v.y * v.y + v.z * v.z)
END FUNCTION

FUNCTION Vec3_LengthSq (v AS Vec3)
    Vec3_LengthSq = v.x * v.x + v.y * v.y + v.z * v.z
END FUNCTION

FUNCTION Vec3_Distance (a AS Vec3, b AS Vec3)
    DIM dx AS SINGLE, dy AS SINGLE, dz AS SINGLE
    dx = a.x - b.x: dy = a.y - b.y: dz = a.z - b.z
    Vec3_Distance = SQR(dx * dx + dy * dy + dz * dz)
END FUNCTION

SUB Vec3_Normalize (out AS Vec3, v AS Vec3)
    DIM len AS SINGLE
    len = Vec3_Length(v)
    IF len = 0 THEN Vec3_Zero out: EXIT SUB
    out.x = v.x / len: out.y = v.y / len: out.z = v.z / len
END SUB

SUB Vec3_Lerp (out AS Vec3, a AS Vec3, b AS Vec3, t AS SINGLE)
    out.x = a.x + (b.x - a.x) * t
    out.y = a.y + (b.y - a.y) * t
    out.z = a.z + (b.z - a.z) * t
END SUB

FUNCTION Vec3_Str$ (v AS Vec3)
    Vec3_Str$ = "(" + _TRIM$(STR$(v.x)) + ", " + _
                      _TRIM$(STR$(v.y)) + ", " + _
                      _TRIM$(STR$(v.z)) + ")"
END FUNCTION

FUNCTION Vec3_Equals& (a AS Vec3, b AS Vec3, epsilon AS SINGLE)
    IF epsilon = 0 THEN epsilon = 0.00001
    Vec3_Equals& = (ABS(a.x - b.x) <= epsilon AND _
                    ABS(a.y - b.y) <= epsilon AND _
                    ABS(a.z - b.z) <= epsilon)
END FUNCTION

' Reflect vector v around normal n (both should be normalised)
SUB Vec3_Reflect (out AS Vec3, v AS Vec3, n AS Vec3)
    DIM d AS SINGLE
    d = 2.0 * Vec3_Dot(v, n)
    out.x = v.x - d * n.x
    out.y = v.y - d * n.y
    out.z = v.z - d * n.z
END SUB
