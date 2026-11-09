' R11 spike: QIR emitter, selected with `qb --ir file.bas`.
'
' Lowers a BASIC subset to QIR text and packages it with the prebuilt Rust
' runner crates/qb-exec/bin/<os>/qb-exec. No C++ is generated and no C++
' compiler runs. Format and lowering rules: docs/rust-port/r11-emitter-design.md.
' The default C++ path is unchanged.
'
' Supported: PRINT, LET, IF (line/block), FOR/NEXT, DO/LOOP, WHILE/WEND,
' SELECT CASE [EVERYCASE], EXIT, _CONTINUE, GOTO/GOSUB/RETURN, ON n GOTO/GOSUB,
' SUB/FUNCTION (+STATIC), CALL, DIM/REDIM [SHARED] scalars and arrays, SWAP,
' MID$ statement, ON ERROR/RESUME/ERROR, END/SYSTEM, DEFtype, OPTION BASE,
' string/math builtins (see IrIsBuiltin%).

SUB IrCompileProgram (src$, outExe$)
    DIM fh AS INTEGER, l$, i AS LONG, runner$, outPath$, ir$, r$, q$

    IF _FILEEXISTS(src$) = 0 THEN IrLine = 0: IrFail "source file not found: " + src$
    fh = FREEFILE
    OPEN src$ FOR INPUT AS #fh
    IrSrcN = 0
    DO UNTIL EOF(fh)
        LINE INPUT #fh, l$
        IrSrcN = IrSrcN + 1
        IF IrSrcN > UBOUND(IrSrc) THEN REDIM _PRESERVE IrSrc(IrSrcN * 2) AS STRING
        IrSrc(IrSrcN) = l$
    LOOP
    CLOSE #fh

    'pass 1: DEFtype and SUB/FUNCTION signatures (calls may precede definitions)
    IrResetDefaults
    IrScope = "MAIN"
    FOR i = 1 TO IrSrcN
        IrLine = i
        IrTokenize IrSrc(i)
        IrP = 1
        IF IrTokN > 0 THEN
            SELECT CASE IrTokV(1)
                CASE "SUB": IrP = 2: IrDeclareSub "S"
                CASE "FUNCTION": IrP = 2: IrDeclareSub "F"
                CASE "DEFINT", "DEFLNG", "DEFSNG", "DEFDBL", "DEFSTR": IrP = 2: IrDefType IrTokV(1)
            END SELECT
        END IF
    NEXT

    'pass 2: emit QIR
    IrResetDefaults
    ir$ = tmpdir$ + "program.qir"
    IrOut = FREEFILE
    OPEN ir$ FOR OUTPUT AS #IrOut
    PRINT #IrOut, "QIR 0"
    IrScope = "MAIN"
    FOR i = 1 TO IrSrcN
        IrLine = i
        IrTokenize IrSrc(i)
        IrP = 1
        IrLineLabels
        IrStatementList -1, 0
    NEXT
    IrLine = IrSrcN
    IF IrBlkN > 0 THEN IrFail IrBlkKind(IrBlkN) + " without end"
    IF IrScope <> "MAIN" THEN IrFail "SUB/FUNCTION without END"
    CLOSE #IrOut

    'package: runner + QIR + u64 length + magic
    IF os$ = "WIN" THEN
        runner$ = "crates\qb-exec\bin\windows\qb-exec.exe"
    ELSEIF MacOSX THEN
        runner$ = "crates/qb-exec/bin/macos/qb-exec"
    ELSE
        runner$ = "crates/qb-exec/bin/linux/qb-exec"
    END IF
    IF _FILEEXISTS(runner$) = 0 THEN IrFail "runner not found: " + runner$ + " (build it with: cargo build --release -p qb-exec)"

    outPath$ = outExe$
    IF outPath$ = "" THEN
        outPath$ = src$
        IF UCASE$(RIGHT$(outPath$, 4)) = ".BAS" THEN outPath$ = LEFT$(outPath$, LEN(outPath$) - 4)
        outPath$ = outPath$ + extension$
    ELSEIF LEFT$(outPath$, 1) <> "/" AND LEFT$(outPath$, 1) <> "\" AND MID$(outPath$, 2, 1) <> ":" THEN
        outPath$ = _STARTDIR$ + pathsep$ + outPath$
    END IF

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

SUB IrFail (m$)
    PRINT "QIR error: line" + STR$(IrLine) + ": " + m$
    SYSTEM 1
END SUB

SUB IrEmit (s$)
    PRINT #IrOut, s$
END SUB

FUNCTION IrNum$ (n AS LONG)
    IrNum$ = LTRIM$(STR$(n))
END FUNCTION

FUNCTION IrNewId& ()
    IrUniq = IrUniq + 1
    IrNewId& = IrUniq
END FUNCTION

FUNCTION IrHex$ (s$)
    DIM i AS LONG, h$
    FOR i = 1 TO LEN(s$)
        h$ = h$ + RIGHT$("0" + HEX$(ASC(s$, i)), 2)
    NEXT
    IrHex$ = h$
END FUNCTION

SUB IrResetDefaults
    DIM i AS LONG
    FOR i = 1 TO 26: IrDefSfx(i) = "!": NEXT
    IrOptBase = 0
END SUB

'---------------------------------------------------------------- tokens

SUB IrTokenize (l$)
    DIM i AS LONG, n AS LONG, c$, j AS LONG, two$
    IrTokN = 0
    IF LEFT$(LTRIM$(l$), 1) = "$" THEN EXIT SUB 'metacommand ($CONSOLE etc.): no code in the IR spike
    n = LEN(l$)
    i = 1
    DO WHILE i <= n
        c$ = MID$(l$, i, 1)
        IF c$ = " " OR c$ = CHR$(9) THEN
            i = i + 1
        ELSEIF c$ = "'" THEN
            EXIT DO
        ELSEIF c$ = CHR$(34) THEN
            j = INSTR(i + 1, l$, CHR$(34))
            IF j = 0 THEN j = n + 1
            IrAddTok 3, MID$(l$, i + 1, j - i - 1)
            i = j + 1
        ELSEIF c$ = "&" AND IrCharIn%(UCASE$(MID$(l$, i + 1, 1)), "HOB") THEN
            j = i + 2
            DO WHILE j <= n
                IF IrCharIn%(UCASE$(MID$(l$, j, 1)), "0123456789ABCDEF") THEN j = j + 1 ELSE EXIT DO
            LOOP
            IF IrCharIn%(MID$(l$, j, 1), "%&") THEN j = j + 1
            IrAddTok 2, UCASE$(MID$(l$, i, j - i))
            i = j
        ELSEIF (c$ >= "0" AND c$ <= "9") OR (c$ = "." AND MID$(l$, i + 1, 1) >= "0" AND MID$(l$, i + 1, 1) <= "9") THEN
            j = i
            DO WHILE j <= n
                c$ = UCASE$(MID$(l$, j, 1))
                IF (c$ >= "0" AND c$ <= "9") OR c$ = "." THEN
                    j = j + 1
                ELSEIF (c$ = "E" OR c$ = "D") AND IrCharIn%(MID$(l$, j + 1, 1), "0123456789+-") THEN
                    j = j + 2
                ELSE
                    EXIT DO
                END IF
            LOOP
            IF IrCharIn%(MID$(l$, j, 1), "%&!#") THEN j = j + 1
            IrAddTok 2, UCASE$(MID$(l$, i, j - i))
            i = j
        ELSEIF (UCASE$(c$) >= "A" AND UCASE$(c$) <= "Z") OR c$ = "_" THEN
            j = i
            DO WHILE j <= n
                c$ = UCASE$(MID$(l$, j, 1))
                IF (c$ >= "A" AND c$ <= "Z") OR (c$ >= "0" AND c$ <= "9") OR c$ = "_" OR c$ = "." THEN j = j + 1 ELSE EXIT DO
            LOOP
            IF IrCharIn%(MID$(l$, j, 1), "%&!#$") THEN j = j + 1
            IF UCASE$(MID$(l$, i, j - i)) = "REM" THEN EXIT DO
            IrAddTok 1, UCASE$(MID$(l$, i, j - i))
            i = j
        ELSE
            two$ = MID$(l$, i, 2)
            IF two$ = "<>" OR two$ = "<=" OR two$ = ">=" THEN
                IrAddTok 4, two$: i = i + 2
            ELSE
                IrAddTok 4, c$: i = i + 1
            END IF
        END IF
    LOOP
