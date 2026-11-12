'DEF FN, both QBasic forms:
'
'    DEF FnName[(parameters)] = expression
'
'    DEF FnName[(parameters)]
'        statements             (FnName = value, EXIT DEF, STATIC variables)
'    END DEF
'
'The main source is rewritten once, when it is loaded (lineinput3load): the DEF FN lines
'become spaces (so offsets and line numbers in diagnostics stay right) and an equivalent
'FUNCTION is appended after the program through imports_body.bas
'(StdLib_PrepareHoistedImports):
'
'    FUNCTION FnName (parameters)
'    SHARED a, b, c()          <- module-level variables and arrays the definition uses
'    statements / FnName = expression
'    END FUNCTION
'
'Like DEF FN, the function can be called anywhere in the module and sees the module's
'variables; variables declared STATIC in a multi-line definition stay local.

FUNCTION DefFn_IsIdStart% (c AS STRING)
    DIM a AS LONG
    a = ASC(UCASE$(c))
    DefFn_IsIdStart% = (a >= 65 AND a <= 90)
END FUNCTION

FUNCTION DefFn_IsIdChar% (c AS STRING)
    DIM a AS LONG
    a = ASC(UCASE$(c))
    DefFn_IsIdChar% = (a >= 65 AND a <= 90) OR (a >= 48 AND a <= 57) OR c = "_" OR c = "."
END FUNCTION

FUNCTION DefFn_IsSuffix% (c AS STRING)
    DefFn_IsSuffix% = INSTR("$%&!#~`", c) > 0
END FUNCTION

'Removes a trailing ' comment that is outside string literals.
FUNCTION DefFn_StripComment$ (t AS STRING)
    DIM i AS LONG, inQuote AS INTEGER, c AS STRING
    FOR i = 1 TO LEN(t)
        c = MID$(t, i, 1)
        IF c = CHR$(34) THEN inQuote = NOT inQuote
        IF c = "'" AND inQuote = 0 THEN DefFn_StripComment$ = RTRIM$(LEFT$(t, i - 1)): EXIT FUNCTION
    NEXT
    DefFn_StripComment$ = RTRIM$(t)
END FUNCTION

'Leading spaces and tabs removed.
FUNCTION DefFn_Trim$ (t AS STRING)
    DIM s AS STRING
    s = LTRIM$(t)
    DO WHILE LEFT$(s, 1) = CHR$(9)
        s = LTRIM$(MID$(s, 2))
    LOOP
    DefFn_Trim$ = s
END FUNCTION

'Splits a DEF FN line. Returns 1 for the single-line form (expr set), 2 for the head of
'the multi-line form, 0 when lineText is not a DEF FN.
FUNCTION DefFn_Parse% (lineText AS STRING, fnName AS STRING, params AS STRING, expr AS STRING)
    DIM t AS STRING, i AS LONG, depth AS LONG, c AS STRING
    DefFn_Parse% = 0
    t = DefFn_Trim$(lineText)
    IF UCASE$(LEFT$(t, 3)) <> "DEF" THEN EXIT FUNCTION
    c = MID$(t, 4, 1)
    IF c <> " " AND c <> CHR$(9) THEN EXIT FUNCTION
    t = DefFn_Trim$(MID$(t, 4))
    IF UCASE$(LEFT$(t, 2)) <> "FN" THEN EXIT FUNCTION
    i = 3
    DO WHILE i <= LEN(t)
        IF DefFn_IsIdChar%(MID$(t, i, 1)) = 0 THEN EXIT DO
        i = i + 1
    LOOP
    DO WHILE i <= LEN(t)
        IF DefFn_IsSuffix%(MID$(t, i, 1)) = 0 THEN EXIT DO
        i = i + 1
    LOOP
    fnName = LEFT$(t, i - 1)
    t = LTRIM$(MID$(t, i))
    params = ""
    IF LEFT$(t, 1) = "(" THEN
        depth = 0
        FOR i = 1 TO LEN(t)
            c = MID$(t, i, 1)
            IF c = "(" THEN depth = depth + 1
            IF c = ")" THEN
                depth = depth - 1
                IF depth = 0 THEN EXIT FOR
            END IF
        NEXT
        IF depth <> 0 THEN EXIT FUNCTION
        params = MID$(t, 2, i - 2)
        t = LTRIM$(MID$(t, i + 1))
    END IF
    t = DefFn_StripComment$(t)
    IF LEN(t) = 0 THEN DefFn_Parse% = 2: EXIT FUNCTION
    IF LEFT$(t, 1) <> "=" THEN EXIT FUNCTION
    expr = LTRIM$(MID$(t, 2))
    IF LEN(expr) = 0 THEN EXIT FUNCTION
    DefFn_Parse% = 1
END FUNCTION

