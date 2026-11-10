'===============================================================================
' QBNex Standard Library - Date and time ('$IMPORT:'ext.datetime')
'===============================================================================
' A point in time is a DOUBLE: milliseconds since 1970-01-01 00:00:00 UTC.
' Functions ending in "Local" use the computer's time zone, the others UTC.
'
'   t# = DT_Now#
'   DT_ISO$(t#)                 "2024-03-05T07:08:09.010Z"
'   DT_ISOLocal$(t#)            "2024-03-05T14:08:09.010+07:00"
'   DT_Format$(t#, "%d/%m/%Y %H:%M")   strftime-like, UTC (see below)
'   DT_FormatLocal$(t#, fmt$)
'   t# = DT_Parse#("2024-03-05T07:08:09Z", "")   "" = ISO 8601 / RFC 3339
'   t# = DT_Parse#("05/03/2024", "%d/%m/%Y")     with a pattern; no offset = UTC
'   DT_ParseLocal#(text$, fmt$)                  no offset = local time
'   t# = DT_Make#(2024, 3, 5, 7, 8, 9, 0)        month 1-12 (UTC)
'   DT_MakeLocal#(...)
'   DT_Year&, DT_Month&, DT_Day&, DT_Hour&, DT_Minute&, DT_Second&,
'   DT_Millisecond&, DT_Weekday& (0 = Sunday), DT_YearDay&, DT_IsoWeek&  (UTC)
'   DT_OffsetSeconds&(t#)       local offset from UTC at t#
'   DT_AddDays#(t#, n#), DT_DiffDays#(a#, b#)
' Invalid input gives NaN (DT_IsValid&(t#) = 0) and sets Ext_LastError$.
'
' Format codes: %Y year, %m month 01-12, %d day, %H hour 00-23, %I 01-12,
' %M minute, %S second, %3f milliseconds, %p AM/PM, %A/%a weekday name,
' %B/%b month name, %j day of year, %V ISO week, %z +0700, %:z +07:00,
' %s Unix seconds, %F = %Y-%m-%d, %T = %H:%M:%S, %% percent sign.
'===============================================================================

'$IMPORT:'ext.core'

DECLARE LIBRARY
    FUNCTION qb_ext_dt_now# ()
    FUNCTION qb_ext_dt_offset& (BYVAL ms AS DOUBLE)
    FUNCTION qb_ext_dt_iso&& (BYVAL ms AS DOUBLE, BYVAL isLocal AS LONG)
    FUNCTION qb_ext_dt_format&& (BYVAL ms AS DOUBLE, BYVAL f AS _OFFSET, BYVAL fl AS _INTEGER64, BYVAL isLocal AS LONG)
    FUNCTION qb_ext_dt_parse# (BYVAL t AS _OFFSET, BYVAL tl AS _INTEGER64, BYVAL f AS _OFFSET, BYVAL fl AS _INTEGER64, BYVAL isLocal AS LONG)
    FUNCTION qb_ext_dt_make# (BYVAL y AS LONG, BYVAL mo AS LONG, BYVAL d AS LONG, BYVAL h AS LONG, BYVAL mi AS LONG, BYVAL s AS LONG, BYVAL ms AS LONG, BYVAL isLocal AS LONG)
    FUNCTION qb_ext_dt_part&& (BYVAL ms AS DOUBLE, BYVAL part AS LONG, BYVAL isLocal AS LONG)
END DECLARE

FUNCTION DT_Now# ()
    DT_Now# = qb_ext_dt_now#
END FUNCTION

FUNCTION DT_IsValid& (t AS DOUBLE)
    IF t = t THEN DT_IsValid& = -1 ' NaN is the only value not equal to itself
END FUNCTION

FUNCTION DT_ISO$ (t AS DOUBLE)
    IF qb_ext_dt_iso&&(t, 0) > 0 THEN DT_ISO$ = Ext_Result$
END FUNCTION

