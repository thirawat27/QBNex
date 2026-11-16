'===============================================================================
' QBNex Standard Library - datetime ('$IMPORT:'datetime')
'===============================================================================
' A point in time is a DOUBLE: milliseconds since 1970-01-01 00:00:00 UTC,
' like JavaScript's Date.now(). Every function works in UTC; give local = -1
' for the computer's time zone. Not available in the web build.
'
'   t# = Date_Now
'   Date_Iso(t#)                        "2024-03-05T07:08:09.010Z"
'   Date_Iso(t#, -1)                    "2024-03-05T14:08:09.010+07:00"
'   Date_Format(t#, "%d/%m/%Y %H:%M")   strftime codes (below)
'   t# = Date_Parse("2024-03-05T07:08:09Z")   ISO 8601 / RFC 3339
'   t# = Date_Parse("05/03/2024", "%d/%m/%Y")
'   t# = Date_Make(2024, 3, 5)          y, month 1-12, day [, h, mi, s, ms]
'   Date_Year(t#), Date_Month, Date_Day, Date_Hour, Date_Minute,
'   Date_Second, Date_Weekday (0 = Sunday), Date_YearDay, Date_IsoWeek
'   Date_AddDays(t#, 7), Date_DiffDays(a#, b#), Date_Offset(t#)
' Invalid input gives NaN (Date_IsValid(t#) = 0) and sets LastError.
'
' Format codes: %Y year, %m month 01-12, %d day, %H hour 00-23, %I 01-12,
' %M minute, %S second, %3f milliseconds, %p AM/PM, %A/%a weekday name,
' %B/%b month name, %j day of year, %V ISO week, %z +0700, %:z +07:00,
' %s Unix seconds, %F = %Y-%m-%d, %T = %H:%M:%S, %% percent sign.
'===============================================================================

'$IMPORT:'ext'

DECLARE LIBRARY
    FUNCTION qb_ext_dt_now# ()
    FUNCTION qb_ext_dt_offset& (BYVAL ms AS DOUBLE)
    FUNCTION qb_ext_dt_iso&& (BYVAL ms AS DOUBLE, BYVAL isLocal AS LONG)
    FUNCTION qb_ext_dt_format&& (BYVAL ms AS DOUBLE, BYVAL f AS _OFFSET, BYVAL fl AS _INTEGER64, BYVAL isLocal AS LONG)
    FUNCTION qb_ext_dt_parse# (BYVAL t AS _OFFSET, BYVAL tl AS _INTEGER64, BYVAL f AS _OFFSET, BYVAL fl AS _INTEGER64, BYVAL isLocal AS LONG)
    FUNCTION qb_ext_dt_make# (BYVAL y AS LONG, BYVAL mo AS LONG, BYVAL d AS LONG, BYVAL h AS LONG, BYVAL mi AS LONG, BYVAL s AS LONG, BYVAL ms AS LONG, BYVAL isLocal AS LONG)
    FUNCTION qb_ext_dt_part&& (BYVAL ms AS DOUBLE, BYVAL part AS LONG, BYVAL isLocal AS LONG)
END DECLARE

FUNCTION Date_Flag& (isLocal AS LONG)
    IF isLocal THEN Date_Flag& = 1
END FUNCTION

' Now, in milliseconds since 1970 (UTC).
FUNCTION Date_Now# ()
    Date_Now# = qb_ext_dt_now#
END FUNCTION

' -1 unless t is the "invalid" value parsing returns.
FUNCTION Date_IsValid& (t AS DOUBLE)
    Date_IsValid& = (t = t) ' NaN is the only value not equal to itself
END FUNCTION

' ISO 8601 text of t.
FUNCTION Date_Iso$ (t AS DOUBLE, OPTIONAL isLocal AS LONG = 0)
    Date_Iso$ = Ext_Text$(qb_ext_dt_iso&&(t, Date_Flag&(isLocal)))
END FUNCTION

' t written with strftime codes (default "%Y-%m-%d %H:%M:%S").
FUNCTION Date_Format$ (t AS DOUBLE, OPTIONAL pattern AS STRING = "%Y-%m-%d %H:%M:%S", OPTIONAL isLocal AS LONG = 0)
    Date_Format$ = Ext_Text$(qb_ext_dt_format&&(t, _OFFSET(pattern), LEN(pattern), Date_Flag&(isLocal)))
