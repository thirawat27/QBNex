' ============================================================================
' QBNex Standard Library - Math: Numeric Helpers
' ============================================================================

FUNCTION Math_Min# (valueA AS DOUBLE, valueB AS DOUBLE)
    IF valueA <= valueB THEN Math_Min = valueA ELSE Math_Min = valueB
END FUNCTION

FUNCTION Math_Max# (valueA AS DOUBLE, valueB AS DOUBLE)
    IF valueA >= valueB THEN Math_Max = valueA ELSE Math_Max = valueB
END FUNCTION

FUNCTION Math_Clamp# (valueX AS DOUBLE, minValue AS DOUBLE, maxValue AS DOUBLE)
    IF valueX < minValue THEN
        Math_Clamp = minValue
    ELSEIF valueX > maxValue THEN
        Math_Clamp = maxValue
    ELSE
        Math_Clamp = valueX
    END IF
END FUNCTION

FUNCTION Math_Lerp# (valueA AS DOUBLE, valueB AS DOUBLE, factor AS DOUBLE)
    Math_Lerp = valueA + (valueB - valueA) * factor
END FUNCTION

FUNCTION Math_Deg2Rad# (degreesValue AS DOUBLE)
    Math_Deg2Rad = degreesValue * 3.141592653589793# / 180#
END FUNCTION

FUNCTION Math_Rad2Deg# (radiansValue AS DOUBLE)
    Math_Rad2Deg = radiansValue * 180# / 3.141592653589793#
END FUNCTION
