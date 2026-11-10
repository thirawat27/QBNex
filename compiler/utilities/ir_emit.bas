' QIR v1 emitter (`qb --ir file.bas`). Spec: docs/rust-port/qir-v1.md
'
' Runs inside the real compiler pipeline: the main pass still does all of its
' usual work (symbol table, types, UDTs, $INCLUDE, $IF, CONST, DEFxxx, C++
' fragments that are simply not compiled) and statement handlers call the
' Ir* routines below to emit QIR as well. Expressions are lowered by IrEval$,
' which walks the same element strings evaluate$ gets (after fixoperationorder)
' and applies the same operator rules (operatorusage), so values carry the C
' type the C++ backend would compute. At the end the QIR is packaged with the
' prebuilt runner crates/qb-exec/bin/<os>/qb-exec instead of running g++.

'================================================================ output

SUB IrInit
    IrFnN = 1: IrFnName(1) = "main": IrFnCur = 1
    IrMod = "": IrModDecls = "|": IrUdtDone = "|": IrFailMsg = ""
END SUB

SUB IrNotSup (w$)
    IF IrFailMsg = "" THEN IrFailMsg = "line" + STR$(linenumber) + ": " + w$
END SUB

SUB IrE (s$)
    IF IrFnCur = 0 THEN EXIT SUB
    IrFnBody(IrFnCur) = IrFnBody(IrFnCur) + s$ + CHR$(10)
END SUB

SUB IrHead (s$)
    IF IrFnCur = 0 THEN EXIT SUB
    IrFnHead(IrFnCur) = IrFnHead(IrFnCur) + s$ + CHR$(10)
END SUB

FUNCTION IrReg$
    IF IrFnCur = 0 THEN IrReg$ = "%0": EXIT FUNCTION
    IrReg$ = "%" + str2$(IrFnRegs(IrFnCur))
    IrFnRegs(IrFnCur) = IrFnRegs(IrFnCur) + 1
END FUNCTION

FUNCTION IrNew& ()
    IrUniq = IrUniq + 1
    IrNew& = IrUniq
END FUNCTION

FUNCTION IrHex$ (s$)
    DIM i AS LONG, h$
    h$ = SPACE$(LEN(s$) * 2)
    FOR i = 1 TO LEN(s$)
        MID$(h$, i * 2 - 1, 2) = RIGHT$("0" + HEX$(ASC(s$, i)), 2)
    NEXT
    IrHex$ = "x" + h$
END FUNCTION

FUNCTION IrFindFn& (nm$)
    DIM i AS LONG
    FOR i = 1 TO IrFnN
        IF IrFnName(i) = nm$ THEN IrFindFn& = i: EXIT FUNCTION
    NEXT
    IrFnN = IrFnN + 1
    IF IrFnN > UBOUND(IrFnName) THEN
        i = UBOUND(IrFnName) * 2
        REDIM _PRESERVE IrFnName(1 TO i) AS STRING, IrFnRet(1 TO i) AS STRING
        REDIM _PRESERVE IrFnHead(1 TO i) AS STRING, IrFnBody(1 TO i) AS STRING
        REDIM _PRESERVE IrFnDecls(1 TO i) AS STRING, IrFnRetVar(1 TO i) AS STRING
        REDIM _PRESERVE IrFnRegs(1 TO i) AS LONG
    END IF
    IrFnName(IrFnN) = nm$
    IrFindFn& = IrFnN
END FUNCTION

'called at SUB/FUNCTION (after the return variable exists)
SUB IrBeginFn (nm$, ret$)
    IrFnCur = IrFindFn(nm$)
    IrFnRet(IrFnCur) = ret$
END SUB

'END SUB / END FUNCTION
SUB IrEndFn
    DIM r$
    IF IrFnCur <= 1 THEN EXIT SUB
    IrE "label ~EXIT"
    IF LEN(IrFnRet(IrFnCur)) THEN
        r$ = IrReg$
        IrE "load." + IrFnRet(IrFnCur) + " " + r$ + " " + IrFnRetVar(IrFnCur)
        IrE "ret " + r$
    ELSE
        IrE "ret"
    END IF
    IrFnCur = 0
END SUB

'main module code ends where the first SUB/FUNCTION starts
SUB IrCloseMain
    IF IrFnCur = 1 THEN IrFnCur = 0
END SUB

SUB IrLine
    IrE "line " + str2$(linenumber)
    IF IrPendLL THEN IrE "rt LASTLINE " + IrI32$(IrPendLL): IrPendLL = 0
END SUB

FUNCTION IrLbl$ (l$)
    IrLbl$ = "L_" + l$
END FUNCTION

SUB IrLabelDef (l$)
    IrE "label " + IrLbl$(l$)
    IrMod = IrMod + "datalabel " + IrLbl$(l$) + CHR$(10)
END SUB

'DATA statement: raw text of the items
SUB IrData (t$)
    DIM i AS LONG, q AS LONG, c$, item$, started AS LONG
    i = 1
    DO
        item$ = "": q = 0: started = 0
        DO WHILE i <= LEN(t$)
            c$ = MID$(t$, i, 1)
            IF c$ = CHR$(34) THEN
                q = 1 - q: started = 1
            ELSEIF c$ = "," AND q = 0 THEN
                EXIT DO
            ELSE
                item$ = item$ + c$
            END IF
            i = i + 1
        LOOP
        IF started = 0 THEN item$ = LTRIM$(RTRIM$(item$))
        IrMod = IrMod + "data " + IrHex$(item$) + CHR$(10)
        IrDataN = IrDataN + 1
        IF i > LEN(t$) THEN EXIT DO
        i = i + 1
    LOOP
END SUB

'================================================================ types

FUNCTION IrTy$ (t AS LONG)
    DIM b AS LONG, r$
    IF t AND ISSTRING THEN IrTy$ = "str": EXIT FUNCTION
    IF t AND ISUDT THEN IrTy$ = "ptr": EXIT FUNCTION
    'a _BIT * n value lives in a whole i64 (u64); stores wrap it to n bits (IrBitWrap$)
    IF t AND ISOFFSETINBITS THEN
        IF t AND ISUNSIGNED THEN IrTy$ = "u64" ELSE IrTy$ = "i64"
        EXIT FUNCTION
    END IF
    b = t AND 511
    IF t AND ISFLOAT THEN
        IF b = 32 THEN r$ = "f32" ELSE IF b = 64 THEN r$ = "f64" ELSE r$ = "f80"
        IrTy$ = r$: EXIT FUNCTION
    END IF
    IF b <= 8 THEN
        r$ = "8"
    ELSEIF b <= 16 THEN
        r$ = "16"
    ELSEIF b <= 32 THEN
        r$ = "32"
    ELSE
        r$ = "64"
    END IF
    IF t AND ISUNSIGNED THEN r$ = "u" + r$ ELSE r$ = "i" + r$
    IrTy$ = r$
END FUNCTION

FUNCTION IrSTy$ (t AS LONG, tsize AS LONG)
    IF (t AND ISSTRING) <> 0 AND (t AND ISFIXEDLENGTH) <> 0 THEN IrSTy$ = "fstr:" + str2$(tsize): EXIT FUNCTION
    IF t AND ISUDT THEN IrSTy$ = "udt:" + IrUdt$(t AND 511): EXIT FUNCTION
    IrSTy$ = IrTy$(t)
END FUNCTION

FUNCTION IrUdt$ (u AS LONG)
    DIM nm$, s$, o AS LONG, e AS LONG, ft AS LONG, fty$
    nm$ = "T_" + RTRIM$(udtxname(u))
    IrUdt$ = nm$
    IF INSTR(IrUdtDone, "|" + nm$ + "|") THEN EXIT FUNCTION
    IrUdtDone = IrUdtDone + nm$ + "|"
    s$ = "type " + nm$ + " " + str2$((udtxsize(u) + 7) \ 8) + CHR$(10)
    o = 0: e = udtxnext(u)
    DO WHILE e
        IF udtebytealign(e) THEN
            IF o MOD 8 THEN o = o + (8 - (o MOD 8))
        END IF
        ft = udtetype(e)
        IF ft AND ISUDT THEN
            fty$ = "udt:" + IrUdt$(ft AND 511)
        ELSEIF (ft AND ISSTRING) <> 0 AND (ft AND ISFIXEDLENGTH) <> 0 THEN
            fty$ = "fstr:" + str2$(udtetypesize(e))
        ELSEIF ft AND ISSTRING THEN
            IrNotSup "variable-length STRING inside TYPE": fty$ = "str"
        ELSE
            fty$ = IrTy$(ft)
        END IF
        fn$ = RTRIM$(udtename(e)): DO WHILE INSTR(fn$, "$"): MID$(fn$, INSTR(fn$, "$"), 1) = "_": LOOP
        s$ = s$ + "field " + fn$ + " " + fty$ + " " + str2$(o \ 8) + CHR$(10)
        o = o + udtesize(e)
        e = udtenext(e)
    LOOP
    IrMod = IrMod + s$ + "endtype" + CHR$(10)
END FUNCTION

FUNCTION IrIsFloat% (t$)
    IrIsFloat% = (LEFT$(t$, 1) = "f")
END FUNCTION

FUNCTION IrIsInt% (t$)
    IrIsInt% = (LEFT$(t$, 1) = "i" OR LEFT$(t$, 1) = "u")
END FUNCTION

'C integer promotion
FUNCTION IrPromote$ (t$)
    SELECT CASE t$
        CASE "i8", "u8", "i16", "u16": IrPromote$ = "i32"
        CASE ELSE: IrPromote$ = t$
    END SELECT
END FUNCTION

'C usual arithmetic conversions
FUNCTION IrCommon$ (a$, b$)
    DIM pa$, pb$
    IF IrIsFloat(a$) OR IrIsFloat(b$) THEN
        IF a$ = "f80" OR b$ = "f80" THEN IrCommon$ = "f80": EXIT FUNCTION
        IF a$ = "f64" OR b$ = "f64" THEN IrCommon$ = "f64": EXIT FUNCTION
        IrCommon$ = "f32": EXIT FUNCTION
    END IF
    pa$ = IrPromote$(a$): pb$ = IrPromote$(b$)
    IF pa$ = pb$ THEN IrCommon$ = pa$: EXIT FUNCTION
    IF pa$ = "u64" OR pb$ = "u64" THEN IrCommon$ = "u64": EXIT FUNCTION
    IF pa$ = "i64" OR pb$ = "i64" THEN IrCommon$ = "i64": EXIT FUNCTION
    IF pa$ = "u32" OR pb$ = "u32" THEN IrCommon$ = "u32": EXIT FUNCTION
    IrCommon$ = "i32"
END FUNCTION

'numeric conversion (float->int rounds half to even, like qbr)
FUNCTION IrCvt$ (r$, from$, to$)
    DIM d$
    IF from$ = to$ OR to$ = "" THEN IrCvt$ = r$: EXIT FUNCTION
    IF from$ = "str" OR to$ = "str" OR from$ = "ptr" OR to$ = "ptr" THEN
        IrNotSup "type mismatch (" + from$ + " -> " + to$ + ")": IrCvt$ = r$: EXIT FUNCTION
    END IF
    d$ = IrReg$
    IrE "cvt." + to$ + "." + from$ + " " + d$ + " " + r$
    IrCvt$ = d$
END FUNCTION

'C cast (float->int truncates)
FUNCTION IrTrunc$ (r$, from$, to$)
    DIM d$
    IF from$ = to$ THEN IrTrunc$ = r$: EXIT FUNCTION
    d$ = IrReg$
    IF IrIsFloat(from$) AND IrIsInt(to$) THEN
        IrE "trunc." + to$ + "." + from$ + " " + d$ + " " + r$
    ELSE
        IrE "cvt." + to$ + "." + from$ + " " + d$ + " " + r$
    END IF
    IrTrunc$ = d$
END FUNCTION

FUNCTION IrConst$ (t$, v$)
    DIM d$
    d$ = IrReg$
    IrE "const." + t$ + " " + d$ + " " + v$
    IrConst$ = d$
END FUNCTION

FUNCTION IrConstStr$ (s$)
    IrConstStr$ = IrConst$("str", IrHex$(s$))
END FUNCTION

FUNCTION IrI32$ (v AS LONG)
    IrI32$ = IrConst$("i32", str2$(v))
END FUNCTION

'================================================================ variables

FUNCTION IrVar$ (idn AS LONG)
    DIM nm$, sc$, st$, f AS LONG, kw$
    nm$ = "$" + LTRIM$(RTRIM$(removecast$(RTRIM$(ids(idn).callname))))
    IrVar$ = nm$
    IF ids(idn).sfarg > 0 THEN EXIT FUNCTION 'parameter: declared with its SUB
    IF ids(idn).arraytype THEN
        st$ = "array " + IrSTy$(ids(idn).arraytype, ids(idn).tsize)
    ELSE
        st$ = IrSTy$(ids(idn).t, ids(idn).tsize)
    END IF
    sc$ = RTRIM$(ids(idn).insubfunc)
    IF sc$ = "" THEN
        IF INSTR(IrModDecls, "|" + nm$ + "|") = 0 THEN
            IrModDecls = IrModDecls + nm$ + "|"
            IrMod = IrMod + "global " + nm$ + " " + st$ + CHR$(10)
        END IF
    ELSE
        f = IrFindFn(sc$)
        IF INSTR(IrFnDecls(f), "|" + nm$ + "|") = 0 THEN
            IrFnDecls(f) = IrFnDecls(f) + "|" + nm$ + "|"
            kw$ = "local": IF ids(idn).staticscope THEN kw$ = "static"
            IrFnHead(f) = IrFnHead(f) + kw$ + " " + nm$ + " " + st$ + CHR$(10)
        END IF
    END IF
END FUNCTION

