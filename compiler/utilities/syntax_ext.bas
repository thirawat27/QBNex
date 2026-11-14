'===============================================================================
' QBNex language extensions lowered to plain QBasic statements
'
'   ENUM [name] ... END ENUM          -> CONST member = value lines
'   FOR EACH v IN array() ... NEXT    -> FOR over LBOUND..UBOUND with v = array(i)
'   TRY ... CATCH [e] ... FINALLY ... END TRY
'                                     -> ON ERROR GOTO / RESUME labels
'
' The lowering runs on every source line before the class syntax and the
' top-level runtime lowering, in the prepass and in the main pass, so the
' generated lines take the normal compile path. A line that
' starts with one of these keywords only changes meaning when it would have
' been a syntax error before (ENUM, EACH and TRY are not QBasic statements).
'===============================================================================

SUB SyntaxExt_Reset
    syntaxExtEnumActive = 0
    syntaxExtEnumNext$ = ""
    syntaxExtForStack$ = ""
    syntaxExtTryStack$ = ""
    syntaxExtLoopStack$ = ""
    syntaxExtCounter = 0
END SUB

FUNCTION SyntaxExt_StackCount& (stack$)
    'number of entries of a stack made by SyntaxExt_PushEntry
    DIM i AS LONG
    DIM n AS LONG

    IF LEN(stack$) = 0 THEN EXIT FUNCTION
    n = 1
    i = INSTR(stack$, CHR$(10))
    DO WHILE i
        n = n + 1
        i = INSTR(i + 1, stack$, CHR$(10))
    LOOP
    SyntaxExt_StackCount& = n
END FUNCTION

FUNCTION SyntaxExt_TryPhase& (index AS LONG)
    'phase (0 body, 1 catch, 2 finally) of the index-th open TRY, the outermost being 1
    DIM i AS LONG
    DIM p AS LONG
    DIM q AS LONG
    DIM s AS STRING

    p = 1
    FOR i = 2 TO index
        p = INSTR(p, syntaxExtTryStack$, CHR$(10)) + 1
        IF p = 1 THEN SyntaxExt_TryPhase& = -1: EXIT FUNCTION
    NEXT
    q = INSTR(p, syntaxExtTryStack$, CHR$(10))
    IF q = 0 THEN q = LEN(syntaxExtTryStack$) + 1
    s = MID$(syntaxExtTryStack$, p, q - p)
    IF LEFT$(s, 1) = CHR$(1) THEN s = MID$(s, 2)
    SyntaxExt_TryPhase& = VAL(MID$(s, INSTR(s, "|") + 1))
END FUNCTION

FUNCTION SyntaxExt_TryNumber& (index AS LONG)
    'number of the index-th open TRY, the outermost being 1
    DIM i AS LONG
    DIM p AS LONG
    DIM s AS STRING

    p = 1
    FOR i = 2 TO index
        p = INSTR(p, syntaxExtTryStack$, CHR$(10)) + 1
        IF p = 1 THEN EXIT FUNCTION
    NEXT
    s = MID$(syntaxExtTryStack$, p, 12)
    IF LEFT$(s, 1) = CHR$(1) THEN s = MID$(s, 2)
    SyntaxExt_TryNumber& = VAL(s)
END FUNCTION

SUB SyntaxExt_RecordTryLabel (codeText$)
    'prepass: a label (or line number) that starts this line lies inside the body of
    'every open TRY that is in its body; GOTO to it does not leave those bodies
    DIM t AS STRING
    DIM i AS LONG
    DIM lbl AS STRING

    t = LTRIM$(codeText$)
    IF TopLevelRuntime_IsLabelLine%(t) = 0 THEN EXIT SUB
    i = 1
    IF MID$(t, 1, 1) >= "0" AND MID$(t, 1, 1) <= "9" THEN
        DO WHILE (MID$(t, i, 1) >= "0" AND MID$(t, i, 1) <= "9") OR MID$(t, i, 1) = ".": i = i + 1: LOOP
        lbl = LEFT$(t, i - 1)
    ELSE
        lbl = UCASE$(LEFT$(t, INSTR(t, ":") - 1))
    END IF
    FOR i = 1 TO SyntaxExt_StackCount&(syntaxExtTryStack$)
        IF SyntaxExt_TryPhase&(i) = 0 THEN syntaxExtTryLabels$ = syntaxExtTryLabels$ + LTRIM$(STR$(SyntaxExt_TryNumber&(i))) + ":" + lbl + "|"
    NEXT
END SUB

FUNCTION SyntaxExt_GotoLeaves$ (label$)
    'the statements that end the TRY bodies a GOTO to label$ leaves (see
    'SyntaxExt_TryLeaves$): from the innermost open TRY out to the first one whose
    'body holds the label
    DIM i AS LONG
    DIM r AS STRING

    FOR i = SyntaxExt_StackCount&(syntaxExtTryStack$) TO 1 STEP -1
        IF SyntaxExt_TryPhase&(i) = 0 THEN
            IF INSTR(syntaxExtTryLabels$, "|" + LTRIM$(STR$(SyntaxExt_TryNumber&(i))) + ":" + UCASE$(label$) + "|") THEN EXIT FOR
            r = r + "ON ERROR GOTO QBNEX_TRY_LEAVE: "
        END IF
    NEXT
    SyntaxExt_GotoLeaves$ = r
END FUNCTION

FUNCTION SyntaxExt_WordAt% (upperText$, at AS LONG, word$)
    'word$ stands at position at of upperText$ as a whole word, outside a string literal
    DIM i AS LONG
    DIM quotes AS LONG
    DIM c AS STRING

    IF MID$(upperText$, at, LEN(word$)) <> word$ THEN EXIT FUNCTION
    IF at > 1 THEN
        c = MID$(upperText$, at - 1, 1)
        IF (c >= "A" AND c <= "Z") OR (c >= "0" AND c <= "9") OR c = "_" OR c = "." OR c = "$" THEN EXIT FUNCTION
    END IF
    c = MID$(upperText$, at + LEN(word$), 1)
    IF (c >= "A" AND c <= "Z") OR (c >= "0" AND c <= "9") OR c = "_" OR c = "." OR c = "$" THEN EXIT FUNCTION
    i = INSTR(upperText$, CHR$(34))
    DO WHILE i > 0 AND i < at
        quotes = quotes + 1
        i = INSTR(i + 1, upperText$, CHR$(34))
    LOOP
    IF quotes AND 1 THEN EXIT FUNCTION
    SyntaxExt_WordAt% = -1
