' R11 spike: QIR emitter, selected with `qb --ir file.bas`.
'
' Lowers a small BASIC subset (PRINT, numeric/string variables, IF, FOR,
' GOTO/GOSUB/RETURN, SUB, ON ERROR/RESUME, a few string functions) to QIR text
' and packages it with the prebuilt Rust runner internal/c/qbrun/bin/<os>/qbrun.
' No C++ is generated and no C++ compiler runs. Format and lowering rules:
' docs/rust-port/r11-emitter-design.md. The default C++ path is unchanged.

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

    'pass 1: SUB signatures (calls may precede the definition)
    IrScope = "MAIN"
    FOR i = 1 TO IrSrcN
        IrLine = i
        IrTokenize IrSrc(i)
        IrP = 1
        IF IrPeek$ = "SUB" THEN IrP = IrP + 1: IrDeclareSub
    NEXT

    'pass 2: emit QIR
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
    IF IrScope <> "MAIN" THEN IrFail "SUB without END SUB"
    CLOSE #IrOut

    'package: runner + QIR + u64 length + magic
    IF os$ = "WIN" THEN
        runner$ = "internal\c\qbrun\bin\win\qbrun.exe"
    ELSEIF MacOSX THEN
        runner$ = "internal/c/qbrun/bin/osx/qbrun"
    ELSE
        runner$ = "internal/c/qbrun/bin/lnx/qbrun"
    END IF
    IF _FILEEXISTS(runner$) = 0 THEN IrFail "runner not found: " + runner$ + " (build it with: cargo build --release -p qbrun)"

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

SUB IrAddDim (n$, sfx$, irScope$)
    IrDimN = IrDimN + 1
    IF IrDimN > UBOUND(IrDimName) THEN
        REDIM _PRESERVE IrDimName(IrDimN * 2) AS STRING
        REDIM _PRESERVE IrDimSfx(IrDimN * 2) AS STRING
        REDIM _PRESERVE IrDimScope(IrDimN * 2) AS STRING
    END IF
    IrDimName(IrDimN) = n$: IrDimSfx(IrDimN) = sfx$: IrDimScope(IrDimN) = irScope$
END SUB

'Variable name as the runner sees it: always carries its type suffix.
FUNCTION IrVarName$ (id$)
    DIM i AS LONG
    IF INSTR("%&!#$", RIGHT$(id$, 1)) THEN IrVarName$ = id$: EXIT FUNCTION
    FOR i = IrDimN TO 1 STEP -1
        IF IrDimName(i) = id$ AND IrDimScope(i) = IrScope THEN IrVarName$ = id$ + IrDimSfx(i): EXIT FUNCTION
    NEXT
    FOR i = IrDimN TO 1 STEP -1
        IF IrDimName(i) = id$ AND IrDimScope(i) = "*" THEN IrVarName$ = id$ + IrDimSfx(i): EXIT FUNCTION
    NEXT
    IrVarName$ = id$ + "!"
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

FUNCTION IrFindSub& (n$)
    DIM i AS LONG
    FOR i = 1 TO IrSubN
        IF IrSubName(i) = n$ THEN IrFindSub& = i: EXIT FUNCTION
    NEXT
END FUNCTION

FUNCTION IrIsKeyword% (k$)
    SELECT CASE k$
        CASE "PRINT", "IF", "THEN", "ELSE", "ELSEIF", "END", "FOR", "TO", "STEP", "NEXT", "GOTO", "GOSUB", "RETURN", "SUB", "CALL", "DIM", "SHARED", "AS", "ON", "ERROR", "RESUME", "SYSTEM", "LET", "EXIT", "DECLARE", "AND", "OR", "XOR", "NOT", "MOD"
            IrIsKeyword% = -1
    END SELECT
END FUNCTION