END SUB

'-1 when c$ is one character found in set$ (INSTR finds "" anywhere).
FUNCTION IrCharIn% (c$, set$)
    IF LEN(c$) = 1 THEN IF INSTR(set$, c$) > 0 THEN IrCharIn% = -1
END FUNCTION

SUB IrAddTok (t AS INTEGER, v$)
    IrTokN = IrTokN + 1
    IF IrTokN > UBOUND(IrTokT) THEN
        REDIM _PRESERVE IrTokT(IrTokN * 2) AS INTEGER
        REDIM _PRESERVE IrTokV(IrTokN * 2) AS STRING
    END IF
    IrTokT(IrTokN) = t
    IrTokV(IrTokN) = v$
END SUB

'Current token text; "" at end of line; strings come back quoted so they never
'look like keywords or punctuation.
FUNCTION IrPeek$ ()
    IF IrP > IrTokN THEN IrPeek$ = "": EXIT FUNCTION
    IF IrTokT(IrP) = 3 THEN IrPeek$ = CHR$(34) + IrTokV(IrP) ELSE IrPeek$ = IrTokV(IrP)
END FUNCTION

FUNCTION IrPeekAt$ (k AS LONG)
    IF k > IrTokN THEN IrPeekAt$ = "": EXIT FUNCTION
    IF IrTokT(k) = 3 THEN IrPeekAt$ = CHR$(34) + IrTokV(k) ELSE IrPeekAt$ = IrTokV(k)
END FUNCTION

FUNCTION IrAccept% (v$)
    IF IrPeek$ = v$ THEN IrP = IrP + 1: IrAccept% = -1
END FUNCTION

SUB IrExpect (v$)
    IF IrAccept%(v$) = 0 THEN IrFail "expected " + v$ + " near '" + IrPeek$ + "'"
END SUB

FUNCTION IrAtEnd% ()
    DIM p$
    p$ = IrPeek$
    IF p$ = "" OR p$ = ":" THEN IrAtEnd% = -1
END FUNCTION

FUNCTION IrIdent$ ()
    IF IrP > IrTokN THEN IrFail "identifier expected"
    IF IrTokT(IrP) <> 1 THEN IrFail "identifier expected near '" + IrPeek$ + "'"
    IrIdent$ = IrTokV(IrP)
    IrP = IrP + 1
END FUNCTION

'Index of the token after the ")" matching the "(" at token k.
FUNCTION IrAfterParens& (k AS LONG)
    DIM depth AS LONG, j AS LONG
    FOR j = k TO IrTokN
        IF IrTokT(j) = 4 THEN
            IF IrTokV(j) = "(" THEN depth = depth + 1
            IF IrTokV(j) = ")" THEN
                depth = depth - 1
                IF depth = 0 THEN IrAfterParens& = j + 1: EXIT FUNCTION
            END IF
        END IF
    NEXT
    IrAfterParens& = IrTokN + 1
END FUNCTION

'---------------------------------------------------------------- names

FUNCTION IrTypeSfx$ (t$)
    SELECT CASE t$
        CASE "INTEGER": IrTypeSfx$ = "%"
        CASE "LONG": IrTypeSfx$ = "&"
        CASE "SINGLE": IrTypeSfx$ = "!"
        CASE "DOUBLE": IrTypeSfx$ = "#"
        CASE "STRING": IrTypeSfx$ = "$"
        CASE ELSE: IrFail "type not supported by the IR spike: " + t$
    END SELECT
END FUNCTION

FUNCTION IrHasSfx% (id$)
    IrHasSfx% = IrCharIn%(RIGHT$(id$, 1), "%&!#$")
END FUNCTION

FUNCTION IrBase$ (id$)
    IF IrHasSfx%(id$) THEN IrBase$ = LEFT$(id$, LEN(id$) - 1) ELSE IrBase$ = id$
END FUNCTION

'Default suffix from DEFtype by first letter.
FUNCTION IrDefault$ (id$)
    DIM c AS LONG
    c = ASC(id$) - 64
    IF c >= 1 AND c <= 26 THEN IrDefault$ = IrDefSfx(c) ELSE IrDefault$ = "!"
END FUNCTION

'DEFINT A-C, X ...
SUB IrDefType (k$)
    DIM sfx$, a$, b$, i AS LONG
    SELECT CASE k$
        CASE "DEFINT": sfx$ = "%"
        CASE "DEFLNG": sfx$ = "&"
        CASE "DEFSNG": sfx$ = "!"
        CASE "DEFDBL": sfx$ = "#"
        CASE "DEFSTR": sfx$ = "$"
    END SELECT
    DO
        a$ = LEFT$(IrIdent$, 1)
        b$ = a$
        IF IrAccept%("-") THEN b$ = LEFT$(IrIdent$, 1)
        FOR i = ASC(a$) - 64 TO ASC(b$) - 64
            IF i >= 1 AND i <= 26 THEN IrDefSfx(i) = sfx$
        NEXT
        IF IrAccept%(",") = 0 THEN EXIT DO
    LOOP
END SUB

SUB IrAddDim (n$, sfx$, irScope$)
    IrDimN = IrDimN + 1
    IF IrDimN > UBOUND(IrDimName) THEN
        REDIM _PRESERVE IrDimName(IrDimN * 2) AS STRING
        REDIM _PRESERVE IrDimSfx(IrDimN * 2) AS STRING
        REDIM _PRESERVE IrDimScope(IrDimN * 2) AS STRING
    END IF
    IrDimName(IrDimN) = n$: IrDimSfx(IrDimN) = sfx$: IrDimScope(IrDimN) = irScope$
END SUB

'Suffix recorded by DIM ... AS for key$ (scalar name or "NAME()"), "" if none.
FUNCTION IrDimLookup$ (key$)
    DIM i AS LONG
    FOR i = IrDimN TO 1 STEP -1
        IF IrDimName(i) = key$ AND IrDimScope(i) = IrScope THEN IrDimLookup$ = IrDimSfx(i): EXIT FUNCTION
    NEXT
    FOR i = IrDimN TO 1 STEP -1
        IF IrDimName(i) = key$ AND IrDimScope(i) = "*" THEN IrDimLookup$ = IrDimSfx(i): EXIT FUNCTION
    NEXT