END FUNCTION

FUNCTION SyntaxExt_TryLeaves$ (kind$)
    'the statements that end the TRY bodies a jump out of a loop leaves: the loop of
    'kind$ (F, D, W; "" = the innermost loop) that the jump belongs to was opened
    'outside them, so their END of body, which puts the ON ERROR handler back, is skipped
    DIM i AS LONG
    DIM loopDepth AS LONG
    DIM found AS INTEGER
    DIM r AS STRING

    FOR i = LEN(syntaxExtLoopStack$) - 1 TO 1 STEP -2
        IF kind$ = "" OR MID$(syntaxExtLoopStack$, i, 1) = kind$ THEN
            loopDepth = ASC(syntaxExtLoopStack$, i + 1) - 1
            found = -1
            EXIT FOR
        END IF
    NEXT
    IF found = 0 THEN EXIT FUNCTION
    FOR i = SyntaxExt_StackCount&(syntaxExtTryStack$) TO loopDepth + 1 STEP -1
        IF SyntaxExt_TryPhase&(i) = 0 THEN r = r + "ON ERROR GOTO QBNEX_TRY_LEAVE: "
    NEXT
    SyntaxExt_TryLeaves$ = r
END FUNCTION

FUNCTION SyntaxExt_TrackLoops$ (codeText$)
    'Keeps the list of open loops (with the number of TRY blocks open where each began)
    'and returns codeText$ with the TRY bodies ended in front of every EXIT FOR / DO /
    'WHILE and _CONTINUE that jumps out of them. Without that the handler of the TRY
    'stayed set after the jump and took every later error of the program.
    DIM n AS LONG
    DIM k AS LONG
    DIM j AS LONG
    DIM p AS LONG
    DIM s AS STRING
    DIM su AS STRING
    DIM kind AS STRING
    DIM leaves AS STRING
    DIM rebuilt AS STRING
    DIM depthChar AS STRING
    DIM closes AS LONG
    DIM anyChange AS INTEGER
    REDIM stmts(1 TO 16) AS STRING
    REDIM items(1 TO 8) AS STRING

    SyntaxExt_TrackLoops$ = codeText$
    su = UCASE$(codeText$)
    IF INSTR(su, "FOR") = 0 AND INSTR(su, "NEXT") = 0 AND INSTR(su, "DO") = 0 AND INSTR(su, "LOOP") = 0 AND INSTR(su, "WHILE") = 0 AND INSTR(su, "WEND") = 0 AND INSTR(su, "_CONTINUE") = 0 AND INSTR(su, "GOTO") = 0 THEN EXIT FUNCTION

    depthChar = CHR$(SyntaxExt_StackCount&(syntaxExtTryStack$) + 1)
    n = SyntaxExt_SplitTop&(codeText$, ":", stmts())
    FOR k = 1 TO n
        s = stmts(k)
        su = UCASE$(LTRIM$(s))
        closes = 0
        kind = ""
        IF SyntaxExt_IsWord%(su, "FOR") THEN
            kind = "F"
        ELSEIF su = "DO" OR SyntaxExt_IsWord%(su, "DO") THEN
            kind = "D"
        ELSEIF SyntaxExt_IsWord%(su, "WHILE") THEN
            kind = "W"
        ELSEIF su = "WEND" OR su = "LOOP" OR SyntaxExt_IsWord%(su, "LOOP") THEN
            closes = 1
        ELSEIF su = "NEXT" OR SyntaxExt_IsWord%(su, "NEXT") THEN
            closes = 1
            IF LEN(LTRIM$(MID$(LTRIM$(s), 5))) THEN closes = SyntaxExt_SplitTop&(LTRIM$(MID$(LTRIM$(s), 5)), ",", items())
        ELSEIF LEN(syntaxExtTryStack$) THEN
            'a jump out of a loop (also after THEN / ELSE of a single-line IF)
            su = UCASE$(s)
            p = INSTR(su, "EXIT")
            DO WHILE p
                leaves = ""
                IF SyntaxExt_WordAt%(su, p, "EXIT") THEN
                    j = p + 4
                    DO WHILE MID$(su, j, 1) = " " OR MID$(su, j, 1) = CHR$(9): j = j + 1: LOOP
                    IF SyntaxExt_WordAt%(su, j, "FOR") THEN
                        leaves = SyntaxExt_TryLeaves$("F")
                    ELSEIF SyntaxExt_WordAt%(su, j, "DO") THEN
                        leaves = SyntaxExt_TryLeaves$("D")
                    ELSEIF SyntaxExt_WordAt%(su, j, "WHILE") THEN
                        leaves = SyntaxExt_TryLeaves$("W")
                    END IF
                END IF
                IF LEN(leaves) THEN
                    s = LEFT$(s, p - 1) + leaves + MID$(s, p)
                    su = UCASE$(s)
                    p = p + LEN(leaves)
                END IF
                p = INSTR(p + 4, su, "EXIT")
            LOOP
            'GOTO label out of the TRY (main pass: the prepass listed the labels inside each
            'TRY body). Not ON ERROR GOTO, and not ON n GOTO a, b with its list of targets.
            p = INSTR(su, "GOTO")
            DO WHILE p AND syntaxExtMainPass <> 0
                leaves = ""
                IF SyntaxExt_WordAt%(su, p, "GOTO") AND INSTR(LEFT$(su, p - 1), "ON ") = 0 THEN
                    j = p + 4
                    DO WHILE MID$(su, j, 1) = " " OR MID$(su, j, 1) = CHR$(9): j = j + 1: LOOP
                    closes = j
                    DO WHILE (MID$(su, closes, 1) >= "A" AND MID$(su, closes, 1) <= "Z") OR (MID$(su, closes, 1) >= "0" AND MID$(su, closes, 1) <= "9") OR MID$(su, closes, 1) = "_" OR MID$(su, closes, 1) = ".": closes = closes + 1: LOOP
                    IF closes > j THEN leaves = SyntaxExt_GotoLeaves$(MID$(su, j, closes - j))
                    closes = 0
                END IF
                IF LEN(leaves) THEN
                    s = LEFT$(s, p - 1) + leaves + MID$(s, p)
                    su = UCASE$(s)
                    p = p + LEN(leaves)
                END IF
                p = INSTR(p + 4, su, "GOTO")
            LOOP
            p = INSTR(su, "_CONTINUE")
            DO WHILE p
                leaves = ""
                IF SyntaxExt_WordAt%(su, p, "_CONTINUE") THEN leaves = SyntaxExt_TryLeaves$("")
                IF LEN(leaves) THEN
                    s = LEFT$(s, p - 1) + leaves + MID$(s, p)
                    su = UCASE$(s)
                    p = p + LEN(leaves)
                END IF
                p = INSTR(p + 9, su, "_CONTINUE")
            LOOP
            IF stmts(k) <> s THEN stmts(k) = s: anyChange = -1
        END IF
        IF LEN(kind) THEN syntaxExtLoopStack$ = syntaxExtLoopStack$ + kind + depthChar
        IF closes > 0 THEN
            IF closes * 2 >= LEN(syntaxExtLoopStack$) THEN syntaxExtLoopStack$ = "" ELSE syntaxExtLoopStack$ = LEFT$(syntaxExtLoopStack$, LEN(syntaxExtLoopStack$) - closes * 2)
        END IF
    NEXT
    IF anyChange = 0 THEN EXIT FUNCTION
    FOR k = 1 TO n
        IF k > 1 THEN rebuilt = rebuilt + ":"
        rebuilt = rebuilt + stmts(k)
    NEXT
    SyntaxExt_TrackLoops$ = rebuilt
