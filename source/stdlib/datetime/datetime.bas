' ============================================================================
' QBNex Standard Library - DateTime
' ============================================================================
' Date and time manipulation with arithmetic and formatting
' ============================================================================

TYPE QBNex_DateTime
    Year AS INTEGER
    Month AS INTEGER
    Day AS INTEGER
    Hour AS INTEGER
    Minute AS INTEGER
    Second AS INTEGER
    DOW AS INTEGER ' Day of week (0=Sun..6=Sat)
END TYPE

' ============================================================================
' SUB: DT_Now
' Fill DateTime with current date/time
' ============================================================================
SUB DT_Now (dt AS QBNex_DateTime)
    DIM d AS STRING, t AS STRING
    
    d = DATE$
    t = TIME$
    
    dt.Month = VAL(LEFT$(d, 2))
    dt.Day = VAL(MID$(d, 4, 2))
    dt.Year = VAL(RIGHT$(d, 4))
    
    dt.Hour = VAL(LEFT$(t, 2))
    dt.Minute = VAL(MID$(t, 4, 2))
    dt.Second = VAL(RIGHT$(t, 2))
    
    dt.DOW = DT_CalcDayOfWeek(dt.Year, dt.Month, dt.Day)
END SUB

' ============================================================================
' SUB: DT_Set
' Manually set DateTime
' ============================================================================
SUB DT_Set (dt AS QBNex_DateTime, year AS INTEGER, month AS INTEGER, day AS INTEGER, hour AS INTEGER, minute AS INTEGER, second AS INTEGER)
    dt.Year = year
    dt.Month = month
    dt.Day = day
    dt.Hour = hour
    dt.Minute = minute
    dt.Second = second
    dt.DOW = DT_CalcDayOfWeek(year, month, day)
END SUB

' ============================================================================
' FUNCTION: DT_CalcDayOfWeek
' Calculate day of week using Zeller's congruence
' ============================================================================
FUNCTION DT_CalcDayOfWeek& (year AS INTEGER, month AS INTEGER, day AS INTEGER)
    DIM y AS INTEGER, m AS INTEGER
    DIM q AS INTEGER, k AS INTEGER, j AS INTEGER
    DIM h AS INTEGER
    
    y = year
    m = month
    
    ' Adjust for January and February
    IF m < 3 THEN
        m = m + 12
        y = y - 1
    END IF
    
    q = day
    k = y MOD 100
    j = y \ 100
    
    h = (q + ((13 * (m + 1)) \ 5) + k + (k \ 4) + (j \ 4) - (2 * j)) MOD 7
    
    ' Convert to 0=Sun format
    DT_CalcDayOfWeek = (h + 6) MOD 7
END FUNCTION

' ============================================================================
' FUNCTION: DT_ToJulianDay
' Convert DateTime to Julian Day Number
' ============================================================================
FUNCTION DT_ToJulianDay& (dt AS QBNex_DateTime)
    DIM a AS LONG, y AS LONG, m AS LONG
    
    a = (14 - dt.Month) \ 12
    y = dt.Year + 4800 - a
    m = dt.Month + 12 * a - 3
    
    DT_ToJulianDay = dt.Day + ((153 * m + 2) \ 5) + 365 * y + (y \ 4) - (y \ 100) + (y \ 400) - 32045
END FUNCTION

' ============================================================================
' SUB: DT_FromJulianDay
' Convert Julian Day Number to DateTime
' ============================================================================
SUB DT_FromJulianDay (dt AS QBNex_DateTime, jd AS LONG)
    DIM a AS LONG, b AS LONG, c AS LONG, d AS LONG, e AS LONG, m AS LONG
    
    a = jd + 32044
    b = (4 * a + 3) \ 146097
    c = a - (146097 * b) \ 4
    d = (4 * c + 3) \ 1461
    e = c - (1461 * d) \ 4
    m = (5 * e + 2) \ 153
    
    dt.Day = e - (153 * m + 2) \ 5 + 1
    dt.Month = m + 3 - 12 * (m \ 10)
    dt.Year = 100 * b + d - 4800 + m \ 10
    dt.DOW = DT_CalcDayOfWeek(dt.Year, dt.Month, dt.Day)
END SUB

' ============================================================================
' SUB: DT_AddDays
' Add days to DateTime
' ============================================================================
SUB DT_AddDays (dt AS QBNex_DateTime, days AS LONG)
    DIM jd AS LONG
    jd = DT_ToJulianDay(dt) + days
    DT_FromJulianDay dt, jd
END SUB