END FUNCTION

'Variable name as the runner sees it: always carries its type suffix.
FUNCTION IrVarName$ (id$)
    DIM s$
    IF IrHasSfx%(id$) THEN IrVarName$ = id$: EXIT FUNCTION
    s$ = IrDimLookup$(id$)
    IF s$ = "" THEN s$ = IrDefault$(id$)
    IrVarName$ = id$ + s$
END FUNCTION

'Array name as the runner sees it: NAME<suffix>().
FUNCTION IrArrName$ (id$)
    DIM s$
    IF IrHasSfx%(id$) THEN IrArrName$ = id$ + "()": EXIT FUNCTION
    s$ = IrDimLookup$(id$ + "()")
    IF s$ = "" THEN s$ = IrDefault$(id$)
    IrArrName$ = id$ + s$ + "()"
END FUNCTION

FUNCTION IrLabel$ (n$)
    IrLabel$ = IrScope + "." + n$
END FUNCTION

'Jump target (name or line number) at the current token, in irScope$.
FUNCTION IrTarget$ (irScope$)
    IF IrP > IrTokN THEN IrFail "label expected"
    IF IrTokT(IrP) = 2 THEN
        IrTarget$ = irScope$ + ".L" + IrTokV(IrP)
    ELSEIF IrTokT(IrP) = 1 THEN
        IrTarget$ = irScope$ + "." + IrTokV(IrP)
    ELSE
        IrFail "label expected"
    END IF
    IrP = IrP + 1
END FUNCTION

'SUB/FUNCTION by identifier (suffix ignored), 0 if none.
FUNCTION IrFindSub& (id$)
    DIM i AS LONG, b$
    b$ = IrBase$(id$)
    FOR i = 1 TO IrSubN
        IF IrSubName(i) = b$ THEN IrFindSub& = i: EXIT FUNCTION
    NEXT
END FUNCTION

FUNCTION IrIsKeyword% (k$)
    SELECT CASE k$
        CASE "PRINT", "IF", "THEN", "ELSE", "ELSEIF", "END", "FOR", "TO", "STEP", "NEXT", "GOTO", "GOSUB", "RETURN", "SUB", "FUNCTION", "CALL", "DIM", "REDIM", "SHARED", "AS", "ON", "ERROR", "RESUME", "SYSTEM", "LET", "EXIT", "DECLARE", "AND", "OR", "XOR", "EQV", "IMP", "NOT", "MOD", "DO", "LOOP", "WHILE", "WEND", "UNTIL", "SELECT", "CASE", "IS", "SWAP", "OPTION", "BASE", "STATIC", "_PRESERVE", "EVERYCASE", "_CONTINUE", "DATA", "READ", "RESTORE", "RANDOMIZE", "USING"
            IrIsKeyword% = -1
    END SELECT
END FUNCTION

FUNCTION IrIsBuiltin% (k$)
    SELECT CASE k$
        CASE "LEN", "LEFT$", "RIGHT$", "MID$", "UCASE$", "LCASE$", "LTRIM$", "RTRIM$", "_TRIM$", "SPACE$", "STRING$", "INSTR", "STR$", "VAL", "CHR$", "ASC", "HEX$", "OCT$", "ABS", "SGN", "SQR", "LOG", "EXP", "SIN", "COS", "TAN", "ATN", "INT", "FIX", "CINT", "CLNG", "CSNG", "CDBL", "_ROUND", "MKI$", "MKL$", "MKS$", "MKD$", "CVI", "CVL", "CVS", "CVD", "MKSMBF$", "MKDMBF$", "CVSMBF", "CVDMBF"
            IrIsBuiltin% = -1
    END SELECT
END FUNCTION

'pass 1: SUB|FUNCTION name [(p [AS type], ...)] [STATIC]
SUB IrDeclareSub (kind$)
    DIM n$, p$, plist$, argc AS LONG, b$
    n$ = IrIdent$
    b$ = IrBase$(n$)
    IF IrFindSub&(b$) THEN IrFail "duplicate SUB/FUNCTION " + b$
    IF IrAccept%("(") THEN
        IF IrAccept%(")") = 0 THEN
            DO
                p$ = IrIdent$
                IF IrPeek$ = "(" THEN IrFail "array parameters are not supported by the IR spike"
                IF IrAccept%("AS") THEN
                    IrAddDim p$, IrTypeSfx$(IrIdent$), b$
                    p$ = p$ + IrDimSfx(IrDimN)
                ELSEIF IrHasSfx%(p$) = 0 THEN
                    p$ = p$ + IrDefault$(p$)
                END IF
                plist$ = plist$ + " " + p$
                argc = argc + 1
                IF IrAccept%(")") THEN EXIT DO
                IrExpect ","
            LOOP
        END IF
    END IF
    IF IrAccept%("STATIC") THEN plist$ = plist$ + " STATIC"
    IrSubN = IrSubN + 1
    IF IrSubN > UBOUND(IrSubName) THEN
        REDIM _PRESERVE IrSubName(IrSubN * 2) AS STRING
        REDIM _PRESERVE IrSubIr(IrSubN * 2) AS STRING
        REDIM _PRESERVE IrSubKind(IrSubN * 2) AS STRING
        REDIM _PRESERVE IrSubParams(IrSubN * 2) AS STRING
        REDIM _PRESERVE IrSubArgc(IrSubN * 2) AS LONG
    END IF
    IrSubName(IrSubN) = b$: IrSubKind(IrSubN) = kind$
    IF kind$ = "F" AND IrHasSfx%(n$) = 0 THEN n$ = n$ + IrDefault$(n$)
    IrSubIr(IrSubN) = n$
    IrSubParams(IrSubN) = plist$: IrSubArgc(IrSubN) = argc
END SUB

'---------------------------------------------------------------- blocks

SUB IrPushBlock (kind$, id AS LONG, v$)
    IrBlkN = IrBlkN + 1
    IF IrBlkN > UBOUND(IrBlkKind) THEN
        REDIM _PRESERVE IrBlkKind(IrBlkN * 2) AS STRING
        REDIM _PRESERVE IrBlkId(IrBlkN * 2) AS LONG
        REDIM _PRESERVE IrBlkPart(IrBlkN * 2) AS LONG
        REDIM _PRESERVE IrBlkVar(IrBlkN * 2) AS STRING
    END IF
    IrBlkKind(IrBlkN) = kind$: IrBlkId(IrBlkN) = id: IrBlkPart(IrBlkN) = 0: IrBlkVar(IrBlkN) = v$
END SUB

SUB IrTopBlock (kind$, what$)
    IF IrBlkN = 0 THEN IrFail what$
    IF IrBlkKind(IrBlkN) <> kind$ THEN IrFail what$
END SUB

'Innermost loop of the given kinds ("FOR", "DO WHILE"), 0 if none.
FUNCTION IrFindLoop& (kinds$)
    DIM i AS LONG
    FOR i = IrBlkN TO 1 STEP -1
        IF INSTR(" " + kinds$ + " ", " " + IrBlkKind(i) + " ") THEN IrFindLoop& = i: EXIT FUNCTION
    NEXT
END FUNCTION