END FUNCTION

FUNCTION SyntaxExt_CodePart$ (text$, commentPart$)
    'split a line into code and trailing comment (' outside a string literal)
    DIM i AS LONG
    DIM inString AS INTEGER
    DIM c AS STRING

    commentPart$ = ""
    IF INSTR(text$, "'") = 0 THEN SyntaxExt_CodePart$ = text$: EXIT FUNCTION 'no comment possible
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

FUNCTION SyntaxExt_LowerLine$ (rawLine$)
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
    DIM idxVar AS STRING
    DIM idxList AS STRING
    DIM idxNext AS STRING
    REDIM stmts(1 TO 16) AS STRING
    REDIM items(1 TO 16) AS STRING

    SyntaxExt_LowerLine$ = rawLine$
    IF rawLine$ = CHR$(13) THEN EXIT FUNCTION

    codeText = SyntaxExt_CodePart$(rawLine$, commentText)
    trimmed = LTRIM$(RTRIM$(codeText))
    upper = UCASE$(trimmed)

    '---------------- ENUM ----------------
    IF syntaxExtEnumActive THEN
        IF upper = "END ENUM" THEN
            syntaxExtEnumActive = 0
            SyntaxExt_LowerLine$ = commentText
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
            SyntaxExt_LowerLine$ = commentText
            EXIT FUNCTION
        END IF
    END IF

    '---------------- TRY / CATCH / FINALLY / END TRY ----------------
    'ON ERROR targets must be module-level labels (QB rule, also checked by QIR), so every TRY
    'uses the one module-level handler QBNEX_TRY_HANDLER (written into the hoisted head by
    'StdLib_PrepareHoistedImports): it stores ERR in the SHARED QBNEX_TRYERR and RESUMEs NEXT.
    'Each line of a TRY body is followed by a check that leaves the body when QBNEX_TRYERR is set.
    IF upper = "TRY" THEN
        syntaxExtCounter = syntaxExtCounter + 1
        s = LTRIM$(STR$(syntaxExtCounter))
        SyntaxExt_PushEntry syntaxExtTryStack$, s + "|0|0"
        SyntaxExt_PushLine outLines, "DIM QBNEX_TRYE" + s + " AS LONG"
        'in a loop the error of an earlier pass must not be raised again by END TRY
        SyntaxExt_PushLine outLines, "QBNEX_TRYE" + s + " = 0"
        SyntaxExt_PushLine outLines, "QBNEX_TRYERR = 0"
        'QBNEX_TRY_ENTER<n> / QBNEX_TRY_LEAVE are not labels: the compiler (ON ERROR GOTO in
        'qbnex.bas) makes the run time keep the handler set before the TRY and put it back
        'when the body ends, so the program's own ON ERROR GOTO, or the TRY around this
        'one, is in force again
        SyntaxExt_PushLine outLines, "ON ERROR GOTO QBNEX_TRY_ENTER" + s
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
                'end of the protected body
                SyntaxExt_PushLine outLines, "ON ERROR GOTO QBNEX_TRY_LEAVE"
                SyntaxExt_PushLine outLines, "GOTO QBNEX_TRY_F" + s
                SyntaxExt_PushLine outLines, "QBNEX_TRY_C" + s + ":"
                SyntaxExt_PushLine outLines, "ON ERROR GOTO QBNEX_TRY_LEAVE"
                SyntaxExt_PushLine outLines, "QBNEX_TRYE" + s + " = QBNEX_TRYERR: QBNEX_TRYERR = 0"
                k = 1
            END IF
            IF LEFT$(upper, 5) = "CATCH" THEN
                j = 1
                su = LTRIM$(MID$(trimmed, 6))
                IF LEN(su) THEN SyntaxExt_PushLine outLines, su + " = QBNEX_TRYE" + s
            ELSE
                IF k < 2 THEN
                    SyntaxExt_PushLine outLines, "QBNEX_TRY_F" + s + ":"
                    k = 2
                END IF
                IF upper = "END TRY" THEN
                    'no CATCH: the error is raised again after FINALLY
                    IF j = 0 THEN SyntaxExt_PushLine outLines, "IF QBNEX_TRYE" + s + " THEN ERROR QBNEX_TRYE" + s
                    'the body of an enclosing TRY is protected again since this one's body
                    'ended; an error raised again here, or in CATCH / FINALLY, leaves it
                    IF LEN(syntaxExtTryStack$) THEN
                        su = SyntaxExt_TopEntry$(syntaxExtTryStack$)
                        IF LEFT$(su, 1) = CHR$(1) THEN su = MID$(su, 2)
                        IF VAL(MID$(su, INSTR(su, "|") + 1)) = 0 THEN SyntaxExt_PushLine outLines, "IF QBNEX_TRYERR THEN GOTO QBNEX_TRY_C" + LTRIM$(STR$(VAL(su)))
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
                    su = RTRIM$(LEFT$(s, p - 1)) 'element variable
                    s = LTRIM$(RTRIM$(MID$(s, p + 4))) 'array
                    IF RIGHT$(s, 2) = "()" THEN s = RTRIM$(LEFT$(s, LEN(s) - 2))
                    'one FOR per dimension (rank from the DIM/REDIM seen for the array, else 1);
                    'the first index is the outermost loop
                    nl = SyntaxExt_ArrayRank&(s)
                    rebuilt = "": idxList = "": idxNext = ""
                    FOR j = 1 TO nl
                        idxVar = "QBNEX_EACH" + LTRIM$(STR$(syntaxExtCounter)) + "_" + LTRIM$(STR$(j)) + "&"
                        IF nl = 1 THEN
                            rebuilt = rebuilt + "FOR " + idxVar + " = LBOUND(" + s + ") TO UBOUND(" + s + "): "
                        ELSE
                            rebuilt = rebuilt + "FOR " + idxVar + " = LBOUND(" + s + ", " + LTRIM$(STR$(j)) + ") TO UBOUND(" + s + ", " + LTRIM$(STR$(j)) + "): "
                        END IF
                        IF j > 1 THEN idxList = idxList + ", ": idxNext = ", " + idxNext
                        idxList = idxList + idxVar
                        idxNext = idxVar + idxNext
                    NEXT
                    stmts(k) = rebuilt + su + " = " + s + "(" + idxList + ")"
                    'the array after a tab: EXIT FOR of a loop over several dimensions needs it
                    IF nl > 1 THEN idxNext = idxNext + CHR$(9) + s
                    SyntaxExt_PushEntry syntaxExtForStack$, idxNext
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
                    IF INSTR(su, CHR$(9)) THEN su = LEFT$(su, INSTR(su, CHR$(9)) - 1)
                    IF LEN(su) AND su <> CHR$(0) THEN items(j) = su: changed = -1
                    IF j > 1 THEN rebuilt = rebuilt + ", "
                    rebuilt = rebuilt + LTRIM$(RTRIM$(items(j)))
                NEXT
                IF nl = 0 THEN
                    su = SyntaxExt_PopEntry$(syntaxExtForStack$)
                    IF LEFT$(su, 1) = CHR$(1) THEN su = MID$(su, 2)
                    IF INSTR(su, CHR$(9)) THEN su = LEFT$(su, INSTR(su, CHR$(9)) - 1)
                    IF LEN(su) AND su <> CHR$(0) THEN stmts(k) = "NEXT " + su: changed = -1 ELSE stmts(k) = "NEXT"
                ELSE
                    stmts(k) = "NEXT " + rebuilt
                END IF
            END IF
        ELSEIF INSTR(su, "EXIT") > 0 AND LEN(syntaxExtForStack$) > 0 THEN
            'EXIT FOR inside FOR EACH over an array of several dimensions: there is one FOR
            'a dimension, and EXIT FOR leaves the innermost only. The outer indexes are set
            'to their last value first, so their NEXT ends them too.
            idxNext = SyntaxExt_TopEntry$(syntaxExtForStack$)
            IF LEFT$(idxNext, 1) = CHR$(1) THEN idxNext = MID$(idxNext, 2)
            p = INSTR(idxNext, CHR$(9))
            IF p THEN
                idxVar = MID$(idxNext, p + 1) 'the array
                nl = SyntaxExt_SplitTop&(LEFT$(idxNext, p - 1), ",", items()) 'innermost index first
                rebuilt = ""
                FOR j = nl TO 2 STEP -1
                    rebuilt = rebuilt + LTRIM$(RTRIM$(items(j))) + " = UBOUND(" + idxVar + ", " + LTRIM$(STR$(nl - j + 1)) + "): "
                NEXT
                p = INSTR(su, "EXIT")
                DO WHILE p
                    IF SyntaxExt_WordAt%(su, p, "EXIT") THEN
                        j = p + 4
                        DO WHILE MID$(su, j, 1) = " " OR MID$(su, j, 1) = CHR$(9): j = j + 1: LOOP
                        IF SyntaxExt_WordAt%(su, j, "FOR") THEN
                            s = LEFT$(s, p - 1) + rebuilt + MID$(s, p)
                            su = UCASE$(s)
                            p = p + LEN(rebuilt)
                            changed = -1
                        END IF
                    END IF
                    p = INSTR(p + 4, su, "EXIT")
                LOOP
                stmts(k) = s
            END IF
        END IF
    NEXT
    IF changed = 0 THEN EXIT FUNCTION
    rebuilt = ""
    FOR k = 1 TO n
        IF k > 1 THEN rebuilt = rebuilt + ": "
        rebuilt = rebuilt + stmts(k)
    NEXT
    SyntaxExt_LowerLine$ = rebuilt + commentText
    EXIT FUNCTION

    emitlines:
    IF LEN(commentText) THEN SyntaxExt_PushLine outLines, commentText
    IF LEN(outLines) = 0 THEN SyntaxExt_LowerLine$ = "": EXIT FUNCTION
    p = INSTR(outLines, CHR$(10))
    IF p = 0 THEN
        SyntaxExt_LowerLine$ = outLines
    ELSE
        SyntaxExt_LowerLine$ = LEFT$(outLines, p - 1)
        outLines = MID$(outLines, p + 1)
        DO
            p = INSTR(outLines, CHR$(10))
            IF p = 0 THEN ClassSyntax_QueueLine outLines: EXIT DO
            ClassSyntax_QueueLine LEFT$(outLines, p - 1)
            outLines = MID$(outLines, p + 1)
        LOOP
    END IF
