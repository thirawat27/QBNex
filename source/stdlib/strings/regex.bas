' =============================================================================
' QBNex String Library — Pattern Matching — regex.bas
' =============================================================================
'
' Provides two levels of pattern matching:
'
'  1. GlobMatch$(text$, pattern$) — shell-style wildcards (* and ?)
'  2. RegexMatch$(text$, pattern$) — minimal POSIX-like regex subset:
'       .   any single character
'       *   zero or more of preceding
'       +   one or more of preceding
'       ?   zero or one of preceding
'       ^   anchor to start
'       $   anchor to end
'       [abc] character class (no ranges yet)
'       [^abc] negated class
'
' Usage:
'
'   '$INCLUDE:'stdlib/strings/regex.bas'
'
'   PRINT GlobMatch$("hello.bas", "*.bas")   ' -1 (TRUE)
'   PRINT GlobMatch$("hello.exe", "*.bas")   '  0 (FALSE)
'
'   PRINT RegexMatch$("abc123", "^[abc]+[0-9]*$")   ' -1
'   PRINT RegexMatch$("hello", "^hell.$")            ' -1
'
' =============================================================================

' ---------------------------------------------------------------------------
' Glob matching  (* = any sequence, ? = any single char)
' ---------------------------------------------------------------------------
FUNCTION GlobMatch& (text$, pattern$)
    DIM ti AS LONG, pi AS LONG, starTI AS LONG, starPI AS LONG
    DIM tc AS STRING, pc AS STRING

    ti = 1: pi = 1
    starTI = -1: starPI = -1

    DO
        IF ti > LEN(text$) AND pi > LEN(pattern$) THEN GlobMatch& = -1: EXIT FUNCTION
        IF pi <= LEN(pattern$) THEN pc = MID$(pattern$, pi, 1) ELSE pc = ""
        IF ti <= LEN(text$)    THEN tc = MID$(text$,    ti, 1) ELSE tc = ""

        IF pc = "*" THEN
            starPI = pi: starTI = ti
            pi = pi + 1
        ELSEIF pc = "?" OR (pc = tc AND pc <> "") THEN
            pi = pi + 1: ti = ti + 1
        ELSEIF starPI > 0 THEN
            starTI = starTI + 1
            pi     = starPI + 1
            ti     = starTI
        ELSE
            GlobMatch& = 0: EXIT FUNCTION
        END IF

        IF ti > LEN(text$) + 1 THEN GlobMatch& = 0: EXIT FUNCTION
    LOOP
END FUNCTION

' ---------------------------------------------------------------------------
' Minimal regex engine
'   Supports: . * + ? ^ $ [abc] [^abc]
' ---------------------------------------------------------------------------