'a SUB/FUNCTION parameter, in order
SUB IrParam (idn AS LONG)
    DIM nm$, st$
    nm$ = "$" + LTRIM$(RTRIM$(removecast$(RTRIM$(ids(idn).callname))))
    IF ids(idn).arraytype THEN
        st$ = "array " + IrSTy$(ids(idn).arraytype, ids(idn).tsize) + " byref"
    ELSE
        st$ = IrSTy$(ids(idn).t, ids(idn).tsize) + " byref"
    END IF
    IrHead "param " + nm$ + " " + st$
    IrFnDecls(IrFnCur) = IrFnDecls(IrFnCur) + "|" + nm$ + "|"
END SUB

'temporary local of a storage type, returns its name
FUNCTION IrTemp$ (st$)
    DIM nm$
    nm$ = "$~T" + str2$(IrNew)
    IrHead "local " + nm$ + " " + st$
    IrTemp$ = nm$
END FUNCTION

'================================================================ places
' "V" sep name sep storagetype        (scalar variable)
' "P" sep ptrreg sep offset sep stype (memory)
' "A" sep name                        (whole array, for passing)

FUNCTION IrPart$ (p$, k AS LONG)
    DIM i AS LONG, j AS LONG, n AS LONG
    i = 1
    FOR n = 1 TO k - 1
        i = INSTR(i, p$, CHR$(2)) + 1
    NEXT
    j = INSTR(i, p$, CHR$(2)): IF j = 0 THEN j = LEN(p$) + 1
    IrPart$ = MID$(p$, i, j - i)
END FUNCTION

FUNCTION IrPlaceSTy$ (p$)
    IF LEFT$(p$, 1) = "V" THEN IrPlaceSTy$ = IrPart$(p$, 3) ELSE IrPlaceSTy$ = IrPart$(p$, 4)
END FUNCTION

'register type used for the value of a storage type
FUNCTION IrRegOf$ (st$)
    IF LEFT$(st$, 5) = "fstr:" THEN IrRegOf$ = "str": EXIT FUNCTION
    IF LEFT$(st$, 4) = "udt:" THEN IrRegOf$ = "": EXIT FUNCTION
    IrRegOf$ = st$
END FUNCTION

FUNCTION IrLoad$ (p$)
    DIM st$, d$
    st$ = IrPlaceSTy$(p$)
    IrT = IrRegOf$(st$)
    IF IrT = "" THEN IrNotSup "user defined type used as a value": IrT = "i32": IrLoad$ = IrI32$(0): EXIT FUNCTION
    d$ = IrReg$
    IF LEFT$(p$, 1) = "V" THEN
        IrE "load." + IrT + " " + d$ + " " + IrPart$(p$, 2)
    ELSEIF LEFT$(st$, 5) = "fstr:" THEN
        IrE "ldfs " + d$ + " " + IrPart$(p$, 2) + " " + IrPart$(p$, 3) + " " + MID$(st$, 6)
    ELSE
        IrE "ld." + IrT + " " + d$ + " " + IrPart$(p$, 2) + " " + IrPart$(p$, 3)
    END IF
    IrLoad$ = d$
END FUNCTION

'stores r$ (already of the place's register type)
SUB IrStore (p$, r$)
    DIM st$, t$
    st$ = IrPlaceSTy$(p$)
    t$ = IrRegOf$(st$)
    IF LEFT$(p$, 1) = "V" THEN
        IrE "store." + t$ + " " + IrPart$(p$, 2) + " " + r$
    ELSEIF LEFT$(st$, 5) = "fstr:" THEN
        IrE "stfs " + IrPart$(p$, 2) + " " + IrPart$(p$, 3) + " " + MID$(st$, 6) + " " + r$
    ELSE
        IrE "st." + t$ + " " + IrPart$(p$, 2) + " " + IrPart$(p$, 3) + " " + r$
    END IF
END SUB

FUNCTION IrAddr$ (p$)
    DIM d$, o$
    IF LEFT$(p$, 1) = "V" THEN
        d$ = IrReg$: IrE "addr " + d$ + " " + IrPart$(p$, 2): IrAddr$ = d$: EXIT FUNCTION
    END IF
    IF LEFT$(p$, 1) = "A" THEN
        d$ = IrReg$: IrE "aref " + d$ + " " + IrPart$(p$, 2): IrAddr$ = d$: EXIT FUNCTION
    END IF
    IF IrPart$(p$, 3) = "0" THEN IrAddr$ = IrPart$(p$, 2): EXIT FUNCTION
    o$ = IrConst$("i64", IrPart$(p$, 3))
    d$ = IrReg$: IrE "padd " + d$ + " " + IrPart$(p$, 2) + " " + o$
    IrAddr$ = d$
END FUNCTION

'size in bytes of a storage type
FUNCTION IrSize& (st$)
    DIM i AS LONG
    IF LEFT$(st$, 5) = "fstr:" THEN IrSize& = VAL(MID$(st$, 6)): EXIT FUNCTION
    IF LEFT$(st$, 4) = "udt:" THEN
        FOR i = 1 TO lasttype
            IF "T_" + RTRIM$(udtxname(i)) = MID$(st$, 5) THEN IrSize& = (udtxsize(i) + 7) \ 8: EXIT FUNCTION
        NEXT
    END IF
    SELECT CASE st$
        CASE "i8", "u8": IrSize& = 1
        CASE "i16", "u16": IrSize& = 2
        CASE "i32", "u32", "f32": IrSize& = 4
        CASE "f80": IrSize& = 32
        CASE ELSE: IrSize& = 8
    END SELECT
END FUNCTION

'index of the matching ")" for the "(" at element i
FUNCTION IrMatch& (a$, i AS LONG, n AS LONG)
    DIM b AS LONG, j AS LONG, e$
    FOR j = i TO n
        e$ = getelement$(a$, j)
        IF e$ = "(" OR e$ = "{" THEN b = b + 1
        IF e$ = ")" OR e$ = "}" THEN
            b = b - 1
            IF b = 0 THEN IrMatch& = j: EXIT FUNCTION
        END IF
    NEXT
    IrMatch& = n
END FUNCTION

'splits a$ at top-level commas into IrArgs(); returns count
FUNCTION IrSplitArgs& (a$)
    DIM n AS LONG, i AS LONG, b AS LONG, e$, c AS LONG, cur$
    n = numelements(a$)
    IF n = 0 THEN IrSplitArgs& = 0: EXIT FUNCTION
    c = 1: cur$ = ""
    FOR i = 1 TO n
        e$ = getelement$(a$, i)
        IF e$ = "(" OR e$ = "{" THEN b = b + 1
        IF e$ = ")" OR e$ = "}" THEN b = b - 1
        IF e$ = "," AND b = 0 THEN
            IrArgs(c) = cur$: c = c + 1: cur$ = ""
        ELSE
            IF LEN(cur$) THEN cur$ = cur$ + sp + e$ ELSE cur$ = e$
        END IF
    NEXT
    IrArgs(c) = cur$
    IrSplitArgs& = c
END FUNCTION

'follows ".field.field" (elements i..n of a$) from base place p$ of UDT u
FUNCTION IrUdtPlace$ (p$, u0 AS LONG, a$, i AS LONG, n AS LONG, typ AS LONG)
    DIM u AS LONG, e AS LONG, o AS LONG, nm$, base$, boff AS LONG, st$, ft AS LONG, sx$
    u = u0
    IF LEFT$(p$, 1) = "V" THEN
        base$ = IrAddr$(p$): boff = 0
    ELSE
        base$ = IrPart$(p$, 2): boff = VAL(IrPart$(p$, 3))
    END IF
    typ = u + ISUDT
    st$ = "udt:" + IrUdt$(u)
    DO WHILE i <= n
        IF getelement$(a$, i) <> "." THEN EXIT DO
        nm$ = UCASE$(getelement$(a$, i + 1))
        sx$ = removesymbol$(nm$): IF Error_Happened THEN Error_Happened = 0
        ft = 0
        e = udtxnext(u): o = 0
        DO WHILE e
            IF udtebytealign(e) THEN
                IF o MOD 8 THEN o = o + (8 - (o MOD 8))
            END IF
            IF UCASE$(RTRIM$(udtename(e))) = nm$ THEN EXIT DO
            o = o + udtesize(e)
            e = udtenext(e)
        LOOP
        IF e = 0 THEN IrNotSup "UDT element " + nm$: EXIT DO
        boff = boff + o \ 8
        ft = udtetype(e)
        IF ft AND ISUDT THEN
            u = ft AND 511: typ = u + ISUDT: st$ = "udt:" + IrUdt$(u)
        ELSEIF (ft AND ISSTRING) <> 0 AND (ft AND ISFIXEDLENGTH) <> 0 THEN
            typ = STRINGTYPE + ISFIXEDLENGTH: st$ = "fstr:" + str2$(udtetypesize(e))
        ELSE
            typ = ft: st$ = IrTy$(ft)
        END IF
        i = i + 2
    LOOP
    IrUdtPlace$ = "P" + CHR$(2) + base$ + CHR$(2) + str2$(boff) + CHR$(2) + st$
END FUNCTION

FUNCTION IrIndex$ (e$)
    DIM r$, t AS LONG
    r$ = IrEval$(e$, t)
    IF IrIsFloat(IrT) THEN r$ = IrCvt$(r$, IrT, "i64"): IrT = "i64"
    IrIndex$ = r$
END FUNCTION

'element of array idn with raw index elements idx$
FUNCTION IrElemPlace$ (idn AS LONG, idx$, typ AS LONG)
    DIM c AS LONG, k AS LONG, regs$, nm$, d$, at AS LONG
    DIM sv(1 TO 64) AS STRING
    nm$ = IrVar$(idn)
    c = IrSplitArgs(idx$)
    FOR k = 1 TO c: sv(k) = IrArgs(k): NEXT
    FOR k = 1 TO c
        regs$ = regs$ + " " + IrIndex$(sv(k))
    NEXT
    d$ = IrReg$
    IrE "aaddr " + d$ + " " + nm$ + regs$
    at = ids(idn).arraytype
    typ = at
    IrElemPlace$ = "P" + CHR$(2) + d$ + CHR$(2) + "0" + CHR$(2) + IrSTy$(at, ids(idn).tsize)
END FUNCTION

'lvalue at elements i.. of a$; advances i. Returns "" (IrQuiet) or fails if not a variable.
FUNCTION IrPlaceAt$ (a$, i AS LONG, n AS LONG, typ AS LONG)
    DIM l$, l2$, nextl$, dtyp$, try AS LONG, try_method AS LONG, v AS LONG, j AS LONG, p$, idn AS LONG
    l2$ = getelement$(a$, i)
    IF i < n THEN nextl$ = getelement$(a$, i + 1) ELSE nextl$ = ""
    FOR try_method = 1 TO 4
        l$ = l2$
        IF try_method = 2 OR try_method = 4 THEN
            dtyp$ = removesymbol(l$): IF Error_Happened THEN dtyp$ = "": Error_Happened = 0
            IF LEN(dtyp$) = 0 THEN
                IF isoperator(l$) = 0 AND isvalidvariable(l$) THEN
                    IF LEFT$(l$, 1) = "_" THEN v = 27 ELSE v = ASC(UCASE$(l$)) - 64
                    l$ = l$ + defineextaz(v)
                END IF
            ELSE
                l$ = l2$
            END IF
        END IF
        try = findid(l$)
        IF Error_Happened THEN Error_Happened = 0: try = 0
        DO WHILE try
            IF (subfuncn = id.insubfuncn AND try_method <= 2) OR try_method >= 3 THEN
                idn = currentid
                IF nextl$ = "(" AND id.arraytype <> 0 THEN
                    j = IrMatch(a$, i + 1, n)
                    IF j = i + 2 THEN 'a() : whole array
                        i = j + 1: typ = id.arraytype
                        p$ = IrVar$(idn)
                        IrPlaceAt$ = "A" + CHR$(2) + p$: EXIT FUNCTION
                    END IF
                    p$ = IrElemPlace$(idn, getelements$(a$, i + 2, j - 1), typ)
                    i = j + 1
                    IF typ AND ISUDT THEN p$ = IrUdtPlace$(p$, typ AND 511, a$, i, n, typ)
                    IrPlaceAt$ = p$: EXIT FUNCTION
                END IF
                IF nextl$ <> "(" AND id.t <> 0 AND id.subfunc = 0 THEN
                    IF id.t AND ISUDT THEN
                        p$ = "V" + CHR$(2) + IrVar$(idn) + CHR$(2) + IrSTy$(id.t, id.tsize)
                        i = i + 1
                        p$ = IrUdtPlace$(p$, id.t AND 511, a$, i, n, typ)
                        IrPlaceAt$ = p$: EXIT FUNCTION
                    END IF
                    typ = id.t
                    i = i + 1
                    IrPlaceAt$ = "V" + CHR$(2) + IrVar$(idn) + CHR$(2) + IrSTy$(id.t, id.tsize): EXIT FUNCTION
                END IF
            END IF
            IF try = 2 THEN findanotherid = 1: try = findid(l$) ELSE try = 0
            IF Error_Happened THEN Error_Happened = 0: try = 0
        LOOP
    NEXT
    IF IrQuiet = 0 THEN IrNotSup "not a variable: " + l2$
    IrPlaceAt$ = ""
END FUNCTION

'lvalue of a whole fixed expression (assignment target, BYREF argument, INPUT/READ target)
FUNCTION IrPlace$ (a$, typ AS LONG)
    DIM i AS LONG, n AS LONG, p$, saveid AS idstruct, savecur AS LONG
    saveid = id: savecur = currentid
    n = numelements(a$): i = 1
    p$ = IrPlaceAt$(a$, i, n, typ)
    IF i <= n THEN
        IF IrQuiet = 0 AND LEN(p$) THEN IrNotSup "unexpected element after variable"
        p$ = ""
    END IF
    id = saveid: currentid = savecur
    IrPlace$ = p$
