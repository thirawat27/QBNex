' ============================================================================
' QBNex Standard Library - System: Date/Time
' JavaScript-style date helpers backed by DATE$, TIME$, and TIMER.
' ============================================================================

TYPE QBNex_Date
    Year AS LONG
    Month AS LONG
    Day AS LONG
    Hour AS LONG
    Minute AS LONG
    Second AS LONG
    Millisecond AS LONG
END TYPE

FUNCTION Date_Pad2$ (valueNumber AS LONG)
    DIM text AS STRING

    text = LTRIM$(RTRIM$(STR$(valueNumber)))
    IF LEN(text) < 2 THEN text = "0" + text
    Date_Pad2 = text
END FUNCTION

FUNCTION Date_Pad3$ (valueNumber AS LONG)
    DIM text AS STRING

    text = LTRIM$(RTRIM$(STR$(valueNumber)))
    DO WHILE LEN(text) < 3
        text = "0" + text
    LOOP
    Date_Pad3 = text
END FUNCTION

FUNCTION Date_PartValue& (sourceText AS STRING, startPos AS LONG, endPos AS LONG)
    IF endPos < startPos THEN EXIT FUNCTION
    Date_PartValue = VAL(MID$(sourceText, startPos, endPos - startPos + 1))
END FUNCTION

FUNCTION Date_IsLeapYear& (yearValue AS LONG)
    IF (yearValue MOD 400) = 0 THEN Date_IsLeapYear = -1: EXIT FUNCTION
    IF (yearValue MOD 100) = 0 THEN EXIT FUNCTION
    IF (yearValue MOD 4) = 0 THEN Date_IsLeapYear = -1
END FUNCTION

FUNCTION Date_DaysInMonth& (yearValue AS LONG, monthValue AS LONG)
    SELECT CASE monthValue
        CASE 1, 3, 5, 7, 8, 10, 12
            Date_DaysInMonth = 31
        CASE 4, 6, 9, 11
            Date_DaysInMonth = 30
        CASE 2
            Date_DaysInMonth = 28
            IF Date_IsLeapYear&(yearValue) THEN Date_DaysInMonth = 29
    END SELECT
END FUNCTION

SUB Date_FromParts (dateRef AS QBNex_Date, yearValue AS LONG, monthIndex AS LONG, dayValue AS LONG, hourValue AS LONG, minuteValue AS LONG, secondValue AS LONG, millisecondValue AS LONG)
    dateRef.Year = yearValue
    dateRef.Month = monthIndex + 1
    dateRef.Day = dayValue
    dateRef.Hour = hourValue
    dateRef.Minute = minuteValue
    dateRef.Second = secondValue
    dateRef.Millisecond = millisecondValue
END SUB

