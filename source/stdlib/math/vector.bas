' ============================================================================
' QBNex Standard Library - Math: Vector Operations
' ============================================================================
' Vec2 and Vec3 with full operator suite
' ============================================================================

TYPE QBNex_Vec2
    X AS DOUBLE
    Y AS DOUBLE
END TYPE

TYPE QBNex_Vec3
    X AS DOUBLE
    Y AS DOUBLE
    Z AS DOUBLE
END TYPE

CONST VEC_EPSILON = 0.000001

' ============================================================================
' Vec2 Functions
' ============================================================================

SUB Vec2_Set (v AS QBNex_Vec2, x AS DOUBLE, y AS DOUBLE)
    v.X = x: v.Y = y
END SUB

SUB Vec2_Add (result AS QBNex_Vec2, a AS QBNex_Vec2, b AS QBNex_Vec2)
    result.X = a.X + b.X
    result.Y = a.Y + b.Y
END SUB

SUB Vec2_Sub (result AS QBNex_Vec2, a AS QBNex_Vec2, b AS QBNex_Vec2)
    result.X = a.X - b.X
    result.Y = a.Y - b.Y
END SUB

SUB Vec2_Scale (result AS QBNex_Vec2, v AS QBNex_Vec2, scalar AS DOUBLE)
    result.X = v.X * scalar
    result.Y = v.Y * scalar
END SUB

FUNCTION Vec2_Dot# (a AS QBNex_Vec2, b AS QBNex_Vec2)
    Vec2_Dot = a.X * b.X + a.Y * b.Y
END FUNCTION

FUNCTION Vec2_Length# (v AS QBNex_Vec2)
    Vec2_Length = SQR(v.X * v.X + v.Y * v.Y)
END FUNCTION

FUNCTION Vec2_LengthSq# (v AS QBNex_Vec2)
    Vec2_LengthSq = v.X * v.X + v.Y * v.Y
END FUNCTION

SUB Vec2_Normalize (result AS QBNex_Vec2, v AS QBNex_Vec2)
    DIM LEN AS DOUBLE
    LEN = Vec2_Length(v)
    IF LEN > VEC_EPSILON THEN
        result.X = v.X / LEN
        result.Y = v.Y / LEN
    ELSE
        result.X = 0: result.Y = 0
    END IF
END SUB

FUNCTION Vec2_Distance# (a AS QBNex_Vec2, b AS QBNex_Vec2)
    DIM dx AS DOUBLE, dy AS DOUBLE
    dx = b.X - a.X
    dy = b.Y - a.Y
    Vec2_Distance = SQR(dx * dx + dy * dy)
END FUNCTION

SUB Vec2_Lerp (result AS QBNex_Vec2, a AS QBNex_Vec2, b AS QBNex_Vec2, t AS DOUBLE)
    result.X = a.X + (b.X - a.X) * t
    result.Y = a.Y + (b.Y - a.Y) * t
END SUB

FUNCTION Vec2_Equals& (a AS QBNex_Vec2, b AS QBNex_Vec2)
    Vec2_Equals = (ABS(a.X - b.X) < VEC_EPSILON AND ABS(a.Y - b.Y) < VEC_EPSILON)
END FUNCTION

' ============================================================================
' Vec3 Functions
' ============================================================================

SUB Vec3_Set (v AS QBNex_Vec3, x AS DOUBLE, y AS DOUBLE, z AS DOUBLE)
    v.X = x: v.Y = y: v.Z = z
END SUB

SUB Vec3_Add (result AS QBNex_Vec3, a AS QBNex_Vec3, b AS QBNex_Vec3)
    result.X = a.X + b.X
    result.Y = a.Y + b.Y
    result.Z = a.Z + b.Z
END SUB

SUB Vec3_Sub (result AS QBNex_Vec3, a AS QBNex_Vec3, b AS QBNex_Vec3)
    result.X = a.X - b.X
    result.Y = a.Y - b.Y
    result.Z = a.Z - b.Z
END SUB

SUB Vec3_Scale (result AS QBNex_Vec3, v AS QBNex_Vec3, scalar AS DOUBLE)
    result.X = v.X * scalar
    result.Y = v.Y * scalar
    result.Z = v.Z * scalar
END SUB

FUNCTION Vec3_Dot# (a AS QBNex_Vec3, b AS QBNex_Vec3)
    Vec3_Dot = a.X * b.X + a.Y * b.Y + a.Z * b.Z
END FUNCTION

SUB Vec3_Cross (result AS QBNex_Vec3, a AS QBNex_Vec3, b AS QBNex_Vec3)
    result.X = a.Y * b.Z - a.Z * b.Y
    result.Y = a.Z * b.X - a.X * b.Z
    result.Z = a.X * b.Y - a.Y * b.X
END SUB

FUNCTION Vec3_Length# (v AS QBNex_Vec3)
    Vec3_Length = SQR(v.X * v.X + v.Y * v.Y + v.Z * v.Z)
END FUNCTION

FUNCTION Vec3_LengthSq# (v AS QBNex_Vec3)
    Vec3_LengthSq = v.X * v.X + v.Y * v.Y + v.Z * v.Z
END FUNCTION

SUB Vec3_Normalize (result AS QBNex_Vec3, v AS QBNex_Vec3)
    DIM LEN AS DOUBLE
    LEN = Vec3_Length(v)
    IF LEN > VEC_EPSILON THEN
        result.X = v.X / LEN
        result.Y = v.Y / LEN
        result.Z = v.Z / LEN
    ELSE
        result.X = 0: result.Y = 0: result.Z = 0
    END IF
END SUB

FUNCTION Vec3_Distance# (a AS QBNex_Vec3, b AS QBNex_Vec3)
    DIM dx AS DOUBLE, dy AS DOUBLE, dz AS DOUBLE
    dx = b.X - a.X
    dy = b.Y - a.Y
    dz = b.Z - a.Z
    Vec3_Distance = SQR(dx * dx + dy * dy + dz * dz)
END FUNCTION

SUB Vec3_Lerp (result AS QBNex_Vec3, a AS QBNex_Vec3, b AS QBNex_Vec3, t AS DOUBLE)
    result.X = a.X + (b.X - a.X) * t
    result.Y = a.Y + (b.Y - a.Y) * t
    result.Z = a.Z + (b.Z - a.Z) * t
END SUB

SUB Vec3_Reflect (result AS QBNex_Vec3, v AS QBNex_Vec3, normal AS QBNex_Vec3)
    DIM dot2 AS DOUBLE
    dot2 = 2 * Vec3_Dot(v, normal)
    result.X = v.X - dot2 * normal.X
    result.Y = v.Y - dot2 * normal.Y
    result.Z = v.Z - dot2 * normal.Z
END SUB

FUNCTION Vec3_Equals& (a AS QBNex_Vec3, b AS QBNex_Vec3)
    Vec3_Equals = (ABS(a.X - b.X) < VEC_EPSILON AND _
    ABS(a.Y - b.Y) < VEC_EPSILON AND _
    ABS(a.Z - b.Z) < VEC_EPSILON)
END FUNCTION