END FUNCTION

'================================================================ expressions

'C string literal element -> bytes
FUNCTION IrStrLit$ (l$)
    DIM x AS LONG, s$, r$, i AS LONG, c$
    x = _INSTRREV(l$, CHR$(34))
    s$ = MID$(l$, 2, x - 2)
    i = 1
    DO WHILE i <= LEN(s$)
        c$ = MID$(s$, i, 1)
        IF c$ = "\" THEN
            IF MID$(s$, i + 1, 1) = "\" THEN
                r$ = r$ + "\": i = i + 2
            ELSE
                r$ = r$ + CHR$((ASC(s$, i + 1) - 48) * 64 + (ASC(s$, i + 2) - 48) * 8 + (ASC(s$, i + 3) - 48)): i = i + 4
            END IF
        ELSE
            r$ = r$ + c$: i = i + 1
        END IF
    LOOP
    IrStrLit$ = r$
END FUNCTION

FUNCTION IrNumLit$ (l$, typ AS LONG)
    DIM num$, x AS LONG, f AS LONG, sym$, t$, v AS DOUBLE
    num$ = l$
    x = INSTR(num$, "E")
    IF x THEN
        f = 1: typ = SINGLETYPE - ISPOINTER
    ELSE
        x = INSTR(num$, "D")
        IF x THEN
            f = 2: typ = DOUBLETYPE - ISPOINTER
        ELSE
            x = INSTR(num$, "F")
            IF x THEN f = 3: typ = FLOATTYPE - ISPOINTER
        END IF
    END IF
    IF f THEN
        MID$(num$, x, 1) = "E"
        IF f = 3 THEN t$ = "f80" ELSE t$ = "f64"
        IrT = t$
        IrNumLit$ = IrConst$(t$, num$)
        EXIT FUNCTION
    END IF
    sym$ = removesymbol$(num$)
    typ = typname2typ(sym$)
    IF typ AND ISPOINTER THEN typ = typ - ISPOINTER
    IF (typ AND 511) > 32 THEN
        IF typ AND ISUNSIGNED THEN t$ = "u64" ELSE t$ = "i64"
    ELSE
        v = VAL(num$)
        IF v >= -2147483648# AND v <= 2147483647# THEN t$ = "i32" ELSE t$ = "i64"
    END IF
    IrT = t$
    IrNumLit$ = IrConst$(t$, num$)
END FUNCTION

FUNCTION IrOperand$ (a$, i AS LONG, n AS LONG, typ AS LONG)
    DIM l$, c AS LONG, r$, p$, nextl$, l2$, dtyp$, try AS LONG, try_method AS LONG, v AS LONG, j AS LONG, idn AS LONG
    l$ = getelement$(a$, i)
    c = ASC(l$)
    IF c = 34 THEN
        i = i + 1: typ = ISSTRING: IrT = "str"
        IrOperand$ = IrConstStr$(IrStrLit$(l$)): EXIT FUNCTION
    END IF
    IF c = 45 OR (c >= 48 AND c <= 57) THEN
        i = i + 1
        IrOperand$ = IrNumLit$(l$, typ): EXIT FUNCTION
    END IF
    'function?
    l2$ = l$
    IF i < n THEN nextl$ = getelement$(a$, i + 1) ELSE nextl$ = ""
    FOR try_method = 1 TO 4
        l$ = l2$
        IF try_method = 2 OR try_method = 4 THEN
            dtyp$ = removesymbol(l$): IF Error_Happened THEN dtyp$ = "": Error_Happened = 0
            IF LEN(dtyp$) = 0 THEN
                IF isoperator(l$) = 0 AND isvalidvariable(l$) THEN
                    IF LEFT$(l$, 1) = "_" THEN v = 27 ELSE v = ASC(UCASE$(l$)) - 64
                    l$ = l$ + defineextaz(v)
                END IF
            ELSE
                l$ = l2$
            END IF
        END IF
        try = findid(l$)
        IF Error_Happened THEN Error_Happened = 0: try = 0
        DO WHILE try
            IF (subfuncn = id.insubfuncn AND try_method <= 2) OR try_method >= 3 THEN
                IF (nextl$ = "(" AND id.arraytype <> 0) OR (nextl$ <> "(" AND id.t <> 0 AND id.subfunc = 0) THEN
                    p$ = IrPlaceAt$(a$, i, n, typ)
                    IF LEN(p$) = 0 THEN IrT = "i32": IrOperand$ = IrI32$(0): EXIT FUNCTION
                    IF LEFT$(p$, 1) = "A" THEN IrNotSup "array used as a value": IrT = "i32": IrOperand$ = IrI32$(0): EXIT FUNCTION
                    r$ = IrLoad$(p$)
                    IF typ AND ISFIXEDLENGTH THEN typ = ISSTRING
                    IrOperand$ = r$: EXIT FUNCTION
                END IF
            END IF
            IF id.subfunc = 1 THEN
                idn = currentid
                IF nextl$ = "(" THEN
                    j = IrMatch(a$, i + 1, n)
                    r$ = IrCallFunc$(idn, getelements$(a$, i + 2, j - 1), typ)
                    i = j + 1
                ELSE
                    r$ = IrCallFunc$(idn, "", typ)
                    i = i + 1
                END IF
                IrOperand$ = r$: EXIT FUNCTION
            END IF
            IF try = 2 THEN findanotherid = 1: try = findid(l$) ELSE try = 0
            IF Error_Happened THEN Error_Happened = 0: try = 0
        LOOP
    NEXT
    IrNotSup "unknown name in expression: " + l2$
    i = i + 1: IrT = "i32": typ = LONGTYPE - ISPOINTER
    IrOperand$ = IrI32$(0)
END FUNCTION

'applies a numeric binary C operator to (a$,ta$) and (b$,tb$)
FUNCTION IrArith$ (op$, a$, ta$, b$, tb$)
    DIM ct$, x$, y$, d$
    ct$ = IrCommon$(ta$, tb$)
    x$ = IrCvt$(a$, ta$, ct$): y$ = IrCvt$(b$, tb$, ct$)
    IF op$ = "div" AND IrIsInt(ct$) THEN op$ = "idiv"
    d$ = IrReg$
    IrE op$ + "." + ct$ + " " + d$ + " " + x$ + " " + y$
    IrT = ct$
    IrArith$ = d$
END FUNCTION

FUNCTION IrCmp$ (op$, a$, ta$, b$, tb$)
    DIM ct$, x$, y$, d$
    IF ta$ = "str" THEN
        ct$ = "str": x$ = a$: y$ = b$
    ELSE
        ct$ = IrCommon$(ta$, tb$)
        x$ = IrCvt$(a$, ta$, ct$): y$ = IrCvt$(b$, tb$, ct$)
    END IF
    d$ = IrReg$
    IrE op$ + "." + ct$ + " " + d$ + " " + x$ + " " + y$
    IrT = "i32"
    IrCmp$ = d$
END FUNCTION

FUNCTION IrEval$ (a2$, typ AS LONG)
    DIM blk(1 TO 200) AS STRING, bty(1 TO 200) AS LONG, bq(1 TO 200) AS STRING, bop(1 TO 200) AS INTEGER
    DIM n AS LONG, i AS LONG, j AS LONG, nb AS LONG, l$, t AS LONG, r$
    DIM o$, u AS LONG, info$, lhs AS LONG, rhs AS LONG, result AS LONG, isop AS LONG
    DIM oldtyp AS LONG, newtyp AS LONG, lr$, lq$, rr$, rq$, b AS LONG, b2 AS LONG, b3 AS LONG, s1 AS LONG, s2 AS LONG
    DIM saveid AS idstruct, savecur AS LONG, unary AS LONG, d$, op$
    saveid = id: savecur = currentid
    n = numelements(a2$)
    i = 1: nb = 0
    DO WHILE i <= n
        l$ = getelement$(a2$, i)
        nb = nb + 1
        IF l$ = "(" OR l$ = "{" THEN
            j = IrMatch(a2$, i, n)
            blk(nb) = IrEval$(getelements$(a2$, i + 1, j - 1), t)
            bty(nb) = t: bq(nb) = IrT: bop(nb) = 0
            i = j + 1
        ELSEIF isoperator(l$) THEN
            blk(nb) = l$: bop(nb) = 1: i = i + 1
        ELSE
            blk(nb) = IrOperand$(a2$, i, n, t)
            bty(nb) = t: bq(nb) = IrT: bop(nb) = 0
        END IF
    LOOP
    IF nb = 0 THEN IrNotSup "empty expression": IrT = "i32": typ = LONGTYPE - ISPOINTER: IrEval$ = IrI32$(0): EXIT FUNCTION

    typ = bty(1): IF bop(1) THEN typ = bty(2)
    i = 1
    DO WHILE i <= nb
        IF bop(i) THEN
            o$ = blk(i)
            isop = isoperator(o$)
            u = operatorusage(o$, typ, info$, lhs, rhs, result)
            unary = (u = 5)
            newtyp = bty(i + 1): rr$ = blk(i + 1): rq$ = bq(i + 1)
            IF unary THEN
                oldtyp = newtyp: lr$ = "": lq$ = ""
            ELSE
                oldtyp = typ: lr$ = blk(i - 1): lq$ = bq(i - 1)
            END IF
            'offset protection (same rules as evaluate$)
            offmode = 0: offcvi = 0
            IF (oldtyp AND ISOFFSET) <> 0 OR (newtyp AND ISOFFSET) <> 0 THEN
                offmode = 2
                IF newtyp AND ISOFFSET THEN
                    IF (newtyp AND ISUNSIGNED) = 0 THEN offmode = 1
                END IF
                IF oldtyp AND ISOFFSET THEN
                    IF (oldtyp AND ISUNSIGNED) = 0 THEN offmode = 1
                END IF
                IF o$ = "*" OR o$ = "/" OR o$ = "^" THEN
                    IF o$ = "*" OR o$ = "^" THEN
                        IF (newtyp AND ISFLOAT) <> 0 OR (oldtyp AND ISFLOAT) <> 0 THEN
                            offcvi = 1
                            IF (oldtyp AND ISFLOAT) = 0 THEN lhs = 2
                            IF (newtyp AND ISFLOAT) = 0 THEN rhs = 2
                        END IF
                    END IF
                    IF o$ = "/" OR o$ = "^" THEN
                        offcvi = 1
                        IF (oldtyp AND ISFLOAT) = 0 THEN lhs = 2
                        IF (newtyp AND ISFLOAT) = 0 THEN rhs = 2
                    END IF
                ELSE
                    IF lhs AND 2 THEN lhs = 1
                    IF rhs AND 2 THEN rhs = 1
                END IF
                IF result = 2 THEN result = 1
            END IF
            'STEP 1: conversions demanded by the operator
            IF unary = 0 AND (oldtyp AND ISSTRING) = 0 THEN
                IF oldtyp AND ISFLOAT THEN
                    IF (lhs AND 2) = 0 THEN lr$ = IrCvt$(lr$, lq$, "i64"): lq$ = "i64": oldtyp = 64&
                ELSE
                    IF (lhs AND 1) = 0 THEN lr$ = IrCvt$(lr$, lq$, "f80"): lq$ = "f80": oldtyp = 256& + ISFLOAT
                END IF
            END IF
            IF (newtyp AND ISSTRING) = 0 THEN
                IF newtyp AND ISFLOAT THEN
                    IF (rhs AND 2) = 0 THEN rr$ = IrCvt$(rr$, rq$, "i64"): rq$ = "i64": newtyp = 64&
                ELSE
                    IF (rhs AND 1) = 0 THEN rr$ = IrCvt$(rr$, rq$, "f80"): rq$ = "f80": newtyp = 256& + ISFLOAT
                END IF
            END IF
            IF unary THEN oldtyp = newtyp
            IF isop = 7 AND unary = 0 THEN
                IF (oldtyp AND ISFLOAT) <> 0 AND (newtyp AND ISFLOAT) <> 0 THEN
                    s1 = oldtyp AND 511: s2 = newtyp AND 511
                    IF s2 < s1 THEN s1 = s2
                    IF s1 = 32 THEN
                        lr$ = IrCvt$(lr$, lq$, "f32"): lq$ = "f32": oldtyp = 32& + ISFLOAT
                        rr$ = IrCvt$(rr$, rq$, "f32"): rq$ = "f32": newtyp = 32& + ISFLOAT
                    END IF
                    IF s1 = 64 THEN
                        lr$ = IrCvt$(lr$, lq$, "f64"): lq$ = "f64": oldtyp = 64& + ISFLOAT
                        rr$ = IrCvt$(rr$, rq$, "f64"): rq$ = "f64": newtyp = 64& + ISFLOAT
                    END IF
                END IF
            END IF
            'STEP 2: markup of the QB type (same rules as evaluate$)
            typ = newtyp
            IF (oldtyp AND ISSTRING) = 0 AND (newtyp AND ISSTRING) = 0 THEN
                IF (oldtyp AND ISFLOAT) <> 0 OR (newtyp AND ISFLOAT) <> 0 THEN
                    b = 0: IF (oldtyp AND ISFLOAT) THEN b = oldtyp AND 511
                    IF (newtyp AND ISFLOAT) THEN
                        b2 = newtyp AND 511: IF b2 > b THEN b = b2
                    END IF
                    typ = ISFLOAT + b
                ELSE
                    b = oldtyp AND 511: b2 = newtyp AND 511: IF b2 > b THEN b = b2
                    typ = 64&
                    IF b = 64 THEN
                        IF (oldtyp AND ISUNSIGNED) <> 0 AND (newtyp AND ISUNSIGNED) <> 0 THEN typ = 64& + ISUNSIGNED
                    END IF
                END IF
            END IF
            IF result = 1 THEN
                IF (typ AND ISFLOAT) <> 0 OR (typ AND ISSTRING) <> 0 THEN typ = 64
            END IF
            IF result = 2 THEN
                IF (typ AND ISFLOAT) = 0 THEN typ = ISFLOAT + 256
            END IF
            IF result = 4 THEN typ = ISSTRING
            IF result = 8 THEN typ = 32
            IF offmode THEN
                IF result <> 8 THEN
                    typ = OFFSETTYPE - ISPOINTER: IF offmode = 2 THEN typ = typ + ISUNSIGNED
                END IF
            END IF
            IF u = 2 AND info$ = "pow2" THEN
                b = oldtyp AND 511
                IF (oldtyp AND ISFLOAT) = 0 THEN
                    IF b > 16 THEN b = 64
                    IF b > 32 THEN b = 256
                    IF b <= 16 THEN b = 32
                END IF
                b2 = newtyp AND 511
                IF newtyp AND ISFLOAT THEN
                    IF b2 > b THEN b = b2
                ELSE
                    b3 = 32
                    IF b2 > 16 THEN b3 = 64
                    IF b2 > 32 THEN b3 = 256
                    IF b3 > b THEN b = b3
                END IF
                typ = ISFLOAT + b
            END IF
            'STEP 3: apply
            d$ = ""
            SELECT CASE u
                CASE 5
                    IF info$ = "-" THEN
                        d$ = IrReg$: q$ = IrPromote$(rq$)
                        rr$ = IrCvt$(rr$, rq$, q$)
                        IrE "neg." + q$ + " " + d$ + " " + rr$: IrT = q$
                    ELSE
                        d$ = IrReg$: q$ = IrPromote$(rq$)
                        rr$ = IrCvt$(rr$, rq$, q$)
                        IrE "not." + q$ + " " + d$ + " " + rr$: IrT = q$
                    END IF
                CASE 1
                    SELECT CASE info$
                        CASE "+": op$ = "add"
                        CASE "-": op$ = "sub"
                        CASE "*": op$ = "mul"
                        CASE "/ ": op$ = "div"
                        CASE "%": op$ = "mod"
                        CASE "^": op$ = "xor"
                        CASE "|": op$ = "or"
                        CASE "&": op$ = "and"
                        CASE ELSE: IrNotSup "operator " + info$: op$ = "add"
                    END SELECT
                    d$ = IrArith$(op$, lr$, lq$, rr$, rq$)
                CASE 2
                    SELECT CASE info$
                        CASE "pow2"
                            lr$ = IrCvt$(lr$, lq$, "f80"): rr$ = IrCvt$(rr$, rq$, "f80")
                            d$ = IrReg$: IrE "pow.f80 " + d$ + " " + lr$ + " " + rr$: IrT = "f80"
                        CASE "qbs_add"
                            d$ = IrReg$: IrE "concat " + d$ + " " + lr$ + " " + rr$: IrT = "str"
                        CASE "qbs_equal": d$ = IrCmp$("eq", lr$, "str", rr$, "str")
                        CASE "qbs_notequal": d$ = IrCmp$("ne", lr$, "str", rr$, "str")
                        CASE "qbs_greaterthan": d$ = IrCmp$("gt", lr$, "str", rr$, "str")
                        CASE "qbs_lessthan": d$ = IrCmp$("lt", lr$, "str", rr$, "str")
                        CASE "qbs_greaterorequal": d$ = IrCmp$("ge", lr$, "str", rr$, "str")
                        CASE "qbs_lessorequal": d$ = IrCmp$("le", lr$, "str", rr$, "str")
                    END SELECT
                CASE 3
                    SELECT CASE info$
                        CASE "==": op$ = "eq"
                        CASE "!=": op$ = "ne"
                        CASE "<": op$ = "lt"
                        CASE ">": op$ = "gt"
                        CASE "<=": op$ = "le"
                        CASE ">=": op$ = "ge"
                    END SELECT
                    d$ = IrCmp$(op$, lr$, lq$, rr$, rq$)
                CASE 4
                    q$ = IrPromote$(lq$)
                    lr$ = IrCvt$(lr$, lq$, q$)
                    d$ = IrReg$: IrE "not." + q$ + " " + d$ + " " + lr$
                    IF info$ = "|" THEN op$ = "or" ELSE op$ = "xor"
                    d$ = IrArith$(op$, d$, q$, rr$, rq$)
            END SELECT
            IF d$ = "" THEN IrNotSup "operator " + o$: d$ = IrI32$(0): IrT = "i32"
            IF offcvi THEN d$ = IrCvt$(d$, IrT, "i64"): IrT = "i64"
            blk(i + 1) = d$: bq(i + 1) = IrT: bty(i + 1) = typ: bop(i + 1) = 0
            i = i + 2
        ELSE
            i = i + 1
        END IF
    LOOP
    IrT = bq(nb)
    IF nb = 1 THEN typ = bty(1)
    IrEval$ = blk(nb)
    id = saveid: currentid = savecur