END FUNCTION

FUNCTION SyntaxExt_ArrayKey$ (arrayName$)
    SyntaxExt_ArrayKey$ = SyntaxExt_OptKey$(arrayName$)
END FUNCTION

FUNCTION SyntaxExt_ArrayRank& (arrayName$)
    'number of dimensions recorded for an array by DIM/REDIM/STATIC/COMMON (1 when unknown)
    DIM k AS LONG
    k = INSTR(syntaxExtArrays$, CHR$(10) + SyntaxExt_ArrayKey$(arrayName$) + CHR$(9))
    IF k = 0 THEN SyntaxExt_ArrayRank& = 1: EXIT FUNCTION
    k = k + LEN(SyntaxExt_ArrayKey$(arrayName$)) + 2
    SyntaxExt_ArrayRank& = VAL(MID$(syntaxExtArrays$, k))
    IF VAL(MID$(syntaxExtArrays$, k)) < 1 THEN SyntaxExt_ArrayRank& = 1
END FUNCTION

SUB SyntaxExt_RecordArrays (codeText$)
    'remember the rank of arrays declared on this line (for FOR EACH over several dimensions)
    REDIM stmts(1 TO 16) AS STRING
    REDIM items(1 TO 16) AS STRING
    REDIM dims(1 TO 16) AS STRING
    DIM n AS LONG
    DIM k AS LONG
    DIM j AS LONG
    DIM m AS LONG
    DIM s AS STRING
    DIM su AS STRING
    DIM nm AS STRING
    DIM p AS LONG
    DIM q AS LONG
    DIM arrKey AS STRING

    n = SyntaxExt_SplitTop&(codeText$, ":", stmts())
    FOR k = 1 TO n
        s = LTRIM$(RTRIM$(stmts(k)))
        su = UCASE$(s)
        IF SyntaxExt_IsWord%(su, "DIM") OR SyntaxExt_IsWord%(su, "REDIM") OR SyntaxExt_IsWord%(su, "STATIC") OR SyntaxExt_IsWord%(su, "COMMON") THEN
            s = LTRIM$(MID$(s, INSTR(s, " ") + 1))
            DO
                su = UCASE$(s)
                IF SyntaxExt_IsWord%(su, "SHARED") OR SyntaxExt_IsWord%(su, "_PRESERVE") OR SyntaxExt_IsWord%(su, "PRESERVE") THEN
                    s = LTRIM$(MID$(s, INSTR(s, " ") + 1))
                ELSE
                    EXIT DO
                END IF
            LOOP
            m = SyntaxExt_SplitTop&(s, ",", items())
            FOR j = 1 TO m
                nm = LTRIM$(items(j))
                p = INSTR(nm, "(")
                IF p > 1 THEN
                    q = _INSTRREV(nm, ")")
                    IF q > p AND LEN(LTRIM$(MID$(nm, p + 1, q - p - 1))) > 0 THEN
                        arrKey = SyntaxExt_ArrayKey$(RTRIM$(LEFT$(nm, p - 1)))
                        q = INSTR(syntaxExtArrays$, CHR$(10) + arrKey + CHR$(9))
                        IF q = 0 THEN
                            IF LEN(syntaxExtArrays$) = 0 THEN syntaxExtArrays$ = CHR$(10)
                            syntaxExtArrays$ = syntaxExtArrays$ + arrKey + CHR$(9) + LTRIM$(STR$(SyntaxExt_SplitTop&(MID$(nm, p + 1, _INSTRREV(nm, ")") - p - 1), ",", dims()))) + CHR$(10)
                        END IF
                    END IF
                END IF
            NEXT
        END IF
    NEXT
