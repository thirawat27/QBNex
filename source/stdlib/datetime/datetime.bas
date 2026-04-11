' =============================================================================
' QBNex DateTime Library — datetime.bas
' =============================================================================
'
' TYPE QBNex_DateTime plus arithmetic, formatting, and parsing helpers.
'
' Usage:
'
'   '$INCLUDE:'stdlib/datetime/datetime.bas'
'
'   DIM now AS QBNex_DateTime
'   DT_Now now                          ' fill with current date+time
'
'   PRINT DT_Format$(now, "YYYY-MM-DD HH:MM:SS")   ' 2026-04-11 22:00:00
'
'   DIM future AS QBNex_DateTime
'   DT_AddDays future, now, 30
'   PRINT DT_DiffDays(now, future)      ' 30
'
'   DIM parsed AS QBNex_DateTime
'   DT_Parse parsed, "2026-12-25", "YYYY-MM-DD"
'
' =============================================================================

TYPE QBNex_DateTime
    Year   AS INTEGER
    Month  AS INTEGER    ' 1-12
    Day    AS INTEGER    ' 1-31
    Hour   AS INTEGER    ' 0-23
    Minute AS INTEGER    ' 0-59
    Second AS INTEGER    ' 0-59
    DOW    AS INTEGER    ' day of week: 0=Sun, 1=Mon ... 6=Sat
END TYPE

' Days per month (non-leap)
DIM SHARED QBNEX_DT_DPM(1 TO 12) AS INTEGER
DATA 31,28,31,30,31,30,31,31,30,31,30,31
FOR _dt_i = 1 TO 12: READ QBNEX_DT_DPM(_dt_i): NEXT _dt_i

FUNCTION _DT_IsLeap& (yr AS INTEGER)
    _DT_IsLeap& = ((yr MOD 4 = 0 AND yr MOD 100 <> 0) OR (yr MOD 400 = 0))
END FUNCTION

FUNCTION _DT_DaysInMonth& (yr AS INTEGER, mo AS INTEGER)
    IF mo = 2 AND _DT_IsLeap&(yr) THEN
        _DT_DaysInMonth& = 29
    ELSE
        _DT_DaysInMonth& = QBNEX_DT_DPM(mo)
    END IF
END FUNCTION

' ---------------------------------------------------------------------------
' SUB  DT_Now(dt)  — fill with current system date + time
' ---------------------------------------------------------------------------
SUB DT_Now (dt AS QBNex_DateTime)
    DIM d AS STRING, t AS STRING
    d = DATE$   ' MM-DD-YYYY
    t = TIME$   ' HH:MM:SS
    dt.Month  = VAL(LEFT$(d, 2))
    dt.Day    = VAL(MID$(d, 4, 2))
    dt.Year   = VAL(RIGHT$(d, 4))
    dt.Hour   = VAL(LEFT$(t, 2))
    dt.Minute = VAL(MID$(t, 4, 2))
    dt.Second = VAL(RIGHT$(t, 2))
    DT_CalcDOW dt
END SUB

' ---------------------------------------------------------------------------
' SUB  DT_Set(dt, yyyy, mm, dd, hh, mi, ss)
' ---------------------------------------------------------------------------
SUB DT_Set (dt AS QBNex_DateTime, yyyy AS INTEGER, mm AS INTEGER, dd AS INTEGER, _
            hh AS INTEGER, mi AS INTEGER, ss AS INTEGER)
    dt.Year = yyyy: dt.Month = mm: dt.Day = dd
    dt.Hour = hh:   dt.Minute = mi: dt.Second = ss
    DT_CalcDOW dt
END SUB

' ---------------------------------------------------------------------------
' SUB  DT_CalcDOW(dt)  — compute day-of-week using Zeller's congruence
' ---------------------------------------------------------------------------
SUB DT_CalcDOW (dt AS QBNex_DateTime)
    DIM m AS INTEGER, y AS INTEGER, k AS LONG, j AS LONG
    m = dt.Month: y = dt.Year
    IF m < 3 THEN m = m + 12: y = y - 1
    k = y MOD 100
    j = y \ 100
    dt.DOW = (dt.Day + (13 * (m + 1)) \ 5 + k + k \ 4 + j \ 4 - 2 * j) MOD 7
    IF dt.DOW < 0 THEN dt.DOW = dt.DOW + 7
    ' Zeller: 0=Sat, 1=Sun ... -> convert to 0=Sun
    dt.DOW = (dt.DOW + 6) MOD 7