END FUNCTION

'value of a$ converted to QB type targettyp the way evaluatetotyp$ does
FUNCTION IrEvalTo$ (a$, targettyp AS LONG)
    DIM r$, t AS LONG, tt$
    r$ = IrEval$(a$, t)
    tt$ = IrTy$(targettyp)
    IF targettyp AND ISSTRING THEN tt$ = "str"
    IF (t AND ISSTRING) <> 0 AND tt$ <> "str" THEN IrNotSup "string used as a number"
    IF (t AND ISSTRING) = 0 AND tt$ = "str" THEN IrNotSup "number used as a string"
    r$ = IrCvt$(r$, IrT, tt$)
    IrT = tt$
    IrEvalTo$ = r$
END FUNCTION

'condition (IF/WHILE/...): integer register
FUNCTION IrCond$ (a$)
    DIM r$, t AS LONG, z$, d$
    r$ = IrEval$(a$, t)
    IF IrIsFloat(IrT) THEN
        z$ = IrConst$(IrT, "0")
        d$ = IrReg$: IrE "ne." + IrT + " " + d$ + " " + r$ + " " + z$
        r$ = d$: IrT = "i32"
    END IF
    IrCond$ = r$
END FUNCTION

'================================================================ calls

'passes one argument to parameter type pt (BYREF: address of a variable of the
'same type, otherwise of a temporary holding the converted value)
FUNCTION IrByRefArg$ (e$, pt AS LONG, ptsize AS LONG)
    DIM p$, t AS LONG, st$, tmp$, r$
    IrQuiet = -1
    p$ = IrPlace$(e$, t)
    IrQuiet = 0
    IF pt AND ISARRAY THEN
        IF LEFT$(p$, 1) = "A" THEN IrByRefArg$ = IrAddr$(p$): EXIT FUNCTION
        IrNotSup "expected array argument": IrByRefArg$ = IrConst$("ptr", "0"): EXIT FUNCTION
    END IF
    st$ = IrSTy$(pt, ptsize)
    IF LEN(p$) THEN
        IF LEFT$(p$, 1) <> "A" AND IrPlaceSTy$(p$) = st$ THEN IrByRefArg$ = IrAddr$(p$): EXIT FUNCTION
    END IF
    IF LEFT$(st$, 4) = "udt:" THEN IrNotSup "UDT argument type mismatch"
    r$ = IrEvalTo$(e$, pt)
    tmp$ = IrTemp$(st$)
    p$ = "V" + CHR$(2) + tmp$ + CHR$(2) + st$
    IrStore p$, r$
    IrByRefArg$ = IrAddr$(p$)
END FUNCTION

'user SUB/FUNCTION call; args$ raw elements joined with CHR$(0) per argument
FUNCTION IrUserCall$ (idn AS LONG, nargs AS LONG)
    DIM k AS LONG, pt AS LONG, ps AS LONG, regs$, d$, nm$, rt$
    DIM sv(1 TO 100) AS STRING
    FOR k = 1 TO nargs: sv(k) = IrArgs(k): NEXT
    nm$ = LTRIM$(RTRIM$(removecast$(RTRIM$(ids(idn).callname))))
    FOR k = 1 TO ids(idn).args
        pt = CVL(MID$(ids(idn).arg, k * 4 - 3, 4))
        ps = CVL(MID$(ids(idn).argsize, k * 4 - 3, 4))
        IF k > nargs THEN IrNotSup "missing argument": EXIT FOR
        regs$ = regs$ + " " + IrByRefArg$(sv(k), pt, ps)
    NEXT
    IF ids(idn).subfunc = 1 THEN
        rt$ = IrTy$(ids(idn).ret)
        d$ = IrReg$
        IrE "call " + d$ + " " + nm$ + regs$
        IrT = rt$
    ELSE
        IrE "call " + nm$ + regs$
    END IF
    IrUserCall$ = d$
END FUNCTION

'one numeric/string argument converted to QIR type t$
FUNCTION IrArgAs$ (e$, t$)
    DIM r$, ty AS LONG
    r$ = IrEval$(e$, ty)
    IF t$ = "str" OR IrT = "str" THEN
        IF t$ <> IrT THEN IrNotSup "argument type mismatch"
        IrArgAs$ = r$: EXIT FUNCTION
    END IF
    IrArgAs$ = IrCvt$(r$, IrT, t$)
    IrT = t$
END FUNCTION


'nm$ names an array visible here (no code emitted; IrT kept)
FUNCTION IrIsArrayName% (nm$)
    DIM x$, t AS LONG, keep$
    keep$ = IrT
    IrQuiet = -1: x$ = IrPlace$(nm$ + sp + "(" + sp + ")", t): IrQuiet = 0
    IrT = keep$
    IrIsArrayName% = (LEFT$(x$, 1) = "A")
END FUNCTION
'the fixed expression e$ is a variable or an array element (C++ evaluate: ISREFERENCE)
FUNCTION IrIsVarRef% (e$)
    DIM n AS LONG, c AS LONG, i AS LONG, b AS LONG, nm$
    n = numelements(e$)
    nm$ = getelement$(e$, 1)
    c = ASC(nm$)
    IF NOT ((c >= 65 AND c <= 90) OR (c >= 97 AND c <= 122) OR c = 95) THEN EXIT FUNCTION
    IF n = 1 THEN IrIsVarRef% = -1: EXIT FUNCTION
    IF getelement$(e$, 2) <> "(" OR getelement$(e$, n) <> ")" THEN EXIT FUNCTION
    FOR i = 2 TO n - 1
        IF getelement$(e$, i) = "(" THEN b = b + 1
        IF getelement$(e$, i) = ")" THEN b = b - 1
        IF b = 0 THEN EXIT FUNCTION
    NEXT
    'name(...) : an array, not a function call
    IrIsVarRef% = IrIsArrayName%(nm$)
END FUNCTION
'nm$(x): x as a 32-bit value when -2^31 <= x < 0, else as i64
FUNCTION IrRadix32$ (nm$, x$)
    DIM tmp$, l$, c$, r$, d$, y$
    l$ = "~H" + str2$(IrNew)
    tmp$ = IrTemp$("str")
    r$ = IrRt$(nm$, " " + x$, "str"): IrE "store.str " + tmp$ + " " + r$
    c$ = IrReg$: IrE "lt.i64 " + c$ + " " + x$ + " " + IrConst$("i64", "0")
    IrE "jz " + c$ + " " + l$ + ".E"
    c$ = IrReg$: IrE "lt.i64 " + c$ + " " + x$ + " " + IrConst$("i64", "-2147483648")
    IrE "jnz " + c$ + " " + l$ + ".E"
    y$ = IrCvt$(x$, "i64", "u32"): y$ = IrCvt$(y$, "u32", "i64")
    r$ = IrRt$(nm$, " " + y$, "str"): IrE "store.str " + tmp$ + " " + r$
    IrE "label " + l$ + ".E"
    d$ = IrReg$: IrE "load.str " + d$ + " " + tmp$
    IrT = "str"
    IrRadix32$ = d$
END FUNCTION

'HEX$(x) with neg_size 0 (libqb radix_string): negative values take 4, 8 or 16 digits, -1 gives ""
FUNCTION IrHex64$ (x$)
    DIM tmp$, l$, c$, r$, d$, y$
    l$ = "~H" + str2$(IrNew)
    tmp$ = IrTemp$("str")
    r$ = IrRt$("HEX$", " " + x$, "str"): IrE "store.str " + tmp$ + " " + r$
    c$ = IrReg$: IrE "lt.i64 " + c$ + " " + x$ + " " + IrConst$("i64", "0")
    IrE "jz " + c$ + " " + l$ + ".E"
    c$ = IrReg$: IrE "lt.i64 " + c$ + " " + x$ + " " + IrConst$("i64", "-1")
    IrE "jnz " + c$ + " " + l$ + ".A"
    IrE "store.str " + tmp$ + " " + IrConstStr$("")
    IrE "jmp " + l$ + ".E"
    IrE "label " + l$ + ".A"
    c$ = IrReg$: IrE "lt.i64 " + c$ + " " + x$ + " " + IrConst$("i64", "-32768")
    IrE "jnz " + c$ + " " + l$ + ".B"
    y$ = IrCvt$(x$, "i64", "u16"): y$ = IrCvt$(y$, "u16", "i64")
    r$ = IrRt$("HEX$", " " + y$, "str"): IrE "store.str " + tmp$ + " " + r$
    IrE "jmp " + l$ + ".E"
    IrE "label " + l$ + ".B"
    c$ = IrReg$: IrE "lt.i64 " + c$ + " " + x$ + " " + IrConst$("i64", "-2147483648")
    IrE "jnz " + c$ + " " + l$ + ".E"
    y$ = IrCvt$(x$, "i64", "u32"): y$ = IrCvt$(y$, "u32", "i64")
    r$ = IrRt$("HEX$", " " + y$, "str"): IrE "store.str " + tmp$ + " " + r$
    IrE "label " + l$ + ".E"
    d$ = IrReg$: IrE "load.str " + d$ + " " + tmp$
    IrT = "str"
    IrHex64$ = d$