END SUB

FUNCTION SyntaxExt_IsBlockWord% (stmt$)
    'TRY/CATCH/FINALLY/END TRY and ENUM/END ENUM statements
    DIM u AS STRING
    u = UCASE$(LTRIM$(RTRIM$(stmt$)))
    IF u = "TRY" OR u = "FINALLY" OR u = "END TRY" OR u = "END ENUM" OR u = "CATCH" THEN SyntaxExt_IsBlockWord% = -1: EXIT FUNCTION
    IF SyntaxExt_IsWord%(u, "CATCH") AND INSTR(u, "=") = 0 THEN SyntaxExt_IsBlockWord% = -1: EXIT FUNCTION
    IF (u = "ENUM" OR SyntaxExt_IsWord%(u, "ENUM")) AND INSTR(u, "=") = 0 AND INSTR(u, "(") = 0 THEN SyntaxExt_IsBlockWord% = -1
END FUNCTION

FUNCTION SyntaxExt_LineHasTry% (rawLine$)
    DIM codeText AS STRING
    DIM commentText AS STRING
    DIM n AS LONG
    DIM k AS LONG
    REDIM stmts(1 TO 16) AS STRING

    codeText = SyntaxExt_CodePart$(rawLine$, commentText)
    IF INSTR(UCASE$(codeText), "TRY") = 0 THEN EXIT FUNCTION
    n = SyntaxExt_SplitTop&(codeText, ":", stmts())
    FOR k = 1 TO n
        IF UCASE$(LTRIM$(RTRIM$(stmts(k)))) = "TRY" THEN SyntaxExt_LineHasTry% = -1: EXIT FUNCTION
    NEXT
END FUNCTION

