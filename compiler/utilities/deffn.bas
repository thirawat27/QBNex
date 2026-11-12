'DEF FN, the QBasic single-line form:  DEF FnName[(parameters)] = expression
'
'The main source is rewritten once, when it is loaded (lineinput3load): each such line
'becomes empty (so line numbers in diagnostics stay right) and an equivalent FUNCTION is
'appended after the program through imports_body.bas (StdLib_PrepareHoistedImports):
'
'    FUNCTION FnName (parameters)
'    SHARED a, b              <- module-level variables the expression reads
'    FnName = expression
'    END FUNCTION
'
'Like DEF FN, the function can be called anywhere in the module and sees the module's
'variables. The multi-line form (DEF FnName ... END DEF) is left as it is.

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

'Splits a single-line DEF FN; returns 0 when lineText is not one.
FUNCTION DefFn_Parse% (lineText AS STRING, fnName AS STRING, params AS STRING, expr AS STRING)
    DIM t AS STRING, i AS LONG, depth AS LONG, c AS STRING
    DefFn_Parse% = 0
    t = LTRIM$(lineText)
    DO WHILE LEFT$(t, 1) = CHR$(9)
        t = LTRIM$(MID$(t, 2))
    LOOP
    IF UCASE$(LEFT$(t, 3)) <> "DEF" THEN EXIT FUNCTION
    c = MID$(t, 4, 1)
    IF c <> " " AND c <> CHR$(9) THEN EXIT FUNCTION
    t = LTRIM$(MID$(t, 4))
    DO WHILE LEFT$(t, 1) = CHR$(9)
        t = LTRIM$(MID$(t, 2))
    LOOP
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
    IF LEFT$(t, 1) <> "=" THEN EXIT FUNCTION 'multi-line form
    expr = DefFn_StripComment$(LTRIM$(MID$(t, 2)))
    IF LEN(expr) = 0 THEN EXIT FUNCTION
    DefFn_Parse% = -1
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

'"|NAME|..." of the CONST names in the source (they are global; never SHARED).
FUNCTION DefFn_ConstNames$ (text AS STRING)
    DIM r AS STRING, p AS LONG, nl AS LONG, t AS STRING, part AS STRING, i AS LONG, j AS LONG
    r = "|"
    p = 1
    DO WHILE p <= LEN(text)
        nl = INSTR(p, text, CHR$(10))
        IF nl = 0 THEN nl = LEN(text) + 1
        t = UCASE$(LTRIM$(MID$(text, p, nl - p)))
        p = nl + 1
        IF LEFT$(t, 6) = "CONST " THEN
            t = DefFn_StripComment$(MID$(t, 7)) + ","
            DO WHILE LEN(t)
                i = INSTR(t, ",")
                part = LEFT$(t, i - 1)
                j = INSTR(part, "=")
                IF j THEN r = r + LTRIM$(RTRIM$(LEFT$(part, j - 1))) + "|"
                t = MID$(t, i + 1)
            LOOP
        END IF
    LOOP
    DefFn_ConstNames$ = r
END FUNCTION

'Comma-separated module variables that expr reads: plain names, not followed by "(",
'not parameters, constants, operators or built-in functions without arguments.
FUNCTION DefFn_SharedList$ (expr AS STRING, fnName AS STRING, paramNames AS STRING, constNames AS STRING)
    DIM i AS LONG, j AS LONG, c AS STRING, nm AS STRING, u AS STRING, k AS LONG
    DIM seen AS STRING, r AS STRING, notShared AS STRING
    notShared = "|AND|OR|XOR|NOT|MOD|EQV|IMP|TIMER|RND|INKEY$|DATE$|TIME$|CSRLIN|FREEFILE|ERR|ERL|COMMAND$|"
    seen = "|"
    i = 1
    DO WHILE i <= LEN(expr)
        c = MID$(expr, i, 1)
        IF c = CHR$(34) THEN
            j = INSTR(i + 1, expr, CHR$(34))
            IF j = 0 THEN EXIT DO
            i = j + 1
        ELSEIF c = "&" THEN
            'a &H/&O/&B literal: skip its digits
            i = i + 2
            DO WHILE i <= LEN(expr)
                IF DefFn_IsIdChar%(MID$(expr, i, 1)) = 0 THEN EXIT DO
                i = i + 1
            LOOP
        ELSEIF (ASC(c) >= 48 AND ASC(c) <= 57) OR c = "." THEN
            'a number, with its exponent (1E-3, 2D+4) and type suffix
            DO WHILE i <= LEN(expr)
                c = UCASE$(MID$(expr, i, 1))
                IF (c = "E" OR c = "D") AND INSTR("+-", MID$(expr, i + 1, 1)) > 0 THEN
                    i = i + 2
                ELSEIF DefFn_IsIdChar%(c) OR DefFn_IsSuffix%(c) THEN
                    i = i + 1
                ELSE
                    EXIT DO
                END IF
            LOOP
        ELSEIF DefFn_IsIdStart%(c) OR c = "_" THEN
            j = i
            DO WHILE j <= LEN(expr)
                IF DefFn_IsIdChar%(MID$(expr, j, 1)) = 0 THEN EXIT DO
                j = j + 1
            LOOP
            DO WHILE j <= LEN(expr)
                IF DefFn_IsSuffix%(MID$(expr, j, 1)) = 0 THEN EXIT DO
                j = j + 1
            LOOP
            nm = MID$(expr, i, j - i)
            i = j
            k = i
            DO WHILE MID$(expr, k, 1) = " "
                k = k + 1
            LOOP
            u = UCASE$(nm)
            IF MID$(expr, k, 1) <> "(" AND LEFT$(u, 1) <> "_" AND u <> UCASE$(fnName) THEN
                IF INSTR(paramNames + constNames + notShared + seen, "|" + u + "|") = 0 THEN
                    seen = seen + u + "|"
                    IF LEN(r) THEN r = r + ", "
                    r = r + nm
                END IF
            END IF
        ELSE
            i = i + 1
        END IF
    LOOP
    DefFn_SharedList$ = r
END FUNCTION

''Rewrites lineinput3buffer$ in place and fills defFnBodyText$ (see the top of this file).
'A converted DEF FN line is overwritten with spaces, so offsets and line numbers stay.
SUB DefFn_RewriteBuffer
    DIM lineText AS STRING, fnName AS STRING, params AS STRING, expr AS STRING
    DIM sharedVars AS STRING, constNames AS STRING, crlf AS STRING
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
        IF DefFn_Parse%(lineText, fnName, params, expr) THEN
            IF LEN(constNames) = 0 THEN constNames = DefFn_ConstNames$(lineinput3buffer$)
            sharedVars = DefFn_SharedList$(expr, fnName, DefFn_ParamNames$(params), constNames)
            IF LEN(params) THEN
                defFnBodyText$ = defFnBodyText$ + "FUNCTION " + fnName + " (" + params + ")" + crlf
            ELSE
                defFnBodyText$ = defFnBodyText$ + "FUNCTION " + fnName + crlf
            END IF
            IF LEN(sharedVars) THEN defFnBodyText$ = defFnBodyText$ + "SHARED " + sharedVars + crlf
            defFnBodyText$ = defFnBodyText$ + fnName + " = " + expr + crlf + "END FUNCTION" + crlf
            MID$(lineinput3buffer$, p, e - p) = SPACE$(e - p)
        END IF
        p = e + 1
    LOOP
END SUB