' ============================================================================
' SUB: DT_AddSeconds
' Add seconds to DateTime (with day rollover)
' ============================================================================
SUB DT_AddSeconds (dt AS QBNex_DateTime, seconds AS LONG)
    DIM totalSeconds AS LONG
    DIM days AS LONG
    
    totalSeconds = dt.Hour * 3600 + dt.Minute * 60 + dt.Second + seconds
    
    ' Handle day rollover
    DO WHILE totalSeconds < 0
        totalSeconds = totalSeconds + 86400
        DT_AddDays dt, -1
    LOOP
    
    DO WHILE totalSeconds >= 86400
        totalSeconds = totalSeconds - 86400
        DT_AddDays dt, 1
    LOOP
    
    dt.Hour = totalSeconds \ 3600
    dt.Minute = (totalSeconds MOD 3600) \ 60
    dt.Second = totalSeconds MOD 60
END SUB

' ============================================================================
' FUNCTION: DT_DiffDays
' Calculate day difference between two DateTimes
' ============================================================================
FUNCTION DT_DiffDays& (dt1 AS QBNex_DateTime, dt2 AS QBNex_DateTime)
    DT_DiffDays = DT_ToJulianDay(dt2) - DT_ToJulianDay(dt1)
END FUNCTION

' ============================================================================
' FUNCTION: DT_DiffSeconds
' Calculate second difference between two DateTimes
' ============================================================================
FUNCTION DT_DiffSeconds& (dt1 AS QBNex_DateTime, dt2 AS QBNex_DateTime)
    DIM dayDiff AS LONG
    DIM secDiff AS LONG
    
    dayDiff = DT_DiffDays(dt1, dt2)
    secDiff = (dt2.Hour - dt1.Hour) * 3600 + _
    (dt2.Minute - dt1.Minute) * 60 + _
    (dt2.Second - dt1.Second)
    
    DT_DiffSeconds = dayDiff * 86400 + secDiff
END FUNCTION

' ============================================================================
' FUNCTION: DT_Compare
' Compare two DateTimes (-1: dt1<dt2, 0: equal, 1: dt1>dt2)
' ============================================================================
FUNCTION DT_Compare& (dt1 AS QBNex_DateTime, dt2 AS QBNex_DateTime)
    DIM diff AS LONG
    diff = DT_DiffSeconds(dt1, dt2)
    
    IF diff < 0 THEN
        DT_Compare = -1
    ELSEIF diff > 0 THEN
        DT_Compare = 1
    ELSE
        DT_Compare = 0
    END IF
END FUNCTION

' ============================================================================
' FUNCTION: DT_Format
' Format DateTime using tokens (YYYY MM DD HH MI SS MMMM DDD DDDD)
' ============================================================================
FUNCTION DT_Format$ (dt AS QBNex_DateTime, format AS STRING)
    DIM result AS STRING
    DIM monthNames AS STRING
    DIM dayNames AS STRING
    
    monthNames = "January,February,March,April,May,June,July,August,September,October,November,December"
    dayNames = "Sunday,Monday,Tuesday,Wednesday,Thursday,Friday,Saturday"
    
    result = format
    
    ' Replace tokens
    result = DT_StrReplace(result, "YYYY", RIGHT$("000" + LTRIM$(STR$(dt.Year)), 4))
    result = DT_StrReplace(result, "MM", RIGHT$("0" + LTRIM$(STR$(dt.Month)), 2))
    result = DT_StrReplace(result, "DD", RIGHT$("0" + LTRIM$(STR$(dt.Day)), 2))
    result = DT_StrReplace(result, "HH", RIGHT$("0" + LTRIM$(STR$(dt.Hour)), 2))
    result = DT_StrReplace(result, "MI", RIGHT$("0" + LTRIM$(STR$(dt.Minute)), 2))
    result = DT_StrReplace(result, "SS", RIGHT$("0" + LTRIM$(STR$(dt.Second)), 2))
    
    DT_Format = result
END FUNCTION

' ============================================================================
' FUNCTION: DT_StrReplace (Internal helper)
' Replace all occurrences of a substring
' ============================================================================
FUNCTION DT_StrReplace$ (text AS STRING, oldStr AS STRING, newStr AS STRING)
    DIM result AS STRING
    DIM POS AS LONG
    
    result = text
    DO
        POS = INSTR(result, oldStr)
        IF POS = 0 THEN EXIT DO
        result = LEFT$(result, POS - 1) + newStr + MID$(result, POS + LEN(oldStr))
    LOOP
    
    DT_StrReplace = result
END FUNCTION
