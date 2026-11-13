' QIR emitter: _MEM blocks (qb_runtime::mem). A _MEM variable is the
' _MEM UDT; builtins receive its address. See utilities/ir_emit.bas.

'address of the _MEM block named by fixed expression e$
FUNCTION IrMemBlk$ (e$)
    DIM p$, t AS LONG
    p$ = IrPlace$(e$, t)
    IF p$ = "" THEN IrMemBlk$ = IrConst$("ptr", "0"): EXIT FUNCTION
    IF IrPlaceSTy$(p$) <> "udt:T__MEM" THEN IrNotSup "expected a _MEM variable"
    IrMemBlk$ = IrAddr$(p$)
END FUNCTION

FUNCTION IrI64At$ (e$)
    DIM r$, t AS LONG
    r$ = IrEval$(e$, t)
    IrI64At$ = IrCvt$(r$, IrT, "i64")
END FUNCTION

'"AS type" elements -> QB type (0 = none)
FUNCTION IrAsType& (a$)
    DIM n AS LONG, i AS LONG, t$, t AS LONG
    n = numelements(a$)
    FOR i = 1 TO n
        IF UCASE$(getelement$(a$, i)) = "AS" THEN
            t$ = UCASE$(getelements$(a$, i + 1, n))
            DO WHILE INSTR(t$, sp): MID$(t$, INSTR(t$, sp), 1) = " ": LOOP
            t = typname2typ(t$)
            IF t AND ISPOINTER THEN t = t - ISPOINTER
            IrAsType& = t
            EXIT FUNCTION
        END IF
    NEXT
END FUNCTION

FUNCTION IrBeforeAs$ (a$)
    DIM n AS LONG, i AS LONG
    n = numelements(a$)
    FOR i = 1 TO n
        IF UCASE$(getelement$(a$, i)) = "AS" THEN IrBeforeAs$ = getelements$(a$, 1, i - 1): EXIT FUNCTION
    NEXT
    IrBeforeAs$ = a$
END FUNCTION

'value of expression e$ written to a new temporary of QB type t; returns its address, sets size
FUNCTION IrValueTemp$ (e$, t AS LONG, size AS LONG)
    DIM st$, tmp$, p$, r$
    st$ = IrSTy$(t, 0)
    size = IrSize&(st$)
    r$ = IrEvalTo$(e$, t)
    tmp$ = IrTemp$(st$)
    p$ = "V" + CHR$(2) + tmp$ + CHR$(2) + st$
    IrStore p$, r$
    IrValueTemp$ = IrAddr$(p$)
END FUNCTION

