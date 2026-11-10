'===============================================================================
' QIR emission of the R13 expression extensions (same results as the C++ path in
' semantics.bas / qbnex.bas):
'   _ANDALSO / _ORELSE   short-circuit, -1/0
'   _IIF(c, a, b)        only the chosen value is evaluated
'   _TOSTR$(x)           x for a string, LTRIM$(STR$(x)) for a number
'   _ASSERT c[, msg$]    only with $ASSERTS: error 314 (315 after echoing msg$ in
'                        a console program) when c truncates to 0
' Every call that emits code is stored in a variable before it is used in a string
' expression: string + is evaluated right to left by the C++ build of the compiler
' and left to right by the VM build, and the register numbers must not depend on it.
'===============================================================================

FUNCTION IrExtTruth$ (e$)
    'value of e$ as -1/0 (i32)
    DIM r$, t AS LONG, rt$, z$, d$
    r$ = IrEval$(e$, t)
    rt$ = IrT
    IF rt$ = "str" THEN IrNotSup "string used as a condition": IrT = "i32": IrExtTruth$ = r$: EXIT FUNCTION
    z$ = IrConst$(rt$, "0")
    d$ = IrCmp$("ne", r$, rt$, z$, rt$)
    IrT = "i32"
    IrExtTruth$ = d$
END FUNCTION

FUNCTION IrExtShortCircuit% (a2$, typ AS LONG, result$)
    'expressions with a top-level _ANDALSO/_ORELSE: split at the last operator of the lowest
    'precedence; a short-circuit operator there gets jumps, any other operator gets its two
    'sides bracketed so that each side is evaluated on its own (returns 0: not handled here)
    DIM n AS LONG, i AS LONG, j AS LONG, b AS LONG, l$, lv AS LONG, best AS LONG, bestAt AS LONG
    DIM lhs$, rhs$, op$, tmp$, lbl$, c1$, c2$, v$, d$, prevOp AS INTEGER, topSc AS INTEGER
    IF INSTR(UCASE$(a2$), "_ANDALSO") = 0 AND INSTR(UCASE$(a2$), "_ORELSE") = 0 THEN EXIT FUNCTION
    n = numelements(a2$)
    best = 99: prevOp = -1
    FOR i = 1 TO n
        l$ = getelement$(a2$, i)
        IF l$ = "(" OR l$ = "{" THEN
            j = IrMatch(a2$, i, n): i = j: prevOp = 0
        ELSE
            lv = isoperator(l$)
            IF UCASE$(l$) = "_ANDALSO" OR UCASE$(l$) = "_ORELSE" THEN topSc = -1
            IF lv THEN
                'a sign or NOT right after an operator (or at the start) is unary
                IF prevOp = 0 OR lv < 6 THEN
                    IF lv <= best THEN best = lv: bestAt = i
                END IF
                prevOp = -1
            ELSE
                prevOp = 0
            END IF
        END IF
    NEXT
    IF topSc = 0 THEN EXIT FUNCTION 'only inside brackets: the normal path recurses into them
    IF bestAt <= 1 OR bestAt >= n THEN EXIT FUNCTION
    op$ = UCASE$(getelement$(a2$, bestAt))
    lhs$ = getelements$(a2$, 1, bestAt - 1)
    rhs$ = getelements$(a2$, bestAt + 1, n)
    IF op$ <> "_ANDALSO" AND op$ <> "_ORELSE" THEN
        'the short-circuit operator is deeper: bracket both sides and use the normal path
        result$ = IrEval$("(" + sp + lhs$ + sp + ")" + sp + getelement$(a2$, bestAt) + sp + "(" + sp + rhs$ + sp + ")", typ)
        IrExtShortCircuit% = -1
        EXIT FUNCTION
    END IF
    tmp$ = IrTemp$("i32")
    lbl$ = "~S" + str2$(IrNew)
    IF op$ = "_ANDALSO" THEN
        v$ = IrI32$(0)
        IrE "store.i32 " + tmp$ + " " + v$
        c1$ = IrExtTruth$(lhs$)
        IrE "jz " + c1$ + " " + lbl$ + ".E"
    ELSE
        v$ = IrI32$(-1)
        IrE "store.i32 " + tmp$ + " " + v$
        c1$ = IrExtTruth$(lhs$)
        IrE "jz " + c1$ + " " + lbl$ + ".R"
        IrE "jmp " + lbl$ + ".E"
        IrE "label " + lbl$ + ".R"
    END IF
    c2$ = IrExtTruth$(rhs$)
    IrE "store.i32 " + tmp$ + " " + c2$
    IrE "label " + lbl$ + ".E"
    d$ = IrReg$
    IrE "load.i32 " + d$ + " " + tmp$
    IrT = "i32"
    typ = 32 'as IrEval$ types a comparison (result 8)
    result$ = d$
    IrExtShortCircuit% = -1
END FUNCTION

FUNCTION IrExtCapture$ (e$, t AS LONG, code$, rt$)
    'evaluates e$ with its code captured into code$ instead of the function body
    DIM saved$, r$
    saved$ = IrFnBody(IrFnCur)
    IrFnBody(IrFnCur) = ""
    r$ = IrEval$(e$, t)
    rt$ = IrT
    code$ = IrFnBody(IrFnCur)
    IrFnBody(IrFnCur) = saved$
    IrExtCapture$ = r$