END FUNCTION

' callnames lowered generically (registry argument types; VM: crates/qb-exec/src/v1/display.rs)
FUNCTION IrGenOK% (cn$)
    DIM l$
    l$ = "|"
    l$ = l$ + "qbg_screen|sub_pset|sub_preset|sub_line|sub_circle|sub_paint|"
    l$ = l$ + "qbg_palette|qbg_sub_color|qbg_sub_view|qbg_sub_view_print|qbg_sub_window|func_pmap|"
    l$ = l$ + "func_point|sub_pcopy|sub_draw|sub_cls|qbg_sub_locate|func_screen|"
    l$ = l$ + "sub__dest|sub__source|func__dest|func__source|func__display|func__console|"
    l$ = l$ + "func__newimage|sub__freeimage|func__copyimage|sub__display|sub__autodisplay|func__autodisplay|"
    l$ = l$ + "func__width|func__height|func__pixelsize|func__rgb|func__rgba|func__red|"
    l$ = l$ + "func__green|func__blue|func__alpha|func__rgb32|func__rgba32|func__red32|"
    l$ = l$ + "func__green32|func__blue32|func__alpha32|func__palettecolor|sub__palettecolor|sub_graphics_get|"
    l$ = l$ + "sub_graphics_put|qbsub_width|sub__font|func__font|func__fontwidth|func__fontheight|"
    l$ = l$ + "sub__limit|sub__delay|sub__title|"
    l$ = l$ + "sub__setalpha|sub__clearcolor|func__clearcolor|sub__putimage|sub__maptriangle|func__errormessage|"
    l$ = l$ + "func__loadfont|sub__freefont|func__printwidth|"
    IrGenOK% = INSTR(l$, "|" + cn$ + "|") > 0
END FUNCTION

'generic call of registry builtin idn with fixed arguments IrArgs() ("" = omitted)
FUNCTION IrGeneric$ (idn AS LONG, nargs AS LONG, passed AS LONG, isfunc AS LONG)
    DIM cn$, k AS LONG, t AS LONG, r$, x$, p$, nm$, sf$, req AS LONG, j AS LONG, ps AS LONG, d$, t2 AS LONG, q$
    DIM sv(1 TO 32) AS STRING
    FOR k = 1 TO nargs: sv(k) = IrArgs(k): NEXT
    cn$ = RTRIM$(ids(idn).callname)
    sf$ = RTRIM$(ids(idn).specialformat)
    FOR k = 1 TO ids(idn).args
        t = CVL(MID$(ids(idn).arg, k * 4 - 3, 4))
        IF k <= nargs THEN x$ = sv(k) ELSE x$ = ""
        IF t = -3 THEN
            IF x$ = "" THEN IrNotSup cn$ + " without an array": EXIT FUNCTION
            nm$ = getelement$(x$, 1)
            IF getelement$(x$, 2) = "(" AND getelement$(x$, 3) <> ")" THEN
                IrQuiet = -1: p$ = IrPlace$(x$, t2): IrQuiet = 0
            ELSE
                p$ = ""
            END IF
            IrQuiet = -1: q$ = IrPlace$(nm$ + sp + "(" + sp + ")", t2): IrQuiet = 0
            IF LEFT$(q$, 1) <> "A" THEN IrNotSup cn$ + " array argument": EXIT FUNCTION
            d$ = IrAddr$(q$)
            IF LEN(p$) THEN
                r$ = r$ + " " + IrAddr$(p$)
            ELSE
                x$ = IrReg$: IrE "rt " + x$ + " _ARRADDR " + d$: r$ = r$ + " " + x$
            END IF
            r$ = r$ + " " + d$
        ELSEIF t < 0 THEN
            IrNotSup cn$ + " argument kind" + STR$(t): EXIT FUNCTION
        ELSE
            IF t AND ISPOINTER THEN t = t - ISPOINTER
            IF cn$ = "sub_paint" AND k = 3 AND LEN(x$) > 0 THEN
                'PAINT fill: colour, or a tile string (the C++ overload on qbs*)
                p$ = IrEval$(x$, t2)
                IF IrT = "str" THEN cn$ = "sub_paint.s": r$ = r$ + " " + p$ ELSE r$ = r$ + " " + IrCvt$(p$, IrT, "u32")
            ELSEIF t AND ISSTRING THEN
                IF LEN(x$) THEN r$ = r$ + " " + IrArgAs$(x$, "str") ELSE r$ = r$ + " " + IrConstStr$("")
            ELSE
                IF LEN(x$) THEN r$ = r$ + " " + IrArgAs$(x$, IrTy$(t)) ELSE r$ = r$ + " " + IrConst$(IrTy$(t), "0")
            END IF
        END IF
    NEXT
    IF LEN(sf$) THEN
        IF isfunc THEN
            'optional arguments are the ones after the '?' marks outside brackets
            req = 0: ps = 0
            FOR j = 1 TO LEN(sf$)
                IF MID$(sf$, j, 1) = "[" THEN EXIT FOR
                IF MID$(sf$, j, 1) = "?" THEN req = req + 1
            NEXT
            FOR k = req + 1 TO nargs
                IF LEN(sv(k)) THEN ps = ps OR (2 ^ (k - req - 1))
            NEXT
            passed = ps
        END IF
        r$ = r$ + " " + IrI32$(passed)
    END IF
    IF cn$ = "func__rgb32" THEN r$ = r$ + " " + IrI32$(nargs) 'C++ overload by argument count
    IF isfunc THEN
        d$ = IrReg$
        IrE "rt " + d$ + " " + cn$ + r$
        t = ids(idn).ret: IF t AND ISPOINTER THEN t = t - ISPOINTER
        IrT = IrTy$(t)
        IrGeneric$ = d$
    ELSE
        IrE "rt " + cn$ + r$
    END IF
END FUNCTION

' " %a %b ..." for raw (fixed) arguments IrArgs() converted to the comma-separated QIR types;
' missing arguments are zero. withpassed: append the C++ backend's optional-argument flags.
FUNCTION IrArgList$ (types$, nargs AS LONG, passed AS LONG, withpassed AS LONG)
    DIM r$, t$, k AS LONG, rest$, p AS LONG
    DIM sv(1 TO 32) AS STRING
    FOR k = 1 TO nargs: sv(k) = IrArgs(k): NEXT
    rest$ = types$ + ","
    k = 0
    DO WHILE LEN(rest$)
        p = INSTR(rest$, ","): t$ = LEFT$(rest$, p - 1): rest$ = MID$(rest$, p + 1)
        k = k + 1
        IF k <= nargs AND LEN(sv(k)) > 0 THEN
            r$ = r$ + " " + IrArgAs$(sv(k), t$)
        ELSEIF t$ = "str" THEN
            r$ = r$ + " " + IrConstStr$("")
        ELSE
            r$ = r$ + " " + IrConst$(t$, "0")
        END IF
    LOOP
    IF withpassed THEN r$ = r$ + " " + IrI32$(passed)
    IrArgList$ = r$
END FUNCTION

FUNCTION IrRt$ (nm$, args$, rt$)
    DIM d$
    IF LEN(rt$) THEN d$ = IrReg$: IrE "rt " + d$ + " " + nm$ + args$: IrT = rt$ ELSE IrE "rt " + nm$ + args$
    IrRt$ = d$
END FUNCTION

'suffix for overloaded numeric builtins chosen by QB type
FUNCTION IrNumSfx$ (t AS LONG)
    IF t AND ISFLOAT THEN
        IF (t AND 511) = 32 THEN IrNumSfx$ = "f32" ELSE IF (t AND 511) = 64 THEN IrNumSfx$ = "f64" ELSE IrNumSfx$ = "f80"
    ELSE
        IF (t AND 511) = 64 AND (t AND ISUNSIGNED) <> 0 THEN IrNumSfx$ = "u64" ELSE IrNumSfx$ = "i64"
    END IF
END FUNCTION