FUNCTION IrLoopPrefix$ (b AS LONG)
    IF IrBlkKind(b) = "FOR" THEN IrLoopPrefix$ = "~F" + IrNum$(IrBlkId(b)) ELSE IrLoopPrefix$ = "~D" + IrNum$(IrBlkId(b))
END FUNCTION

'---------------------------------------------------------------- statements

'Line number or "name:" at the start of a line (a SUB name is a call).
SUB IrLineLabels
    IF IrTokN = 0 THEN EXIT SUB
    IF IrTokT(1) = 2 THEN
        IrEmit "LABEL " + IrLabel$("L" + IrTokV(1))
        IrP = 2
    ELSEIF IrTokT(1) = 1 AND IrTokN >= 2 THEN
        IF IrTokV(2) = ":" AND IrIsKeyword%(IrTokV(1)) = 0 AND IrFindSub&(IrTokV(1)) = 0 THEN
            IrEmit "LABEL " + IrLabel$(IrTokV(1))
            IrP = 3
        END IF
    END IF
END SUB

'Statements separated by ":". `top` = real statements of the line (they get a
'LINE marker, the RESUME granularity); stopAtElse ends a single-line THEN part.
SUB IrStatementList (top AS INTEGER, stopAtElse AS INTEGER)
    DIM p$
    DO
        DO WHILE IrAccept%(":"): LOOP
        p$ = IrPeek$
        IF p$ = "" THEN EXIT SUB
        IF stopAtElse AND p$ = "ELSE" THEN EXIT SUB
        IrStatement top
        p$ = IrPeek$
        IF p$ <> "" AND p$ <> ":" AND NOT (stopAtElse AND p$ = "ELSE") THEN IrFail "unexpected '" + p$ + "'"
    LOOP
END SUB

SUB IrStatement (top AS INTEGER)
    DIM k$, n$, id AS LONG, s AS LONG, b AS LONG
    k$ = IrPeek$
    IF top THEN IrEmit "LINE " + IrNum$(IrLine)
    IF IrTokT(IrP) <> 1 AND k$ <> "?" THEN IrFail "statement expected near '" + k$ + "'"
    IrP = IrP + 1
    SELECT CASE k$
        CASE "PRINT", "?"
            IrPrint
        CASE "LET"
            IrAssign IrIdent$
        CASE "IF"
            IrIf top
        CASE "ELSEIF"
            IrTopBlock "IF", "ELSEIF without IF"
            id = IrBlkId(IrBlkN)
            IrEmit "JMP ~IF" + IrNum$(id) + ".END"
            IrEmit "LABEL ~IF" + IrNum$(id) + "." + IrNum$(IrBlkPart(IrBlkN))
            IrBlkPart(IrBlkN) = IrBlkPart(IrBlkN) + 1
            IrExpr
            IrExpect "THEN"
            IrEmit "JZ ~IF" + IrNum$(id) + "." + IrNum$(IrBlkPart(IrBlkN))
        CASE "ELSE"
            IrTopBlock "IF", "ELSE without IF"
            id = IrBlkId(IrBlkN)
            IrEmit "JMP ~IF" + IrNum$(id) + ".END"
            IrEmit "LABEL ~IF" + IrNum$(id) + "." + IrNum$(IrBlkPart(IrBlkN))
            IrBlkPart(IrBlkN) = IrBlkPart(IrBlkN) + 1
            IF IrAtEnd% = 0 THEN IrStatementList 0, 0
        CASE "END"
            n$ = IrPeek$
            IF n$ = "IF" THEN
                IrP = IrP + 1
                IrTopBlock "IF", "END IF without IF"
                id = IrBlkId(IrBlkN)
                IrEmit "LABEL ~IF" + IrNum$(id) + "." + IrNum$(IrBlkPart(IrBlkN))
                IrEmit "LABEL ~IF" + IrNum$(id) + ".END"
                IrBlkN = IrBlkN - 1
            ELSEIF n$ = "SELECT" THEN
                IrP = IrP + 1
                IrTopBlock "SELECT", "END SELECT without SELECT CASE"
                id = IrBlkId(IrBlkN)
                IrEmit "LABEL ~S" + IrNum$(id) + "." + IrNum$(IrBlkPart(IrBlkN))
                IrEmit "LABEL ~S" + IrNum$(id) + ".END"
                IrBlkN = IrBlkN - 1
            ELSEIF n$ = "SUB" OR n$ = "FUNCTION" THEN
                IrP = IrP + 1
                IF IrScope = "MAIN" THEN IrFail "END " + n$ + " without " + n$
                IF IrBlkN > 0 THEN IrFail IrBlkKind(IrBlkN) + " without end"
                IrEmit "LABEL " + IrLabel$("~EXIT")
                IrEmit "ENDSUB"
                IrScope = "MAIN"
            ELSE
                IrEmit "END"
            END IF
        CASE "SYSTEM"
            IrEmit "END"
        CASE "FOR"
            IrFor
        CASE "NEXT"
            IrNext
        CASE "DO"
            IrDo
        CASE "LOOP"
            IrLoop
        CASE "WHILE"
            id = IrNewId&
            IrEmit "LABEL ~D" + IrNum$(id) + ".T"
            IrExpr
            IrEmit "JZ ~D" + IrNum$(id) + ".X"
            IrPushBlock "WHILE", id, ""
        CASE "WEND"
            IrTopBlock "WHILE", "WEND without WHILE"
            n$ = "~D" + IrNum$(IrBlkId(IrBlkN))
            IrEmit "LABEL " + n$ + ".C"
            IrEmit "JMP " + n$ + ".T"
            IrEmit "LABEL " + n$ + ".X"
            IrBlkN = IrBlkN - 1
        CASE "SELECT"
            IrSelect
        CASE "CASE"
            IrCase
        CASE "GOTO"
            IrEmit "JMP " + IrTarget$(IrScope)
        CASE "GOSUB"
            IrEmit "GOSUB " + IrTarget$(IrScope)
        CASE "RETURN"
            IrEmit "RETURN"
        CASE "SUB", "FUNCTION"
            IF top = 0 OR IrScope <> "MAIN" THEN IrFail k$ + " not allowed here"
            n$ = IrIdent$
            s = IrFindSub&(n$)
            IrEmit RTRIM$(k$ + " " + IrSubIr(s) + " " + IrNum$(IrSubArgc(s)) + IrSubParams(s))
            IrScope = IrSubName(s)
            IrP = IrTokN + 1 'parameters were read in pass 1
        CASE "EXIT"
            n$ = IrIdent$
            IF n$ = "SUB" OR n$ = "FUNCTION" THEN
                IF IrScope = "MAIN" THEN IrFail "EXIT " + n$ + " outside " + n$
                IrEmit "JMP " + IrLabel$("~EXIT")
            ELSEIF n$ = "FOR" THEN
                b = IrFindLoop&("FOR")
                IF b = 0 THEN IrFail "EXIT FOR outside FOR"
                IrEmit "JMP " + IrLoopPrefix$(b) + ".X"
            ELSEIF n$ = "DO" THEN
                b = IrFindLoop&("DO")
                IF b = 0 THEN IrFail "EXIT DO outside DO"
                IrEmit "JMP " + IrLoopPrefix$(b) + ".X"
            ELSE
                IrFail "EXIT " + n$ + " not supported"
            END IF
        CASE "_CONTINUE"
            b = IrFindLoop&("FOR DO WHILE")
            IF b = 0 THEN IrFail "_CONTINUE outside a loop"
            IrEmit "JMP " + IrLoopPrefix$(b) + ".C"
        CASE "DECLARE"
            IrP = IrTokN + 1
        CASE "DEFINT", "DEFLNG", "DEFSNG", "DEFDBL", "DEFSTR"
            IrDefType k$
        CASE "OPTION"
            IrExpect "BASE"
            IrOptBase = VAL(IrTokV(IrP)): IrP = IrP + 1
        CASE "DIM"
            IrDim 0
        CASE "REDIM"
            IrDim -1
        CASE "SWAP"
            IrRefArg
            IrExpect ","
            IrRefArg
            IrEmit "SWAP"
        CASE "ON"
            IrOn
        CASE "ERROR"
            IrExpr
            IrEmit "ERROR"
        CASE "RESUME"
            IF IrAtEnd% THEN
                IrEmit "RESUME 0"
            ELSEIF IrAccept%("NEXT") THEN
                IrEmit "RESUME NEXT"
            ELSE
                IF IrPeek$ = "0" THEN
                    IrEmit "RESUME 0": IrP = IrP + 1
                ELSE
                    IrEmit "RESUME " + IrTarget$("MAIN")
                END IF
            END IF
        CASE "CALL"
            IrCall IrIdent$, -1
        CASE "DATA"
            IrData
        CASE "READ"
            DO
                n$ = IrIdent$
                IF IrAccept%("(") THEN
                    s = IrIndices&
                    IrEmit "READ " + IrNum$(-(RIGHT$(IrArrName$(n$), 3) = "$()"))
                    IrEmit "STOREE " + IrArrName$(n$) + " " + IrNum$(s)
                ELSE
                    IrEmit "READ " + IrNum$(-(RIGHT$(IrVarName$(n$), 1) = "$"))
                    IrEmit "STORE " + IrVarName$(n$)
                END IF
                IF IrAccept%(",") = 0 THEN EXIT DO
            LOOP
        CASE "RESTORE"
            IF IrAtEnd% THEN IrEmit "RESTORE" ELSE IrEmit "RESTORE " + IrTarget$(IrScope)
        CASE "RANDOMIZE"
            IF IrAccept%("USING") THEN
                IrExpr: IrEmit "RANDOMIZE 1"
            ELSE
                IF IrAtEnd% THEN IrFail "RANDOMIZE without a seed (prompt) is not supported by the IR spike"
                IrExpr: IrEmit "RANDOMIZE 0"
            END IF
        CASE "MID$"
            'MID$(v$, start[, n]) = r$
            IrExpect "("
            n$ = IrVarName$(IrIdent$)
            IrEmit "LOAD " + n$
            IrExpect ","
            IrExpr
            s = 3
            IF IrAccept%(",") THEN IrExpr: s = 4
            IrExpect ")"
            IrExpect "="
            IrExpr
            IrEmit "FN MID$SET " + IrNum$(s)
            IrEmit "STORE " + n$
        CASE ELSE
            IF IrPeek$ = "=" THEN
                IrAssign k$
            ELSEIF IrFindSub&(k$) THEN
                IrCall k$, 0
            ELSEIF IrPeek$ = "(" THEN
                IrAssign k$
            ELSE
                IrFail "statement not supported by the IR spike: " + k$
            END IF
    END SELECT