FUNCTION SyntaxExt_ProcessLine$ (rawLine$)
    'lower one source line; a line inside a TRY body is followed by the error check
    DIM codeText AS STRING
    DIM commentText AS STRING
    DIM upper AS STRING
    DIM topEntry AS STRING
    DIM checkIt AS INTEGER
    DIM procLine AS STRING
    DIM savedQueue AS STRING
    DIM collected AS STRING
    DIM piece AS STRING
    DIM n AS LONG
    DIM k AS LONG
    DIM hit AS INTEGER
    DIM p AS LONG
    REDIM stmts(1 TO 16) AS STRING

    IF rawLine$ = CHR$(13) THEN SyntaxExt_ProcessLine$ = rawLine$: EXIT FUNCTION

    procLine = rawLine$
    IF INSTR(procLine, "$" + CHR$(34)) THEN procLine = SyntaxExt_Interpolate$(procLine)
    codeText = SyntaxExt_CodePart$(procLine, commentText)
    upper = UCASE$(LTRIM$(RTRIM$(codeText)))

    'TRY/CATCH/FINALLY/END TRY/ENUM keywords joined with other statements by ':' are lowered
    'one statement at a time
    IF INSTR(codeText, ":") THEN
        n = SyntaxExt_SplitTop&(codeText, ":", stmts())
        IF n > 1 THEN
            FOR k = 1 TO n
                IF SyntaxExt_IsBlockWord%(stmts(k)) THEN hit = -1
            NEXT
            IF hit THEN
                savedQueue = classSyntaxQueue$
                classSyntaxQueue$ = ""
                FOR k = 1 TO n
                    piece = LTRIM$(RTRIM$(stmts(k)))
                    'a leading label keeps its colon
                    IF k = 1 AND LEN(piece) > 0 AND INSTR(piece, " ") = 0 AND SyntaxExt_IsBlockWord%(piece) = 0 THEN piece = piece + ":"
                    IF LEN(piece) THEN
                        piece = SyntaxExt_ProcessLine$(piece)
                        IF LEN(collected) THEN collected = collected + CHR$(10)
                        collected = collected + piece
                        IF LEN(classSyntaxQueue$) THEN collected = collected + CHR$(10) + classSyntaxQueue$
                        classSyntaxQueue$ = ""
                    END IF
                NEXT
                IF LEN(commentText) THEN collected = collected + CHR$(10) + commentText
                classSyntaxQueue$ = savedQueue
                p = INSTR(collected, CHR$(10))
                IF p = 0 THEN SyntaxExt_ProcessLine$ = collected: EXIT FUNCTION
                SyntaxExt_ProcessLine$ = LEFT$(collected, p - 1)
                collected = MID$(collected, p + 1)
                IF LEN(classSyntaxQueue$) THEN classSyntaxQueue$ = classSyntaxQueue$ + CHR$(10)
                classSyntaxQueue$ = classSyntaxQueue$ + collected
                EXIT FUNCTION
            END IF
        END IF
    END IF

    IF LEN(syntaxExtTryStack$) > 0 AND LEN(upper) > 0 THEN
        topEntry = SyntaxExt_TopEntry$(syntaxExtTryStack$)
        IF LEFT$(topEntry, 1) = CHR$(1) THEN topEntry = MID$(topEntry, 2)
        IF VAL(MID$(topEntry, INSTR(topEntry, "|") + 1)) = 0 THEN checkIt = -1
        IF SyntaxExt_IsBlockWord%(upper) THEN checkIt = 0
        IF RIGHT$(upper, 1) = "_" OR LEFT$(upper, 1) = "$" OR SyntaxExt_IsWord%(upper, "SELECT") THEN checkIt = 0
    END IF
    IF syntaxExtUsesTry <> 0 AND LEN(upper) > 0 THEN
        IF syntaxExtMainPass = 0 AND LEN(syntaxExtTryStack$) > 0 THEN SyntaxExt_RecordTryLabel codeText
        topEntry = SyntaxExt_TrackLoops$(codeText)
        IF topEntry <> codeText THEN codeText = topEntry: procLine = codeText + commentText
        IF LEN(syntaxExtTryStack$) THEN topEntry = SyntaxExt_TopEntry$(syntaxExtTryStack$): IF LEFT$(topEntry, 1) = CHR$(1) THEN topEntry = MID$(topEntry, 2)
    END IF
    IF INSTR(upper, "DIM") OR INSTR(upper, "STATIC") OR INSTR(upper, "COMMON") THEN SyntaxExt_RecordArrays codeText
    IF LEN(syntaxExtOptional$) OR INSTR(UCASE$(procLine), "OPTIONAL") THEN procLine = SyntaxExt_Optional$(procLine)
    SyntaxExt_ProcessLine$ = SyntaxExt_LowerLine$(procLine)
    IF checkIt THEN ClassSyntax_QueueLine "IF QBNEX_TRYERR THEN GOTO QBNEX_TRY_C" + LTRIM$(STR$(VAL(topEntry)))
END FUNCTION
FUNCTION SyntaxExt_InterpEnd& (text$, startPos AS LONG)
    'position of the " that closes a $"..." starting at startPos: quotes inside {...} belong to
    'string literals of the expression
    DIM i AS LONG
    DIM depth AS LONG
    DIM c AS STRING

    i = startPos
    DO WHILE i <= LEN(text$)
        c = MID$(text$, i, 1)
        IF depth = 0 THEN
            IF c = CHR$(34) THEN SyntaxExt_InterpEnd& = i: EXIT FUNCTION
            IF (c = "{" OR c = "}") AND MID$(text$, i + 1, 1) = c THEN
                i = i + 1
            ELSEIF c = "{" THEN
                depth = 1
            END IF
        ELSE
            IF c = CHR$(34) THEN
                i = INSTR(i + 1, text$, CHR$(34))
                IF i = 0 THEN EXIT DO
            ELSEIF c = "{" THEN
                depth = depth + 1
            ELSEIF c = "}" THEN
                depth = depth - 1
            END IF
        END IF
        i = i + 1
    LOOP
    SyntaxExt_InterpEnd& = LEN(text$) + 1
END FUNCTION
FUNCTION SyntaxExt_InterpolateBody$ (body$)
    'text of $"..." -> ("lit" + _TOSTR$(expr) + "lit"); {{ and }} are literal braces
    DIM i AS LONG
    DIM j AS LONG
    DIM c AS STRING
    DIM lit AS STRING
    DIM parts AS STRING
    DIM nparts AS LONG
    DIM depth AS LONG

    i = 1
    DO WHILE i <= LEN(body$)
        c = MID$(body$, i, 1)
        IF c = "{" AND MID$(body$, i + 1, 1) = "{" THEN lit = lit + "{": i = i + 2: GOTO interpnext
        IF c = "}" AND MID$(body$, i + 1, 1) = "}" THEN lit = lit + "}": i = i + 2: GOTO interpnext
        IF c = "{" THEN
            'matching } (string literals and nested braces of the expression are skipped)
            depth = 1: j = i + 1
            DO WHILE j <= LEN(body$)
                c = MID$(body$, j, 1)
                IF c = CHR$(34) THEN
                    j = INSTR(j + 1, body$, CHR$(34))
                    IF j = 0 THEN j = LEN(body$) + 1: EXIT DO
                ELSEIF c = "{" THEN
                    depth = depth + 1
                ELSEIF c = "}" THEN
                    depth = depth - 1
                    IF depth = 0 THEN EXIT DO
                END IF
                j = j + 1
            LOOP
            IF LEN(lit) THEN
                IF nparts THEN parts = parts + " + "
                parts = parts + CHR$(34) + lit + CHR$(34): nparts = nparts + 1
                lit = ""
            END IF
            IF nparts THEN parts = parts + " + "
            parts = parts + "_TOSTR$(" + MID$(body$, i + 1, j - i - 1) + ")": nparts = nparts + 1
            i = j + 1
            GOTO interpnext
        END IF
        lit = lit + c
        i = i + 1
        interpnext:
    LOOP
    IF LEN(lit) OR nparts = 0 THEN
        IF nparts THEN parts = parts + " + "
        parts = parts + CHR$(34) + lit + CHR$(34)
    END IF
    SyntaxExt_InterpolateBody$ = "(" + parts + ")"
END FUNCTION

