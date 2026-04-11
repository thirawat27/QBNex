' ============================================================================
' QBNex Standard Library - Math: Statistics
' ============================================================================
' Statistical functions over DOUBLE arrays
' ============================================================================

' ============================================================================
' FUNCTION: Stats_Sum
' Sum of values in range
' ============================================================================
FUNCTION Stats_Sum# (arr() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    DIM sum AS DOUBLE
    DIM i AS LONG
    
    sum = 0
    FOR i = fromIdx TO toIdx
        sum = sum + arr(i)
    NEXT i
    
    Stats_Sum = sum
END FUNCTION

' ============================================================================
' FUNCTION: Stats_Mean
' Mean (average) of values
' ============================================================================
FUNCTION Stats_Mean# (arr() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    DIM count AS LONG
    count = toIdx - fromIdx + 1
    
    IF count = 0 THEN
        Stats_Mean = 0
    ELSE
        Stats_Mean = Stats_Sum(arr(), fromIdx, toIdx) / count
    END IF
END FUNCTION

' ============================================================================
' FUNCTION: Stats_Min
' Minimum value
' ============================================================================
FUNCTION Stats_Min# (arr() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    DIM minVal AS DOUBLE
    DIM i AS LONG
    
    minVal = arr(fromIdx)
    FOR i = fromIdx + 1 TO toIdx
        IF arr(i) < minVal THEN minVal = arr(i)
    NEXT i
    
    Stats_Min = minVal
END FUNCTION

' ============================================================================
' FUNCTION: Stats_Max
' Maximum value
' ============================================================================
FUNCTION Stats_Max# (arr() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    DIM maxVal AS DOUBLE
    DIM i AS LONG
    
    maxVal = arr(fromIdx)
    FOR i = fromIdx + 1 TO toIdx
        IF arr(i) > maxVal THEN maxVal = arr(i)
    NEXT i
    
    Stats_Max = maxVal
END FUNCTION

' ============================================================================
' FUNCTION: Stats_Range
' Range (max - min)
' ============================================================================
FUNCTION Stats_Range# (arr() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    Stats_Range = Stats_Max(arr(), fromIdx, toIdx) - Stats_Min(arr(), fromIdx, toIdx)
END FUNCTION

' ============================================================================
' FUNCTION: Stats_Variance
' Variance of values
' ============================================================================
FUNCTION Stats_Variance# (arr() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    DIM mean AS DOUBLE
    DIM sum AS DOUBLE
    DIM count AS LONG
    DIM i AS LONG
    DIM diff AS DOUBLE
    
    count = toIdx - fromIdx + 1
    IF count <= 1 THEN
        Stats_Variance = 0
        EXIT FUNCTION
    END IF
    
    mean = Stats_Mean(arr(), fromIdx, toIdx)
    sum = 0
    
    FOR i = fromIdx TO toIdx
        diff = arr(i) - mean
        sum = sum + diff * diff
    NEXT i
    
    Stats_Variance = sum / (count - 1)
END FUNCTION

' ============================================================================
' FUNCTION: Stats_StdDev
' Standard deviation
' ============================================================================
FUNCTION Stats_StdDev# (arr() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    Stats_StdDev = SQR(Stats_Variance(arr(), fromIdx, toIdx))
END FUNCTION

' ============================================================================
' FUNCTION: Stats_Median
' Median value (sorts a copy)
' ============================================================================
FUNCTION Stats_Median# (arr() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG)
    DIM count AS LONG
    DIM mid AS LONG
    DIM i AS LONG, j AS LONG
    DIM temp AS DOUBLE
    DIM sorted() AS DOUBLE
    
    count = toIdx - fromIdx + 1
    REDIM sorted(1 TO count) AS DOUBLE
    
    ' Copy to temp array
    FOR i = fromIdx TO toIdx
        sorted(i - fromIdx + 1) = arr(i)
    NEXT i
    
    ' Bubble sort
    FOR i = 1 TO count - 1
        FOR j = 1 TO count - i
            IF sorted(j) > sorted(j + 1) THEN
                temp = sorted(j)
                sorted(j) = sorted(j + 1)
                sorted(j + 1) = temp
            END IF
        NEXT j
    NEXT i
    
    ' Get median
    mid = count \ 2
    IF count MOD 2 = 1 THEN
        Stats_Median = sorted(mid + 1)
    ELSE
        Stats_Median = (sorted(mid) + sorted(mid + 1)) / 2
    END IF
END FUNCTION

' ============================================================================
' SUB: Stats_LinearRegression
' Linear regression (y = slope * x + intercept)
' ============================================================================
SUB Stats_LinearRegression (xArr() AS DOUBLE, yArr() AS DOUBLE, fromIdx AS LONG, toIdx AS LONG, slope AS DOUBLE, intercept AS DOUBLE)
    DIM count AS LONG
    DIM sumX AS DOUBLE, sumY AS DOUBLE
    DIM sumXY AS DOUBLE, sumXX AS DOUBLE
    DIM meanX AS DOUBLE, meanY AS DOUBLE
    DIM i AS LONG
    
    count = toIdx - fromIdx + 1
    IF count < 2 THEN
        slope = 0
        intercept = 0
        EXIT SUB
    END IF
    
    sumX = 0: sumY = 0: sumXY = 0: sumXX = 0
    
    FOR i = fromIdx TO toIdx
        sumX = sumX + xArr(i)
        sumY = sumY + yArr(i)
        sumXY = sumXY + xArr(i) * yArr(i)
        sumXX = sumXX + xArr(i) * xArr(i)
    NEXT i
    
    meanX = sumX / count
    meanY = sumY / count
    
    slope = (sumXY - count * meanX * meanY) / (sumXX - count * meanX * meanX)
    intercept = meanY - slope * meanX
END SUB