' PRIVATE: match character against a regex character class [...]
' Returns (end-position-in-pattern, matched?)
FUNCTION _Regex_MatchClass& (text$, ti AS LONG, pat$, pi AS LONG, endPI AS LONG)
    DIM negate AS LONG, ch AS STRING, p AS LONG, matched AS LONG
    p = pi + 1   ' skip '['
    negate = 0
    IF p <= LEN(pat$) AND MID$(pat$, p, 1) = "^" THEN negate = -1: p = p + 1

    matched = 0
    DO WHILE p <= LEN(pat$) AND MID$(pat$, p, 1) <> "]"
        ' simple range: a-z
        IF p + 2 <= LEN(pat$) AND MID$(pat$, p + 1, 1) = "-" AND _
           MID$(pat$, p + 2, 1) <> "]" THEN
            DIM lo AS INTEGER, hi AS INTEGER
            lo = ASC(MID$(pat$, p,     1))
            hi = ASC(MID$(pat$, p + 2, 1))
            IF ti <= LEN(text$) THEN
                DIM tcode AS INTEGER
                tcode = ASC(MID$(text$, ti, 1))
                IF tcode >= lo AND tcode <= hi THEN matched = -1
            END IF
            p = p + 3
        ELSE
            ch = MID$(pat$, p, 1)
            IF ti <= LEN(text$) AND MID$(text$, ti, 1) = ch THEN matched = -1
            p = p + 1
        END IF
    LOOP
    IF MID$(pat$, p, 1) = "]" THEN p = p + 1
    endPI = p

    IF negate THEN matched = NOT matched
    _Regex_MatchClass& = matched
END FUNCTION

' PRIVATE: recursive regex matcher — returns length consumed in text$ or -1
FUNCTION _Regex_Match& (text$, ti AS LONG, pat$, pi AS LONG)
    DIM pc AS STRING, nc AS STRING, endPI AS LONG, cm AS LONG
    DIM res AS LONG

    DO
        IF pi > LEN(pat$) THEN _Regex_Match& = ti: EXIT FUNCTION

        pc = MID$(pat$, pi, 1)
        IF pc = "$" AND pi = LEN(pat$) THEN
            IF ti > LEN(text$) THEN _Regex_Match& = ti ELSE _Regex_Match& = -1
            EXIT FUNCTION
        END IF

        ' peek at quantifier
        IF pi + 1 <= LEN(pat$) THEN nc = MID$(pat$, pi + 1, 1) ELSE nc = ""
        DIM isClass AS LONG
        isClass = 0
        DIM classEnd AS LONG
        classEnd = pi

        IF pc = "[" THEN
            isClass = -1
            ' find the class end to get the quantifier
            DIM cp AS LONG
            cp = pi + 1
            IF cp <= LEN(pat$) AND MID$(pat$, cp, 1) = "^" THEN cp = cp + 1
            DO WHILE cp <= LEN(pat$) AND MID$(pat$, cp, 1) <> "]": cp = cp + 1: LOOP
            cp = cp + 1
            classEnd = cp
            IF classEnd <= LEN(pat$) THEN nc = MID$(pat$, classEnd, 1) ELSE nc = ""
        END IF

        ' --- handle quantifiers: * + ? ---
        SELECT CASE nc
            CASE "*"
                DIM i AS LONG
                ' try from most to least (greedy): find max match first
                DIM maxI AS LONG
                maxI = ti
                DO
                    DIM matchedC AS LONG
                    IF isClass THEN
                        matchedC = _Regex_MatchClass&(text$, maxI, pat$, pi, endPI)
                    ELSEIF pc = "." THEN
                        matchedC = (maxI <= LEN(text$))
                    ELSE
                        matchedC = (maxI <= LEN(text$) AND MID$(text$, maxI, 1) = pc)
                    END IF
                    IF NOT matchedC THEN EXIT DO
                    maxI = maxI + 1
                LOOP
                ' backtrack
                DIM nextPI2 AS LONG
                IF isClass THEN nextPI2 = classEnd + 1 ELSE nextPI2 = pi + 2
                FOR i = maxI TO ti STEP -1
                    res = _Regex_Match&(text$, i, pat$, nextPI2)
                    IF res >= 0 THEN _Regex_Match& = res: EXIT FUNCTION
                NEXT i
                _Regex_Match& = -1: EXIT FUNCTION

            CASE "+"
                ' at least one
                DIM matched1 AS LONG
                IF isClass THEN
                    matched1 = _Regex_MatchClass&(text$, ti, pat$, pi, endPI)
                ELSEIF pc = "." THEN
                    matched1 = (ti <= LEN(text$))
                ELSE
                    matched1 = (ti <= LEN(text$) AND MID$(text$, ti, 1) = pc)
                END IF
                IF NOT matched1 THEN _Regex_Match& = -1: EXIT FUNCTION
                DIM nextPIp AS LONG
                IF isClass THEN nextPIp = classEnd ELSE nextPIp = pi + 1
                ' reuse * logic on remainder
                res = _Regex_Match&(text$, ti + 1, _
                      LEFT$(pat$, nextPIp - 1) + "*" + MID$(pat$, nextPIp + 1), pi)
                _Regex_Match& = res: EXIT FUNCTION

            CASE "?"
                DIM nextPIq AS LONG
                IF isClass THEN nextPIq = classEnd + 1 ELSE nextPIq = pi + 2
                ' try with and without consuming
                DIM matchedQ AS LONG
                IF isClass THEN
                    matchedQ = _Regex_MatchClass&(text$, ti, pat$, pi, endPI)
                ELSEIF pc = "." THEN
                    matchedQ = (ti <= LEN(text$))
                ELSE
                    matchedQ = (ti <= LEN(text$) AND MID$(text$, ti, 1) = pc)
                END IF
                IF matchedQ THEN
                    res = _Regex_Match&(text$, ti + 1, pat$, nextPIq)
                    IF res >= 0 THEN _Regex_Match& = res: EXIT FUNCTION
                END IF
                pi = nextPIq  ' skip quantifier, try without
        END SELECT

        ' no quantifier — match one
        IF pc = "^" THEN
            IF ti = 1 THEN pi = pi + 1: GOTO _regex_continue
            _Regex_Match& = -1: EXIT FUNCTION
        END IF

        DIM singleMatch AS LONG
        singleMatch = 0
        IF isClass THEN
            singleMatch = _Regex_MatchClass&(text$, ti, pat$, pi, endPI)
            IF singleMatch THEN pi = classEnd
        ELSEIF pc = "." THEN
            singleMatch = (ti <= LEN(text$))
            IF singleMatch THEN ti = ti + 1: pi = pi + 1: GOTO _regex_continue
        ELSE
            IF ti <= LEN(text$) AND MID$(text$, ti, 1) = pc THEN
                singleMatch = -1
                ti = ti + 1: pi = pi + 1: GOTO _regex_continue
            END IF
        END IF

        IF isClass AND singleMatch THEN
            ti = ti + 1
            pi = classEnd
            GOTO _regex_continue
        END IF

        _Regex_Match& = -1: EXIT FUNCTION
        _regex_continue:
    LOOP
END FUNCTION

' Public: returns -1 (TRUE) if text$ matches pattern$, 0 if not
FUNCTION RegexMatch& (text$, pattern$)
    DIM pi AS LONG, anchor AS LONG
    pi = 1: anchor = 0
    IF LEN(pattern$) > 0 AND LEFT$(pattern$, 1) = "^" THEN anchor = -1: pi = 2

    IF anchor THEN
        RegexMatch& = (_Regex_Match&(text$, 1, pattern$, pi) >= 0)
    ELSE
        ' try matching from each position
        DIM i AS LONG
        FOR i = 1 TO LEN(text$) + 1
            IF _Regex_Match&(text$, i, pattern$, pi) >= 0 THEN
                RegexMatch& = -1: EXIT FUNCTION
            END IF
        NEXT i
        RegexMatch& = 0
    END IF
END FUNCTION
