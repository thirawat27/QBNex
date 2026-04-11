' =============================================================================
' QBNex Math Library — Descriptive Statistics — stats.bas
' =============================================================================
'
' Operates on a DOUBLE array passed by reference.
'
' Usage:
'
'   '$INCLUDE:'stdlib/math/stats.bas'
'
'   DIM data(5) AS DOUBLE
'   data(1)=4 : data(2)=8 : data(3)=15 : data(4)=16 : data(5)=23
'
'   PRINT Stats_Sum(data(), 1, 5)       ' 66
'   PRINT Stats_Mean(data(), 1, 5)      ' 13.2
'   PRINT Stats_Median(data(), 1, 5)    ' 15
'   PRINT Stats_StdDev(data(), 1, 5)    ' 6.499...
'   PRINT Stats_Min(data(), 1, 5)       ' 4
'   PRINT Stats_Max(data(), 1, 5)       ' 23
'
' =============================================================================

FUNCTION Stats_Sum# (data() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    DIM i AS LONG, s AS DOUBLE
    s = 0
    FOR i = fromIdx TO toIdx: s = s + data(i): NEXT i
    Stats_Sum# = s
END FUNCTION

FUNCTION Stats_Mean# (data() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    DIM n AS LONG
    n = toIdx - fromIdx + 1
    IF n = 0 THEN Stats_Mean# = 0: EXIT FUNCTION
    Stats_Mean# = Stats_Sum#(data(), fromIdx, toIdx) / n
END FUNCTION

FUNCTION Stats_Min# (data() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    DIM i AS LONG, mn AS DOUBLE
    mn = data(fromIdx)
    FOR i = fromIdx + 1 TO toIdx
        IF data(i) < mn THEN mn = data(i)
    NEXT i
    Stats_Min# = mn
END FUNCTION

FUNCTION Stats_Max# (data() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    DIM i AS LONG, mx AS DOUBLE
    mx = data(fromIdx)
    FOR i = fromIdx + 1 TO toIdx
        IF data(i) > mx THEN mx = data(i)
    NEXT i
    Stats_Max# = mx
END FUNCTION

FUNCTION Stats_Variance# (data() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    DIM i AS LONG, n AS LONG, mean AS DOUBLE, vsum AS DOUBLE, diff AS DOUBLE
    n = toIdx - fromIdx + 1
    IF n < 2 THEN Stats_Variance# = 0: EXIT FUNCTION
    mean = Stats_Mean#(data(), fromIdx, toIdx)
    vsum = 0
    FOR i = fromIdx TO toIdx
        diff = data(i) - mean
        vsum = vsum + diff * diff
    NEXT i
    Stats_Variance# = vsum / (n - 1)  ' sample variance (Bessel's correction)
END FUNCTION

FUNCTION Stats_StdDev# (data() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    Stats_StdDev# = SQR(Stats_Variance#(data(), fromIdx, toIdx))
END FUNCTION

FUNCTION Stats_Median# (data() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    ' Sort a local copy then pick middle
    DIM n AS LONG, i AS LONG, j AS LONG
    n = toIdx - fromIdx + 1
    IF n = 0 THEN Stats_Median# = 0: EXIT FUNCTION
    REDIM tmp(1 TO n) AS DOUBLE
    FOR i = 1 TO n: tmp(i) = data(fromIdx + i - 1): NEXT i
    ' Insertion sort
    FOR i = 2 TO n
        DIM key AS DOUBLE
        key = tmp(i): j = i - 1
        DO WHILE j >= 1 AND tmp(j) > key
            tmp(j + 1) = tmp(j): j = j - 1
        LOOP
        tmp(j + 1) = key
    NEXT i
    IF n MOD 2 = 1 THEN
        Stats_Median# = tmp((n + 1) \ 2)
    ELSE
        Stats_Median# = (tmp(n \ 2) + tmp(n \ 2 + 1)) / 2.0
    END IF
END FUNCTION

FUNCTION Stats_Range# (data() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    Stats_Range# = Stats_Max#(data(), fromIdx, toIdx) - _
                   Stats_Min#(data(), fromIdx, toIdx)
END FUNCTION

' Linear regression: returns slope and intercept for y = m*x + b
SUB Stats_LinearRegression (data_x() AS DOUBLE, data_y() AS DOUBLE, _
                             fromIdx AS LONG, toIdx AS LONG, _
                             slope AS DOUBLE, intercept AS DOUBLE)
    DIM n AS LONG, sumX AS DOUBLE, sumY AS DOUBLE
    DIM sumXY AS DOUBLE, sumXX AS DOUBLE, i AS LONG
    n = toIdx - fromIdx + 1
    IF n < 2 THEN slope = 0: intercept = 0: EXIT SUB
    sumX = 0: sumY = 0: sumXY = 0: sumXX = 0
    FOR i = fromIdx TO toIdx
        sumX  = sumX  + data_x(i)
        sumY  = sumY  + data_y(i)
        sumXY = sumXY + data_x(i) * data_y(i)
        sumXX = sumXX + data_x(i) * data_x(i)
    NEXT i
    DIM denom AS DOUBLE
    denom = n * sumXX - sumX * sumX
    IF denom = 0 THEN slope = 0: intercept = sumY / n: EXIT SUB
    slope     = (n * sumXY - sumX * sumY) / denom
    intercept = (sumY - slope * sumX) / n
END SUB