END SUB

'DATA items come from the raw source line (case and spaces matter); DATA
'takes the rest of the line. Each item becomes `DATA <hex>`.
SUB IrData
    DIM raw$, i AS LONG, n AS LONG, c$, q AS INTEGER, item$, dataPos AS LONG, quoted AS INTEGER
    raw$ = IrSrc(IrLine)
    n = LEN(raw$)
    FOR i = 1 TO n - 3
        c$ = MID$(raw$, i, 1)
        IF c$ = CHR$(34) THEN q = NOT q
        IF q = 0 AND UCASE$(MID$(raw$, i, 4)) = "DATA" THEN
            IF i = 1 OR IrCharIn%(MID$(raw$, i - 1, 1), " :" + CHR$(9)) THEN
                IF i + 4 > n OR IrCharIn%(MID$(raw$, i + 4, 1), " " + CHR$(9)) THEN dataPos = i + 4: EXIT FOR
            END IF
        END IF
    NEXT
    IF dataPos = 0 THEN IrFail "DATA must be the first statement of its line in the IR spike"
    q = 0: item$ = "": quoted = 0
    FOR i = dataPos TO n + 1
        IF i <= n THEN c$ = MID$(raw$, i, 1) ELSE c$ = ","
        IF c$ = CHR$(34) THEN
            q = NOT q: quoted = -1
        ELSEIF c$ = "," AND q = 0 THEN
            IF quoted = 0 THEN item$ = LTRIM$(RTRIM$(item$))
            IF item$ = "" THEN IrEmit "DATA" ELSE IrEmit "DATA " + IrHex$(item$)
            item$ = "": quoted = 0
        ELSEIF q OR quoted = 0 THEN
            item$ = item$ + c$
        END IF
    NEXT
    IrP = IrTokN + 1
END SUB

SUB IrAssign (id$)
    DIM n AS LONG, a$
    IF IrAccept%("(") THEN
        a$ = IrArrName$(id$)
        n = IrIndices&
        IrExpect "="
        IrExpr
        IrEmit "STOREE " + a$ + " " + IrNum$(n)
        EXIT SUB
    END IF
    IrExpect "="
    IrExpr
    IrEmit "STORE " + IrVarName$(id$)
END SUB

'Index expressions after "(" up to ")"; returns how many.
FUNCTION IrIndices& ()
    DIM n AS LONG
    DO
        IrExpr
        n = n + 1
        IF IrAccept%(",") = 0 THEN EXIT DO
    LOOP
    IrExpect ")"
    IrIndices& = n
END FUNCTION

'Reference to a variable or array element (SWAP, BYREF arguments).
SUB IrRefArg
    DIM id$, n AS LONG
    id$ = IrIdent$
    IF IrAccept%("(") THEN
        n = IrIndices&
        IrEmit "REFE " + IrArrName$(id$) + " " + IrNum$(n)
    ELSE
        IrEmit "REF " + IrVarName$(id$)
    END IF
END SUB

SUB IrPrint
    DIM nl AS INTEGER, p$
    nl = -1
    DO
        p$ = IrPeek$
        IF p$ = "" OR p$ = ":" OR p$ = "ELSE" THEN EXIT DO
        IF p$ = ";" THEN
            IrP = IrP + 1: nl = 0
        ELSEIF p$ = "," THEN
            IrP = IrP + 1: nl = 0
            IrEmit "PRINTZONE"
        ELSE
            IrExpr
            IrEmit "PRINT"
            nl = -1
        END IF
    LOOP
    IF nl THEN IrEmit "PRINTNL"
END SUB

'THEN/ELSE part of a single-line IF: a line number means GOTO.
SUB IrThenPart
    IF IrP <= IrTokN THEN
        IF IrTokT(IrP) = 2 THEN IrEmit "JMP " + IrTarget$(IrScope): EXIT SUB
    END IF
    IrStatementList 0, -1
END SUB