FUNCTION IrCallFunc$ (idn AS LONG, a$, typ AS LONG)
    DIM nargs AS LONG, nm$, r$, t AS LONG, s$, k AS LONG, x$, y$, z$, sx$
    DIM sv(1 TO 100) AS STRING
    nargs = IrSplitArgs(a$)
    FOR k = 1 TO nargs: sv(k) = IrArgs(k): NEXT
    typ = ids(idn).ret
    IF typ AND ISPOINTER THEN typ = typ - ISPOINTER
    IF ids(idn).internal_subfunc = 0 THEN
        FOR k = 1 TO nargs: IrArgs(k) = sv(k): NEXT
        IrCallFunc$ = IrUserCall$(idn, nargs)
        EXIT FUNCTION
    END IF
    nm$ = UCASE$(RTRIM$(ids(idn).n)) + RTRIM$(ids(idn).musthave)
    SELECT CASE nm$
        CASE "LEN"
            IrQuiet = -1: x$ = IrPlace$(sv(1), t): IrQuiet = 0
            IF LEN(x$) THEN
                IF (t AND ISSTRING) = 0 THEN
                    IrCallFunc$ = IrI32$(IrSize&(IrPlaceSTy$(x$))): IrT = "i32": EXIT FUNCTION
                END IF
            END IF
            r$ = IrRt$("LEN", " " + IrArgAs$(sv(1), "str"), "i32")
        CASE "LEFT$", "RIGHT$"
            x$ = IrArgAs$(sv(1), "str"): y$ = IrArgAs$(sv(2), "i32")
            r$ = IrRt$(nm$, " " + x$ + " " + y$, "str")
        CASE "MID$"
            x$ = IrArgAs$(sv(1), "str"): y$ = IrArgAs$(sv(2), "i32")
            IF nargs >= 3 THEN
                IF LEN(sv(3)) THEN z$ = IrArgAs$(sv(3), "i32"): r$ = IrRt$("MID$.n", " " + x$ + " " + y$ + " " + z$, "str"): GOTO irfdone
            END IF
            r$ = IrRt$("MID$", " " + x$ + " " + y$, "str")
        CASE "CHR$", "SPACE$"
            r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "i32"), "str")
        CASE "ASC"
            x$ = IrArgAs$(sv(1), "str")
            IF nargs >= 2 THEN
                y$ = IrArgAs$(sv(2), "i32"): r$ = IrRt$("ASC.n", " " + x$ + " " + y$, "i32")
            ELSE
                r$ = IrRt$("ASC", " " + x$, "i32")
            END IF
        CASE "STR$"
            x$ = IrEval$(sv(1), t)
            sx$ = IrNumSfx$(t)
            x$ = IrCvt$(x$, IrT, sx$)
            r$ = IrRt$("STR$." + sx$, " " + x$, "str")
        CASE "VAL"
            'func_val returns long double whatever the registry's type says
            r$ = IrRt$("VAL.f80", " " + IrArgAs$(sv(1), "str"), "f80")
        CASE "INSTR", "_INSTRREV"
            IF nargs = 3 THEN
                x$ = IrArgAs$(sv(1), "i32"): y$ = IrArgAs$(sv(2), "str"): z$ = IrArgAs$(sv(3), "str")
                r$ = IrRt$(nm$ + ".n", " " + x$ + " " + y$ + " " + z$, "i32")
            ELSE
                x$ = IrArgAs$(sv(1), "str"): y$ = IrArgAs$(sv(2), "str")
                r$ = IrRt$(nm$, " " + x$ + " " + y$, "i32")
            END IF
        CASE "UCASE$", "LCASE$", "LTRIM$", "RTRIM$", "_TRIM$"
            r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "str"), "str")
        CASE "STRING$"
            x$ = IrArgAs$(sv(1), "i32")
            y$ = IrEval$(sv(2), t)
            IF IrT = "str" THEN
                r$ = IrRt$("STRING$.s", " " + x$ + " " + y$, "str")
            ELSE
                y$ = IrCvt$(y$, IrT, "i32")
                r$ = IrRt$("STRING$", " " + x$ + " " + y$, "str")
            END IF
        CASE "VARPTR$"
            'type code as semantics.bas (func_varptr_helper); the value is snapshotted in DBLOCK
            x$ = IrEval$(sv(1), t)
            sx$ = IrT
            IF sx$ = "str" THEN IrNotSup "VARPTR$ of a string": r$ = IrConstStr$(""): IrT = "str": GOTO irfdone
            k = 0
            IF t AND ISUNSIGNED THEN k = k + 128
            IF t AND ISOFFSETINBITS THEN
                k = k + 64 + (t AND 63)
            ELSEIF t AND ISFLOAT THEN
                IF (t AND 511) = 32 THEN k = k + 4
                IF (t AND 511) = 64 THEN k = k + 8
                IF (t AND 511) = 256 THEN k = k + 6
            ELSE
                IF (t AND 511) = 8 THEN k = k + 1
                IF (t AND 511) = 16 THEN k = k + 2
                IF (t AND 511) = 32 THEN k = k + 20
                IF (t AND 511) = 64 THEN k = k + 5
            END IF
            r$ = IrRt$("VARPTR$", " " + IrI32$(k) + " " + IrCvt$(x$, sx$, "f64"), "str")
        CASE "HEX$", "OCT$", "_BIN$"
            x$ = IrEval$(sv(1), t)
            sx$ = IrT
            IF LEFT$(sx$, 1) = "f" THEN
                'func_*_float: rounded; negative values in LONG range use 32-bit digits
                x$ = IrCvt$(x$, IrT, "i64"): IrT = "i64"
                IF nm$ = "_BIN$" THEN nm$ = "BIN$"
                r$ = IrRadix32$(nm$, x$): GOTO irfdone
            END IF
            IF nm$ = "HEX$" AND (t AND ISOFFSETINBITS) = 0 AND (t AND 511) = 64 AND (t AND ISUNSIGNED) = 0 THEN
                'C++: a 64-bit HEX$ argument that is not a variable gets neg_size 0 (semantics.bas)
                IF IrIsVarRef%(sv(1)) = 0 THEN r$ = IrHex64$(IrCvt$(x$, IrT, "i64")): GOTO irfdone
            END IF
            IF (t AND 511) <= 8 AND (t AND ISFLOAT) = 0 AND nm$ = "HEX$" AND (t AND ISOFFSETINBITS) = 0 THEN
                x$ = IrCvt$(x$, IrT, "u8"): IrT = "u8"
            ELSEIF (t AND 511) <= 16 AND (t AND ISFLOAT) = 0 THEN
                x$ = IrCvt$(x$, IrT, "u16"): IrT = "u16"
            ELSEIF (t AND 511) <= 32 AND (t AND ISFLOAT) = 0 THEN
                x$ = IrCvt$(x$, IrT, "u32"): IrT = "u32"
            END IF
            x$ = IrCvt$(x$, IrT, "i64")
            IF nm$ = "_BIN$" THEN nm$ = "BIN$"
            r$ = IrRt$(nm$, " " + x$, "str")
        CASE "ABS"
            x$ = IrEval$(sv(1), t)
            IF IrIsFloat(IrT) THEN
                r$ = IrRt$("ABS." + IrT, " " + x$, IrT)
            ELSE
                sx$ = IrT: x$ = IrCvt$(x$, IrT, "i64")
                r$ = IrRt$("ABS.i64", " " + x$, "i64")
                r$ = IrCvt$(r$, "i64", sx$): IrT = sx$
            END IF
            typ = t
        CASE "SGN"
            x$ = IrEval$(sv(1), t)
            IF IrIsFloat(IrT) THEN
                IF IrT = "f80" THEN sx$ = "f80" ELSE sx$ = "f64"
                x$ = IrCvt$(x$, IrT, sx$)
                r$ = IrRt$("SGN." + sx$, " " + x$, "i32")
            ELSE
                x$ = IrCvt$(x$, IrT, "i64")
                r$ = IrRt$("SGN.i64", " " + x$, "i32")
            END IF
        CASE "INT", "FIX"
            x$ = IrEval$(sv(1), t)
            IF IrIsFloat(IrT) THEN
                r$ = IrRt$(nm$ + "." + IrT, " " + x$, IrT)
            ELSE
                r$ = x$
            END IF
            typ = t
        CASE "SQR", "SIN", "COS", "TAN", "ATN", "LOG", "EXP"
            x$ = IrEval$(sv(1), t)
            sx$ = IrT
            x$ = IrCvt$(x$, IrT, "f64")
            sx$ = nm$
            IF nm$ = "EXP" AND (t AND ISOFFSETINBITS) = 0 THEN
                'func_exp_single (overflow above 88.03) for SINGLE and <= 16-bit integer arguments
                IF (t AND ISFLOAT) <> 0 AND (t AND 511) = 32 THEN sx$ = "EXP.s"
                IF (t AND ISFLOAT) = 0 AND (t AND 511) <= 16 THEN sx$ = "EXP.s"
            END IF
            r$ = IrRt$(sx$, " " + x$, "f64")
            'QB-like result type: single for <= INTEGER/SINGLE arguments
            IF t AND ISFLOAT THEN
                IF (t AND 511) = 32 THEN r$ = IrCvt$(r$, "f64", "f32"): IrT = "f32"
                IF (t AND 511) = 256 THEN r$ = IrCvt$(r$, "f64", "f80"): IrT = "f80"
                typ = t
            ELSE
                IF (t AND 511) <= 16 THEN
                    r$ = IrCvt$(r$, "f64", "f32"): IrT = "f32": typ = SINGLETYPE - ISPOINTER
                ELSE
                    typ = DOUBLETYPE - ISPOINTER
                END IF
            END IF
        CASE "CINT", "CLNG", "CSNG", "CDBL", "_ROUND"
            'register type = the C function's return type (the registry's typ is not always it)
            SELECT CASE nm$
                CASE "CINT": sx$ = "i16"
                CASE "CLNG": sx$ = "i32"
                CASE "CSNG": sx$ = "f32"
                CASE "CDBL": sx$ = "f64"
                CASE ELSE: sx$ = "i64"
            END SELECT
            x$ = IrEval$(sv(1), t)
            r$ = IrCvt$(x$, IrT, sx$): IrT = sx$
        CASE "RND"
            IF nargs >= 1 THEN
                IF LEN(sv(1)) THEN r$ = IrRt$("RND.n", " " + IrArgAs$(sv(1), "f64"), "f32"): GOTO irfdone
            END IF
            r$ = IrRt$("RND", "", "f32")
        CASE "TIMER"
            IF nargs >= 1 THEN
                IF LEN(sv(1)) THEN r$ = IrRt$("TIMER.n", " " + IrArgAs$(sv(1), "f64"), "f64") ELSE r$ = IrRt$("TIMER", "", "f64")
            ELSE
                r$ = IrRt$("TIMER", "", "f64")
            END IF
        CASE "ERR", "ERL", "_ERRORLINE", "FREEFILE", "POS", "CSRLIN"
            IF nm$ = "POS" THEN
                IF nargs >= 1 THEN x$ = IrArgAs$(sv(1), "i32")
            END IF
            r$ = IrRt$(nm$, "", "i32")
        CASE "INKEY$", "COMMAND$", "DATE$", "TIME$"
            r$ = IrRt$(nm$, "", "str")
        CASE "ENVIRON$"
            x$ = IrEval$(sv(1), t)
            IF IrT = "str" THEN r$ = IrRt$("ENVIRON$", " " + x$, "str") ELSE r$ = IrRt$("ENVIRON$.n", " " + IrCvt$(x$, IrT, "i32"), "str")
        CASE "EOF"
            r$ = IrRt$("EOF", " " + IrArgAs$(sv(1), "i32"), "i32")
        CASE "LOF", "LOC", "SEEK"
            r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "i32"), "i64")
        CASE "INPUT$"
            x$ = IrArgAs$(sv(1), "i32")
            IF nargs >= 2 THEN
                IF getelement$(sv(2), 1) = "#" THEN sv(2) = getelements$(sv(2), 2, numelements(sv(2)))
                y$ = IrArgAs$(sv(2), "i32")
            ELSE
                y$ = IrI32$(0)
            END IF
            r$ = IrRt$("INPUT$", " " + x$ + " " + y$, "str")
        CASE "_FILEEXISTS", "_DIREXISTS"
            r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "str"), "i32")
        CASE "MKI$": r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "i16"), "str")
        CASE "MKL$": r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "i32"), "str")
        CASE "MKS$", "MKSMBF$": r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "f32"), "str")
        CASE "MKD$", "MKDMBF$": r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "f64"), "str")
        CASE "CVI": r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "str"), "i16")
        CASE "CVL": r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "str"), "i32")
        CASE "CVS", "CVSMBF": r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "str"), "f32")
        CASE "CVD", "CVDMBF": r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "str"), "f64")
        CASE "SHELL", "_SHELLHIDE"
            r$ = IrRt$("SHELL", " " + IrArgAs$(sv(1), "str"), "i32")
        CASE "_STRCMP", "_STRICMP"
            x$ = IrArgAs$(sv(1), "str"): y$ = IrArgAs$(sv(2), "str")
            r$ = IrRt$(nm$, " " + x$ + " " + y$, "i32")
        CASE "_DEFLATE$"
            r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "str"), "str")
        CASE "_INFLATE$"
            x$ = IrArgAs$(sv(1), "str")
            IF nargs >= 2 THEN
                IF LEN(sv(2)) THEN r$ = IrRt$("_INFLATE$.n", " " + x$ + " " + IrArgAs$(sv(2), "i64"), "str"): GOTO irfdone
            END IF
            r$ = IrRt$("_INFLATE$", " " + x$, "str")
        CASE "_OPENHOST", "_OPENCLIENT"
            r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "str"), "i32")
        CASE "_OPENCONNECTION", "_CONNECTED"
            r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "i32"), "i32")
        CASE "_CONNECTIONADDRESS$", "_CONNECTIONADDRESS"
            r$ = IrRt$("_CONNECTIONADDRESS$", " " + IrArgAs$(sv(1), "i32"), "str")
        CASE "_SNDOPEN"
            x$ = IrArgAs$(sv(1), "str")
            IF nargs >= 2 THEN y$ = IrArgAs$(sv(2), "str"): z$ = IrI32$(1) ELSE y$ = IrConstStr$(""): z$ = IrI32$(0)
            r$ = IrRt$("_SNDOPEN", " " + x$ + " " + y$ + " " + z$, "i32")
            IF IrTy$(typ) <> "i32" THEN r$ = IrCvt$(r$, "i32", IrTy$(typ)): IrT = IrTy$(typ)
        CASE "_SNDCOPY", "_SNDPLAYING", "_SNDPAUSED"
            r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "i32"), "i32")
            IF IrTy$(typ) <> "i32" THEN r$ = IrCvt$(r$, "i32", IrTy$(typ)): IrT = IrTy$(typ)
        CASE "_SNDLEN", "_SNDGETPOS"
            r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "i32"), "f64")
        CASE "_SNDOPENRAW", "_SNDRATE"
            r$ = IrRt$(nm$, "", "i32")
            IF IrTy$(typ) <> "i32" THEN r$ = IrCvt$(r$, "i32", IrTy$(typ)): IrT = IrTy$(typ)
        CASE "_SNDRAWLEN"
            IF nargs >= 1 THEN
                IF LEN(sv(1)) THEN r$ = IrRt$(nm$, " " + IrArgAs$(sv(1), "i32") + " " + IrI32$(1), "f64"): GOTO irfdone
            END IF
            r$ = IrRt$(nm$, " " + IrI32$(0) + " " + IrI32$(0), "f64")
        CASE "PLAY"
            r$ = IrRt$("PLAY.f", " " + IrI32$(0), "i32")
        CASE "VARPTR", "SADD"
            IrQuiet = -1: x$ = IrPlace$(sv(1), t): IrQuiet = 0
            IF x$ = "" THEN IrNotSup nm$ + " argument": r$ = IrI32$(0): IrT = "i32": GOTO irfdone
            IF IrPlaceSTy$(x$) <> "str" THEN IrNotSup nm$ + " of a non-STRING variable": r$ = IrI32$(0): IrT = "i32": GOTO irfdone
            IF nm$ = "VARPTR" THEN y$ = "VARPTR.s" ELSE y$ = "SADD"
            r$ = IrRt$(y$, " " + IrAddr$(x$), "i32")
        CASE "VARSEG"
            r$ = IrRt$("VARSEG", "", "i32")
        CASE "PEEK"
            r$ = IrRt$("PEEK", " " + IrArgAs$(sv(1), "i32"), "i32")
        CASE "_CWD$", "_STARTDIR$"
            r$ = IrRt$(nm$, "", "str")
        CASE "_MK$", "_CV"
            t = typname2typ(UCASE$(sv(1)))
            IF t AND ISPOINTER THEN t = t - ISPOINTER
            sx$ = IrTy$(t)
            SELECT CASE sx$
                CASE "i16": z$ = "I"
                CASE "i32": z$ = "L"
                CASE "f32": z$ = "S"
                CASE "f64": z$ = "D"
                CASE "i64": z$ = "i64"
                CASE ELSE: z$ = ""
            END SELECT
            IF z$ = "" THEN IrNotSup nm$ + " of " + sv(1): r$ = IrI32$(0): IrT = "i32": GOTO irfdone
            IF nm$ = "_MK$" THEN
                x$ = IrArgAs$(sv(2), sx$)
                IF z$ = "i64" THEN r$ = IrRt$("_MK$.i64", " " + x$, "str") ELSE r$ = IrRt$("MK" + z$ + "$", " " + x$, "str")
            ELSE
                x$ = IrArgAs$(sv(2), "str")
                IF z$ = "i64" THEN r$ = IrRt$("_CV.i64", " " + x$, "i64") ELSE r$ = IrRt$("CV" + z$, " " + x$, sx$)
                typ = t
            END IF
        CASE "_MEMEXISTS", "_MEMGET"
            FOR k = 1 TO nargs: IrArgs(k) = sv(k): NEXT
            r$ = IrMemFunc$(nm$, nargs, typ)
        CASE "LBOUND", "UBOUND"
            IrQuiet = -1: x$ = IrPlace$(sv(1) + sp + "(" + sp + ")", t): IrQuiet = 0
            IF LEFT$(x$, 1) <> "A" THEN IrNotSup nm$ + " argument": r$ = IrI32$(0): IrT = "i32": GOTO irfdone
            IF nargs >= 2 THEN y$ = IrArgAs$(sv(2), "i32") ELSE y$ = IrI32$(1)
            r$ = IrReg$
            IrE LCASE$(nm$) + " " + r$ + " " + IrPart$(x$, 2) + " " + y$
            IrT = "i32"
        CASE ELSE
            IF IrGenOK%(RTRIM$(ids(idn).callname)) THEN
                FOR k = 1 TO nargs: IrArgs(k) = sv(k): NEXT
                r$ = IrGeneric$(idn, nargs, 0, -1)
            ELSE
                IrNotSup "function " + nm$
                r$ = IrI32$(0): IrT = "i32"
            END IF
    END SELECT
    irfdone:
    IF IrT <> IrTy$(typ) AND (typ AND ISSTRING) = 0 THEN
        IF IrT = "str" THEN
            IrNotSup "type of " + nm$
        ELSEIF IrIsFloat(IrT) = 0 OR IrIsFloat(IrTy$(typ)) = 0 OR nm$ = "ABS" OR nm$ = "INT" OR nm$ = "FIX" THEN
            'keep the C type of the value
        END IF
    END IF
    IrCallFunc$ = r$
END FUNCTION

'================================================================ statements

