' QIR v1 emitter: statements lowered when they reach finishedline (the C++
' handler has created every symbol by then). IrLine2 records the statement.
' See utilities/ir_emit.bas.

SUB IrLine2 (ca$, n AS LONG, first$)
    IrStmt = ca$: IrStmtN = n: IrStmtFirst = first$
    IrLine
END SUB

FUNCTION IrEl$ (k AS LONG)
    IF k > IrStmtN THEN IrEl$ = "": EXIT FUNCTION
    IrEl$ = getelement$(IrStmt, k)
END FUNCTION

FUNCTION IrEls$ (a AS LONG, b AS LONG)
    IF b < a THEN IrEls$ = "": EXIT FUNCTION
    IrEls$ = getelements$(IrStmt, a, b)
END FUNCTION

'next top-level element index >= k equal to one of the separators ("|;|,|"), or n+1
FUNCTION IrFindSep& (k AS LONG, seps$)
    DIM i AS LONG, b AS LONG, e$
    FOR i = k TO IrStmtN
        e$ = UCASE$(IrEl$(i))
        IF e$ = "(" THEN b = b + 1
        IF e$ = ")" THEN b = b - 1
        IF b = 0 THEN
            IF INSTR(seps$, "|" + e$ + "|") THEN IrFindSep& = i: EXIT FUNCTION
        END IF
    NEXT
    IrFindSep& = IrStmtN + 1
END FUNCTION

FUNCTION IrExprAt$ (a AS LONG, b AS LONG, t AS LONG)
    IrExprAt$ = IrEval$(fixoperationorder$(IrEls$(a, b)), t)
END FUNCTION

FUNCTION IrPlaceAtRaw$ (a AS LONG, b AS LONG, t AS LONG)
    IrPlaceAtRaw$ = IrPlace$(fixoperationorder$(IrEls$(a, b)), t)
END FUNCTION

'file number at elements a..b (optional leading #) -> i32 register
FUNCTION IrChan$ (a0 AS LONG, b AS LONG)
    DIM t AS LONG, r$, a AS LONG
    a = a0
    IF IrEl$(a) = "#" THEN a = a + 1
    r$ = IrExprAt$(a, b, t)
    IrChan$ = IrCvt$(r$, IrT, "i32")
END FUNCTION