END FUNCTION

' The time in text: ISO 8601 when pattern is "", else strftime codes.
' Without an offset in the text it is UTC (local time when isLocal).
FUNCTION Date_Parse# (text AS STRING, OPTIONAL pattern AS STRING = "", OPTIONAL isLocal AS LONG = 0)
    Date_Parse# = qb_ext_dt_parse#(_OFFSET(text), LEN(text), _OFFSET(pattern), LEN(pattern), Date_Flag&(isLocal))
    Ext_Sync
END FUNCTION

' The time of a date (month 1-12) and clock time.
FUNCTION Date_Make# (y AS LONG, mo AS LONG, d AS LONG, OPTIONAL h AS LONG = 0, OPTIONAL mi AS LONG = 0, OPTIONAL s AS LONG = 0, OPTIONAL ms AS LONG = 0, OPTIONAL isLocal AS LONG = 0)
    Date_Make# = qb_ext_dt_make#(y, mo, d, h, mi, s, ms, Date_Flag&(isLocal))
    Ext_Sync
END FUNCTION

FUNCTION Date_Part& (t AS DOUBLE, part AS LONG, isLocal AS LONG)
    Date_Part& = qb_ext_dt_part&&(t, part, Date_Flag&(isLocal))
END FUNCTION

' Year of t.
FUNCTION Date_Year& (t AS DOUBLE, OPTIONAL isLocal AS LONG = 0)
    Date_Year& = Date_Part&(t, 1, isLocal)
END FUNCTION

' Month of t, 1-12.
FUNCTION Date_Month& (t AS DOUBLE, OPTIONAL isLocal AS LONG = 0)
    Date_Month& = Date_Part&(t, 2, isLocal)
END FUNCTION

' Day of the month of t.
FUNCTION Date_Day& (t AS DOUBLE, OPTIONAL isLocal AS LONG = 0)
    Date_Day& = Date_Part&(t, 3, isLocal)
END FUNCTION

' Hour of t, 0-23.
FUNCTION Date_Hour& (t AS DOUBLE, OPTIONAL isLocal AS LONG = 0)
    Date_Hour& = Date_Part&(t, 4, isLocal)
END FUNCTION

' Minute of t.
FUNCTION Date_Minute& (t AS DOUBLE, OPTIONAL isLocal AS LONG = 0)
    Date_Minute& = Date_Part&(t, 5, isLocal)
END FUNCTION

' Second of t.
FUNCTION Date_Second& (t AS DOUBLE, OPTIONAL isLocal AS LONG = 0)
    Date_Second& = Date_Part&(t, 6, isLocal)
END FUNCTION

' Millisecond of t.
FUNCTION Date_Millisecond& (t AS DOUBLE, OPTIONAL isLocal AS LONG = 0)
    Date_Millisecond& = Date_Part&(t, 7, isLocal)
END FUNCTION

' Day of the week of t (0 = Sunday).
FUNCTION Date_Weekday& (t AS DOUBLE, OPTIONAL isLocal AS LONG = 0)
    Date_Weekday& = Date_Part&(t, 8, isLocal)
END FUNCTION

' Day of the year of t (1-366).
FUNCTION Date_YearDay& (t AS DOUBLE, OPTIONAL isLocal AS LONG = 0)
    Date_YearDay& = Date_Part&(t, 9, isLocal)
END FUNCTION

' ISO 8601 week number of t.
FUNCTION Date_IsoWeek& (t AS DOUBLE, OPTIONAL isLocal AS LONG = 0)
    Date_IsoWeek& = Date_Part&(t, 10, isLocal)
END FUNCTION

' Seconds the local time zone is ahead of UTC at t.
FUNCTION Date_Offset& (t AS DOUBLE)
    Date_Offset& = qb_ext_dt_offset&(t)
END FUNCTION

' t plus days (may be negative or a fraction).
FUNCTION Date_AddDays# (t AS DOUBLE, days AS DOUBLE)
    Date_AddDays# = t + days * 86400000#
END FUNCTION

' Days from b to a.
FUNCTION Date_DiffDays# (a AS DOUBLE, b AS DOUBLE)
    Date_DiffDays# = (a - b) / 86400000#
END FUNCTION