SUB IrIf (top AS INTEGER)
    DIM id AS LONG, t$
    id = IrNewId&
    t$ = "~IF" + IrNum$(id)
    IrExpr
    IF IrAccept%("GOTO") THEN
        IrEmit "JNZ " + IrTarget$(IrScope)
        EXIT SUB
    END IF
    IrExpect "THEN"
    IF IrPeek$ = "" THEN
        IF top = 0 THEN IrFail "block IF inside a single-line IF"
        IrPushBlock "IF", id, ""
        IrBlkPart(IrBlkN) = 1
        IrEmit "JZ " + t$ + ".1"
        EXIT SUB
    END IF
    IrEmit "JZ " + t$ + ".ELSE"
    IrThenPart
    IF IrAccept%("ELSE") THEN
        IrEmit "JMP " + t$ + ".END"
        IrEmit "LABEL " + t$ + ".ELSE"
        IrThenPart
    ELSE
        IrEmit "LABEL " + t$ + ".ELSE"
    END IF
    IrEmit "LABEL " + t$ + ".END"
END SUB

'FOR v = a TO b [STEP c]: limit and step are evaluated once into hidden
'untyped temporaries; FORTEST picks the comparison by the sign of the step.
SUB IrFor
    DIM id AS LONG, v$, t$
    id = IrNewId&
    t$ = "~F" + IrNum$(id)
    v$ = IrVarName$(IrIdent$)
    IrExpect "="
    IrExpr
    IrEmit "STORE " + v$
    IrExpect "TO"
    IrExpr
    IrEmit "STORE " + t$ + ".L"
    IF IrAccept%("STEP") THEN IrExpr ELSE IrEmit "PUSH I 1"
    IrEmit "STORE " + t$ + ".S"
    IrEmit "LABEL " + t$ + ".T"
    IrEmit "LOAD " + v$
    IrEmit "LOAD " + t$ + ".L"
    IrEmit "LOAD " + t$ + ".S"
    IrEmit "FORTEST"
    IrEmit "JZ " + t$ + ".X"
    IrPushBlock "FOR", id, v$
END SUB

SUB IrNext
    DIM t$, v$
    DO
        IrTopBlock "FOR", "NEXT without FOR"
        v$ = IrBlkVar(IrBlkN)
        IF IrAtEnd% = 0 AND IrPeek$ <> "ELSE" THEN
            IF IrVarName$(IrIdent$) <> v$ THEN IrFail "NEXT variable does not match FOR"
        END IF
        t$ = "~F" + IrNum$(IrBlkId(IrBlkN))
        IrEmit "LABEL " + t$ + ".C"
        IrEmit "LOAD " + v$
        IrEmit "LOAD " + t$ + ".S"
        IrEmit "OP ADD"
        IrEmit "STORE " + v$
        IrEmit "JMP " + t$ + ".T"
        IrEmit "LABEL " + t$ + ".X"
        IrBlkN = IrBlkN - 1
        IF IrAccept%(",") = 0 THEN EXIT DO
    LOOP
END SUB

SUB IrDo
    DIM id AS LONG, t$
    id = IrNewId&
    t$ = "~D" + IrNum$(id)
    IrEmit "LABEL " + t$ + ".T"
    IF IrAccept%("WHILE") THEN
        IrExpr: IrEmit "JZ " + t$ + ".X"
    ELSEIF IrAccept%("UNTIL") THEN
        IrExpr: IrEmit "JNZ " + t$ + ".X"
    END IF
    IrPushBlock "DO", id, ""
END SUB

SUB IrLoop
    DIM t$
    IrTopBlock "DO", "LOOP without DO"
    t$ = "~D" + IrNum$(IrBlkId(IrBlkN))
    IrEmit "LABEL " + t$ + ".C"
    IF IrAccept%("WHILE") THEN
        IrExpr: IrEmit "JNZ " + t$ + ".T"
    ELSEIF IrAccept%("UNTIL") THEN
        IrExpr: IrEmit "JZ " + t$ + ".T"
    ELSE
        IrEmit "JMP " + t$ + ".T"
    END IF
    IrEmit "LABEL " + t$ + ".X"
    IrBlkN = IrBlkN - 1
END SUB

'SELECT [CASE|EVERYCASE] expr: selector in ~Sn.V, "matched" flag in ~Sn.M.
'IrBlkVar holds "E" for EVERYCASE.
SUB IrSelect
    DIM id AS LONG, t$, every$
    IF IrAccept%("EVERYCASE") THEN every$ = "E" ELSE IrExpect "CASE"
    id = IrNewId&
    t$ = "~S" + IrNum$(id)
    IrExpr
    IrEmit "STORE " + t$ + ".V"
    IrEmit "PUSH I 0"
    IrEmit "STORE " + t$ + ".M"
    IrPushBlock "SELECT", id, every$
END SUB

SUB IrCase
    DIM t$, o$, items AS LONG
    IrTopBlock "SELECT", "CASE without SELECT CASE"
    t$ = "~S" + IrNum$(IrBlkId(IrBlkN))
    IF IrBlkPart(IrBlkN) > 0 AND IrBlkVar(IrBlkN) <> "E" THEN IrEmit "JMP " + t$ + ".END"
    IrEmit "LABEL " + t$ + "." + IrNum$(IrBlkPart(IrBlkN))
    IrBlkPart(IrBlkN) = IrBlkPart(IrBlkN) + 1
    IF IrAccept%("ELSE") THEN
        IrEmit "LOAD " + t$ + ".M"
        IrEmit "JNZ " + t$ + "." + IrNum$(IrBlkPart(IrBlkN))
        EXIT SUB
    END IF
    DO
        IrEmit "LOAD " + t$ + ".V"
        IF IrAccept%("IS") THEN
            SELECT CASE IrPeek$
                CASE "=": o$ = "EQ"
                CASE "<>": o$ = "NE"
                CASE "<": o$ = "LT"
                CASE ">": o$ = "GT"
                CASE "<=": o$ = "LE"
                CASE ">=": o$ = "GE"
                CASE ELSE: IrFail "relational operator expected after IS"
            END SELECT
            IrP = IrP + 1
            IrExpr
            IrEmit "OP " + o$
        ELSE
            IrExpr
            IF IrAccept%("TO") THEN
                IrEmit "OP GE"
                IrEmit "LOAD " + t$ + ".V"
                IrExpr
                IrEmit "OP LE"
                IrEmit "OP AND"
            ELSE
                IrEmit "OP EQ"
            END IF
        END IF
        items = items + 1
        IF items > 1 THEN IrEmit "OP OR"
        IF IrAccept%(",") = 0 THEN EXIT DO
    LOOP
    IrEmit "JZ " + t$ + "." + IrNum$(IrBlkPart(IrBlkN))
    IrEmit "PUSH I -1"
    IrEmit "STORE " + t$ + ".M"
END SUB