SUB Date_SetNow (dateRef AS QBNex_Date)
    DIM rawDate AS STRING
    DIM rawTime AS STRING
    DIM firstSep AS LONG
    DIM secondSep AS LONG
    DIM timerValue AS DOUBLE

    rawDate = DATE$
    rawTime = TIME$

    firstSep = INSTR(rawDate, "/")
    IF firstSep = 0 THEN firstSep = INSTR(rawDate, "-")
    secondSep = INSTR(firstSep + 1, rawDate, "/")
    IF secondSep = 0 THEN secondSep = INSTR(firstSep + 1, rawDate, "-")

    dateRef.Month = Date_PartValue&(rawDate, 1, firstSep - 1)
    dateRef.Day = Date_PartValue&(rawDate, firstSep + 1, secondSep - 1)
    dateRef.Year = VAL(MID$(rawDate, secondSep + 1))

    dateRef.Hour = VAL(LEFT$(rawTime, 2))
    dateRef.Minute = VAL(MID$(rawTime, 4, 2))
    dateRef.Second = VAL(MID$(rawTime, 7, 2))

    timerValue = TIMER
    dateRef.Millisecond = INT((timerValue - INT(timerValue)) * 1000#)
END SUB

FUNCTION Date_ValueOf# (dateRef AS QBNex_Date)
    DIM yearValue AS LONG
    DIM monthValue AS LONG
    DIM totalDays AS DOUBLE

    FOR yearValue = 1970 TO dateRef.Year - 1
        totalDays = totalDays + 365
        IF Date_IsLeapYear&(yearValue) THEN totalDays = totalDays + 1
    NEXT

    FOR monthValue = 1 TO dateRef.Month - 1
        totalDays = totalDays + Date_DaysInMonth&(dateRef.Year, monthValue)
    NEXT

    totalDays = totalDays + (dateRef.Day - 1)
    Date_ValueOf = (((((totalDays * 24#) + dateRef.Hour) * 60# + dateRef.Minute) * 60# + dateRef.Second) * 1000#) + dateRef.Millisecond
END FUNCTION

FUNCTION Date_NowMs# ()
    DIM nowValue AS QBNex_Date

    Date_SetNow nowValue
    Date_NowMs = Date_ValueOf#(nowValue)
END FUNCTION

SUB Date_FromUnixMs (dateRef AS QBNex_Date, epochMs AS DOUBLE)
    DIM totalSeconds AS DOUBLE
    DIM wholeDays AS LONG
    DIM secondsOfDay AS LONG
    DIM yearValue AS LONG
    DIM monthValue AS LONG
    DIM yearDays AS LONG

    totalSeconds = INT(epochMs / 1000#)
    dateRef.Millisecond = epochMs - totalSeconds * 1000#

    wholeDays = INT(totalSeconds / 86400#)
    secondsOfDay = totalSeconds - wholeDays * 86400

    yearValue = 1970
    DO
        yearDays = 365
        IF Date_IsLeapYear&(yearValue) THEN yearDays = 366
        IF wholeDays < yearDays THEN EXIT DO
        wholeDays = wholeDays - yearDays
        yearValue = yearValue + 1
    LOOP

    monthValue = 1
    DO
        yearDays = Date_DaysInMonth&(yearValue, monthValue)
        IF wholeDays < yearDays THEN EXIT DO
        wholeDays = wholeDays - yearDays
        monthValue = monthValue + 1
    LOOP

    dateRef.Year = yearValue
    dateRef.Month = monthValue
    dateRef.Day = wholeDays + 1
    dateRef.Hour = INT(secondsOfDay / 3600)
    secondsOfDay = secondsOfDay - dateRef.Hour * 3600
    dateRef.Minute = INT(secondsOfDay / 60)
    dateRef.Second = secondsOfDay - dateRef.Minute * 60
END SUB

FUNCTION Date_GetFullYear& (dateRef AS QBNex_Date)
    Date_GetFullYear = dateRef.Year
END FUNCTION

FUNCTION Date_GetMonth& (dateRef AS QBNex_Date)
    Date_GetMonth = dateRef.Month - 1
END FUNCTION

FUNCTION Date_GetDate& (dateRef AS QBNex_Date)
    Date_GetDate = dateRef.Day
END FUNCTION

FUNCTION Date_GetHours& (dateRef AS QBNex_Date)
    Date_GetHours = dateRef.Hour
END FUNCTION

FUNCTION Date_GetMinutes& (dateRef AS QBNex_Date)
    Date_GetMinutes = dateRef.Minute
END FUNCTION

FUNCTION Date_GetSeconds& (dateRef AS QBNex_Date)
    Date_GetSeconds = dateRef.Second
END FUNCTION

FUNCTION Date_GetMilliseconds& (dateRef AS QBNex_Date)
    Date_GetMilliseconds = dateRef.Millisecond
END FUNCTION

FUNCTION Date_GetDay& (dateRef AS QBNex_Date)
    DIM daysSinceEpoch AS DOUBLE

    daysSinceEpoch = INT(Date_ValueOf#(dateRef) / 86400000#)
    Date_GetDay = (daysSinceEpoch + 4) MOD 7
END FUNCTION

FUNCTION Date_ToISOString$ (dateRef AS QBNex_Date)
    Date_ToISOString = LTRIM$(RTRIM$(STR$(dateRef.Year))) + "-" + Date_Pad2$(dateRef.Month) + "-" + Date_Pad2$(dateRef.Day) + "T" + Date_Pad2$(dateRef.Hour) + ":" + Date_Pad2$(dateRef.Minute) + ":" + Date_Pad2$(dateRef.Second) + "." + Date_Pad3$(dateRef.Millisecond)
END FUNCTION

FUNCTION Date_ToJSON$ (dateRef AS QBNex_Date)
    DIM isoText AS STRING

    isoText = Date_ToISOString$(dateRef)
    Date_ToJSON = CHR$(34) + isoText + CHR$(34)
END FUNCTION
