'===============================================================================
' QBNex language extensions lowered to plain QBasic statements (R13)
'
'   ENUM [name] ... END ENUM          -> CONST member = value lines
'   FOR EACH v IN array() ... NEXT    -> FOR over LBOUND..UBOUND with v = array(i)
'   TRY ... CATCH [e] ... FINALLY ... END TRY
'                                     -> ON ERROR GOTO / RESUME labels
'
' The lowering runs on every source line before the class syntax and the
' top-level runtime lowering, in the prepass and in the main pass, so the
' generated lines take the normal compile path of both backends. A line that
' starts with one of these keywords only changes meaning when it would have
' been a syntax error before (ENUM, EACH and TRY are not QBasic statements).
'===============================================================================

SUB SyntaxExt_Reset
    syntaxExtEnumActive = 0
    syntaxExtEnumNext$ = ""
    syntaxExtForStack$ = ""
    syntaxExtTryStack$ = ""
    syntaxExtCounter = 0
END SUB

FUNCTION SyntaxExt_CodePart$ (text$, commentPart$)
    'split a line into code and trailing comment (' outside a string literal)
    DIM i AS LONG
    DIM inString AS INTEGER
    DIM c AS STRING

    commentPart$ = ""
    FOR i = 1 TO LEN(text$)
        c = MID$(text$, i, 1)
        IF c = CHR$(34) THEN inString = NOT inString
        IF c = "'" AND inString = 0 THEN
            commentPart$ = MID$(text$, i)
            SyntaxExt_CodePart$ = LEFT$(text$, i - 1)
            EXIT FUNCTION
        END IF
    NEXT
    SyntaxExt_CodePart$ = text$
END FUNCTION

FUNCTION SyntaxExt_SplitTop& (text$, sep$, parts$())
    'split on sep$ outside string literals and parentheses; returns the number of parts
    DIM i AS LONG
    DIM n AS LONG
    DIM depth AS LONG
    DIM inString AS INTEGER
    DIM c AS STRING
    DIM startPos AS LONG

    startPos = 1
    FOR i = 1 TO LEN(text$)
        c = MID$(text$, i, 1)
        IF c = CHR$(34) THEN inString = NOT inString
        IF inString = 0 THEN
            IF c = "(" THEN depth = depth + 1
            IF c = ")" THEN depth = depth - 1
            IF c = sep$ AND depth = 0 THEN
                n = n + 1
                IF n > UBOUND(parts$) THEN REDIM _PRESERVE parts$(1 TO n + 16)
                parts$(n) = MID$(text$, startPos, i - startPos)
                startPos = i + 1
            END IF
        END IF
    NEXT
    n = n + 1
    IF n > UBOUND(parts$) THEN REDIM _PRESERVE parts$(1 TO n + 16)
    parts$(n) = MID$(text$, startPos)
    SyntaxExt_SplitTop& = n
END FUNCTION

FUNCTION SyntaxExt_IsWord% (upperText$, word$)
    'upperText$ starts with word$ followed by end, space or tab
    IF LEFT$(upperText$, LEN(word$)) <> word$ THEN EXIT FUNCTION
    IF LEN(upperText$) = LEN(word$) THEN SyntaxExt_IsWord% = -1: EXIT FUNCTION
    SELECT CASE MID$(upperText$, LEN(word$) + 1, 1)
        CASE " ", CHR$(9): SyntaxExt_IsWord% = -1
    END SELECT
END FUNCTION

SUB SyntaxExt_PushLine (outLines$, line$)
    IF LEN(outLines$) THEN outLines$ = outLines$ + CHR$(10)
    outLines$ = outLines$ + line$
END SUB

FUNCTION SyntaxExt_PopEntry$ (stack$)
    DIM p AS LONG

    IF LEN(stack$) = 0 THEN SyntaxExt_PopEntry$ = CHR$(0): EXIT FUNCTION
    p = _INSTRREV(stack$, CHR$(10))
    IF p = 0 THEN
        SyntaxExt_PopEntry$ = stack$
        stack$ = ""
    ELSE
        SyntaxExt_PopEntry$ = MID$(stack$, p + 1)
        stack$ = LEFT$(stack$, p - 1)
    END IF
END FUNCTION

FUNCTION SyntaxExt_TopEntry$ (stack$)
    DIM p AS LONG

    p = _INSTRREV(stack$, CHR$(10))
    IF p = 0 THEN SyntaxExt_TopEntry$ = stack$ ELSE SyntaxExt_TopEntry$ = MID$(stack$, p + 1)
END FUNCTION

SUB SyntaxExt_PushEntry (stack$, entry$)
    IF LEN(stack$) THEN stack$ = stack$ + CHR$(10) + entry$ ELSE stack$ = CHR$(1) + entry$
END SUB