'ON ERROR GOTO label|0, ON n GOTO|GOSUB l1, l2, ...
SUB IrOn
    DIM id AS LONG, t$, how$, k AS LONG
    IF IrAccept%("ERROR") THEN
        IrExpect "GOTO"
        IF IrPeek$ = "0" THEN
            IrEmit "ONERROR 0": IrP = IrP + 1
        ELSE
            IrEmit "ONERROR " + IrTarget$("MAIN")
        END IF
        EXIT SUB
    END IF
    id = IrNewId&
    t$ = "~O" + IrNum$(id)
    IrExpr
    IrEmit "STORE " + t$ + ".V"
    how$ = IrIdent$
    IF how$ <> "GOTO" AND how$ <> "GOSUB" THEN IrFail "GOTO or GOSUB expected"
    DO
        k = k + 1
        IrEmit "LOAD " + t$ + ".V"
        IrEmit "PUSH I " + IrNum$(k)
        IrEmit "OP EQ"
        IrEmit "JZ " + t$ + "." + IrNum$(k)
        IF how$ = "GOTO" THEN
            IrEmit "JMP " + IrTarget$(IrScope)
        ELSE
            IrEmit "GOSUB " + IrTarget$(IrScope)
            IrEmit "JMP " + t$ + ".END"
        END IF
        IrEmit "LABEL " + t$ + "." + IrNum$(k)
        IF IrAccept%(",") = 0 THEN EXIT DO
    LOOP
    IrEmit "LABEL " + t$ + ".END"
END SUB

'DIM/REDIM [SHARED] [_PRESERVE] name[(dims)] [AS type], ...
SUB IrDim (isRedim AS INTEGER)
    DIM isShared AS INTEGER, preserve AS INTEGER, n$, irScope$, nm$, sfx$, isArr AS INTEGER, dims AS LONG
    IF IrAccept%("SHARED") THEN isShared = -1
    IF IrAccept%("_PRESERVE") THEN preserve = -1
    irScope$ = IrScope
    IF isShared THEN irScope$ = "*"
    DO
        n$ = IrIdent$
        isArr = 0: dims = 0
        IF IrAccept%("(") THEN
            isArr = -1
            IF IrAccept%(")") = 0 THEN
                DO
                    IF IrDimHasTo% = 0 THEN IrEmit "PUSH I " + IrNum$(IrOptBase) 'lower bound = OPTION BASE
                    IrExpr
                    IF IrAccept%("TO") THEN IrExpr
                    dims = dims + 1
                    IF IrAccept%(",") = 0 THEN EXIT DO
                LOOP
                IrExpect ")"
            END IF
        END IF
        sfx$ = ""
        IF IrAccept%("AS") THEN sfx$ = IrTypeSfx$(IrIdent$)
        IF isArr THEN
            IF sfx$ <> "" THEN IrAddDim n$ + "()", sfx$, irScope$
            nm$ = IrArrName$(n$)
        ELSE
            IF sfx$ <> "" THEN IrAddDim n$, sfx$, irScope$
            nm$ = IrVarName$(n$)
        END IF
        IF isShared THEN IrEmit "SHARED " + nm$
        IF isArr AND dims > 0 THEN
            IF isRedim THEN
                IF preserve THEN IrEmit "REDIMA " + nm$ + " " + IrNum$(dims) + " 1" ELSE IrEmit "REDIMA " + nm$ + " " + IrNum$(dims) + " 0"
            ELSE
                IrEmit "DIMA " + nm$ + " " + IrNum$(dims)
            END IF
        END IF
        IF IrAccept%(",") = 0 THEN EXIT DO
    LOOP
END SUB

'-1 when the current array dimension is written `lb TO ub`.
FUNCTION IrDimHasTo% ()
    DIM j AS LONG, depth AS LONG
    FOR j = IrP TO IrTokN
        IF IrTokT(j) = 4 THEN
            IF IrTokV(j) = "(" THEN depth = depth + 1
            IF IrTokV(j) = ")" THEN
                IF depth = 0 THEN EXIT FUNCTION
                depth = depth - 1
            END IF
            IF IrTokV(j) = "," AND depth = 0 THEN EXIT FUNCTION
        ELSEIF IrTokT(j) = 1 THEN
            IF IrTokV(j) = "TO" AND depth = 0 THEN IrDimHasTo% = -1: EXIT FUNCTION
        END IF
    NEXT
END FUNCTION

'Arguments that are a bare variable or array element are passed by reference
'(QBasic rule); `name (x)` without CALL passes (x) as a value. Returns argc.
FUNCTION IrArgs& (parens AS INTEGER)
    DIM argc AS LONG, j AS LONG, nx$
    DO
        IF parens THEN
            IF IrAccept%(")") THEN EXIT DO
        ELSE
            IF IrAtEnd% OR IrPeek$ = "ELSE" THEN EXIT DO
        END IF
        IF IrTokT(IrP) = 1 AND IrIsBuiltin%(IrTokV(IrP)) = 0 AND IrFindSub&(IrTokV(IrP)) = 0 AND IrIsKeyword%(IrTokV(IrP)) = 0 THEN
            j = IrP + 1
            IF IrPeekAt$(j) = "(" THEN j = IrAfterParens&(j)
            nx$ = IrPeekAt$(j)
            IF nx$ = "," OR nx$ = ")" OR nx$ = ":" OR nx$ = "" OR nx$ = "ELSE" THEN IrRefArg ELSE IrExpr
        ELSE
            IrExpr
        END IF
        argc = argc + 1
        IF IrAccept%(",") = 0 THEN
            IF parens THEN IrExpect ")"
            EXIT DO
        END IF
    LOOP
    IrArgs& = argc
END FUNCTION

SUB IrCall (n$, parens AS INTEGER)
    DIM s AS LONG, argc AS LONG
    s = IrFindSub&(n$)
    IF s = 0 THEN IrFail "SUB not found: " + n$
    IF parens THEN
        IF IrAccept%("(") = 0 THEN parens = 0
    END IF
    argc = IrArgs&(parens)
    IF argc <> IrSubArgc(s) THEN IrFail "wrong number of arguments for " + n$
    IrEmit "CALL " + IrSubIr(s) + " " + IrNum$(argc)
END SUB

'---------------------------------------------------------------- expressions
'Precedence, lowest first: IMP, EQV, XOR, OR, AND, NOT, relational, + -, MOD,
'\, * /, unary -, ^. Each level emits its operands first (postfix order).

SUB IrExpr
    IrImp
END SUB

SUB IrImp
    IrEqv
    DO WHILE IrAccept%("IMP"): IrEqv: IrEmit "OP IMP": LOOP
END SUB

SUB IrEqv
    IrXor
    DO WHILE IrAccept%("EQV"): IrXor: IrEmit "OP EQV": LOOP
END SUB

SUB IrXor
    IrOr
    DO WHILE IrAccept%("XOR"): IrOr: IrEmit "OP XOR": LOOP
END SUB

SUB IrOr
    IrAnd
    DO WHILE IrAccept%("OR"): IrAnd: IrEmit "OP OR": LOOP
END SUB

SUB IrAnd
    IrNot
    DO WHILE IrAccept%("AND"): IrNot: IrEmit "OP AND": LOOP
END SUB

SUB IrNot
    IF IrAccept%("NOT") THEN
        IrNot
        IrEmit "UN NOT"
    ELSE
        IrRel
    END IF
END SUB

SUB IrRel
    DIM o$
    IrAdd
    DO
        SELECT CASE IrPeek$
            CASE "=": o$ = "EQ"
            CASE "<>": o$ = "NE"
            CASE "<": o$ = "LT"
            CASE ">": o$ = "GT"
            CASE "<=": o$ = "LE"
            CASE ">=": o$ = "GE"
            CASE ELSE: EXIT DO
        END SELECT
        IrP = IrP + 1
        IrAdd
        IrEmit "OP " + o$
    LOOP