'm = _MEMNEW(n) / _MEM(x) / _MEM(offset, size): writes the block at dst$
'returns 0 if rhs$ is not one of those
FUNCTION IrMemAssign% (dst$, rhs$)
    DIM f$, n AS LONG, j AS LONG, inner$, k AS LONG, x$, y$, p$, t AS LONG, st$, sz AS LONG, mt AS LONG, es AS LONG, vn$, sy$
    DIM sv(1 TO 4) AS STRING
    n = numelements(rhs$)
    f$ = UCASE$(getelement$(rhs$, 1))
    IF f$ = "_MEMIMAGE" THEN
        'm = _MEMIMAGE / _MEMIMAGE(handle): the pixels of the destination image or of an image
        IF n = 1 THEN
            IrE "rt _MEMIMAGE " + dst$ + " " + IrI32$(0) + " " + IrI32$(0)
            IrMemAssign% = -1
            EXIT FUNCTION
        END IF
        IF getelement$(rhs$, 2) <> "(" THEN EXIT FUNCTION
        IF IrMatch(rhs$, 2, n) <> n THEN EXIT FUNCTION
        x$ = IrCvt$(IrEval$(getelements$(rhs$, 3, n - 1), t), IrT, "i32")
        IrE "rt _MEMIMAGE " + dst$ + " " + x$ + " " + IrI32$(1)
        IrMemAssign% = -1
        EXIT FUNCTION
    END IF
    IF (f$ <> "_MEMNEW" AND f$ <> "_MEM" AND f$ <> "_MEMSOUND") OR getelement$(rhs$, 2) <> "(" THEN EXIT FUNCTION
    j = IrMatch(rhs$, 2, n)
    IF j <> n THEN EXIT FUNCTION
    IrMemAssign% = -1
    inner$ = getelements$(rhs$, 3, n - 1)
    IF f$ = "_MEMNEW" THEN
        IrE "rt _MEMNEW " + dst$ + " " + IrI64At$(inner$)
        EXIT FUNCTION
    END IF
    k = IrSplitArgs(inner$)
    FOR j = 1 TO k: sv(j) = IrArgs(j): NEXT
    IF f$ = "_MEMSOUND" THEN
        x$ = IrCvt$(IrEval$(sv(1), t), IrT, "i32")
        IF k >= 2 THEN y$ = IrCvt$(IrEval$(sv(2), t), IrT, "i32") ELSE y$ = IrI32$(0)
        IrE "rt _MEMSOUND " + dst$ + " " + x$ + " " + y$
        EXIT FUNCTION
    END IF
    IF k = 2 THEN
        IrE "rt _MEMAT " + dst$ + " " + IrI64At$(sv(1)) + " " + IrI64At$(sv(2))
        EXIT FUNCTION
    END IF
    IrQuiet = -1: p$ = IrPlace$(sv(1), t): IrQuiet = 0
    IF p$ = "" THEN IrNotSup "_MEM argument": EXIT FUNCTION
    IF LEFT$(p$, 1) = "A" THEN
        x$ = IrAddr$(p$)
        y$ = IrReg$: IrE "rt " + y$ + " _ARRADDR " + x$
        sz = 0
        z$ = IrReg$: IrE "rt " + z$ + " _ARRBYTES " + x$
        mt = Type2MemTypeValue(t + ISARRAY)
        es = IrSize&(IrSTy$(t, 0))
        IF t AND ISSTRING THEN 'STRING * n: the length is the array's (the place holds only its name)
            vn$ = getelement$(sv(1), 1): sy$ = removesymbol$(vn$)
            k = IrFindArrId&(vn$): IF k THEN es = ids(k).tsize
        END IF
        IF (t AND ISSTRING) <> 0 AND (t AND ISFIXEDLENGTH) = 0 THEN IrNotSup "_MEM of a variable-length STRING array (its elements are not stored together); use _MEM of one STRING variable": EXIT FUNCTION
        IrE "rt _MEMVAR " + dst$ + " " + y$ + " " + z$ + " " + IrConst$("i64", str2$(mt)) + " " + IrConst$("i64", str2$(es))
        EXIT FUNCTION
    END IF
    st$ = IrPlaceSTy$(p$)
    IF st$ = "str" THEN
        'variable-length STRING (an extension): its current characters, one byte per element
        IrE "rt _MEMSTR " + dst$ + " " + IrAddr$(p$) + " " + IrConst$("i64", str2$(Type2MemTypeValue(t)))
        EXIT FUNCTION
    END IF
    sz = IrSize&(st$)
    mt = Type2MemTypeValue(t)
    x$ = IrAddr$(p$)
    IrE "rt _MEMVAR " + dst$ + " " + x$ + " " + IrConst$("i64", str2$(sz)) + " " + IrConst$("i64", str2$(mt)) + " " + IrConst$("i64", str2$(sz))
END FUNCTION