'"|NAME1|NAME2|" of a parameter list such as "x, y AS INTEGER, s$".
FUNCTION DefFn_ParamNames$ (params AS STRING)
    DIM r AS STRING, p AS STRING, i AS LONG, j AS LONG
    r = "|"
    p = params + ","
    DO WHILE LEN(p)
        i = INSTR(p, ",")
        j = INSTR(LTRIM$(LEFT$(p, i - 1)), " ")
        IF j THEN r = r + UCASE$(LEFT$(LTRIM$(LEFT$(p, i - 1)), j - 1)) + "|" ELSE r = r + UCASE$(LTRIM$(RTRIM$(LEFT$(p, i - 1)))) + "|"
        p = MID$(p, i + 1)
    LOOP
    DefFn_ParamNames$ = r
END FUNCTION

'Names declared in the source that are never SHARED: CONSTs (global), the program's
'SUB/FUNCTION names and every DEF FN name, as "|NAME|...".
FUNCTION DefFn_KnownNames$ (text AS STRING)
    DIM r AS STRING, p AS LONG, nl AS LONG, t AS STRING, part AS STRING, i AS LONG, j AS LONG
    DIM nm AS STRING, d AS STRING, e AS STRING
    r = "|"
    p = 1
    DO WHILE p <= LEN(text)
        nl = INSTR(p, text, CHR$(10))
        IF nl = 0 THEN nl = LEN(text) + 1
        t = UCASE$(DefFn_Trim$(MID$(text, p, nl - p)))
        p = nl + 1
        IF LEFT$(t, 8) = "DECLARE " THEN t = DefFn_Trim$(MID$(t, 9))
        IF LEFT$(t, 6) = "CONST " THEN
            t = DefFn_StripComment$(MID$(t, 7)) + ","
            DO WHILE LEN(t)
                i = INSTR(t, ",")
                part = LEFT$(t, i - 1)
                j = INSTR(part, "=")
                IF j THEN r = r + LTRIM$(RTRIM$(LEFT$(part, j - 1))) + "|"
                t = MID$(t, i + 1)
            LOOP
        ELSEIF LEFT$(t, 4) = "SUB " OR LEFT$(t, 9) = "FUNCTION " THEN
            t = DefFn_Trim$(MID$(t, INSTR(t, " ") + 1))
            i = 1
            DO WHILE i <= LEN(t)
                IF DefFn_IsIdChar%(MID$(t, i, 1)) = 0 AND DefFn_IsSuffix%(MID$(t, i, 1)) = 0 THEN EXIT DO
                i = i + 1
            LOOP
            nm = LEFT$(t, i - 1)
            IF LEN(nm) THEN r = r + nm + "|"
        ELSEIF DefFn_Parse%(t, nm, d, e) THEN
            r = r + nm + "|"
        END IF
    LOOP
    DefFn_KnownNames$ = r
END FUNCTION