FUNCTION SyntaxExt_ProcessLine$ (rawLine$)
    'returns the first lowered line; further lines are queued on the class-syntax queue
    DIM codeText AS STRING
    DIM commentText AS STRING
    DIM trimmed AS STRING
    DIM upper AS STRING
    DIM outLines AS STRING
    DIM changed AS INTEGER
    DIM n AS LONG
    DIM k AS LONG
    DIM j AS LONG
    DIM s AS STRING
    DIM su AS STRING
    DIM rebuilt AS STRING
    DIM p AS LONG
    DIM nl AS LONG
    REDIM stmts(1 TO 16) AS STRING
    REDIM items(1 TO 16) AS STRING

    SyntaxExt_ProcessLine$ = rawLine$
    IF rawLine$ = CHR$(13) THEN EXIT FUNCTION

    codeText = SyntaxExt_CodePart$(rawLine$, commentText)
    trimmed = LTRIM$(RTRIM$(codeText))
    upper = UCASE$(trimmed)

    '---------------- ENUM ----------------
    IF syntaxExtEnumActive THEN
        IF upper = "END ENUM" THEN
            syntaxExtEnumActive = 0
            SyntaxExt_ProcessLine$ = commentText
            EXIT FUNCTION
        END IF
        IF LEN(trimmed) = 0 THEN EXIT FUNCTION
        n = SyntaxExt_SplitTop&(trimmed, ",", items())
        FOR k = 1 TO n
            s = LTRIM$(RTRIM$(items(k)))
            IF LEN(s) THEN
                p = INSTR(s, "=")
                IF p THEN
                    SyntaxExt_PushLine outLines, "CONST " + RTRIM$(LEFT$(s, p - 1)) + " = " + LTRIM$(MID$(s, p + 1))
                    syntaxExtEnumNext$ = "(" + RTRIM$(LEFT$(s, p - 1)) + ") + 1"
                ELSE
                    SyntaxExt_PushLine outLines, "CONST " + s + " = " + syntaxExtEnumNext$
                    syntaxExtEnumNext$ = "(" + s + ") + 1"
                END IF
            END IF
        NEXT
        GOTO emitlines
    END IF
    IF upper = "ENUM" OR SyntaxExt_IsWord%(upper, "ENUM") THEN
        IF INSTR(trimmed, ":") = 0 AND INSTR(trimmed, "=") = 0 AND INSTR(trimmed, "(") = 0 THEN
            syntaxExtEnumActive = -1
            syntaxExtEnumNext$ = "0"
            SyntaxExt_ProcessLine$ = commentText
            EXIT FUNCTION
        END IF
    END IF

    '---------------- TRY / CATCH / FINALLY / END TRY ----------------
    IF upper = "TRY" THEN
        syntaxExtCounter = syntaxExtCounter + 1
        SyntaxExt_PushEntry syntaxExtTryStack$, LTRIM$(STR$(syntaxExtCounter)) + "|0|0"
        s = LTRIM$(STR$(syntaxExtCounter))
        SyntaxExt_PushLine outLines, "QBNEX_TRYERR" + s + "& = 0"
        SyntaxExt_PushLine outLines, "ON ERROR GOTO QBNEX_TRY_H" + s
        GOTO emitlines
    END IF
    IF upper = "CATCH" OR SyntaxExt_IsWord%(upper, "CATCH") OR upper = "FINALLY" OR upper = "END TRY" THEN
        IF LEN(syntaxExtTryStack$) AND INSTR(trimmed, "=") = 0 AND INSTR(trimmed, ":") = 0 THEN
            s = SyntaxExt_PopEntry$(syntaxExtTryStack$)
            IF LEFT$(s, 1) = CHR$(1) THEN s = MID$(s, 2)
            n = VAL(s) 'try number
            k = VAL(MID$(s, INSTR(s, "|") + 1)) 'phase: 0 body, 1 catch, 2 finally
            j = VAL(MID$(s, _INSTRREV(s, "|") + 1)) 'caught
            s = LTRIM$(STR$(n))
            IF k = 0 THEN
                'leave the protected body: no error -> skip to FINALLY
                SyntaxExt_PushLine outLines, "ON ERROR GOTO 0"
                SyntaxExt_PushLine outLines, "GOTO QBNEX_TRY_F" + s
                SyntaxExt_PushLine outLines, "QBNEX_TRY_H" + s + ":"
                SyntaxExt_PushLine outLines, "QBNEX_TRYERR" + s + "& = ERR"
                SyntaxExt_PushLine outLines, "RESUME QBNEX_TRY_C" + s
                SyntaxExt_PushLine outLines, "QBNEX_TRY_C" + s + ":"
                SyntaxExt_PushLine outLines, "ON ERROR GOTO 0"
                k = 1
            END IF
            IF LEFT$(upper, 5) = "CATCH" THEN
                j = 1
                su = LTRIM$(MID$(trimmed, 6))
                IF LEN(su) THEN SyntaxExt_PushLine outLines, su + " = QBNEX_TRYERR" + s + "&"
            ELSE
                IF k < 2 THEN
                    SyntaxExt_PushLine outLines, "QBNEX_TRY_F" + s + ":"
                    k = 2
                END IF
                IF upper = "END TRY" THEN
                    IF j = 0 THEN SyntaxExt_PushLine outLines, "IF QBNEX_TRYERR" + s + "& THEN ERROR QBNEX_TRYERR" + s + "&"
                    'an enclosing TRY body is protected again
                    IF LEN(syntaxExtTryStack$) THEN
                        su = SyntaxExt_TopEntry$(syntaxExtTryStack$)
                        IF LEFT$(su, 1) = CHR$(1) THEN su = MID$(su, 2)
                        IF VAL(MID$(su, INSTR(su, "|") + 1)) = 0 THEN SyntaxExt_PushLine outLines, "ON ERROR GOTO QBNEX_TRY_H" + LTRIM$(STR$(VAL(su)))
                    END IF
                    GOTO emitlines
                END IF
            END IF
            SyntaxExt_PushEntry syntaxExtTryStack$, s + "|" + LTRIM$(STR$(k)) + "|" + LTRIM$(STR$(j))
            GOTO emitlines
        END IF
    END IF

    '---------------- FOR EACH / NEXT ----------------
    IF INSTR(upper, "EACH") = 0 AND LEN(syntaxExtForStack$) = 0 THEN EXIT FUNCTION
    IF INSTR(upper, "FOR") = 0 AND INSTR(upper, "NEXT") = 0 THEN EXIT FUNCTION

    n = SyntaxExt_SplitTop&(codeText, ":", stmts())
    FOR k = 1 TO n
        s = LTRIM$(RTRIM$(stmts(k)))
        su = UCASE$(s)
        IF SyntaxExt_IsWord%(su, "FOR") THEN
            IF SyntaxExt_IsWord%(LTRIM$(MID$(su, 4)), "EACH") THEN
                'FOR EACH v IN array[()]
                s = LTRIM$(MID$(s, 4))
                s = LTRIM$(MID$(s, 5))
                p = INSTR(UCASE$(s), " IN ")
                IF p THEN
                    syntaxExtCounter = syntaxExtCounter + 1
                    rebuilt = "QBNEX_EACH" + LTRIM$(STR$(syntaxExtCounter)) + "&"
                    su = RTRIM$(LEFT$(s, p - 1)) 'element variable
                    s = LTRIM$(RTRIM$(MID$(s, p + 4))) 'array
                    IF RIGHT$(s, 2) = "()" THEN s = RTRIM$(LEFT$(s, LEN(s) - 2))
                    stmts(k) = "FOR " + rebuilt + " = LBOUND(" + s + ") TO UBOUND(" + s + "): " + su + " = " + s + "(" + rebuilt + ")"
                    SyntaxExt_PushEntry syntaxExtForStack$, rebuilt
                    changed = -1
                END IF
            ELSE
                IF LEN(syntaxExtForStack$) THEN SyntaxExt_PushEntry syntaxExtForStack$, ""
            END IF
        ELSEIF su = "NEXT" OR SyntaxExt_IsWord%(su, "NEXT") THEN
            IF LEN(syntaxExtForStack$) THEN
                nl = SyntaxExt_SplitTop&(LTRIM$(MID$(s, 5)), ",", items())
                IF LEN(LTRIM$(MID$(s, 5))) = 0 THEN nl = 0
                rebuilt = ""
                FOR j = 1 TO nl
                    su = SyntaxExt_PopEntry$(syntaxExtForStack$)
                    IF LEFT$(su, 1) = CHR$(1) THEN su = MID$(su, 2)
                    IF LEN(su) AND su <> CHR$(0) THEN items(j) = su: changed = -1
                    IF j > 1 THEN rebuilt = rebuilt + ", "
                    rebuilt = rebuilt + LTRIM$(RTRIM$(items(j)))
                NEXT
                IF nl = 0 THEN su = SyntaxExt_PopEntry$(syntaxExtForStack$): rebuilt = "": stmts(k) = "NEXT" ELSE stmts(k) = "NEXT " + rebuilt
            END IF
        END IF
    NEXT
    IF changed = 0 THEN EXIT FUNCTION
    rebuilt = ""
    FOR k = 1 TO n
        IF k > 1 THEN rebuilt = rebuilt + ": "
        rebuilt = rebuilt + stmts(k)
    NEXT
    SyntaxExt_ProcessLine$ = rebuilt + commentText
    EXIT FUNCTION

    emitlines:
    IF LEN(commentText) THEN SyntaxExt_PushLine outLines, commentText
    IF LEN(outLines) = 0 THEN SyntaxExt_ProcessLine$ = "": EXIT FUNCTION
    p = INSTR(outLines, CHR$(10))
    IF p = 0 THEN
        SyntaxExt_ProcessLine$ = outLines
    ELSE
        SyntaxExt_ProcessLine$ = LEFT$(outLines, p - 1)
        outLines = MID$(outLines, p + 1)
        DO
            p = INSTR(outLines, CHR$(10))
            IF p = 0 THEN ClassSyntax_QueueLine outLines: EXIT DO
            ClassSyntax_QueueLine LEFT$(outLines, p - 1)
            outLines = MID$(outLines, p + 1)
        LOOP
    END IF
END FUNCTION