FUNCTION DT_ISOLocal$ (t AS DOUBLE)
    IF qb_ext_dt_iso&&(t, 1) > 0 THEN DT_ISOLocal$ = Ext_Result$
END FUNCTION

FUNCTION DT_Format$ (t AS DOUBLE, fmt AS STRING)
    IF qb_ext_dt_format&&(t, _OFFSET(fmt), LEN(fmt), 0) > 0 THEN DT_Format$ = Ext_Result$
END FUNCTION

FUNCTION DT_FormatLocal$ (t AS DOUBLE, fmt AS STRING)
    IF qb_ext_dt_format&&(t, _OFFSET(fmt), LEN(fmt), 1) > 0 THEN DT_FormatLocal$ = Ext_Result$
END FUNCTION

FUNCTION DT_Parse# (text AS STRING, fmt AS STRING)
    DT_Parse# = qb_ext_dt_parse#(_OFFSET(text), LEN(text), _OFFSET(fmt), LEN(fmt), 0)
END FUNCTION

FUNCTION DT_ParseLocal# (text AS STRING, fmt AS STRING)
    DT_ParseLocal# = qb_ext_dt_parse#(_OFFSET(text), LEN(text), _OFFSET(fmt), LEN(fmt), 1)
END FUNCTION

FUNCTION DT_Make# (y AS LONG, mo AS LONG, d AS LONG, h AS LONG, mi AS LONG, s AS LONG, ms AS LONG)
    DT_Make# = qb_ext_dt_make#(y, mo, d, h, mi, s, ms, 0)
END FUNCTION

FUNCTION DT_MakeLocal# (y AS LONG, mo AS LONG, d AS LONG, h AS LONG, mi AS LONG, s AS LONG, ms AS LONG)
    DT_MakeLocal# = qb_ext_dt_make#(y, mo, d, h, mi, s, ms, 1)
END FUNCTION

FUNCTION DT_Year& (t AS DOUBLE)
    DT_Year& = qb_ext_dt_part&&(t, 1, 0)
END FUNCTION

FUNCTION DT_Month& (t AS DOUBLE)
    DT_Month& = qb_ext_dt_part&&(t, 2, 0)
END FUNCTION

FUNCTION DT_Day& (t AS DOUBLE)
    DT_Day& = qb_ext_dt_part&&(t, 3, 0)
END FUNCTION

FUNCTION DT_Hour& (t AS DOUBLE)
    DT_Hour& = qb_ext_dt_part&&(t, 4, 0)
END FUNCTION

FUNCTION DT_Minute& (t AS DOUBLE)
    DT_Minute& = qb_ext_dt_part&&(t, 5, 0)
END FUNCTION

FUNCTION DT_Second& (t AS DOUBLE)
    DT_Second& = qb_ext_dt_part&&(t, 6, 0)
END FUNCTION

FUNCTION DT_Millisecond& (t AS DOUBLE)
    DT_Millisecond& = qb_ext_dt_part&&(t, 7, 0)
END FUNCTION

FUNCTION DT_Weekday& (t AS DOUBLE)
    DT_Weekday& = qb_ext_dt_part&&(t, 8, 0)
END FUNCTION

FUNCTION DT_YearDay& (t AS DOUBLE)
    DT_YearDay& = qb_ext_dt_part&&(t, 9, 0)
END FUNCTION

FUNCTION DT_IsoWeek& (t AS DOUBLE)
    DT_IsoWeek& = qb_ext_dt_part&&(t, 10, 0)
END FUNCTION

FUNCTION DT_OffsetSeconds& (t AS DOUBLE)
    DT_OffsetSeconds& = qb_ext_dt_offset&(t)
END FUNCTION

FUNCTION DT_AddDays# (t AS DOUBLE, days AS DOUBLE)
    DT_AddDays# = t + days * 86400000#
END FUNCTION

FUNCTION DT_DiffDays# (a AS DOUBLE, b AS DOUBLE)
    DT_DiffDays# = (a - b) / 86400000#
END FUNCTION