'Comma-separated module variables and arrays ("name()") that code reads or writes. Skipped:
'parameters, the names in notHere, keywords and built-in SUB/FUNCTIONs, labels (after
'GOTO/GOSUB/RESUME/RESTORE/RUN, or "name:" starting a line), DATA items and names
'starting with "_".
FUNCTION DefFn_SharedList$ (code AS STRING, fnName AS STRING, paramNames AS STRING, notHere AS STRING)
    DIM i AS LONG, j AS LONG, c AS STRING, nm AS STRING, u AS STRING, k AS LONG
    DIM seen AS STRING, r AS STRING, notShared AS STRING, prevWord AS STRING
    DIM lineStart AS LONG, isArray AS LONG, rf AS LONG, rr AS LONG, baseName AS STRING
    notShared = "|AND|OR|XOR|NOT|MOD|EQV|IMP|TIMER|RND|INKEY$|DATE$|TIME$|CSRLIN|FREEFILE|ERR|ERL|COMMAND$|"
    notShared = notShared + "IF|THEN|ELSE|ELSEIF|END|FOR|TO|STEP|NEXT|DO|LOOP|WHILE|WEND|UNTIL|GOTO|GOSUB|RETURN|"
    notShared = notShared + "EXIT|FUNCTION|DEF|SELECT|CASE|IS|LET|DIM|REDIM|AS|STATIC|SHARED|CONST|INTEGER|LONG|"
    notShared = notShared + "SINGLE|DOUBLE|STRING|PRINT|USING|INPUT|LINE|ON|ERROR|RESUME|CALL|STOP|SYSTEM|SWAP|"
    seen = "|"
    prevWord = ""
    lineStart = -1
    i = 1
    DO WHILE i <= LEN(code)
        c = MID$(code, i, 1)
        IF c = CHR$(10) THEN
            lineStart = -1: prevWord = "": i = i + 1
        ELSEIF c = CHR$(34) THEN
            j = INSTR(i + 1, code, CHR$(34))
            IF j = 0 THEN EXIT DO
            i = j + 1: lineStart = 0
        ELSEIF c = "&" THEN
            'a &H/&O/&B literal: skip its digits
            i = i + 2
            DO WHILE i <= LEN(code)
                IF DefFn_IsIdChar%(MID$(code, i, 1)) = 0 THEN EXIT DO
                i = i + 1
            LOOP
            lineStart = 0
        ELSEIF (ASC(c) >= 48 AND ASC(c) <= 57) OR c = "." THEN
            'a number (or a line number), with its exponent (1E-3, 2D+4) and type suffix
            DO WHILE i <= LEN(code)
                c = UCASE$(MID$(code, i, 1))
                IF (c = "E" OR c = "D") AND INSTR("+-", MID$(code, i + 1, 1)) > 0 THEN
                    i = i + 2
                ELSEIF DefFn_IsIdChar%(c) OR DefFn_IsSuffix%(c) THEN
                    i = i + 1
                ELSE
                    EXIT DO
                END IF
            LOOP
        ELSEIF DefFn_IsIdStart%(c) OR c = "_" THEN
            j = i
            DO WHILE j <= LEN(code)
                IF DefFn_IsIdChar%(MID$(code, j, 1)) = 0 THEN EXIT DO
                j = j + 1
            LOOP
            DO WHILE j <= LEN(code)
                IF DefFn_IsSuffix%(MID$(code, j, 1)) = 0 THEN EXIT DO
                j = j + 1
            LOOP
            nm = MID$(code, i, j - i)
            i = j
            k = i
            DO WHILE MID$(code, k, 1) = " "
                k = k + 1
            LOOP
            u = UCASE$(nm)
            isArray = (MID$(code, k, 1) = "(")
            IF u = "DATA" OR u = "REM" THEN
                'the rest of the line is not code
                DO WHILE i <= LEN(code)
                    IF MID$(code, i, 1) = CHR$(10) THEN EXIT DO
                    i = i + 1
                LOOP
            ELSEIF lineStart <> 0 AND MID$(code, k, 1) = ":" AND isArray = 0 THEN
                'a label definition
            ELSEIF INSTR("|GOTO|GOSUB|RESUME|RESTORE|RUN|", "|" + prevWord + "|") THEN
                'a label reference
            ELSEIF LEFT$(u, 1) <> "_" AND u <> UCASE$(fnName) AND INSTR(paramNames + notHere + notShared + seen, "|" + u + "|") = 0 THEN
                'built-in names are hashed without their type suffix (LEFT$ is "LEFT")
                baseName = u
                IF isArray THEN
                    DO WHILE LEN(baseName) > 1 AND DefFn_IsSuffix%(RIGHT$(baseName, 1))
                        baseName = LEFT$(baseName, LEN(baseName) - 1)
                    LOOP
                END IF
                IF HashFind(u, HASHFLAG_RESERVED + HASHFLAG_OPERATOR + HASHFLAG_TYPE + HASHFLAG_SUB + HASHFLAG_FUNCTION + HASHFLAG_CUSTOMSYNTAX, rf, rr) = 0 AND HashFind(baseName, HASHFLAG_RESERVED + HASHFLAG_SUB + HASHFLAG_FUNCTION, rf, rr) = 0 THEN
                    seen = seen + u + "|"
                    IF LEN(r) THEN r = r + ", "
                    IF isArray THEN r = r + nm + "()" ELSE r = r + nm
                END IF
            END IF
            prevWord = u
            lineStart = 0
        ELSE
            IF c = ":" THEN prevWord = ""
            IF c <> " " AND c <> CHR$(9) THEN lineStart = 0
            i = i + 1
        END IF
    LOOP
    DefFn_SharedList$ = r
END FUNCTION

'text with every "EXIT DEF" outside string literals replaced by "EXIT FUNCTION".
FUNCTION DefFn_ExitDef$ (text AS STRING)
    DIM t AS STRING, u AS STRING, i AS LONG, inQuote AS INTEGER, c AS STRING
    t = text
    i = 1
    DO WHILE i <= LEN(t)
        c = MID$(t, i, 1)
        IF c = CHR$(34) THEN inQuote = NOT inQuote
        IF inQuote = 0 AND UCASE$(c) = "E" THEN
            u = UCASE$(MID$(t, i, 8))
            IF u = "EXIT DEF" AND DefFn_IsIdChar%(MID$(t, i + 8, 1) + " ") = 0 THEN
                IF i = 1 OR DefFn_IsIdChar%(MID$(t, i - 1, 1) + " ") = 0 THEN
                    t = LEFT$(t, i - 1) + "EXIT FUNCTION" + MID$(t, i + 8)
                    i = i + 12
                END IF
            END IF
        END IF
        i = i + 1
    LOOP
    DefFn_ExitDef$ = t