SUB IrStmtEnd
    DIM f$, s2$, n AS LONG, i AS LONG, j AS LONG, k AS LONG, t AS LONG, t2 AS LONG, p$, q$, x$, y$, z$, w$, ch$, m$
    DIM mv(1 TO 8) AS STRING
    f$ = UCASE$(IrStmtFirst): IrStmtFirst = ""
    IF f$ = "" OR IrFnCur = 0 THEN EXIT SUB
    n = IrStmtN
    s2$ = UCASE$(IrEl$(2))
    SELECT CASE f$
        CASE "MID$"
            j = IrMatch(IrStmt, 2, n)
            k = IrSplitArgs(IrEls$(3, j - 1))
            FOR i = 1 TO k: mv(i) = IrArgs(i): NEXT
            p$ = IrPlace$(fixoperationorder$(mv(1)), t)
            IF p$ = "" THEN EXIT SUB
            x$ = IrLoad$(p$)
            y$ = IrArgAs$(fixoperationorder$(mv(2)), "i32")
            z$ = IrEvalTo$(fixoperationorder$(IrEls$(j + 2, n)), ISSTRING)
            IF k >= 3 THEN
                w$ = IrArgAs$(fixoperationorder$(mv(3)), "i32")
                m$ = IrRt$("MIDSET.n", " " + x$ + " " + y$ + " " + w$ + " " + z$, "str")
            ELSE
                m$ = IrRt$("MIDSET", " " + x$ + " " + y$ + " " + z$, "str")
            END IF
            IrStore p$, m$
        CASE "LSET", "RSET"
            j = IrFindSep(2, "|=|")
            p$ = IrPlaceAtRaw$(2, j - 1, t)
            IF p$ = "" THEN EXIT SUB
            IF LEFT$(IrPlaceSTy$(p$), 4) = "udt:" THEN IrNotSup f$ + " of a TYPE": EXIT SUB
            x$ = IrLoad$(p$)
            z$ = IrEvalTo$(fixoperationorder$(IrEls$(j + 1, n)), ISSTRING)
            IrStore p$, IrRt$(f$, " " + x$ + " " + z$, "str")
        CASE "SWAP"
            j = IrFindSep(2, "|,|")
            p$ = IrPlaceAtRaw$(2, j - 1, t)
            q$ = IrPlaceAtRaw$(j + 1, n, t2)
            IF p$ = "" OR q$ = "" THEN EXIT SUB
            IF LEFT$(IrPlaceSTy$(p$), 4) = "udt:" THEN
                k = IrSize&(IrPlaceSTy$(p$))
                w$ = IrTemp$(IrPlaceSTy$(p$))
                z$ = "V" + CHR$(2) + w$ + CHR$(2) + IrPlaceSTy$(p$)
                IrE "copy " + IrAddr$(z$) + " " + IrAddr$(p$) + " " + str2$(k)
                IrE "copy " + IrAddr$(p$) + " " + IrAddr$(q$) + " " + str2$(k)
                IrE "copy " + IrAddr$(q$) + " " + IrAddr$(z$) + " " + str2$(k)
                EXIT SUB
            END IF
            x$ = IrLoad$(p$): y$ = IrLoad$(q$)
            IrStore p$, y$: IrStore q$, x$
        CASE "ERASE"
            i = 2
            DO WHILE i <= n
                j = IrFindSep(i, "|,|")
                IrQuiet = -1: p$ = IrPlace$(fixoperationorder$(IrEls$(i, j - 1) + sp + "(" + sp + ")"), t): IrQuiet = 0
                IF LEFT$(p$, 1) = "A" THEN IrE "erase " + IrPart$(p$, 2) ELSE IrNotSup "ERASE argument"
                i = j + 1
            LOOP
        CASE "CLOSE", "RESET"
            IF n = 1 THEN
                IrE "rt CLOSEALL"
            ELSE
                i = 2
                DO WHILE i <= n
                    j = IrFindSep(i, "|,|")
                    IrE "rt CLOSE " + IrChan$(i, j - 1)
                    i = j + 1
                LOOP
            END IF
        CASE "OPEN"
            IrOpen
        CASE "PRINT"
            IF IrEl$(2) = "#" THEN IrFilePrint
        CASE "WRITE"
            IrWrite
        CASE "INPUT"
            IrInput 0
        CASE "LINE"
            IF s2$ = "INPUT" THEN IrInput 1 ELSE IrNotSup "LINE (graphics)"
        CASE "GET", "PUT"
            IrGetPut f$
        CASE "SEEK"
            j = IrFindSep(2, "|,|")
            ch$ = IrChan$(2, j - 1)
            x$ = IrExprAt$(j + 1, n, t)
            IrE "rt SEEKSET " + ch$ + " " + IrCvt$(x$, IrT, "i64")
        CASE "NAME"
            j = IrFindSep(2, "|AS|")
            x$ = IrEvalTo$(fixoperationorder$(IrEls$(2, j - 1)), ISSTRING)
            y$ = IrEvalTo$(fixoperationorder$(IrEls$(j + 1, n)), ISSTRING)
            IrE "rt NAME " + x$ + " " + y$
        CASE "DEF"
            IF s2$ = "SEG" THEN
                IF n > 2 THEN IrE "rt DEFSEG " + IrCvt$(IrExprAt$(4, n, t), IrT, "i32") ELSE IrE "rt DEFSEG.d"
            END IF
        CASE "_MEMFILL", "_MEMPUT", "_MEMGET", "_MEMCOPY", "_MEMFREE"
            IrMemStmt f$
        CASE "PALETTE", "KEY", "FIELD", "ECHO", "CHAIN", "RUN", "INTERRUPTX", "_GL", "LPRINT", "CLEAR", "TIMER", "PLAY", "STRIG", "DRAW", "PSET", "PRESET", "CIRCLE", "PAINT", "WINDOW", "PCOPY"
            IrNotSup "statement " + f$
        CASE "ON"
            IF s2$ <> "ERROR" THEN
                IF s2$ = "TIMER" OR s2$ = "KEY" OR s2$ = "STRIG" OR s2$ = "PLAY" OR IrFindSep(3, "|GOTO|GOSUB|") > n THEN IrNotSup "ON " + s2$
            END IF
    END SELECT
END SUB