END SUB

' ---------------------------------------------------------------------------
' Convert to Julian Day Number (for arithmetic)
' ---------------------------------------------------------------------------
FUNCTION DT_ToJDN& (dt AS QBNex_DateTime)
    DIM a AS LONG, y AS LONG, m AS LONG
    a = (14 - dt.Month) \ 12
    y = dt.Year + 4800 - a
    m = dt.Month + 12 * a - 3
    DT_ToJDN& = dt.Day + (153 * m + 2) \ 5 + y * 365 + y \ 4 - y \ 100 + y \ 400 - 32045
END FUNCTION

' ---------------------------------------------------------------------------
' Convert from Julian Day Number to date
' ---------------------------------------------------------------------------
SUB DT_FromJDN (dt AS QBNex_DateTime, jdn AS LONG)
    DIM a AS LONG, b AS LONG, c AS LONG, d AS LONG, e AS LONG, m AS LONG
    a = jdn + 32044
    b = (4 * a + 3) \ 146097
    c = a - (146097 * b) \ 4
    d = (4 * c + 3) \ 1461
    e = c - (1461 * d) \ 4
    m = (5 * e + 2) \ 153
    dt.Day   = e - (153 * m + 2) \ 5 + 1
    dt.Month = m + 3 - 12 * (m \ 10)
    dt.Year  = 100 * b + d - 4800 + (m \ 10)
    DT_CalcDOW dt
END SUB

' ---------------------------------------------------------------------------
' Date arithmetic
' ---------------------------------------------------------------------------
SUB DT_AddDays (out AS QBNex_DateTime, src AS QBNex_DateTime, days AS LONG)
    DIM jdn AS LONG
    jdn = DT_ToJDN&(src) + days
    DT_FromJDN out, jdn
    out.Hour = src.Hour: out.Minute = src.Minute: out.Second = src.Second
END SUB

SUB DT_AddSeconds (out AS QBNex_DateTime, src AS QBNex_DateTime, secs AS LONG)
    DIM totSec AS LONG, jdn AS LONG
    totSec = src.Hour * 3600& + src.Minute * 60& + src.Second + secs
    DIM dayOff AS LONG
    dayOff = totSec \ 86400&
    totSec = totSec MOD 86400&
    IF totSec < 0 THEN totSec = totSec + 86400&: dayOff = dayOff - 1
    jdn = DT_ToJDN&(src) + dayOff
    DT_FromJDN out, jdn
    out.Hour   = totSec \ 3600
    out.Minute = (totSec MOD 3600) \ 60
    out.Second = totSec MOD 60
END SUB

' Returns difference in whole days (a - b)
FUNCTION DT_DiffDays& (a AS QBNex_DateTime, b AS QBNex_DateTime)
    DT_DiffDays& = DT_ToJDN&(a) - DT_ToJDN&(b)
END FUNCTION

' Returns total seconds difference (a - b)
FUNCTION DT_DiffSeconds& (a AS QBNex_DateTime, b AS QBNex_DateTime)
    DIM dayDiff AS LONG, secA AS LONG, secB AS LONG
    dayDiff = DT_DiffDays&(a, b)
    secA    = a.Hour * 3600& + a.Minute * 60& + a.Second
    secB    = b.Hour * 3600& + b.Minute * 60& + b.Second
    DT_DiffSeconds& = dayDiff * 86400& + secA - secB
END FUNCTION

' ---------------------------------------------------------------------------
' Comparison
' ---------------------------------------------------------------------------
FUNCTION DT_Compare& (a AS QBNex_DateTime, b AS QBNex_DateTime)
    DIM sa AS LONG, sb AS LONG
    sa = DT_ToJDN&(a) * 86400& + a.Hour * 3600& + a.Minute * 60& + a.Second
    sb = DT_ToJDN&(b) * 86400& + b.Hour * 3600& + b.Minute * 60& + b.Second
    IF sa < sb THEN DT_Compare& = -1
    IF sa = sb THEN DT_Compare& = 0
    IF sa > sb THEN DT_Compare& = 1
END FUNCTION

' ---------------------------------------------------------------------------
' Formatting  — tokens: YYYY MM DD HH MM SS DOW
' ---------------------------------------------------------------------------
DIM SHARED QBNEX_DT_DAYNAMES(0 TO 6) AS STRING
DATA "Sunday","Monday","Tuesday","Wednesday","Thursday","Friday","Saturday"
FOR _dt_i = 0 TO 6: READ QBNEX_DT_DAYNAMES(_dt_i): NEXT _dt_i