'statements _MEMFILL, _MEMPUT, _MEMGET, _MEMCOPY, _MEMFREE (raw statement in IrStmt)
SUB IrMemStmt (f$)
    DIM n AS LONG, k AS LONG, i AS LONG, j AS LONG, blk$, off$, x$, y$, t AS LONG, sz AS LONG, p$, st$, b2$
    DIM sv(1 TO 8) AS STRING
    n = IrStmtN
    IF f$ = "_MEMCOPY" THEN
        j = IrFindSep(2, "|TO|")
        k = IrSplitArgs(IrEls$(2, j - 1))
        FOR i = 1 TO k: sv(i) = fixoperationorder$(IrArgs(i)): NEXT
        k = IrSplitArgs(IrEls$(j + 1, n))
        sv(4) = fixoperationorder$(IrArgs(1)): sv(5) = fixoperationorder$(IrArgs(2))
        blk$ = IrMemBlk$(sv(1)): off$ = IrI64At$(sv(2)): x$ = IrI64At$(sv(3))
        b2$ = IrMemBlk$(sv(4)): y$ = IrI64At$(sv(5))
        IrE "rt _MEMCOPY " + blk$ + " " + off$ + " " + x$ + " " + b2$ + " " + y$
        EXIT SUB
    END IF
    k = IrSplitArgs(IrEls$(2, n))
    FOR i = 1 TO k: sv(i) = IrArgs(i): NEXT
    blk$ = IrMemBlk$(fixoperationorder$(sv(1)))
    IF f$ = "_MEMFREE" THEN IrE "rt _MEMFREE " + blk$: EXIT SUB
    off$ = IrI64At$(fixoperationorder$(sv(2)))
    SELECT CASE f$
        CASE "_MEMFILL"
            x$ = IrI64At$(fixoperationorder$(sv(3)))
            t = IrAsType&(sv(4))
            IF t = 0 THEN
                'without AS: the bytes of a variable (as QB64 does), or of a STRING value
                IrQuiet = -1: p$ = IrPlace$(fixoperationorder$(sv(4)), t): IrQuiet = 0
                IF LEN(p$) AND LEFT$(p$, 1) <> "A" AND IrPlaceSTy$(p$) <> "str" THEN
                    sz = IrSize&(IrPlaceSTy$(p$))
                    IrE "rt _MEMFILLP " + blk$ + " " + off$ + " " + x$ + " " + IrAddr$(p$) + " " + IrConst$("i64", str2$(sz))
                    EXIT SUB
                END IF
                y$ = IrEval$(fixoperationorder$(sv(4)), t)
                IF t AND ISSTRING THEN IrE "rt _MEMFILLS " + blk$ + " " + off$ + " " + x$ + " " + y$: EXIT SUB
                'a value that is not a variable (an extension): its bytes in its own type
                IF t AND ISPOINTER THEN t = t - ISPOINTER
                st$ = IrSTy$(t, 0): sz = IrSize&(st$)
                p$ = "V" + CHR$(2) + IrTemp$(st$) + CHR$(2) + st$
                IrStore p$, IrCvt$(y$, IrT, IrTy$(t))
                IrE "rt _MEMFILLP " + blk$ + " " + off$ + " " + x$ + " " + IrAddr$(p$) + " " + IrConst$("i64", str2$(sz))
                EXIT SUB
            END IF
            y$ = IrValueTemp$(fixoperationorder$(IrBeforeAs$(sv(4))), t, sz)
            IrE "rt _MEMFILLP " + blk$ + " " + off$ + " " + x$ + " " + y$ + " " + IrConst$("i64", str2$(sz))
        CASE "_MEMPUT"
            t = IrAsType&(sv(3))
            IF t = 0 THEN
                IrQuiet = -1: p$ = IrPlace$(fixoperationorder$(sv(3)), t): IrQuiet = 0
                IF LEN(p$) AND LEFT$(p$, 1) <> "A" AND IrPlaceSTy$(p$) <> "str" THEN
                    sz = IrSize&(IrPlaceSTy$(p$))
                    y$ = IrReg$: IrE "rt " + y$ + " _MEMGETP " + blk$ + " " + off$ + " " + IrConst$("i64", str2$(sz))
                    IrE "copy " + y$ + " " + IrAddr$(p$) + " " + str2$(sz)
                    EXIT SUB
                END IF
                y$ = IrEval$(fixoperationorder$(sv(3)), t)
                IF t AND ISSTRING THEN IrE "rt _MEMPUTS " + blk$ + " " + off$ + " " + y$: EXIT SUB
                IrNotSup "_MEMPUT value": EXIT SUB
            END IF
            st$ = IrSTy$(t, 0): sz = IrSize&(st$)
            x$ = IrEvalTo$(fixoperationorder$(IrBeforeAs$(sv(3))), t)
            y$ = IrReg$: IrE "rt " + y$ + " _MEMGETP " + blk$ + " " + off$ + " " + IrConst$("i64", str2$(sz))
            IrE "st." + IrTy$(t) + " " + y$ + " 0 " + x$
        CASE "_MEMGET"
            p$ = IrPlace$(fixoperationorder$(sv(3)), t)
            IF p$ = "" THEN EXIT SUB
            st$ = IrPlaceSTy$(p$)
            IF st$ = "str" THEN IrE "rt _MEMGETS " + blk$ + " " + off$ + " " + IrAddr$(p$): EXIT SUB
            sz = IrSize&(st$)
            y$ = IrReg$: IrE "rt " + y$ + " _MEMGETP " + blk$ + " " + off$ + " " + IrConst$("i64", str2$(sz))
            IrE "copy " + IrAddr$(p$) + " " + y$ + " " + str2$(sz)
    END SELECT
END SUB

'_MEMGET(m, off, type) and _MEMEXISTS(m) in expressions
FUNCTION IrMemFunc$ (nm$, nargs AS LONG, typ AS LONG)
    DIM blk$, off$, t AS LONG, y$, d$, t$
    DIM sv(1 TO 4) AS STRING
    FOR t = 1 TO nargs: sv(t) = IrArgs(t): NEXT
    blk$ = IrMemBlk$(sv(1))
    IF nm$ = "_MEMEXISTS" THEN IrMemFunc$ = IrRt$("_MEMEXISTS", " " + blk$, "i32"): EXIT FUNCTION
    off$ = IrI64At$(sv(2))
    t$ = UCASE$(sv(3))
    DO WHILE INSTR(t$, sp): MID$(t$, INSTR(t$, sp), 1) = " ": LOOP
    t = typname2typ(t$)
    IF t AND ISPOINTER THEN t = t - ISPOINTER
    typ = t
    y$ = IrReg$: IrE "rt " + y$ + " _MEMGETP " + blk$ + " " + off$ + " " + IrConst$("i64", str2$(IrSize&(IrSTy$(t, 0))))
    d$ = IrReg$: IrE "ld." + IrTy$(t) + " " + d$ + " " + y$ + " 0"
    IrT = IrTy$(t)
    IrMemFunc$ = d$
END FUNCTION