END FUNCTION

FUNCTION IrExtFunc% (nm$, sv() AS STRING, nargs AS LONG, typ AS LONG, result$)
    'R13 functions; returns 0 when nm$ is not one of them
    DIM r$, t AS LONG, sx$, x$, y$, c$, codeA$, codeB$, ra$, rb$, ta$, tb$, ct$, tA AS LONG, tB AS LONG
    DIM tmp$, lbl$, d$, bitsA AS LONG, bitsB AS LONG
    SELECT CASE nm$
        CASE "_TOSTR$"
            x$ = IrEval$(sv(1), t)
            IF IrT = "str" THEN
                r$ = x$
            ELSE
                sx$ = IrNumSfx$(t)
                y$ = IrCvt$(x$, IrT, sx$)
                x$ = IrRt$("STR$." + sx$, " " + y$, "str")
                r$ = IrRt$("LTRIM$", " " + x$, "str")
            END IF
            IrT = "str"
            typ = STRINGTYPE - ISPOINTER
        CASE "_IIF"
            IF nargs <> 3 THEN IrNotSup "_IIF needs 3 arguments": r$ = IrI32$(0): IrT = "i32": GOTO irextdone
            'both values are generated first (captured) to know the result type, then
            'placed in their branches
            ra$ = IrExtCapture$(sv(2), tA, codeA$, ta$)
            rb$ = IrExtCapture$(sv(3), tB, codeB$, tb$)
            IF (ta$ = "str") <> (tb$ = "str") THEN IrNotSup "_IIF values must both be strings or both be numbers"
            IF ta$ = "str" THEN
                ct$ = "str": typ = STRINGTYPE - ISPOINTER
            ELSEIF IrIsFloat(ta$) OR IrIsFloat(tb$) THEN
                ct$ = "f64": typ = DOUBLETYPE - ISPOINTER
                IF ta$ = "f80" OR tb$ = "f80" THEN ct$ = "f80": typ = FLOATTYPE - ISPOINTER
            ELSE
                ct$ = "i64": typ = INTEGER64TYPE - ISPOINTER
                bitsA = tA AND 511: bitsB = tB AND 511
                IF bitsA <= 32 AND bitsB <= 32 AND (tA AND ISUNSIGNED) = 0 AND (tB AND ISUNSIGNED) = 0 THEN ct$ = "i32": typ = LONGTYPE - ISPOINTER
            END IF
            tmp$ = IrTemp$(ct$)
            lbl$ = "~F" + str2$(IrNew)
            c$ = IrExtTruth$(sv(1))
            IrE "jz " + c$ + " " + lbl$ + ".B"
            IrFnBody(IrFnCur) = IrFnBody(IrFnCur) + codeA$
            IF ct$ <> "str" THEN x$ = IrCvt$(ra$, ta$, ct$) ELSE x$ = ra$
            IrE "store." + ct$ + " " + tmp$ + " " + x$
            IrE "jmp " + lbl$ + ".E"
            IrE "label " + lbl$ + ".B"
            IrFnBody(IrFnCur) = IrFnBody(IrFnCur) + codeB$
            IF ct$ <> "str" THEN y$ = IrCvt$(rb$, tb$, ct$) ELSE y$ = rb$
            IrE "store." + ct$ + " " + tmp$ + " " + y$
            IrE "label " + lbl$ + ".E"
            d$ = IrReg$
            IrE "load." + ct$ + " " + d$ + " " + tmp$
            IrT = ct$
            r$ = d$
        CASE ELSE
            EXIT FUNCTION
    END SELECT
    irextdone:
    result$ = r$
    IrExtFunc% = -1
END FUNCTION

FUNCTION IrExtSub% (nm$, sv() AS STRING, nargs AS LONG)
    'R13 statements; returns 0 when nm$ is not one of them
    DIM r$, t AS LONG, c$, lbl$, e$
    SELECT CASE nm$
        CASE "_ASSERT"
            IF Asserts = 0 THEN IrExtSub% = -1: EXIT FUNCTION
            'the C++ path passes (int32)(condition): a float condition is truncated
            r$ = IrEval$(sv(1), t)
            c$ = IrTrunc$(r$, IrT, "i32")
            lbl$ = "~A" + str2$(IrNew)
            IrE "jz " + c$ + " " + lbl$ + ".F"
            IrE "jmp " + lbl$ + ".E"
            IrE "label " + lbl$ + ".F"
            IF Console <> 0 AND nargs >= 2 AND LEN(sv(2)) > 0 THEN
                IrPrintStr "0", sv(2)
                IrRtCh "PRINTNL", "0"
                e$ = IrI32$(315)
            ELSE
                e$ = IrI32$(314)
            END IF
            IrE "error " + e$
            IrE "label " + lbl$ + ".E"
        CASE ELSE
            EXIT FUNCTION
    END SELECT
    IrExtSub% = -1
END FUNCTION