'OPEN f$ FOR mode [ACCESS ...] [LOCK ...] AS [#]n [LEN = r]   |   OPEN mode$, [#]n, f$[, r]
SUB IrOpen
    DIM n AS LONG, i AS LONG, j AS LONG, k AS LONG, a AS LONG, f$, mode AS LONG, ch$, ln$, md$, e$
    DIM gv(1 TO 8) AS STRING
    n = IrStmtN
    j = IrFindSep(2, "|FOR|AS|")
    IF j > n THEN
        k = IrSplitArgs(IrEls$(2, n))
        FOR i = 1 TO k: gv(i) = IrArgs(i): NEXT
        md$ = IrEvalTo$(fixoperationorder$(gv(1)), ISSTRING)
        e$ = gv(2): IF LEFT$(e$, 1) = "#" THEN e$ = MID$(e$, 3)
        ch$ = IrArgAs$(fixoperationorder$(e$), "i32")
        f$ = IrEvalTo$(fixoperationorder$(gv(3)), ISSTRING)
        IrE "rt OPENOLD " + f$ + " " + ch$ + " " + md$
        EXIT SUB
    END IF
    f$ = IrEvalTo$(fixoperationorder$(IrEls$(2, j - 1)), ISSTRING)
    mode = 5 'RANDOM
    IF UCASE$(IrEl$(j)) = "FOR" THEN
        SELECT CASE UCASE$(IrEl$(j + 1))
            CASE "INPUT": mode = 1
            CASE "OUTPUT": mode = 2
            CASE "APPEND": mode = 3
            CASE "BINARY": mode = 4
            CASE "RANDOM": mode = 5
        END SELECT
    END IF
    a = IrFindSep(j, "|AS|")
    k = IrFindSep(a + 1, "|LEN|")
    ch$ = IrChan$(a + 1, k - 1)
    IF k <= n THEN ln$ = IrArgAs$(fixoperationorder$(IrEls$(k + 2, n)), "i32") ELSE ln$ = IrI32$(0)
    IrE "rt OPEN " + f$ + " " + IrI32$(mode) + " " + ch$ + " " + ln$
END SUB

'PRINT #n, items
SUB IrFilePrint
    DIM n AS LONG, i AS LONG, j AS LONG, ch$, e$, last$, sep$, usng AS LONG
    n = IrStmtN
    j = IrFindSep(3, "|,|")
    ch$ = IrChan$(3, j - 1)
    i = j + 1
    IF UCASE$(IrEl$(i)) = "USING" THEN
        j = IrFindSep(i + 1, "|;|")
        IrE "rt USINGBEGIN " + ch$ + " " + IrEvalTo$(fixoperationorder$(IrEls$(i + 1, j - 1)), ISSTRING)
        i = j + 1: usng = -1
    END IF
    last$ = ""
    DO WHILE i <= n
        j = IrFindSep(i, "|;|,|")
        IF j > i THEN
            e$ = fixoperationorder$(IrEls$(i, j - 1))
            IF usng THEN IrUsingItem e$ ELSE IrPrintValue ch$, e$
        END IF
        sep$ = IrEl$(j)
        IF sep$ = "," AND usng = 0 THEN IrE "rt PRINTZONE " + ch$
        last$ = sep$
        i = j + 1
    LOOP
    IF usng THEN IrE "rt USINGEND"
    IF last$ = "" THEN IrE "rt PRINTNL " + ch$
END SUB

'one PRINT item to channel ch$ (numbers as STR$ + " ")
SUB IrPrintValue (ch$, e$)
    DIM r$, t AS LONG, sx$, u$
    u$ = UCASE$(getelement$(e$, 1))
    IF (u$ = "TAB" OR u$ = "SPC") AND getelement$(e$, 2) = "(" THEN
        r$ = IrEval$(getelements$(e$, 3, numelements(e$) - 1), t)
        r$ = IrCvt$(r$, IrT, "i64")
        IrE "rt PRINT" + u$ + " " + ch$ + " " + r$
        EXIT SUB
    END IF
    r$ = IrEval$(e$, t)
    IF t AND ISSTRING THEN
        IrE "rt PRINTS " + ch$ + " " + r$
    ELSE
        sx$ = IrNumSfx$(t)
        r$ = IrCvt$(r$, IrT, sx$)
        IrE "rt PRINTN." + sx$ + " " + ch$ + " " + r$
    END IF
END SUB