END SUB

SUB IrAdd
    DIM o$
    IrMod
    DO
        o$ = IrPeek$
        IF o$ = "+" THEN
            IrP = IrP + 1: IrMod: IrEmit "OP ADD"
        ELSEIF o$ = "-" THEN
            IrP = IrP + 1: IrMod: IrEmit "OP SUB"
        ELSE
            EXIT DO
        END IF
    LOOP
END SUB

SUB IrMod
    IrIDiv
    DO WHILE IrAccept%("MOD"): IrIDiv: IrEmit "OP MOD": LOOP
END SUB

SUB IrIDiv
    IrMul
    DO WHILE IrAccept%("\"): IrMul: IrEmit "OP IDIV": LOOP
END SUB

SUB IrMul
    DIM o$
    IrNeg
    DO
        o$ = IrPeek$
        IF o$ = "*" THEN
            IrP = IrP + 1: IrNeg: IrEmit "OP MUL"
        ELSEIF o$ = "/" THEN
            IrP = IrP + 1: IrNeg: IrEmit "OP DIV"
        ELSE
            EXIT DO
        END IF
    LOOP
END SUB

SUB IrNeg
    IF IrAccept%("-") THEN
        IrNeg
        IrEmit "UN NEG"
    ELSEIF IrAccept%("+") THEN
        IrNeg
    ELSE
        IrPow
    END IF
END SUB

SUB IrPow
    IrPrimary
    DO WHILE IrAccept%("^")
        IF IrAccept%("-") THEN
            IrPrimary: IrEmit "UN NEG"
        ELSE
            IrPrimary
        END IF
        IrEmit "OP POW"
    LOOP
END SUB

SUB IrPrimary
    DIM v$, t AS INTEGER, argc AS LONG, s AS LONG, a$
    IF IrP > IrTokN THEN IrFail "expression expected"
    t = IrTokT(IrP)
    v$ = IrTokV(IrP)
    IrP = IrP + 1
    IF t = 3 THEN
        IrEmit "PUSH S " + IrHex$(v$)
    ELSEIF t = 2 THEN
        IrEmit "PUSH " + IrNumLit$(v$)
    ELSEIF v$ = "(" THEN
        IrExpr
        IrExpect ")"
    ELSEIF t = 1 THEN
        IF v$ = "ERR" OR v$ = "ERL" THEN
            IrEmit "FN " + v$ + " 0"
        ELSEIF v$ = "RND" THEN
            IF IrAccept%("(") THEN
                IrExpr
                IrExpect ")"
                IrEmit "FN RND 1"
            ELSE
                IrEmit "FN RND 0"
            END IF
        ELSEIF v$ = "LBOUND" OR v$ = "UBOUND" THEN
            IrExpect "("
            a$ = IrArrName$(IrIdent$)
            IF IrAccept%(",") THEN IrExpr ELSE IrEmit "PUSH I 1"
            IrExpect ")"
            IF v$ = "UBOUND" THEN IrEmit "BOUND " + a$ + " U" ELSE IrEmit "BOUND " + a$ + " L"
        ELSEIF IrIsBuiltin%(v$) THEN
            IrExpect "("
            DO
                IrExpr
                argc = argc + 1
                IF IrAccept%(",") = 0 THEN EXIT DO
            LOOP
            IrExpect ")"
            IrEmit "FN " + v$ + " " + IrNum$(argc)
        ELSE
            s = IrFindSub&(v$)
            IF s THEN
                IF IrSubKind(s) <> "F" THEN IrFail "SUB " + v$ + " used as a value"
                IF IrAccept%("(") THEN argc = IrArgs&(-1)
                IF argc <> IrSubArgc(s) THEN IrFail "wrong number of arguments for " + v$
                IrEmit "CALL " + IrSubIr(s) + " " + IrNum$(argc)
            ELSEIF IrIsKeyword%(v$) THEN
                IrFail "unexpected " + v$
            ELSEIF IrAccept%("(") THEN
                a$ = IrArrName$(v$)
                argc = IrIndices&
                IrEmit "LOADE " + a$ + " " + IrNum$(argc)
            ELSE
                IrEmit "LOAD " + IrVarName$(v$)
            END IF
        END IF
    ELSE
        IrFail "unexpected '" + v$ + "'"
    END IF
END SUB

'Numeric literal -> "<type> <text>": suffix wins; integers are INTEGER/LONG
'by range; decimals are SINGLE up to 7 digits, DOUBLE beyond or with D.
'&H/&O/&B literals of up to 16 bits are INTEGER (so &HFFFF = -1), else LONG.
FUNCTION IrNumLit$ (v$)
    DIM s$, body$, digits AS LONG, i AS LONG, c$, d AS DOUBLE, radix AS LONG, lv AS LONG
    s$ = RIGHT$(v$, 1)
    body$ = v$
    IF IrCharIn%(s$, "%&!#") THEN body$ = LEFT$(v$, LEN(v$) - 1) ELSE s$ = ""
    IF LEFT$(body$, 1) = "&" THEN
        SELECT CASE MID$(body$, 2, 1)
            CASE "H": radix = 16
            CASE "O": radix = 8
            CASE ELSE: radix = 2
        END SELECT
        FOR i = 3 TO LEN(body$)
            d = d * radix + INSTR("0123456789ABCDEF", MID$(body$, i, 1)) - 1
        NEXT
        IF s$ = "" AND d <= 65535 THEN
            IF d > 32767 THEN d = d - 65536
            lv = d
            IrNumLit$ = "I " + LTRIM$(STR$(lv))
        ELSE
            IF d > 2147483647 THEN d = d - 4294967296#
            lv = d
            IrNumLit$ = "L " + LTRIM$(STR$(lv))
        END IF
        EXIT FUNCTION
    END IF
    FOR i = 1 TO LEN(body$)
        c$ = MID$(body$, i, 1)
        IF c$ = "E" OR c$ = "D" THEN EXIT FOR
        IF c$ >= "0" AND c$ <= "9" THEN digits = digits + 1
    NEXT
    IF INSTR(body$, "D") THEN
        IF s$ = "" THEN s$ = "#"
        i = INSTR(body$, "D")
        body$ = LEFT$(body$, i - 1) + "E" + MID$(body$, i + 1)
    END IF
    IF s$ = "" THEN
        IF INSTR(body$, ".") OR INSTR(body$, "E") THEN
            IF digits > 7 THEN s$ = "#" ELSE s$ = "!"
        ELSE
            d = VAL(body$)
            IF d <= 32767 THEN
                s$ = "%"
            ELSEIF d <= 2147483647 THEN
                s$ = "&"
            ELSE
                s$ = "#"
            END IF
        END IF
    END IF
    IF LEFT$(body$, 1) = "." THEN body$ = "0" + body$
    IF RIGHT$(body$, 1) = "." THEN body$ = body$ + "0"
    SELECT CASE s$
        CASE "%": IrNumLit$ = "I " + body$
        CASE "&": IrNumLit$ = "L " + body$
        CASE "!": IrNumLit$ = "F " + body$
        CASE ELSE: IrNumLit$ = "D " + body$
    END SELECT
END FUNCTION