'pass 1: SUB name [(p [AS type], ...)] [STATIC]
SUB IrDeclareSub
    DIM n$, p$, plist$, argc AS LONG
    n$ = IrIdent$
    IF IrFindSub&(n$) THEN IrFail "duplicate SUB " + n$
    IF IrAccept%("(") THEN
        IF IrAccept%(")") = 0 THEN
            DO
                p$ = IrIdent$
                IF IrAccept%("AS") THEN
                    IrAddDim p$, IrTypeSfx$(IrIdent$), n$
                    p$ = p$ + IrDimSfx(IrDimN)
                ELSEIF INSTR("%&!#$", RIGHT$(p$, 1)) = 0 THEN
                    p$ = p$ + "!"
                END IF
                plist$ = plist$ + " " + p$
                argc = argc + 1
                IF IrAccept%(")") THEN EXIT DO
                IrExpect ","
            LOOP
        END IF
    END IF
    IrSubN = IrSubN + 1
    IF IrSubN > UBOUND(IrSubName) THEN
        REDIM _PRESERVE IrSubName(IrSubN * 2) AS STRING
        REDIM _PRESERVE IrSubParams(IrSubN * 2) AS STRING
        REDIM _PRESERVE IrSubArgc(IrSubN * 2) AS LONG
    END IF
    IrSubName(IrSubN) = n$: IrSubParams(IrSubN) = plist$: IrSubArgc(IrSubN) = argc
END SUB

'---------------------------------------------------------------- statements

'Line number or "name:" at the start of a line.
SUB IrLineLabels
    IF IrTokN = 0 THEN EXIT SUB
    IF IrTokT(1) = 2 THEN
        IrEmit "LABEL " + IrLabel$("L" + IrTokV(1))
        IrP = 2
    ELSEIF IrTokT(1) = 1 AND IrTokN >= 2 THEN
        IF IrTokV(2) = ":" AND IrIsKeyword%(IrTokV(1)) = 0 THEN
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
    DIM k$, n$, id AS LONG, s AS LONG
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
            IF IrBlkN = 0 THEN IrFail "ELSEIF without IF"
            IF IrBlkKind(IrBlkN) <> "IF" THEN IrFail "ELSEIF without IF"
            id = IrBlkId(IrBlkN)
            IrEmit "JMP ~IF" + IrNum$(id) + ".END"
            IrEmit "LABEL ~IF" + IrNum$(id) + "." + IrNum$(IrBlkPart(IrBlkN))
            IrBlkPart(IrBlkN) = IrBlkPart(IrBlkN) + 1
            IrExpr
            IrExpect "THEN"
            IrEmit "JZ ~IF" + IrNum$(id) + "." + IrNum$(IrBlkPart(IrBlkN))
        CASE "ELSE"
            IF IrBlkN = 0 THEN IrFail "ELSE without IF"
            IF IrBlkKind(IrBlkN) <> "IF" THEN IrFail "ELSE without IF"
            id = IrBlkId(IrBlkN)
            IrEmit "JMP ~IF" + IrNum$(id) + ".END"
            IrEmit "LABEL ~IF" + IrNum$(id) + "." + IrNum$(IrBlkPart(IrBlkN))
            IrBlkPart(IrBlkN) = IrBlkPart(IrBlkN) + 1
            IF IrAtEnd% = 0 THEN IrStatementList 0, 0
        CASE "END"
            n$ = IrPeek$
            IF n$ = "IF" THEN
                IrP = IrP + 1
                IF IrBlkN = 0 THEN IrFail "END IF without IF"
                IF IrBlkKind(IrBlkN) <> "IF" THEN IrFail "END IF without IF"
                id = IrBlkId(IrBlkN)
                IrEmit "LABEL ~IF" + IrNum$(id) + "." + IrNum$(IrBlkPart(IrBlkN))
                IrEmit "LABEL ~IF" + IrNum$(id) + ".END"
                IrBlkN = IrBlkN - 1
            ELSEIF n$ = "SUB" THEN
                IrP = IrP + 1
                IF IrScope = "MAIN" THEN IrFail "END SUB without SUB"
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
        CASE "GOTO"
            IrEmit "JMP " + IrTarget$(IrScope)
        CASE "GOSUB"
            IrEmit "GOSUB " + IrTarget$(IrScope)
        CASE "RETURN"
            IrEmit "RETURN"
        CASE "SUB"
            IF top = 0 OR IrScope <> "MAIN" THEN IrFail "SUB not allowed here"
            n$ = IrIdent$
            s = IrFindSub&(n$)
            IrEmit RTRIM$("SUB " + n$ + " " + IrNum$(IrSubArgc(s)) + IrSubParams(s))
            IrScope = n$
            IrP = IrTokN + 1 'parameters were read in pass 1
        CASE "EXIT"
            IrExpect "SUB"
            IF IrScope = "MAIN" THEN IrFail "EXIT SUB outside SUB"
            IrEmit "JMP " + IrLabel$("~EXIT")
        CASE "DECLARE"
            IrP = IrTokN + 1
        CASE "DIM"
            IrDim
        CASE "ON"
            IrExpect "ERROR"
            IrExpect "GOTO"
            IF IrPeek$ = "0" THEN
                IrEmit "ONERROR 0": IrP = IrP + 1
            ELSE
                IrEmit "ONERROR " + IrTarget$("MAIN")
            END IF
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
        CASE ELSE
            IF IrPeek$ = "=" THEN
                IrAssign k$
            ELSEIF IrFindSub&(k$) THEN
                IrCall k$, 0
            ELSE
                IrFail "statement not supported by the IR spike: " + k$
            END IF
    END SELECT