DIM SHARED QBNEX_DT_MONTHNAMES(1 TO 12) AS STRING
DATA "January","February","March","April","May","June","July","August","September","October","November","December"
FOR _dt_i = 1 TO 12: READ QBNEX_DT_MONTHNAMES(_dt_i): NEXT _dt_i

FUNCTION _DT_ZP$ (n AS INTEGER, w AS INTEGER)
    DIM s AS STRING
    s = _TRIM$(STR$(n))
    DO WHILE LEN(s) < w: s = "0" + s: LOOP
    _DT_ZP$ = s
END FUNCTION

FUNCTION DT_Format$ (dt AS QBNex_DateTime, fmt$)
    DIM result AS STRING, i AS LONG, tok AS STRING
    result = fmt$
    ' Replace longest tokens first to avoid partial matches
    result = StrReplace$(result, "YYYY",  _DT_ZP$(dt.Year,   4))
    result = StrReplace$(result, "YY",    _DT_ZP$(dt.Year MOD 100, 2))
    result = StrReplace$(result, "MMMM",  QBNEX_DT_MONTHNAMES(dt.Month))
    result = StrReplace$(result, "MMM",   LEFT$(QBNEX_DT_MONTHNAMES(dt.Month), 3))
    result = StrReplace$(result, "MM",    _DT_ZP$(dt.Month,  2))
    result = StrReplace$(result, "DD",    _DT_ZP$(dt.Day,    2))
    result = StrReplace$(result, "HH",    _DT_ZP$(dt.Hour,   2))
    result = StrReplace$(result, "MI",    _DT_ZP$(dt.Minute, 2))
    result = StrReplace$(result, "SS",    _DT_ZP$(dt.Second, 2))
    result = StrReplace$(result, "DDDD",  QBNEX_DT_DAYNAMES(dt.DOW))
    result = StrReplace$(result, "DDD",   LEFT$(QBNEX_DT_DAYNAMES(dt.DOW), 3))
    DT_Format$ = result
END FUNCTION

' ---------------------------------------------------------------------------
' Parsing  — format tokens: YYYY MM DD HH MI SS
' ---------------------------------------------------------------------------
SUB DT_Parse (dt AS QBNex_DateTime, dateStr$, fmt$)
    DIM pos AS LONG, fpos AS LONG, tok AS STRING, num AS INTEGER
    pos  = 1: fpos = 1
    DO WHILE fpos <= LEN(fmt$) AND pos <= LEN(dateStr$)
        DIM fch AS STRING
        fch = MID$(fmt$, fpos, 1)
        SELECT CASE fch
            CASE "Y"
                DIM ylen AS INTEGER: ylen = 0
                DO WHILE fpos + ylen <= LEN(fmt$) AND MID$(fmt$, fpos + ylen, 1) = "Y": ylen = ylen + 1: LOOP
                dt.Year  = VAL(MID$(dateStr$, pos, ylen))
                pos = pos + ylen: fpos = fpos + ylen
            CASE "M"
                DIM mlen AS INTEGER: mlen = 0
                IF MID$(fmt$, fpos, 2) = "MI" THEN
                    dt.Minute = VAL(MID$(dateStr$, pos, 2))
                    pos = pos + 2: fpos = fpos + 2
                ELSE
                    DO WHILE fpos + mlen <= LEN(fmt$) AND MID$(fmt$, fpos + mlen, 1) = "M": mlen = mlen + 1: LOOP
                    dt.Month = VAL(MID$(dateStr$, pos, mlen))
                    pos = pos + mlen: fpos = fpos + mlen
                END IF
            CASE "D"
                dt.Day = VAL(MID$(dateStr$, pos, 2)): pos = pos + 2: fpos = fpos + 2
            CASE "H"
                dt.Hour = VAL(MID$(dateStr$, pos, 2)): pos = pos + 2: fpos = fpos + 2
            CASE "S"
                dt.Second = VAL(MID$(dateStr$, pos, 2)): pos = pos + 2: fpos = fpos + 2
            CASE ELSE
                pos = pos + 1: fpos = fpos + 1  ' literal separator
        END SELECT
    LOOP
    DT_CalcDOW dt
END SUB