FUNCTION SyntaxExt_Interpolate$ (text$)
    'replace every $"..." string (a $ that does not end a name) outside string literals and comments
    DIM i AS LONG
    DIM j AS LONG
    DIM c AS STRING
    DIM prevChar AS STRING
    DIM inString AS INTEGER
    DIM result AS STRING

    i = 1
    DO WHILE i <= LEN(text$)
        c = MID$(text$, i, 1)
        IF inString = 0 AND c = "'" THEN result = result + MID$(text$, i): EXIT DO
        IF c = CHR$(34) THEN
            IF inString = 0 AND i > 1 THEN
                IF MID$(text$, i - 1, 1) = "$" THEN
                    prevChar = ""
                    IF i > 2 THEN prevChar = MID$(text$, i - 2, 1)
                    IF ClassSyntax_IsIdentifierChar%(prevChar) = 0 THEN
                        j = SyntaxExt_InterpEnd&(text$, i + 1)
                        result = LEFT$(result, LEN(result) - 1) + SyntaxExt_InterpolateBody$(MID$(text$, i + 1, j - i - 1))
                        i = j + 1
                        GOTO interpchar
                    END IF
                END IF
            END IF
            inString = NOT inString
        END IF
        result = result + c
        i = i + 1
        interpchar:
    LOOP
    SyntaxExt_Interpolate$ = result
END FUNCTION
'---------------- OPTIONAL parameters with defaults ----------------
' SUB/FUNCTION/DECLARE headers: "OPTIONAL p AS T = default" -> "p AS T"; the defaults are kept in
' syntaxExtOptional$ (built in the prepass, kept for the main pass). A call that passes fewer
' arguments gets the missing trailing defaults appended as text, so the compiler sees a full call.

FUNCTION SyntaxExt_OptKey$ (procName$)
    DIM k AS STRING
    k = UCASE$(LTRIM$(RTRIM$(procName$)))
    DO WHILE LEN(k)
        IF INSTR("%&!#$~`", RIGHT$(k, 1)) = 0 THEN EXIT DO
        k = LEFT$(k, LEN(k) - 1)
    LOOP
    SyntaxExt_OptKey$ = k
END FUNCTION

FUNCTION SyntaxExt_OptHeader$ (codeText$)
    'strip OPTIONAL / defaults from a procedure header and register the defaults
    DIM upper AS STRING
    DIM kw AS LONG
    DIM p1 AS LONG
    DIM p2 AS LONG
    DIM depth AS LONG
    DIM i AS LONG
    DIM k AS LONG
    DIM n AS LONG
    DIM q AS LONG
    DIM inString AS INTEGER
    DIM s AS STRING
    DIM d AS STRING
    DIM isOpt AS INTEGER
    DIM procName AS STRING
    DIM newParams AS STRING
    DIM entry AS STRING
    DIM anyOpt AS INTEGER
    REDIM parts(1 TO 16) AS STRING

    SyntaxExt_OptHeader$ = codeText$
    upper = UCASE$(codeText$)
    p1 = INSTR(codeText$, "(")
    IF p1 = 0 THEN EXIT FUNCTION
    FOR i = p1 TO LEN(codeText$)
        IF MID$(codeText$, i, 1) = CHR$(34) THEN inString = NOT inString
        IF inString = 0 THEN
            IF MID$(codeText$, i, 1) = "(" THEN depth = depth + 1
            IF MID$(codeText$, i, 1) = ")" THEN
                depth = depth - 1
                IF depth = 0 THEN p2 = i: EXIT FOR
            END IF
        END IF
    NEXT
    IF p2 = 0 THEN EXIT FUNCTION
    kw = INSTR(upper, "FUNCTION ")
    IF kw = 0 OR (INSTR(upper, "SUB ") > 0 AND INSTR(upper, "SUB ") < kw) THEN kw = INSTR(upper, "SUB ") + 4 ELSE kw = kw + 9
    procName = MID$(codeText$, kw, p1 - kw)
    n = SyntaxExt_SplitTop&(MID$(codeText$, p1 + 1, p2 - p1 - 1), ",", parts())
    entry = SyntaxExt_OptKey$(procName) + CHR$(9) + LTRIM$(STR$(n))
    FOR k = 1 TO n
        s = LTRIM$(RTRIM$(parts(k)))
        isOpt = 0
        IF UCASE$(LEFT$(s, 9)) = "OPTIONAL " THEN s = LTRIM$(MID$(s, 10)): isOpt = -1: anyOpt = -1
        d = ""
        inString = 0: q = 0
        FOR i = 1 TO LEN(s)
            IF MID$(s, i, 1) = CHR$(34) THEN inString = NOT inString
            IF MID$(s, i, 1) = "=" AND inString = 0 THEN q = i: EXIT FOR
        NEXT
        IF q THEN d = LTRIM$(RTRIM$(MID$(s, q + 1))): s = RTRIM$(LEFT$(s, q - 1)): anyOpt = -1: isOpt = -1
        IF isOpt AND LEN(d) = 0 THEN
            IF RIGHT$(s, 1) = "$" OR INSTR(UCASE$(s), " AS STRING") THEN d = CHR$(34) + CHR$(34) ELSE d = "0"
        END IF
        IF k > 1 THEN newParams = newParams + ", "
        newParams = newParams + s
        entry = entry + CHR$(9) + d
    NEXT
    IF anyOpt = 0 THEN EXIT FUNCTION
    'replace an older entry of the same procedure
    k = INSTR(syntaxExtOptional$, CHR$(10) + SyntaxExt_OptKey$(procName) + CHR$(9))
    IF k THEN
        i = INSTR(k + 1, syntaxExtOptional$, CHR$(10))
        syntaxExtOptional$ = LEFT$(syntaxExtOptional$, k) + MID$(syntaxExtOptional$, i + 1)
    END IF
    IF LEN(syntaxExtOptional$) = 0 THEN syntaxExtOptional$ = CHR$(10)
    syntaxExtOptional$ = syntaxExtOptional$ + entry + CHR$(10)
    SyntaxExt_OptHeader$ = LEFT$(codeText$, p1) + newParams + MID$(codeText$, p2)
END FUNCTION

FUNCTION SyntaxExt_OptDefaults$ (entry$, given AS LONG)
    'defaults for the parameters after the given ones ("" when one of them is required)
    REDIM f(1 TO 16) AS STRING
    DIM n AS LONG
    DIM total AS LONG
    DIM k AS LONG
    DIM r AS STRING

    n = SyntaxExt_SplitTop&(entry$, CHR$(9), f())
    total = VAL(f(2))
    IF given >= total THEN EXIT FUNCTION
    FOR k = given + 1 TO total
        IF k + 2 > n THEN EXIT FUNCTION
        IF LEN(f(k + 2)) = 0 THEN EXIT FUNCTION
        IF LEN(r) THEN r = r + ", "
        r = r + f(k + 2)
    NEXT
    SyntaxExt_OptDefaults$ = r
