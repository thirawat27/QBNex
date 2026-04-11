' ============================================================================
' QBNex Standard Library - Math: Matrix Operations
' ============================================================================
' 4×4 matrix with transforms, perspective, look-at (row-major)
' ============================================================================

'$INCLUDE:'vector.bas'

TYPE QBNex_Mat4
    M(0 TO 3, 0 TO 3) AS DOUBLE
END TYPE

' ============================================================================
' SUB: Mat4_Identity
' Set matrix to identity
' ============================================================================
SUB Mat4_Identity (m AS QBNex_Mat4)
    DIM i AS LONG, j AS LONG
    FOR i = 0 TO 3
        FOR j = 0 TO 3
            IF i = j THEN
                m.M(i, j) = 1
            ELSE
                m.M(i, j) = 0
            END IF
        NEXT j
    NEXT i
END SUB

' ============================================================================
' SUB: Mat4_Zero
' Set all elements to zero
' ============================================================================
SUB Mat4_Zero (m AS QBNex_Mat4)
    DIM i AS LONG, j AS LONG
    FOR i = 0 TO 3
        FOR j = 0 TO 3
            m.M(i, j) = 0
        NEXT j
    NEXT i
END SUB

' ============================================================================
' SUB: Mat4_Copy
' Copy matrix
' ============================================================================
SUB Mat4_Copy (dest AS QBNex_Mat4, src AS QBNex_Mat4)
    DIM i AS LONG, j AS LONG
    FOR i = 0 TO 3
        FOR j = 0 TO 3
            dest.M(i, j) = src.M(i, j)
        NEXT j
    NEXT i
END SUB

' ============================================================================
' SUB: Mat4_Mul
' Multiply two matrices (result = a × b)
' ============================================================================
SUB Mat4_Mul (result AS QBNex_Mat4, a AS QBNex_Mat4, b AS QBNex_Mat4)
    DIM temp AS QBNex_Mat4
    DIM i AS LONG, j AS LONG, k AS LONG
    DIM sum AS DOUBLE
    
    FOR i = 0 TO 3
        FOR j = 0 TO 3
            sum = 0
            FOR k = 0 TO 3
                sum = sum + a.M(i, k) * b.M(k, j)
            NEXT k
            temp.M(i, j) = sum
        NEXT j
    NEXT i
    
    Mat4_Copy result, temp
END SUB

' ============================================================================
' SUB: Mat4_Translate
' Create translation matrix
' ============================================================================
SUB Mat4_Translate (m AS QBNex_Mat4, x AS DOUBLE, y AS DOUBLE, z AS DOUBLE)
    Mat4_Identity m
    m.M(0, 3) = x
    m.M(1, 3) = y
    m.M(2, 3) = z
END SUB

' ============================================================================
' SUB: Mat4_Scale
' Create scale matrix
' ============================================================================
SUB Mat4_Scale (m AS QBNex_Mat4, x AS DOUBLE, y AS DOUBLE, z AS DOUBLE)
    Mat4_Identity m
    m.M(0, 0) = x
    m.M(1, 1) = y
    m.M(2, 2) = z
END SUB

' ============================================================================
' SUB: Mat4_RotateX
' Create rotation matrix around X axis (angle in radians)
' ============================================================================
SUB Mat4_RotateX (m AS QBNex_Mat4, angle AS DOUBLE)
    DIM c AS DOUBLE, s AS DOUBLE
    c = COS(angle)
    s = SIN(angle)
    
    Mat4_Identity m
    m.M(1, 1) = c
    m.M(1, 2) = -s
    m.M(2, 1) = s
    m.M(2, 2) = c
END SUB

' ============================================================================
' SUB: Mat4_RotateY
' Create rotation matrix around Y axis (angle in radians)
' ============================================================================
SUB Mat4_RotateY (m AS QBNex_Mat4, angle AS DOUBLE)
    DIM c AS DOUBLE, s AS DOUBLE
    c = COS(angle)
    s = SIN(angle)
    
    Mat4_Identity m
    m.M(0, 0) = c
    m.M(0, 2) = s
    m.M(2, 0) = -s
    m.M(2, 2) = c
END SUB

' ============================================================================
' SUB: Mat4_RotateZ
' Create rotation matrix around Z axis (angle in radians)
' ============================================================================
SUB Mat4_RotateZ (m AS QBNex_Mat4, angle AS DOUBLE)
    DIM c AS DOUBLE, s AS DOUBLE
    c = COS(angle)
    s = SIN(angle)
    
    Mat4_Identity m
    m.M(0, 0) = c
    m.M(0, 1) = -s
    m.M(1, 0) = s
    m.M(1, 1) = c
END SUB

' ============================================================================
' SUB: Mat4_Perspective
' Create perspective projection matrix
' ============================================================================
SUB Mat4_Perspective (m AS QBNex_Mat4, fov AS DOUBLE, aspect AS DOUBLE, nearPlane AS DOUBLE, farPlane AS DOUBLE)
    DIM f AS DOUBLE
    f = 1 / TAN(fov / 2)
    
    Mat4_Zero m
    m.M(0, 0) = f / aspect
    m.M(1, 1) = f
    m.M(2, 2) = (farPlane + nearPlane) / (nearPlane - farPlane)
    m.M(2, 3) = (2 * farPlane * nearPlane) / (nearPlane - farPlane)
    m.M(3, 2) = -1
END SUB

' ============================================================================
' SUB: Mat4_TransformVec3
' Transform Vec3 by matrix (w=1 assumed)
' ============================================================================
SUB Mat4_TransformVec3 (result AS QBNex_Vec3, m AS QBNex_Mat4, v AS QBNex_Vec3)
    DIM x AS DOUBLE, y AS DOUBLE, z AS DOUBLE, w AS DOUBLE
    
    x = m.M(0, 0) * v.X + m.M(0, 1) * v.Y + m.M(0, 2) * v.Z + m.M(0, 3)
    y = m.M(1, 0) * v.X + m.M(1, 1) * v.Y + m.M(1, 2) * v.Z + m.M(1, 3)
    z = m.M(2, 0) * v.X + m.M(2, 1) * v.Y + m.M(2, 2) * v.Z + m.M(2, 3)
    w = m.M(3, 0) * v.X + m.M(3, 1) * v.Y + m.M(3, 2) * v.Z + m.M(3, 3)
    
    IF ABS(w) > 0.000001 THEN
        result.X = x / w
        result.Y = y / w
        result.Z = z / w
    ELSE
        result.X = x
        result.Y = y
        result.Z = z
    END IF
END SUB