'assignment: lhs$ fixed target, rhs$ raw expression
SUB IrAssign (lhs$, rhs$)
    DIM p$, t AS LONG, r$, e$, st$, t2 AS LONG, q$
    p$ = IrPlace$(lhs$, t)
    IF p$ = "" THEN EXIT SUB
    e$ = fixoperationorder$(rhs$)
    st$ = IrPlaceSTy$(p$)
    IF LEFT$(st$, 4) = "udt:" THEN
        IF IrMemAssign%(IrAddr$(p$), e$) THEN EXIT SUB
        q$ = IrPlace$(e$, t2)
        IF q$ = "" THEN EXIT SUB
        IrE "copy " + IrAddr$(p$) + " " + IrAddr$(q$) + " " + str2$(IrSize&(st$))
        EXIT SUB
    END IF
    IF LEFT$(st$, 5) = "fstr:" THEN t = ISSTRING
    r$ = IrEvalTo$(e$, t)
    IF t AND ISOFFSETINBITS THEN r$ = IrBitWrap$(r$, t)
    IrStore p$, r$
END SUB

'value r$ (of IrTy$(t)) wrapped to the n bits of a _BIT * n type: masked, sign-extended if signed
FUNCTION IrBitWrap$ (r$, t AS LONG)
    DIM n AS LONG, ty$, m$, x$, s$, y$, k AS LONG, h AS _INTEGER64
    n = t AND 511
    ty$ = IrTy$(t)
    IF n >= 64 THEN IrBitWrap$ = r$: EXIT FUNCTION
    h = 1
    FOR k = 2 TO n: h = h * 2: NEXT 'h = 2^(n-1)
    m$ = IrConst$(ty$, _TRIM$(STR$(h * 2 - 1)))
    x$ = IrReg$: IrE "and." + ty$ + " " + x$ + " " + r$ + " " + m$
    IF (t AND ISUNSIGNED) = 0 THEN
        s$ = IrConst$(ty$, _TRIM$(STR$(h)))
        y$ = IrReg$: IrE "xor." + ty$ + " " + y$ + " " + x$ + " " + s$
        x$ = IrReg$: IrE "sub." + ty$ + " " + x$ + " " + y$ + " " + s$
    END IF
    IrBitWrap$ = x$
END FUNCTION

'PRINT item: a fixed string expression
SUB IrPrintStr (ch$, e$)
    DIM r$, u$, t AS LONG
    u$ = UCASE$(getelement$(e$, 1))
    IF (u$ = "TAB" OR u$ = "SPC") AND getelement$(e$, 2) = "(" AND getelement$(e$, numelements(e$)) = ")" THEN
        'TAB(n) / SPC(n): position-dependent, not a string value
        r$ = IrEval$(getelements$(e$, 3, numelements(e$) - 1), t)
        r$ = IrCvt$(r$, IrT, "i64")
        IrE "rt PRINT" + u$ + " " + IrI32$(VAL(ch$)) + " " + r$
        EXIT SUB
    END IF
    r$ = IrEvalTo$(e$, ISSTRING)
    IrE "rt PRINTS " + IrI32$(VAL(ch$)) + " " + r$
END SUB

SUB IrRtCh (nm$, ch$)
    IrE "rt " + nm$ + " " + IrI32$(VAL(ch$))
END SUB

'PRINT USING item (fixed expression)
SUB IrUsingItem (e$)
    DIM r$, t AS LONG, sx$
    r$ = IrEval$(e$, t)
    IF t AND ISSTRING THEN
        IrE "rt USINGS " + r$
    ELSE
        sx$ = IrNumSfx$(t)
        r$ = IrCvt$(r$, IrT, sx$)
        IrE "rt USINGN." + sx$ + " " + r$
    END IF
END SUB

'ON expr GOTO/GOSUB
SUB IrOnStart (e$)
    DIM r$
    r$ = IrEvalTo$(e$, LONGTYPE - ISPOINTER)
    IrFix3 = IrTemp$("i32")
    IrE "store.i32 " + IrFix3 + " " + r$
END SUB

SUB IrOnItem (g AS LONG, ln AS LONG, l$, u$)
    DIM x$, c$
    x$ = IrReg$: IrE "load.i32 " + x$ + " " + IrFix3
    c$ = IrCmp$("eq", x$, "i32", IrI32$(ln), "i32")
    IF g THEN
        IrE "jz " + c$ + " ~O" + u$ + "." + str2$(ln)
        IrE "gosub " + IrLbl$(l$)
        IrE "jmp ~O" + u$ + ".S"
        IrE "label ~O" + u$ + "." + str2$(ln)
    ELSE
        IrE "jnz " + c$ + " " + IrLbl$(l$)
    END IF
END SUB

SUB IrOnEnd (g AS LONG, u$)
    DIM x$, c$
    x$ = IrReg$: IrE "load.i32 " + x$ + " " + IrFix3
    c$ = IrCmp$("lt", x$, "i32", IrI32$(0), "i32")
    IrE "jz " + c$ + " ~O" + u$ + ".S"
    IrE "error " + IrI32$(5)
    IrE "label ~O" + u$ + ".S"
END SUB

'READ target (fixed)
SUB IrRead (e$)
    DIM p$, t AS LONG, rt$, d$
    p$ = IrPlace$(e$, t)
    IF p$ = "" THEN EXIT SUB
    rt$ = IrRegOf$(IrPlaceSTy$(p$))
    d$ = IrReg$
    IrE "read." + rt$ + " " + d$
    IrStore p$, d$
    IF rt$ <> "str" THEN IrE "rt READCHK"
END SUB

'DIM/REDIM statement for id idn; elements$ raw bounds ("" = not an array)
SUB IrDimStmt (idn0 AS LONG, elements$, redimopt AS LONG, vn$)
    DIM nm$, k$, aid AS LONG, p$, t AS LONG, saveid AS idstruct, savecur AS LONG
    aid = idn0
    IF aid >= 1 AND aid <= idn THEN
        IF ids(aid).arraytype = 0 AND LEN(elements$) THEN aid = 0
    ELSE
        aid = 0
    END IF
    IF LEN(elements$) = 0 OR LEFT$(elements$, 1) = "?" THEN
        IF aid THEN nm$ = IrVar$(aid)
        EXIT SUB
    END IF
    IF aid = 0 THEN
        saveid = id: savecur = currentid
        IrQuiet = -1: p$ = IrPlace$(UCASE$(vn$) + sp + "(" + sp + ")", t): IrQuiet = 0
        id = saveid: currentid = savecur
        IF LEFT$(p$, 1) <> "A" THEN IrNotSup "REDIM of an unknown array": EXIT SUB
        nm$ = IrPart$(p$, 2)
    ELSE
        nm$ = IrVar$(aid)
    END IF
    k$ = "dim"
    IF redimopt = 1 THEN k$ = "redim"
    IF redimopt = 2 THEN k$ = "redimp"
    IrDimArray nm$, elements$, k$
END SUB

'statement call of a SUB (user or builtin); raw arguments in IrArgs()
SUB IrSubCall (idn AS LONG, nargs AS LONG, passed AS LONG)
    DIM k AS LONG, nm$, x$, y$
    DIM sv(1 TO 100) AS STRING
    FOR k = 1 TO nargs
        IF LEN(IrArgs(k)) AND IrArgs(k) <> "N-LL" THEN sv(k) = fixoperationorder$(IrArgs(k)) ELSE sv(k) = ""
    NEXT
    IF ids(idn).internal_subfunc = 0 THEN
        FOR k = 1 TO nargs: IrArgs(k) = sv(k): NEXT
        x$ = IrUserCall$(idn, nargs)
        EXIT SUB
    END IF
    nm$ = UCASE$(RTRIM$(ids(idn).n)) + RTRIM$(ids(idn).musthave)
    FOR k = 1 TO nargs: IrArgs(k) = sv(k): NEXT
    SELECT CASE nm$
        CASE "BEEP": IrE "rt BEEP"
        CASE "SOUND": IrE "rt SOUND" + IrArgList$("f64,f64", nargs, -1, 0)
        CASE "PLAY": IrE "rt PLAY" + IrArgList$("str", nargs, -1, 0)
        CASE "_VOICE", "_SNDCLOSE", "_SNDPLAY", "_SNDPAUSE", "_SNDLOOP", "_SNDSTOP": IrE "rt " + nm$ + IrArgList$("i32", nargs, -1, 0)
        CASE "_ADSR": IrE "rt _ADSR" + IrArgList$("f64,f64,f64,f64,i32", nargs, passed, 1)
        CASE "_WAVE": IrE "rt _WAVE" + IrArgList$("str,i32", nargs, passed, 1)
        CASE "_SNDPLAYCOPY": IrE "rt _SNDPLAYCOPY" + IrArgList$("i32,f64", nargs, passed, 1)
        CASE "_SNDPLAYFILE": IrE "rt _SNDPLAYFILE" + IrArgList$("str,i32,f64", nargs, passed, 1)
        CASE "_SNDVOL": IrE "rt _SNDVOL" + IrArgList$("i32,f32", nargs, -1, 0)
        CASE "_SNDBAL": IrE "rt _SNDBAL" + IrArgList$("i32,f64,f64,f64,i32", nargs, passed, 1)
        CASE "_SNDSETPOS", "_SNDLIMIT": IrE "rt " + nm$ + IrArgList$("i32,f64", nargs, -1, 0)
        CASE "_SNDRAW": IrE "rt _SNDRAW" + IrArgList$("f32,f32,i32", nargs, passed, 1)
        CASE "_SNDRAWDONE": IrE "rt _SNDRAWDONE" + IrArgList$("i32", nargs, passed, 1)
        CASE "_SCREENMOVE", "_CONSOLETITLE", "_CONSOLE"
            'window state: no visible effect headless
        CASE "SLEEP"
            IrE "rt SLEEP " + IrI32$(0)
        CASE "POKE"
            x$ = IrArgAs$(sv(1), "i32"): y$ = IrArgAs$(sv(2), "i32")
            IrE "rt POKE " + x$ + " " + y$
        CASE "OPEN", "CLOSE", "RESET", "NAME", "SEEK", "GET", "PUT", "LINE", "INPUT", "WRITE", "PRINT", "_MEMCOPY", "_MEMFREE", "_MEMFILL", "_MEMPUT", "_MEMGET", "DEF", "FIELD"
            'lowered by IrStmtEnd (graphics LINE/GET/PUT: generic call)
            IF IrGenOK%(RTRIM$(ids(idn).callname)) THEN x$ = IrGeneric$(idn, nargs, passed, 0)
        CASE "ERROR"
            IrE "error " + IrArgAs$(sv(1), "i32")
        CASE "RANDOMIZE"
            IF passed = 3 THEN
                IrE "rt RANDOMIZEUSING " + IrArgAs$(sv(1), "f64")
            ELSEIF passed = 1 THEN
                IrE "rt RANDOMIZE " + IrArgAs$(sv(1), "f64")
            ELSE
                IrNotSup "RANDOMIZE without a seed"
            END IF
        CASE "KILL", "MKDIR", "RMDIR", "CHDIR", "ENVIRON"
            IrE "rt " + nm$ + " " + IrArgAs$(sv(1), "str")
        CASE "SHELL"
            x$ = IrReg$: IrE "rt " + x$ + " SHELL " + IrArgAs$(sv(1), "str")
        CASE ELSE
            IF IrGenOK%(RTRIM$(ids(idn).callname)) THEN
                x$ = IrGeneric$(idn, nargs, passed, 0)
            ELSE
                IrNotSup "statement " + nm$
            END IF
    END SELECT
END SUB

SUB IrGoto (l$)
    IrE "jmp " + IrLbl$(l$)
END SUB

SUB IrGosub (l$)
    IrE "gosub " + IrLbl$(l$)
END SUB

'--- IF
SUB IrIf (e$)
    DIM c$, bid AS LONG
    bid = IrNew
    IrCtlId(controllevel) = bid: IrCtlPart(controllevel) = 1
    c$ = IrCond$(e$)
    IrE "jz " + c$ + " ~I" + str2$(bid) + ".1"
END SUB

SUB IrElseIf (e$)
    DIM c$, bid AS LONG, p AS LONG
    bid = IrCtlId(controllevel): p = IrCtlPart(controllevel)
    IrE "jmp ~I" + str2$(bid) + ".E"
    IrE "label ~I" + str2$(bid) + "." + str2$(p)
    p = p + 1: IrCtlPart(controllevel) = p
    c$ = IrCond$(e$)
    IrE "jz " + c$ + " ~I" + str2$(bid) + "." + str2$(p)
END SUB

SUB IrElse
    DIM bid AS LONG
    bid = IrCtlId(controllevel)
    IrE "jmp ~I" + str2$(bid) + ".E"
    IrE "label ~I" + str2$(bid) + "." + str2$(IrCtlPart(controllevel))
    IrCtlPart(controllevel) = 0
END SUB

SUB IrEndIf
    DIM bid AS LONG
    bid = IrCtlId(controllevel)
    IF IrCtlPart(controllevel) THEN IrE "label ~I" + str2$(bid) + "." + str2$(IrCtlPart(controllevel))
    IrE "label ~I" + str2$(bid) + ".E"
END SUB

'--- loops (labels use controlid)
SUB IrWhile (cid AS LONG, e$, untl AS LONG)
    DIM c$
    IrE "label ~W" + str2$(cid) + ".C"
    c$ = IrCond$(e$)
    IF untl THEN IrE "jnz " + c$ + " ~W" + str2$(cid) + ".X" ELSE IrE "jz " + c$ + " ~W" + str2$(cid) + ".X"
END SUB

SUB IrDo (cid AS LONG)
    IrE "label ~W" + str2$(cid) + ".T"
END SUB