END SUB

SUB IrAssign (id$)
    IF IrPeek$ = "(" THEN IrFail "arrays are not supported by the IR spike: " + id$
    IrExpect "="
    IrExpr
    IrEmit "STORE " + IrVarName$(id$)
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

SUB IrIf (top AS INTEGER)
    DIM id AS LONG, t$
    id = IrNewId&
    t$ = "~IF" + IrNum$(id)
    IrExpr
    IrExpect "THEN"
    IF IrPeek$ = "" THEN
        IF top = 0 THEN IrFail "block IF inside a single-line IF"
        IrBlkN = IrBlkN + 1
        IF IrBlkN > UBOUND(IrBlkKind) THEN
            REDIM _PRESERVE IrBlkKind(IrBlkN * 2) AS STRING
            REDIM _PRESERVE IrBlkId(IrBlkN * 2) AS LONG
            REDIM _PRESERVE IrBlkPart(IrBlkN * 2) AS LONG
            REDIM _PRESERVE IrBlkVar(IrBlkN * 2) AS STRING
        END IF
        IrBlkKind(IrBlkN) = "IF": IrBlkId(IrBlkN) = id: IrBlkPart(IrBlkN) = 1
        IrEmit "JZ " + t$ + ".1"
        EXIT SUB
    END IF
    IrEmit "JZ " + t$ + ".ELSE"
    IrStatementList 0, -1
    IF IrAccept%("ELSE") THEN
        IrEmit "JMP " + t$ + ".END"
        IrEmit "LABEL " + t$ + ".ELSE"
        IrStatementList 0, 0
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
    IrBlkN = IrBlkN + 1
    IF IrBlkN > UBOUND(IrBlkKind) THEN
        REDIM _PRESERVE IrBlkKind(IrBlkN * 2) AS STRING
        REDIM _PRESERVE IrBlkId(IrBlkN * 2) AS LONG
        REDIM _PRESERVE IrBlkPart(IrBlkN * 2) AS LONG
        REDIM _PRESERVE IrBlkVar(IrBlkN * 2) AS STRING
    END IF
    IrBlkKind(IrBlkN) = "FOR": IrBlkId(IrBlkN) = id: IrBlkVar(IrBlkN) = v$
END SUB

SUB IrNext
    DIM t$, v$
    IF IrBlkN = 0 THEN IrFail "NEXT without FOR"
    IF IrBlkKind(IrBlkN) <> "FOR" THEN IrFail "NEXT without FOR"
    v$ = IrBlkVar(IrBlkN)
    IF IrAtEnd% = 0 THEN
        IF IrVarName$(IrIdent$) <> v$ THEN IrFail "NEXT variable does not match FOR"
    END IF
    t$ = "~F" + IrNum$(IrBlkId(IrBlkN))
    IrEmit "LOAD " + v$
    IrEmit "LOAD " + t$ + ".S"
    IrEmit "OP ADD"
    IrEmit "STORE " + v$
    IrEmit "JMP " + t$ + ".T"
    IrEmit "LABEL " + t$ + ".X"
    IrBlkN = IrBlkN - 1
END SUB