'WRITE [#n,] items
SUB IrWrite
    DIM n AS LONG, i AS LONG, j AS LONG, ch$, t AS LONG, r$, first AS LONG, sx$
    n = IrStmtN
    i = 2
    IF IrEl$(2) = "#" THEN
        j = IrFindSep(3, "|,|")
        ch$ = IrChan$(3, j - 1)
        i = j + 1
    ELSE
        ch$ = IrI32$(0)
    END IF
    first = -1
    DO WHILE i <= n
        j = IrFindSep(i, "|,|")
        IF first = 0 THEN IrE "rt WRITESEP " + ch$
        first = 0
        r$ = IrExprAt$(i, j - 1, t)
        IF t AND ISSTRING THEN
            IrE "rt WRITES " + ch$ + " " + r$
        ELSE
            sx$ = IrNumSfx$(t)
            r$ = IrCvt$(r$, IrT, sx$)
            IrE "rt WRITEN." + sx$ + " " + ch$ + " " + r$
        END IF
        i = j + 1
    LOOP
    IrE "rt PRINTNL " + ch$
END SUB

'INPUT [#n,] [prompt;|,] vars   /   LINE INPUT [#n,] [prompt;] var
SUB IrInput (lineinput AS LONG)
    DIM n AS LONG, i AS LONG, j AS LONG, ch$, t AS LONG, p$, r$, st$, e$, pr$
    n = IrStmtN
    i = 2 + lineinput
    IF IrEl$(i) = "#" THEN
        j = IrFindSep(i + 1, "|,|")
        ch$ = IrChan$(i + 1, j - 1)
        i = j + 1
    ELSE
        ch$ = IrI32$(0)
        IF IrEl$(i) = ";" THEN i = i + 1
        e$ = IrEl$(i)
        pr$ = "? ": IF lineinput THEN pr$ = ""
        IF LEFT$(e$, 1) = CHR$(34) AND (IrEl$(i + 1) = ";" OR IrEl$(i + 1) = ",") THEN
            pr$ = IrStrLit$(e$)
            IF IrEl$(i + 1) = ";" AND lineinput = 0 THEN pr$ = pr$ + "? "
            i = i + 2
        END IF
        IrE "rt INPUTPROMPT " + IrConstStr$(pr$) + " " + ch$
    END IF
    DO WHILE i <= n
        j = IrFindSep(i, "|,|")
        p$ = IrPlaceAtRaw$(i, j - 1, t)
        IF p$ = "" THEN EXIT SUB
        st$ = IrRegOf$(IrPlaceSTy$(p$))
        IF lineinput THEN
            r$ = IrRt$("LINEINPUT", " " + ch$, "str")
        ELSEIF st$ = "str" THEN
            r$ = IrRt$("INPUTITEM.s", " " + ch$, "str")
        ELSE
            r$ = IrRt$("INPUTITEM.n", " " + ch$, "f64")
            r$ = IrCvt$(r$, "f64", st$)
        END IF
        IrStore p$, r$
        i = j + 1
    LOOP
END SUB

'GET/PUT #n, [pos], var
SUB IrGetPut (f$)
    DIM n AS LONG, j AS LONG, k AS LONG, ch$, t AS LONG, p$, pos$, st$, r$, sz AS LONG
    n = IrStmtN
    IF IrEl$(2) = "(" OR UCASE$(IrEl$(2)) = "STEP" THEN IrNotSup f$ + " (graphics)": EXIT SUB
    j = IrFindSep(2, "|,|")
    ch$ = IrChan$(2, j - 1)
    k = IrFindSep(j + 1, "|,|")
    IF k > j + 1 THEN
        r$ = IrExprAt$(j + 1, k - 1, t)
        pos$ = IrCvt$(r$, IrT, "i64")
    ELSE
        pos$ = IrConst$("i64", "-1")
    END IF
    IF k > n THEN IrNotSup f$ + " without a variable": EXIT SUB
    p$ = IrPlaceAtRaw$(k + 1, n, t)
    IF p$ = "" THEN EXIT SUB
    IF LEFT$(p$, 1) = "A" THEN IrNotSup f$ + " of a whole array": EXIT SUB
    st$ = IrPlaceSTy$(p$)
    IF st$ = "str" THEN
        IF f$ = "PUT" THEN
            IrE "rt PUTSTR " + ch$ + " " + pos$ + " " + IrLoad$(p$)
        ELSE
            r$ = IrLoad$(p$)
            r$ = IrRt$("LEN", " " + r$, "i32")
            r$ = IrRt$("GETSTR", " " + ch$ + " " + pos$ + " " + r$, "str")
            IrStore p$, r$
        END IF
        EXIT SUB
    END IF
    sz = IrSize&(st$)
    IrE "rt " + f$ + "REC " + ch$ + " " + pos$ + " " + IrAddr$(p$) + " " + IrI32$(sz)
END SUB