END FUNCTION

FUNCTION SyntaxExt_OptCalls$ (codeText$)
    'append missing defaults to calls of procedures with OPTIONAL parameters
    DIM i AS LONG
    DIM j AS LONG
    DIM k AS LONG
    DIM depth AS LONG
    DIM c AS STRING
    DIM word AS STRING
    DIM entry AS STRING
    DIM result AS STRING
    DIM stmtStart AS INTEGER
    DIM inner AS STRING
    DIM given AS LONG
    DIM defs AS STRING
    DIM e AS LONG
    REDIM parts(1 TO 16) AS STRING

    stmtStart = -1
    i = 1
    DO WHILE i <= LEN(codeText$)
        c = MID$(codeText$, i, 1)
        IF c = CHR$(34) THEN
            j = INSTR(i + 1, codeText$, CHR$(34))
            IF j = 0 THEN j = LEN(codeText$)
            result = result + MID$(codeText$, i, j - i + 1)
            i = j + 1
            stmtStart = 0
            GOTO optnext
        END IF
        IF c = ":" THEN result = result + c: i = i + 1: stmtStart = -1: GOTO optnext
        IF c = " " OR c = CHR$(9) THEN result = result + c: i = i + 1: GOTO optnext
        IF ClassSyntax_IsIdentifierChar%(c) = 0 OR (c >= "0" AND c <= "9") THEN result = result + c: i = i + 1: stmtStart = 0: GOTO optnext
        'identifier (with dots) and type suffix
        j = i
        DO WHILE j <= LEN(codeText$)
            c = MID$(codeText$, j, 1)
            IF ClassSyntax_IsIdentifierChar%(c) = 0 AND c <> "." THEN EXIT DO
            j = j + 1
        LOOP
        DO WHILE j <= LEN(codeText$)
            IF INSTR("%&!#$~`", MID$(codeText$, j, 1)) = 0 THEN EXIT DO
            j = j + 1
        LOOP
        word = MID$(codeText$, i, j - i)
        result = result + word
        i = j
        IF UCASE$(word) = "THEN" OR UCASE$(word) = "ELSE" THEN stmtStart = -1: GOTO optnext
        IF UCASE$(word) = "CALL" THEN GOTO optnext
        k = INSTR(syntaxExtOptional$, CHR$(10) + SyntaxExt_OptKey$(word) + CHR$(9))
        IF k = 0 THEN stmtStart = 0: GOTO optnext
        entry = MID$(syntaxExtOptional$, k + 1, INSTR(k + 1, syntaxExtOptional$, CHR$(10)) - k - 1)
        'next non-blank character
        j = i
        DO WHILE MID$(codeText$, j, 1) = " "
            j = j + 1
        LOOP
        c = MID$(codeText$, j, 1)
        IF c = "(" THEN
            depth = 0
            FOR e = j TO LEN(codeText$)
                IF MID$(codeText$, e, 1) = "(" THEN depth = depth + 1
                IF MID$(codeText$, e, 1) = ")" THEN
                    depth = depth - 1
                    IF depth = 0 THEN EXIT FOR
                END IF
            NEXT
            inner = MID$(codeText$, j + 1, e - j - 1)
            given = 0
            IF LEN(LTRIM$(inner)) THEN given = SyntaxExt_SplitTop&(inner, ",", parts())
            defs = SyntaxExt_OptDefaults$(entry, given)
            IF LEN(defs) THEN
                IF given THEN defs = ", " + defs
                result = result + MID$(codeText$, i, e - i) + defs
                i = e
            END IF
            stmtStart = 0
            GOTO optnext
        END IF
        IF c = "=" THEN stmtStart = 0: GOTO optnext 'function return value / assignment
        IF stmtStart THEN
            'statement call: name args [: ...]
            e = j
            depth = 0
            DO WHILE e <= LEN(codeText$)
                c = MID$(codeText$, e, 1)
                IF c = CHR$(34) THEN
                    k = INSTR(e + 1, codeText$, CHR$(34))
                    IF k = 0 THEN e = LEN(codeText$) + 1: EXIT DO
                    e = k
                ELSEIF c = "(" THEN
                    depth = depth + 1
                ELSEIF c = ")" THEN
                    depth = depth - 1
                ELSEIF c = ":" AND depth = 0 THEN
                    EXIT DO
                ELSEIF depth = 0 AND UCASE$(MID$(codeText$, e, 6)) = " ELSE " THEN
                    EXIT DO
                END IF
                e = e + 1
            LOOP
            inner = RTRIM$(MID$(codeText$, j, e - j))
            given = 0
            IF LEN(LTRIM$(inner)) THEN given = SyntaxExt_SplitTop&(inner, ",", parts())
            defs = SyntaxExt_OptDefaults$(entry, given)
            IF LEN(defs) THEN
                IF given THEN defs = ", " + defs ELSE defs = " " + defs
                result = result + MID$(codeText$, i, j - i) + inner + defs
                i = j + LEN(inner)
            END IF
            stmtStart = 0
            GOTO optnext
        END IF
        'function used without arguments inside an expression
        defs = SyntaxExt_OptDefaults$(entry, 0)
        IF LEN(defs) THEN result = result + "(" + defs + ")"
        stmtStart = 0
        optnext:
    LOOP
    SyntaxExt_OptCalls$ = result
END FUNCTION

FUNCTION SyntaxExt_Optional$ (rawLine$)
    DIM codeText AS STRING
    DIM commentText AS STRING
    DIM upper AS STRING

    SyntaxExt_Optional$ = rawLine$
    codeText = SyntaxExt_CodePart$(rawLine$, commentText)
    upper = UCASE$(LTRIM$(codeText))
    IF LEFT$(upper, 8) = "DECLARE " THEN upper = LTRIM$(MID$(upper, 9))
    IF LEFT$(upper, 4) = "SUB " OR LEFT$(upper, 9) = "FUNCTION " THEN
        IF INSTR(upper, "OPTIONAL") OR INSTR(upper, "=") THEN SyntaxExt_Optional$ = SyntaxExt_OptHeader$(codeText) + commentText
        EXIT FUNCTION
    END IF
    IF LEN(syntaxExtOptional$) THEN SyntaxExt_Optional$ = SyntaxExt_OptCalls$(codeText) + commentText
END FUNCTION