SUB IrDim
    DIM isShared AS INTEGER, n$, irScope$, nm$
    IF IrAccept%("SHARED") THEN isShared = -1
    irScope$ = IrScope
    IF isShared THEN irScope$ = "*"
    DO
        n$ = IrIdent$
        IF IrPeek$ = "(" THEN IrFail "arrays are not supported by the IR spike: " + n$
        IF IrAccept%("AS") THEN
            IrAddDim n$, IrTypeSfx$(IrIdent$), irScope$
            nm$ = n$ + IrDimSfx(IrDimN)
        ELSEIF INSTR("%&!#$", RIGHT$(n$, 1)) THEN
            nm$ = n$
        ELSE
            IrAddDim n$, "!", irScope$
            nm$ = n$ + "!"
        END IF
        IF isShared THEN IrEmit "SHARED " + nm$
        IF IrAccept%(",") = 0 THEN EXIT DO
    LOOP
END SUB

'Arguments that are a bare variable are passed by reference (QBasic rule).
SUB IrCall (n$, parens AS INTEGER)
    DIM s AS LONG, argc AS LONG, closer$
    s = IrFindSub&(n$)
    IF s = 0 THEN IrFail "SUB not found: " + n$
    IF parens THEN
        IF IrAccept%("(") = 0 THEN parens = 0
    END IF
    IF parens THEN closer$ = ")" ELSE closer$ = ""
    DO
        IF parens THEN
            IF IrAccept%(")") THEN EXIT DO
        ELSE
            IF IrAtEnd% OR IrPeek$ = "ELSE" THEN EXIT DO
        END IF
        IF IrTokT(IrP) = 1 AND IrP < IrTokN THEN
            IF IrTokV(IrP + 1) = "," OR IrTokV(IrP + 1) = ")" OR IrTokV(IrP + 1) = ":" THEN
                IrEmit "REF " + IrVarName$(IrTokV(IrP)): IrP = IrP + 1
            ELSE
                IrExpr
            END IF
        ELSEIF IrTokT(IrP) = 1 AND IrP = IrTokN THEN
            IrEmit "REF " + IrVarName$(IrTokV(IrP)): IrP = IrP + 1
        ELSE
            IrExpr
        END IF
        argc = argc + 1
        IF IrAccept%(",") = 0 THEN
            IF parens THEN IrExpect ")"
            EXIT DO
        END IF
    LOOP
    IF argc <> IrSubArgc(s) THEN IrFail "wrong number of arguments for " + n$
    IrEmit "CALL " + n$ + " " + IrNum$(argc)
END SUB

'---------------------------------------------------------------- expressions
'Precedence, lowest first: XOR, OR, AND, NOT, relational, + -, MOD, \, * /,
'unary -, ^. Each level emits its operands first (postfix order).

SUB IrExpr
    IrXor
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
    DIM v$, t AS INTEGER, argc AS LONG
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
        SELECT CASE v$
            CASE "ERR", "ERL"
                IrEmit "FN " + v$ + " 0"
            CASE "LEN", "LEFT$", "RIGHT$", "MID$", "UCASE$", "LCASE$", "STR$", "CHR$", "ASC"
                IrExpect "("
                DO
                    IrExpr
                    argc = argc + 1
                    IF IrAccept%(",") = 0 THEN EXIT DO
                LOOP
                IrExpect ")"
                IrEmit "FN " + v$ + " " + IrNum$(argc)
            CASE ELSE
                IF IrIsKeyword%(v$) THEN IrFail "unexpected " + v$
                IF IrPeek$ = "(" THEN IrFail "arrays/functions are not supported by the IR spike: " + v$
                IrEmit "LOAD " + IrVarName$(v$)
        END SELECT
    ELSE
        IrFail "unexpected '" + v$ + "'"
    END IF
END SUB

'Numeric literal -> "<type> <text>": suffix wins; integers are INTEGER/LONG
'by range; decimals are SINGLE up to 7 digits, DOUBLE beyond or with D.
FUNCTION IrNumLit$ (v$)
    DIM s$, body$, digits AS LONG, i AS LONG, c$, d AS DOUBLE
    s$ = RIGHT$(v$, 1)
    body$ = v$
    IF INSTR("%&!#", s$) THEN body$ = LEFT$(v$, LEN(v$) - 1) ELSE s$ = ""
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