END FUNCTION

'"|NAME|..." of the variables a STATIC statement in code declares.
FUNCTION DefFn_StaticNames$ (code AS STRING)
    DIM r AS STRING, p AS LONG, nl AS LONG, t AS STRING, part AS STRING, i AS LONG, j AS LONG
    r = "|"
    p = 1
    DO WHILE p <= LEN(code)
        nl = INSTR(p, code, CHR$(10))
        IF nl = 0 THEN nl = LEN(code) + 1
        t = UCASE$(DefFn_Trim$(MID$(code, p, nl - p)))
        p = nl + 1
        IF LEFT$(t, 7) = "STATIC " THEN
            t = MID$(t, 8) + ","
            DO WHILE LEN(t)
                i = INSTR(t, ",")
                part = DefFn_Trim$(LEFT$(t, i - 1))
                j = 1
                DO WHILE j <= LEN(part)
                    IF DefFn_IsIdChar%(MID$(part, j, 1)) = 0 AND DefFn_IsSuffix%(MID$(part, j, 1)) = 0 THEN EXIT DO
                    j = j + 1
                LOOP
                IF j > 1 THEN r = r + LEFT$(part, j - 1) + "|"
                t = MID$(t, i + 1)
            LOOP
        END IF
    LOOP
    DefFn_StaticNames$ = r
END FUNCTION

'Rewrites lineinput3buffer$ in place and fills defFnBodyText$ (see the top of this file).
'A converted line is overwritten with spaces, so offsets and line numbers stay.
SUB DefFn_RewriteBuffer
    DIM lineText AS STRING, fnName AS STRING, params AS STRING, expr AS STRING
    DIM sharedVars AS STRING, knownNames AS STRING, crlf AS STRING, head AS STRING
    DIM body AS STRING, code AS STRING, t AS STRING, kind AS LONG, inDef AS LONG
    DIM p AS LONG, e AS LONG, c13 AS LONG, c10 AS LONG, n AS LONG

    defFnBodyText$ = ""
    n = LEN(lineinput3buffer$)
    IF INSTR(UCASE$(lineinput3buffer$), "FN") = 0 THEN EXIT SUB
    crlf = CHR$(13) + CHR$(10)
    p = 1
    DO WHILE p <= n
        c13 = INSTR(p, lineinput3buffer$, CHR$(13))
        c10 = INSTR(p, lineinput3buffer$, CHR$(10))
        IF c13 = 0 THEN c13 = n + 1
        IF c10 = 0 THEN c10 = n + 1
        IF c13 < c10 THEN e = c13 ELSE e = c10
        lineText = MID$(lineinput3buffer$, p, e - p)
        IF inDef THEN
            t = UCASE$(DefFn_StripComment$(DefFn_Trim$(lineText)))
            IF t = "END DEF" THEN
                inDef = 0
                sharedVars = DefFn_SharedList$(code, fnName, DefFn_ParamNames$(params), knownNames + MID$(DefFn_StaticNames$(code), 2))
                defFnBodyText$ = defFnBodyText$ + head
                IF LEN(sharedVars) THEN defFnBodyText$ = defFnBodyText$ + "SHARED " + sharedVars + crlf
                defFnBodyText$ = defFnBodyText$ + body + "END FUNCTION" + crlf
            ELSEIF LEN(lineText) THEN
                body = body + DefFn_ExitDef$(lineText) + crlf
                IF UCASE$(LEFT$(DefFn_Trim$(lineText), 4)) <> "REM " THEN code = code + DefFn_StripComment$(lineText) + CHR$(10)
            END IF
            MID$(lineinput3buffer$, p, e - p) = SPACE$(e - p)
        ELSE
            kind = DefFn_Parse%(lineText, fnName, params, expr)
            IF kind THEN
                IF LEN(knownNames) = 0 THEN knownNames = DefFn_KnownNames$(lineinput3buffer$)
                IF LEN(params) THEN head = "FUNCTION " + fnName + " (" + params + ")" + crlf ELSE head = "FUNCTION " + fnName + crlf
                IF kind = 1 THEN
                    sharedVars = DefFn_SharedList$(expr, fnName, DefFn_ParamNames$(params), knownNames)
                    defFnBodyText$ = defFnBodyText$ + head
                    IF LEN(sharedVars) THEN defFnBodyText$ = defFnBodyText$ + "SHARED " + sharedVars + crlf
                    defFnBodyText$ = defFnBodyText$ + fnName + " = " + expr + crlf + "END FUNCTION" + crlf
                ELSE
                    inDef = -1: body = "": code = ""
                END IF
                MID$(lineinput3buffer$, p, e - p) = SPACE$(e - p)
            END IF
        END IF
        p = e + 1
    LOOP
END SUB
