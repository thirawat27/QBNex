'===============================================================================
' QBNex Standard Library - math ('$IMPORT:'math')
'===============================================================================
'   Math_Min(a, b)  Math_Max(a, b)  Math_Clamp(x, low, high)
'   Math_Lerp(a, b, t)               a + (b - a) * t
'   Math_Radians(deg)  Math_Degrees(rad)  Math_Pi
'===============================================================================

' The smaller of a and b.
FUNCTION Math_Min# (a AS DOUBLE, b AS DOUBLE)
    IF a <= b THEN Math_Min# = a ELSE Math_Min# = b
END FUNCTION

' The larger of a and b.
FUNCTION Math_Max# (a AS DOUBLE, b AS DOUBLE)
    IF a >= b THEN Math_Max# = a ELSE Math_Max# = b
END FUNCTION

' x kept between low and high.
FUNCTION Math_Clamp# (x AS DOUBLE, low AS DOUBLE, high AS DOUBLE)
    IF x < low THEN
        Math_Clamp# = low
    ELSEIF x > high THEN
        Math_Clamp# = high
    ELSE
        Math_Clamp# = x
    END IF
END FUNCTION

' The value at t (0 to 1) of the way from a to b.
FUNCTION Math_Lerp# (a AS DOUBLE, b AS DOUBLE, t AS DOUBLE)
    Math_Lerp# = a + (b - a) * t
END FUNCTION

' 3.14159...
FUNCTION Math_Pi# ()
    Math_Pi# = 3.141592653589793#
END FUNCTION

' Degrees to radians.
FUNCTION Math_Radians# (degrees AS DOUBLE)
    Math_Radians# = degrees * 3.141592653589793# / 180#
END FUNCTION

' Radians to degrees.
FUNCTION Math_Degrees# (radians AS DOUBLE)
    Math_Degrees# = radians * 180# / 3.141592653589793#
END FUNCTION