'LOOP [WHILE|UNTIL] (kind 0 none, 1 while, 2 until); pre = DO had the test
SUB IrLoop (cid AS LONG, kind AS LONG, e$, pre AS LONG)
    DIM c$
    IF pre THEN
        IrE "jmp ~W" + str2$(cid) + ".C"
    ELSE
        IrE "label ~W" + str2$(cid) + ".C"
        IF kind = 0 THEN
            IrE "jmp ~W" + str2$(cid) + ".T"
        ELSE
            c$ = IrCond$(e$)
            IF kind = 1 THEN IrE "jnz " + c$ + " ~W" + str2$(cid) + ".T" ELSE IrE "jz " + c$ + " ~W" + str2$(cid) + ".T"
        END IF
    END IF
    IrE "label ~W" + str2$(cid) + ".X"
END SUB

'--- FOR: var$ fixed variable, ctyp loop type (C++ markup), raw start/final/step
SUB IrFor (cid AS LONG, var$, ctyp AS LONG, st$, fi$, sp$)
    DIM p$, t AS LONG, ct$, vv$, ff$, ss$, r$, x$, z$, c$, n$
    p$ = IrPlace$(var$, t)
    IF p$ = "" THEN EXIT SUB
    ct$ = IrTy$(ctyp)
    n$ = str2$(cid)
    vv$ = IrTemp$(ct$): ff$ = IrTemp$(ct$): ss$ = IrTemp$(ct$)
    IrCtlVar(controllevel) = p$ + CHR$(3) + vv$ + CHR$(3) + ff$ + CHR$(3) + ss$ + CHR$(3) + ct$
    r$ = IrEvalTo$(fixoperationorder$(st$), ctyp): IrE "store." + ct$ + " " + vv$ + " " + r$
    r$ = IrEvalTo$(fixoperationorder$(fi$), ctyp): IrE "store." + ct$ + " " + ff$ + " " + r$
    r$ = IrEvalTo$(fixoperationorder$(sp$), ctyp): IrE "store." + ct$ + " " + ss$ + " " + r$
    IrE "jmp ~F" + n$ + ".E"
    IrE "label ~F" + n$ + ".T"
    x$ = IrLoad$(p$)
    x$ = IrCvt$(x$, IrT, ct$)
    r$ = IrReg$: IrE "load." + ct$ + " " + r$ + " " + ss$
    z$ = IrReg$: IrE "add." + ct$ + " " + z$ + " " + r$ + " " + x$
    IrE "store." + ct$ + " " + vv$ + " " + z$
    IrE "label ~F" + n$ + ".E"
    r$ = IrReg$: IrE "load." + ct$ + " " + r$ + " " + vv$
    IrStore p$, IrTrunc$(r$, ct$, IrRegOf$(IrPlaceSTy$(p$)))
    x$ = IrReg$: IrE "load." + ct$ + " " + x$ + " " + ss$
    z$ = IrConst$(ct$, "0")
    c$ = IrReg$: IrE "lt." + ct$ + " " + c$ + " " + x$ + " " + z$
    IrE "jz " + c$ + " ~F" + n$ + ".P"
    x$ = IrReg$: IrE "load." + ct$ + " " + x$ + " " + ff$
    c$ = IrReg$: IrE "lt." + ct$ + " " + c$ + " " + r$ + " " + x$
    IrE "jnz " + c$ + " ~F" + n$ + ".X"
    IrE "jmp ~F" + n$ + ".B"
    IrE "label ~F" + n$ + ".P"
    x$ = IrReg$: IrE "load." + ct$ + " " + x$ + " " + ff$
    c$ = IrReg$: IrE "gt." + ct$ + " " + c$ + " " + r$ + " " + x$
    IrE "jnz " + c$ + " ~F" + n$ + ".X"
    IrE "label ~F" + n$ + ".B"
END SUB

SUB IrNext (cid AS LONG)
    IrE "label ~F" + str2$(cid) + ".C"
    IrE "jmp ~F" + str2$(cid) + ".T"
    IrE "label ~F" + str2$(cid) + ".X"
END SUB

'--- SELECT CASE
SUB IrSelect (e$, cid AS LONG)
    DIM r$, t AS LONG, nm$, bid AS LONG
    bid = cid
    IrCtlId(controllevel) = bid: IrCtlPart(controllevel) = 0
    r$ = IrEval$(e$, t)
    nm$ = IrTemp$(IrT)
    IrE "store." + IrT + " " + nm$ + " " + r$
    IrCtlVar(controllevel) = nm$ + CHR$(3) + IrT + CHR$(3) + str2$(t)
END SUB

'CASE: raw text after CASE (comma separated items)
SUB IrCase (raw$, everycase AS LONG, iselse AS LONG)
    DIM bid AS LONG, p AS LONG, nm$, st$, c AS LONG, k AS LONG, it$, acc$, one$, x$, y$, t AS LONG, op$, i AS LONG, n AS LONG, sty AS LONG
    DIM sv(1 TO 100) AS STRING
    bid = IrCtlId(controllevel): p = IrCtlPart(controllevel)
    nm$ = IrPart$(IrCtlVar(controllevel) + CHR$(3), 1)
    nm$ = LEFT$(IrCtlVar(controllevel), INSTR(IrCtlVar(controllevel), CHR$(3)) - 1)
    st$ = MID$(IrCtlVar(controllevel), LEN(nm$) + 2)
    sty = VAL(MID$(st$, INSTR(st$, CHR$(3)) + 1))
    st$ = LEFT$(st$, INSTR(st$, CHR$(3)) - 1)
    IF p > 0 THEN
        IF everycase = 0 THEN IrE "jmp ~S" + str2$(bid) + ".E"
        IrE "label ~S" + str2$(bid) + "." + str2$(p)
    END IF
    p = p + 1: IrCtlPart(controllevel) = p
    IF iselse THEN
        IF everycase THEN
            x$ = IrReg$: IrE "load.i32 " + x$ + " $~SM" + str2$(bid)
            IrE "jnz " + x$ + " ~S" + str2$(bid) + "." + str2$(p)
        END IF
        EXIT SUB
    END IF
    c = IrSplitArgs(raw$)
    FOR k = 1 TO c: sv(k) = IrArgs(k): NEXT
    acc$ = ""
    FOR k = 1 TO c
        it$ = sv(k)
        n = numelements(it$)
        one$ = ""
        IF UCASE$(getelement$(it$, 1)) = "IS" THEN
            op$ = getelement$(it$, 2)
            SELECT CASE op$
                CASE "=": op$ = "eq"
                CASE "<>": op$ = "ne"
                CASE "<": op$ = "lt"
                CASE ">": op$ = "gt"
                CASE "<=": op$ = "le"
                CASE ">=": op$ = "ge"
            END SELECT
            x$ = IrReg$: IrE "load." + st$ + " " + x$ + " " + nm$
            y$ = IrEval$(fixoperationorder$(getelements$(it$, 3, n)), t)
            one$ = IrCmp$(op$, x$, st$, y$, IrT)
        ELSE
            i = 0
            FOR k2 = 1 TO n
                IF UCASE$(getelement$(it$, k2)) = "TO" THEN i = k2: EXIT FOR
            NEXT
            IF i THEN
                x$ = IrReg$: IrE "load." + st$ + " " + x$ + " " + nm$
                y$ = IrEval$(fixoperationorder$(getelements$(it$, 1, i - 1)), t)
                one$ = IrCmp$("ge", x$, st$, y$, IrT)
                x$ = IrReg$: IrE "load." + st$ + " " + x$ + " " + nm$
                y$ = IrEval$(fixoperationorder$(getelements$(it$, i + 1, n)), t)
                y$ = IrCmp$("le", x$, st$, y$, IrT)
                x$ = IrReg$: IrE "and.i32 " + x$ + " " + one$ + " " + y$
                one$ = x$
            ELSE
                x$ = IrReg$: IrE "load." + st$ + " " + x$ + " " + nm$
                y$ = IrEval$(fixoperationorder$(it$), t)
                one$ = IrCmp$("eq", x$, st$, y$, IrT)
            END IF
        END IF
        IF acc$ = "" THEN
            acc$ = one$
        ELSE
            x$ = IrReg$: IrE "or.i32 " + x$ + " " + acc$ + " " + one$: acc$ = x$
        END IF
    NEXT
    IrE "jz " + acc$ + " ~S" + str2$(bid) + "." + str2$(p)
    IF everycase THEN
        IF INSTR(IrFnDecls(IrFnCur), "|$~SM" + str2$(bid) + "|") = 0 THEN
            IrFnDecls(IrFnCur) = IrFnDecls(IrFnCur) + "|$~SM" + str2$(bid) + "|"
            IrHead "local $~SM" + str2$(bid) + " i32"
        END IF
        x$ = IrI32$(-1): IrE "store.i32 $~SM" + str2$(bid) + " " + x$
    END IF
END SUB

SUB IrEndSelect
    DIM bid AS LONG, p AS LONG
    bid = IrCtlId(controllevel): p = IrCtlPart(controllevel)
    IF p > 0 THEN IrE "label ~S" + str2$(bid) + "." + str2$(p)
    IrE "label ~S" + str2$(bid) + ".E"
END SUB

'--- DIM / REDIM of an array: idn, raw bounds "a TO b, c"
SUB IrDimArray (nm$, e$, kind$)
    DIM c AS LONG, k AS LONG, it$, n AS LONG, i AS LONG, lo$, hi$, regs$, k2 AS LONG
    DIM sv(1 TO 64) AS STRING
    c = IrSplitArgs(e$)
    FOR k = 1 TO c: sv(k) = IrArgs(k): NEXT
    FOR k = 1 TO c
        it$ = sv(k): n = numelements(it$): i = 0
        FOR k2 = 1 TO n
            IF UCASE$(getelement$(it$, k2)) = "TO" THEN i = k2: EXIT FOR
        NEXT
        IF i THEN
            lo$ = IrIndex$(fixoperationorder$(getelements$(it$, 1, i - 1)))
            hi$ = IrIndex$(fixoperationorder$(getelements$(it$, i + 1, n)))
        ELSE
            lo$ = IrConst$("i64", str2$(optionbase))
            hi$ = IrIndex$(fixoperationorder$(it$))
        END IF
        regs$ = regs$ + " " + lo$ + " " + hi$
    NEXT
    IrE kind$ + " " + nm$ + " " + str2$(c) + regs$
END SUB

'================================================================ packaging

SUB IrFinish (src$, outExe$)
    DIM fh AS INTEGER, i AS LONG, runner$, outPath$, ir$, r$, q$, l$, top AS LONG
    IF IrFailMsg <> "" THEN
        PRINT "QIR: unsupported: " + IrFailMsg
        SYSTEM 1
    END IF
    ir$ = tmpdir$ + "program.qir"
    fh = FREEFILE
    OPEN ir$ FOR OUTPUT AS #fh
    PRINT #fh, "QIR 1"
    IF DEPENDENCY(DEPENDENCY_CONSOLE_ONLY) THEN
        PRINT #fh, "meta console only"
    ELSEIF Console THEN
        PRINT #fh, "meta console"
    END IF
    PRINT #fh, IrMod;
    top = 0
    FOR i = 2 TO IrFnN
        IF IrFnName(i) = "SUB_QBNEX_TOPLEVEL_RUNTIME0" THEN top = i
    NEXT
    FOR i = 1 TO IrFnN
        IF IrFnRet(i) <> "" THEN PRINT #fh, "func " + IrFnName(i) + " -> " + IrFnRet(i) ELSE PRINT #fh, "func " + IrFnName(i)
        PRINT #fh, IrFnHead(i);
        PRINT #fh, IrFnBody(i);
        IF i = 1 THEN
            IF top THEN PRINT #fh, "call SUB_QBNEX_TOPLEVEL_RUNTIME0"
            PRINT #fh, "end"
        END IF
        PRINT #fh, "endfunc"
    NEXT
    CLOSE #fh

    IF os$ = "WIN" THEN
        runner$ = "crates\qb-exec\bin\windows\qb-exec.exe"
    ELSEIF MacOSX THEN
        runner$ = "crates/qb-exec/bin/macos/qb-exec"
    ELSE
        runner$ = "crates/qb-exec/bin/linux/qb-exec"
    END IF
    IF _FILEEXISTS(runner$) = 0 THEN PRINT "QIR: runner not found: " + runner$: SYSTEM 1

    outPath$ = outExe$
    IF outPath$ = "" THEN
        outPath$ = src$
        IF UCASE$(RIGHT$(outPath$, 4)) = ".BAS" THEN outPath$ = LEFT$(outPath$, LEN(outPath$) - 4)
        outPath$ = outPath$ + extension$
    ELSEIF LEFT$(outPath$, 1) <> "/" AND LEFT$(outPath$, 1) <> "\" AND MID$(outPath$, 2, 1) <> ":" THEN
        outPath$ = _STARTDIR$ + pathsep$ + outPath$
    END IF
    IF outPath$ <> "" AND INSTR(UCASE$(outPath$), ".EXE") = 0 AND os$ = "WIN" THEN outPath$ = outPath$ + ".exe"

    fh = FREEFILE: OPEN runner$ FOR BINARY AS #fh: r$ = SPACE$(LOF(fh)): GET #fh, , r$: CLOSE #fh
    fh = FREEFILE: OPEN ir$ FOR BINARY AS #fh: q$ = SPACE$(LOF(fh)): GET #fh, , q$: CLOSE #fh
    IF _FILEEXISTS(outPath$) THEN KILL outPath$
    fh = FREEFILE
    OPEN outPath$ FOR BINARY AS #fh
    PUT #fh, , r$
    PUT #fh, , q$
    l$ = MKL$(LEN(q$)) + MKL$(0) + "QBNEXIR0"
    PUT #fh, , l$
    CLOSE #fh
    IF os$ <> "WIN" THEN SHELL _HIDE "chmod +x " + CHR$(34) + outPath$ + CHR$(34)
    IF QuietMode = 0 THEN PRINT "QIR: wrote " + outPath$
    SYSTEM 0
END SUB
