FUNCTION TopLevelRuntime_ShouldCapture% (trimmedLine$, upperLine$)
    IF LEN(trimmedLine$) = 0 THEN EXIT FUNCTION
    IF LEFT$(trimmedLine$, 1) = "'" THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 4) = "REM " THEN EXIT FUNCTION
    IF LEFT$(trimmedLine$, 1) = "$" THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 7) = "OPTION " THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 6) = "CONST " THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 11) = "DIM SHARED " THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 5) = "DATA " THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 7) = "COMMON " THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 6) = "DEFINT" THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 6) = "DEFLNG" THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 6) = "DEFSNG" THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 6) = "DEFDBL" THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 6) = "DEFSTR" THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 7) = "_DEFINE" THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 6) = "DEFINE" THEN EXIT FUNCTION
    TopLevelRuntime_ShouldCapture = -1
END FUNCTION

FUNCTION TopLevelRuntime_IsKeyword% (upperLine$, keyword$)
    'keyword alone, or followed by a space or a comment
    IF LEFT$(upperLine$, LEN(keyword$)) <> keyword$ THEN EXIT FUNCTION
    IF LEN(upperLine$) = LEN(keyword$) THEN TopLevelRuntime_IsKeyword = -1: EXIT FUNCTION
    SELECT CASE MID$(upperLine$, LEN(keyword$) + 1, 1)
        CASE " ", "'", CHR$(9): TopLevelRuntime_IsKeyword = -1
    END SELECT
END FUNCTION

FUNCTION TopLevelRuntime_IsDeclareLibrary% (upperLine$)
    DIM rest AS STRING

    IF LEFT$(upperLine$, 8) <> "DECLARE " THEN EXIT FUNCTION
    rest = LTRIM$(MID$(upperLine$, 9))
    IF LEFT$(rest, 8) = "DYNAMIC " THEN rest = LTRIM$(MID$(rest, 9))
    IF LEFT$(rest, 11) = "CUSTOMTYPE " THEN rest = LTRIM$(MID$(rest, 12))
    IF LEFT$(rest, 7) = "STATIC " THEN rest = LTRIM$(MID$(rest, 8))
    IF rest = "LIBRARY" OR LEFT$(rest, 8) = "LIBRARY " OR LEFT$(rest, 8) = "LIBRARY'" THEN TopLevelRuntime_IsDeclareLibrary = -1
END FUNCTION

SUB TopLevelRuntime_Finalize
    DIM remainingLines AS STRING
    DIM nextBreak AS LONG
    DIM nextLine AS STRING

    IF topLevelRuntimeFinalized THEN EXIT SUB
    topLevelRuntimeFinalized = -1
    ClassSyntax_QueueDeferredLine "SUB QBNEX_TOPLEVEL_RUNTIME0 ()"
    remainingLines = topLevelRuntimeLines
    DO WHILE LEN(remainingLines)
        nextBreak = INSTR(remainingLines, CHR$(10))
        IF nextBreak = 0 THEN
            nextLine = remainingLines
            remainingLines = ""
        ELSE
            nextLine = LEFT$(remainingLines, nextBreak - 1)
            remainingLines = MID$(remainingLines, nextBreak + 1)
        END IF
        ClassSyntax_QueueDeferredLine nextLine
    LOOP
    ClassSyntax_QueueDeferredLine "END SUB"
END SUB

FUNCTION TopLevelRuntime_IsLabelLine% (trimmedLine$)
    'a line number, or "name:" at the start of the line
    DIM c AS LONG
    DIM i AS LONG
    DIM p AS LONG

    IF LEN(trimmedLine$) = 0 THEN EXIT FUNCTION
    c = ASC(trimmedLine$)
    IF c >= 48 AND c <= 57 THEN TopLevelRuntime_IsLabelLine = -1: EXIT FUNCTION
    p = INSTR(trimmedLine$, ":")
    IF p < 2 THEN EXIT FUNCTION
    FOR i = 1 TO p - 1
        c = ASC(MID$(trimmedLine$, i, 1))
        IF (c < 65 OR c > 90) AND (c < 97 OR c > 122) AND (c < 48 OR c > 57) AND c <> 95 AND c <> 46 THEN EXIT FUNCTION
    NEXT
    TopLevelRuntime_IsLabelLine = -1
END FUNCTION

FUNCTION TopLevelRuntime_MarkLine$ (lineText$)
    'A line moved in front of the first SUB carries where it came from:
    'CHR$(1) number CHR$(2) text, where number is the main-file line (for a line of an
    'included file: the line of the top-level $INCLUDE, like every included line). A line
    'of an included file also carries CHR$(3) line-in-file CHR$(3) file before CHR$(2), so
    'that an error in it still names the included file and its line.
    IF inclevel = 0 THEN
        TopLevelRuntime_MarkLine$ = CHR$(1) + LTRIM$(STR$(linenumber + 1)) + CHR$(2) + lineText$
    ELSE
        TopLevelRuntime_MarkLine$ = CHR$(1) + LTRIM$(STR$(linenumber)) + CHR$(3) + LTRIM$(STR$(inclinenumber(inclevel))) + CHR$(3) + incname$(inclevel) + CHR$(2) + lineText$
    END IF
END FUNCTION

FUNCTION TopLevelRuntime_ProcessLine$ (rawLine$)
    DIM trimmedLine AS STRING
    DIM upperLine AS STRING
    DIM lineText AS STRING
    DIM markEnd AS LONG
    DIM markHead AS STRING
    DIM fieldEnd AS LONG
    DIM fileStart AS LONG

    'only a moved line sets these (TopLevelRuntime_MarkLine$): its included file and line, or
    'that it comes from the main file
    topLevelHoistSrcFile = "": topLevelHoistSrcLine = 0: topLevelHoistFromMain = 0

    IF rawLine$ = CHR$(13) THEN
        TopLevelRuntime_ProcessLine$ = rawLine$
        EXIT FUNCTION
    END IF

    lineText = rawLine$
    'A line moved in front of the first SUB (TopLevelRuntime_MarkLine$). Fed at the main
    'level, the caller adds 1 to linenumber after this call; fed inside an included file
    '(the first SUB is there), the caller takes 1 away and adds it back.
    IF LEFT$(lineText, 1) = CHR$(1) THEN
        markEnd = INSTR(lineText, CHR$(2))
        IF markEnd > 2 THEN
            markHead = MID$(lineText, 2, markEnd - 2)
            lineText = MID$(lineText, markEnd + 1)
            fieldEnd = INSTR(markHead, CHR$(3))
            IF fieldEnd THEN
                fileStart = INSTR(fieldEnd + 1, markHead, CHR$(3))
                IF fileStart THEN
                    topLevelHoistSrcLine = VAL(MID$(markHead, fieldEnd + 1, fileStart - fieldEnd - 1))
                    topLevelHoistSrcFile = MID$(markHead, fileStart + 1)
                END IF
                markHead = LEFT$(markHead, fieldEnd - 1)
            ELSE
                topLevelHoistFromMain = -1
            END IF
            IF inclevel THEN linenumber = VAL(markHead) ELSE linenumber = VAL(markHead) - 1
            classSyntaxQueuedLine = 0 'the caller counts this line: it carries its own number
        END IF
    END IF

    trimmedLine = LTRIM$(RTRIM$(lineText))
    upperLine = UCASE$(trimmedLine)

    IF TopLevelRuntime_IsKeyword%(upperLine, "END SUB") OR TopLevelRuntime_IsKeyword%(upperLine, "END FUNCTION") THEN
        IF topLevelRuntimeProcDepth > 0 THEN topLevelRuntimeProcDepth = topLevelRuntimeProcDepth - 1
        TopLevelRuntime_ProcessLine$ = lineText
        EXIT FUNCTION
    END IF
    IF TopLevelRuntime_IsKeyword%(upperLine, "END TYPE") THEN
        IF topLevelRuntimeTypeDepth > 0 THEN topLevelRuntimeTypeDepth = topLevelRuntimeTypeDepth - 1
        TopLevelRuntime_ProcessLine$ = lineText
        EXIT FUNCTION
    END IF
    IF TopLevelRuntime_IsKeyword%(upperLine, "END DECLARE") THEN
        IF topLevelRuntimeDeclareDepth > 0 THEN topLevelRuntimeDeclareDepth = topLevelRuntimeDeclareDepth - 1
        TopLevelRuntime_ProcessLine$ = lineText
        EXIT FUNCTION
    END IF

    IF TopLevelRuntime_IsDeclareLibrary%(upperLine) THEN
        topLevelRuntimeDeclareDepth = topLevelRuntimeDeclareDepth + 1
        TopLevelRuntime_ProcessLine$ = lineText
        EXIT FUNCTION
    END IF
    'SUB/FUNCTION lines inside DECLARE LIBRARY are prototypes, not procedure bodies
    IF topLevelRuntimeDeclareDepth THEN
        TopLevelRuntime_ProcessLine$ = lineText
        EXIT FUNCTION
    END IF
    IF LEFT$(upperLine, 5) = "TYPE " THEN
        topLevelRuntimeTypeDepth = topLevelRuntimeTypeDepth + 1
        TopLevelRuntime_ProcessLine$ = lineText
        EXIT FUNCTION
    END IF
    IF LEFT$(upperLine, 4) = "SUB " OR LEFT$(upperLine, 9) = "FUNCTION " THEN
        IF topLevelRuntimeSeenProc = 0 AND topLevelRuntimeProcDepth = 0 THEN
            topLevelHoistSubSeen = -1
            'Feed the module code found after the SUBs (marked lines) in front of this first
            'SUB, then this line again. Inside an included file it lands at that include
            'level, after what the file declares before its first SUB (DIM SHARED, TYPE...).
            IF topLevelHoistReady <> 0 AND topLevelHoistInjected = 0 AND LEN(topLevelHoist) <> 0 THEN
                topLevelHoistInjected = -1
                ClassSyntax_PushFrontLine TopLevelRuntime_MarkLine$(lineText)
                classSyntaxQueue$ = topLevelHoist + CHR$(10) + classSyntaxQueue$
                TopLevelRuntime_ProcessLine$ = TopLevelRuntime_ProcessLine$(ClassSyntax_DequeueLine$)
                EXIT FUNCTION
            END IF
        END IF
        topLevelRuntimeProcDepth = topLevelRuntimeProcDepth + 1
        topLevelRuntimeSeenProc = -1
        TopLevelRuntime_ProcessLine$ = lineText
        EXIT FUNCTION
    END IF

    IF topLevelRuntimeProcDepth OR topLevelRuntimeTypeDepth OR topLevelRuntimeDeclareDepth THEN
        TopLevelRuntime_ProcessLine$ = lineText
        EXIT FUNCTION
    END IF

    'Code before the first SUB/FUNCTION is ordinary main-module code and stays in place
    '(only code that follows a procedure is moved: into the runtime SUB, or in front of the first SUB when it has labels)
    IF topLevelRuntimeSeenProc = 0 THEN
        TopLevelRuntime_ProcessLine$ = lineText
        EXIT FUNCTION
    END IF

    IF TopLevelRuntime_ShouldCapture%(trimmedLine, upperLine) THEN
        'Already fed in front of the first SUB (lines of an included file too: they are
        'fed at the main level and keep their file and line for error messages)
        IF topLevelHoistInjected THEN TopLevelRuntime_ProcessLine$ = "": EXIT FUNCTION
        'Module code after a SUB is always moved in front of the first SUB: it is module
        'code, with the module's variables and its real line numbers. (Without a label it
        'used to go into a SUB of its own, where x of the module was a new, empty x.)
        topLevelHoistWanted = -1
        ClassSyntax_AppendLine topLevelHoistScan, TopLevelRuntime_MarkLine$(lineText)
        topLevelRuntimeCallInjected = -1
        ClassSyntax_AppendLine topLevelRuntimeLines, lineText
        TopLevelRuntime_ProcessLine$ = ""
        EXIT FUNCTION
    END IF

    TopLevelRuntime_ProcessLine$ = lineText
END FUNCTION

FUNCTION ClassSyntax_IsIdentifierChar% (c$)
    DIM c AS LONG

    IF LEN(c$) = 0 THEN EXIT FUNCTION
    c = ASC(c$)
    IF c >= 48 AND c <= 57 THEN ClassSyntax_IsIdentifierChar = -1: EXIT FUNCTION
    IF c >= 65 AND c <= 90 THEN ClassSyntax_IsIdentifierChar = -1: EXIT FUNCTION
    IF c >= 97 AND c <= 122 THEN ClassSyntax_IsIdentifierChar = -1: EXIT FUNCTION
    IF c$ = "_" THEN ClassSyntax_IsIdentifierChar = -1
END FUNCTION

FUNCTION ClassSyntax_MatchTextAt% (text$, position AS LONG, pattern$)
    IF position < 1 THEN EXIT FUNCTION
    IF LEN(pattern$) = 0 THEN EXIT FUNCTION
    IF position + LEN(pattern$) - 1 > LEN(text$) THEN EXIT FUNCTION
    IF UCASE$(MID$(text$, position, LEN(pattern$))) = UCASE$(pattern$) THEN ClassSyntax_MatchTextAt = -1
END FUNCTION

FUNCTION ClassSyntax_MatchTokenAt% (text$, position AS LONG, token$)
    DIM beforeChar$
    DIM afterChar$

    IF ClassSyntax_MatchTextAt%(text$, position, token$) = 0 THEN EXIT FUNCTION

    IF position > 1 THEN
        beforeChar$ = MID$(text$, position - 1, 1)
        IF ClassSyntax_IsIdentifierChar%(beforeChar$) THEN EXIT FUNCTION
    END IF

    IF position + LEN(token$) <= LEN(text$) THEN
        afterChar$ = MID$(text$, position + LEN(token$), 1)
        IF ClassSyntax_IsIdentifierChar%(afterChar$) THEN EXIT FUNCTION
    END IF

    ClassSyntax_MatchTokenAt = -1
END FUNCTION

FUNCTION ClassSyntax_FirstToken$ (text$)
    DIM i AS LONG
    DIM c$

    text$ = LTRIM$(RTRIM$(text$))
    FOR i = 1 TO LEN(text$)
        c$ = MID$(text$, i, 1)
        IF c$ = " " OR c$ = CHR$(9) THEN
            ClassSyntax_FirstToken$ = LEFT$(text$, i - 1)
            EXIT FUNCTION
        END IF
    NEXT
    ClassSyntax_FirstToken$ = text$
END FUNCTION

FUNCTION ClassSyntax_FindMatchingOpenParen& (text$, closePos AS LONG)
    DIM i AS LONG
    DIM depth AS LONG
    DIM currentChar AS STRING

    FOR i = closePos TO 1 STEP -1
        currentChar = MID$(text$, i, 1)
        IF currentChar = ")" THEN depth = depth + 1
        IF currentChar = "(" THEN
            depth = depth - 1
            IF depth = 0 THEN
                ClassSyntax_FindMatchingOpenParen = i
                EXIT FUNCTION
            END IF
        END IF
    NEXT
END FUNCTION

FUNCTION ClassSyntax_FindMatchingCloseParen& (text$, openPos AS LONG)
    DIM i AS LONG
    DIM depth AS LONG
    DIM currentChar AS STRING

    FOR i = openPos TO LEN(text$)
        currentChar = MID$(text$, i, 1)
        IF currentChar = "(" THEN depth = depth + 1
        IF currentChar = ")" THEN
            depth = depth - 1
            IF depth = 0 THEN
                ClassSyntax_FindMatchingCloseParen = i
                EXIT FUNCTION
            END IF
        END IF
    NEXT
END FUNCTION

FUNCTION ClassSyntax_IsTypeSuffixChar% (c$)
    IF c$ = "$" OR c$ = "%" OR c$ = "&" OR c$ = "!" OR c$ = "#" THEN ClassSyntax_IsTypeSuffixChar = -1
END FUNCTION

FUNCTION ClassSyntax_SkipLeftSpaces& (text$, position AS LONG)
    DO WHILE position > 0
        IF MID$(text$, position, 1) <> " " AND MID$(text$, position, 1) <> CHR$(9) THEN EXIT DO
        position = position - 1
    LOOP
    ClassSyntax_SkipLeftSpaces = position
END FUNCTION

FUNCTION ClassSyntax_ConsumeDispatchSegmentStart& (text$, endPos AS LONG)
    DIM startPos AS LONG
    DIM currentChar AS STRING
    DIM openPos AS LONG
    DIM previousPos AS LONG

    endPos = ClassSyntax_SkipLeftSpaces&(text$, endPos)
    IF endPos < 1 THEN EXIT FUNCTION

    currentChar = MID$(text$, endPos, 1)
    IF currentChar = ")" THEN
        openPos = ClassSyntax_FindMatchingOpenParen&(text$, endPos)
        IF openPos = 0 THEN EXIT FUNCTION
        startPos = openPos
        previousPos = ClassSyntax_SkipLeftSpaces&(text$, openPos - 1)
        IF previousPos > 0 AND previousPos = openPos - 1 THEN
            currentChar = MID$(text$, previousPos, 1)
            IF ClassSyntax_IsIdentifierChar%(currentChar) OR ClassSyntax_IsTypeSuffixChar%(currentChar) OR currentChar = ")" THEN
                startPos = ClassSyntax_ConsumeDispatchSegmentStart&(text$, previousPos)
            END IF
        END IF
        ClassSyntax_ConsumeDispatchSegmentStart = startPos
        EXIT FUNCTION
    END IF

    IF ClassSyntax_IsIdentifierChar%(currentChar) OR ClassSyntax_IsTypeSuffixChar%(currentChar) THEN
        startPos = endPos
        DO WHILE startPos > 1
            currentChar = MID$(text$, startPos - 1, 1)
            IF ClassSyntax_IsIdentifierChar%(currentChar) OR ClassSyntax_IsTypeSuffixChar%(currentChar) THEN
                startPos = startPos - 1
            ELSE
                EXIT DO
            END IF
        LOOP
        ClassSyntax_ConsumeDispatchSegmentStart = startPos
    END IF
END FUNCTION

FUNCTION ClassSyntax_FindDispatchObjectStart& (text$, endPos AS LONG)
    DIM startPos AS LONG
    DIM previousPos AS LONG

    startPos = ClassSyntax_ConsumeDispatchSegmentStart&(text$, endPos)
    IF startPos = 0 THEN EXIT FUNCTION

    DO
        previousPos = ClassSyntax_SkipLeftSpaces&(text$, startPos - 1)
        IF previousPos < 1 THEN EXIT DO
        IF MID$(text$, previousPos, 1) <> "." THEN EXIT DO
        previousPos = ClassSyntax_SkipLeftSpaces&(text$, previousPos - 1)
        startPos = ClassSyntax_ConsumeDispatchSegmentStart&(text$, previousPos)
        IF startPos = 0 THEN EXIT DO
    LOOP

    ClassSyntax_FindDispatchObjectStart = startPos
END FUNCTION

FUNCTION ClassSyntax_IsWrappedExpression% (text$)
    DIM closePos AS LONG

    text$ = LTRIM$(RTRIM$(text$))
    IF LEN(text$) < 2 THEN EXIT FUNCTION
    IF LEFT$(text$, 1) <> "(" OR RIGHT$(text$, 1) <> ")" THEN EXIT FUNCTION
    closePos = ClassSyntax_FindMatchingCloseParen&(text$, 1)
    IF closePos = LEN(text$) THEN ClassSyntax_IsWrappedExpression = -1
END FUNCTION

FUNCTION ClassSyntax_FindLastTopLevelDot& (text$)
    DIM i AS LONG
    DIM depth AS LONG
    DIM inString AS LONG
    DIM currentChar AS STRING

    FOR i = 1 TO LEN(text$)
        currentChar = MID$(text$, i, 1)
        IF currentChar = CHR$(34) THEN
            IF inString THEN inString = 0 ELSE inString = -1
        ELSEIF inString = 0 THEN
            IF currentChar = "(" THEN depth = depth + 1
            IF currentChar = ")" THEN depth = depth - 1
            IF currentChar = "." AND depth = 0 THEN ClassSyntax_FindLastTopLevelDot = i
        END IF
    NEXT
END FUNCTION

FUNCTION ClassSyntax_FindUDTIndex& (typeName$)
    DIM i AS LONG
    DIM lookupName AS STRING

    lookupName = UCASE$(RTRIM$(typeName$))
    FOR i = 1 TO lasttype
        IF UCASE$(RTRIM$(udtxname(i))) = lookupName THEN
            ClassSyntax_FindUDTIndex = i
            EXIT FUNCTION
        END IF
    NEXT
END FUNCTION

FUNCTION ClassSyntax_FindFieldType$ (typeName$, fieldName$)
    DIM typeIndex AS LONG
    DIM elementIndex AS LONG
    DIM lookupField AS STRING

    typeIndex = ClassSyntax_FindUDTIndex&(typeName$)
    IF typeIndex = 0 THEN EXIT FUNCTION

    lookupField = UCASE$(RTRIM$(fieldName$))
    elementIndex = udtxnext(typeIndex)
    DO WHILE elementIndex
        IF UCASE$(RTRIM$(udtename(elementIndex))) = lookupField THEN
            IF udtetype(elementIndex) AND ISUDT THEN
                ClassSyntax_FindFieldType$ = RTRIM$(udtxname(udtetype(elementIndex) AND 511))
            END IF
            EXIT FUNCTION
        END IF
        elementIndex = udtenext(elementIndex)
    LOOP
END FUNCTION

FUNCTION ClassSyntax_ResolveDispatchType$ (exprText$, selfType$)
    DIM trimmedExpr AS STRING
    DIM dotPos AS LONG
    DIM leftExpr AS STRING
    DIM memberExpr AS STRING
    DIM memberName AS STRING
    DIM baseType AS STRING
    DIM openPos AS LONG
    DIM prefixExpr AS STRING

    trimmedExpr = LTRIM$(RTRIM$(exprText$))
    DO WHILE ClassSyntax_IsWrappedExpression%(trimmedExpr)
        trimmedExpr = LTRIM$(RTRIM$(MID$(trimmedExpr, 2, LEN(trimmedExpr) - 2)))
    LOOP
    IF LEN(trimmedExpr) = 0 THEN EXIT FUNCTION

    dotPos = ClassSyntax_FindLastTopLevelDot&(trimmedExpr)
    IF dotPos THEN
        leftExpr = LEFT$(trimmedExpr, dotPos - 1)
        memberExpr = LTRIM$(RTRIM$(MID$(trimmedExpr, dotPos + 1)))
        memberName = ClassSyntax_FirstIdentifier$(memberExpr)
        baseType = ClassSyntax_ResolveDispatchType$(leftExpr, selfType$)
        IF LEN(baseType) AND LEN(memberName) THEN
            ClassSyntax_ResolveDispatchType$ = ClassSyntax_FindFieldType$(baseType, memberName)
        END IF
        EXIT FUNCTION
    END IF

    IF RIGHT$(trimmedExpr, 1) = ")" THEN
        openPos = ClassSyntax_FindMatchingOpenParen&(trimmedExpr, LEN(trimmedExpr))
        IF openPos > 1 THEN
            prefixExpr = RTRIM$(LEFT$(trimmedExpr, openPos - 1))
            IF LEN(prefixExpr) THEN
                ClassSyntax_ResolveDispatchType$ = ClassSyntax_ResolveDispatchType$(prefixExpr, selfType$)
                EXIT FUNCTION
            END IF
        END IF
    END IF

    IF UCASE$(trimmedExpr) = "SELF" OR UCASE$(trimmedExpr) = "THIS" OR UCASE$(trimmedExpr) = "ME" THEN
        ClassSyntax_ResolveDispatchType$ = selfType$
        EXIT FUNCTION
    END IF

    ClassSyntax_ResolveDispatchType$ = ClassSyntax_LookupVarType$(trimmedExpr)
END FUNCTION

FUNCTION ClassSyntax_SafeIdentifier$ (text$)
    DIM i AS LONG
    DIM c$
    DIM c AS LONG
    DIM resultText AS STRING

    FOR i = 1 TO LEN(text$)
        c$ = MID$(text$, i, 1)
        c = ASC(c$)
        IF c >= 48 AND c <= 57 THEN resultText = resultText + c$
        IF c >= 65 AND c <= 90 THEN resultText = resultText + c$
        IF c >= 97 AND c <= 122 THEN resultText = resultText + c$
        IF c$ = "_" THEN resultText = resultText + c$
    NEXT

    IF LEN(resultText) = 0 THEN resultText = "ClassValue"
    c = ASC(LEFT$(resultText, 1))
    IF c >= 48 AND c <= 57 THEN resultText = "_" + resultText
    ClassSyntax_SafeIdentifier$ = resultText
END FUNCTION

FUNCTION ClassSyntax_TypeSuffix$ (name$)
    DIM suffix$

    IF LEN(name$) = 0 THEN EXIT FUNCTION
    suffix$ = RIGHT$(RTRIM$(name$), 1)
    IF suffix$ = "$" OR suffix$ = "%" OR suffix$ = "&" OR suffix$ = "!" OR suffix$ = "#" THEN
        ClassSyntax_TypeSuffix$ = suffix$
    END IF
END FUNCTION

FUNCTION ClassSyntax_RemoveTypeSuffix$ (name$)
    DIM suffix$

    name$ = RTRIM$(name$)
    suffix$ = ClassSyntax_TypeSuffix$(name$)
    IF LEN(suffix$) THEN
        ClassSyntax_RemoveTypeSuffix$ = LEFT$(name$, LEN(name$) - 1)
    ELSE
        ClassSyntax_RemoveTypeSuffix$ = name$
    END IF
END FUNCTION

FUNCTION ClassSyntax_SelfParams$ (paramsClause$, className$)
    DIM innerText AS STRING

    innerText = LTRIM$(RTRIM$(paramsClause$))
    IF LEFT$(innerText, 1) = "(" AND RIGHT$(innerText, 1) = ")" THEN
        innerText = MID$(innerText, 2, LEN(innerText) - 2)
    ELSE
        innerText = ""
    END IF
    innerText = LTRIM$(RTRIM$(innerText))

    IF LEN(innerText) THEN
        ClassSyntax_SelfParams$ = "(self AS " + className$ + ", " + innerText + ")"
    ELSE
        ClassSyntax_SelfParams$ = "(self AS " + className$ + ")"
    END IF
END FUNCTION

FUNCTION ClassSyntax_TransformBody$ (sourceLine$)
    DIM outputText AS STRING
    DIM inString AS LONG
    DIM i AS LONG
    DIM currentChar$

    DO WHILE i < LEN(sourceLine$)
        i = i + 1
        currentChar$ = MID$(sourceLine$, i, 1)

        IF currentChar$ = CHR$(34) THEN
            outputText = outputText + currentChar$
            IF inString THEN
                inString = 0
            ELSE
                inString = -1
            END IF
            GOTO ClassSyntax_TransformBody_NextChar
        END IF

        IF inString = 0 THEN
            IF currentChar$ = "'" THEN
                outputText = outputText + MID$(sourceLine$, i)
                EXIT DO
            END IF

            IF ClassSyntax_MatchTextAt%(sourceLine$, i, "ME.") THEN
                outputText = outputText + "self."
                i = i + 2
                GOTO ClassSyntax_TransformBody_NextChar
            END IF

            IF ClassSyntax_MatchTextAt%(sourceLine$, i, "THIS.") THEN
                outputText = outputText + "self."
                i = i + 4
                GOTO ClassSyntax_TransformBody_NextChar
            END IF

            IF classSyntaxMethodKind$ = "FUNCTION" THEN
                IF ClassSyntax_MatchTokenAt%(sourceLine$, i, classSyntaxMethodAlias$) THEN
                    outputText = outputText + classSyntaxGeneratedName$
                    i = i + LEN(classSyntaxMethodAlias$) - 1
                    GOTO ClassSyntax_TransformBody_NextChar
                END IF
            END IF
        END IF

        outputText = outputText + currentChar$
        ClassSyntax_TransformBody_NextChar:
    LOOP

    ClassSyntax_TransformBody$ = ClassSyntax_RewriteDispatch$(outputText, classSyntaxClassName$)
END FUNCTION

SUB ClassSyntax_EmitHelper
    DIM helperName$
    DIM interfacesText$
    DIM interfaceName$
    DIM separatorPos AS LONG

    IF classSyntaxHelperEmitted THEN EXIT SUB

    helperName$ = "__QBNEX_CLASSID_" + ClassSyntax_SafeIdentifier$(classSyntaxClassName$) + "&"
    ClassSyntax_QueueDeferredLine "FUNCTION " + helperName$ + " ()"
    ClassSyntax_QueueDeferredLine "    STATIC cachedClassID AS LONG"
    ClassSyntax_QueueDeferredLine "    IF cachedClassID = 0 THEN"
    IF LEN(classSyntaxBaseName$) THEN
        ClassSyntax_QueueDeferredLine "        cachedClassID = QBNEX_EnsureClass(" + CHR$(34) + classSyntaxClassName$ + CHR$(34) + ", QBNEX_FindClass(" + CHR$(34) + classSyntaxBaseName$ + CHR$(34) + "))"
    ELSE
        ClassSyntax_QueueDeferredLine "        cachedClassID = QBNEX_EnsureClass(" + CHR$(34) + classSyntaxClassName$ + CHR$(34) + ", 0)"
    END IF

    interfacesText$ = classSyntaxInterfaces$
    DO WHILE LEN(interfacesText$)
        separatorPos = INSTR(interfacesText$, ",")
        IF separatorPos = 0 THEN
            interfaceName$ = LTRIM$(RTRIM$(interfacesText$))
            interfacesText$ = ""
        ELSE
            interfaceName$ = LTRIM$(RTRIM$(LEFT$(interfacesText$, separatorPos - 1)))
            interfacesText$ = MID$(interfacesText$, separatorPos + 1)
        END IF
        IF LEN(interfaceName$) THEN
            ClassSyntax_QueueDeferredLine "        QBNEX_RegisterInterface cachedClassID, " + CHR$(34) + interfaceName$ + CHR$(34)
        END IF
    LOOP

    ClassSyntax_QueueDeferredLine "    END IF"
    ClassSyntax_QueueDeferredLine "    " + helperName$ + " = cachedClassID"
    ClassSyntax_QueueDeferredLine "END FUNCTION"
    classSyntaxHelperEmitted = -1
END SUB

FUNCTION ClassSyntax_BuildMethodHeader$ (rawLine$)
    DIM trimmedLine$
    DIM upperLine$
    DIM signatureText$
    DIM nameText$
    DIM paramsClause$
    DIM returnClause$
    DIM openPos AS LONG
    DIM closePos AS LONG
    DIM generatedBase$
    DIM suffixText$
    DIM kindText$
    DIM headerLine AS STRING

    trimmedLine$ = LTRIM$(RTRIM$(rawLine$))
    upperLine$ = UCASE$(trimmedLine$)

    IF upperLine$ = "CONSTRUCTOR" OR LEFT$(upperLine$, 12) = "CONSTRUCTOR " THEN
        signatureText$ = LTRIM$(MID$(trimmedLine$, 12))
        nameText$ = "CONSTRUCTOR"
        kindText$ = "SUB"
        generatedBase$ = "__QBNEX_" + ClassSyntax_SafeIdentifier$(classSyntaxClassName$) + "_CTOR"
        openPos = INSTR(signatureText$, "(")
        IF openPos THEN
            closePos = INSTR(signatureText$, ")")
            IF closePos = 0 THEN closePos = LEN(signatureText$)
            paramsClause$ = MID$(signatureText$, openPos, closePos - openPos + 1)
        ELSE
            paramsClause$ = "()"
        END IF
    ELSE
        IF LEFT$(upperLine$, 4) = "SUB " THEN
            signatureText$ = LTRIM$(MID$(trimmedLine$, 4))
            kindText$ = "SUB"
        ELSEIF LEFT$(upperLine$, 9) = "FUNCTION " THEN
            signatureText$ = LTRIM$(MID$(trimmedLine$, 9))
            kindText$ = "FUNCTION"
        ELSE
            signatureText$ = LTRIM$(MID$(trimmedLine$, 7))
        END IF

        openPos = INSTR(signatureText$, "(")
        IF openPos THEN
            nameText$ = RTRIM$(LEFT$(signatureText$, openPos - 1))
            closePos = INSTR(signatureText$, ")")
            IF closePos = 0 THEN closePos = LEN(signatureText$)
            paramsClause$ = MID$(signatureText$, openPos, closePos - openPos + 1)
            returnClause$ = LTRIM$(MID$(signatureText$, closePos + 1))
        ELSE
            nameText$ = RTRIM$(signatureText$)
            paramsClause$ = "()"
        END IF

        IF kindText$ = "" THEN
            suffixText$ = ClassSyntax_TypeSuffix$(nameText$)
            IF LEN(returnClause$) OR LEN(suffixText$) THEN
                kindText$ = "FUNCTION"
            ELSE
                kindText$ = "SUB"
            END IF
        END IF

        generatedBase$ = "__QBNEX_" + ClassSyntax_SafeIdentifier$(classSyntaxClassName$) + "_" + ClassSyntax_SafeIdentifier$(ClassSyntax_RemoveTypeSuffix$(nameText$))
    END IF

    suffixText$ = ClassSyntax_TypeSuffix$(nameText$)
    classSyntaxMethodKind$ = kindText$
    classSyntaxMethodAlias$ = nameText$
    classSyntaxGeneratedName$ = generatedBase$
    IF kindText$ = "FUNCTION" AND LEN(suffixText$) THEN classSyntaxGeneratedName$ = classSyntaxGeneratedName$ + suffixText$

    headerLine = kindText$ + " " + classSyntaxGeneratedName$ + ClassSyntax_SelfParams$(paramsClause$, classSyntaxClassName$)
    IF kindText$ = "FUNCTION" AND LEN(returnClause$) THEN headerLine = headerLine + " " + returnClause$
    ClassSyntax_BuildMethodHeader$ = headerLine
END FUNCTION

FUNCTION ClassSyntax_ProcessLine$ (rawLine$)
    DIM trimmedLine$
    DIM upperLine$
    DIM headerText$
    DIM remainderText$
    DIM className$
    DIM savedQueue$
    DIM methodHeader$

    IF rawLine$ = CHR$(13) THEN
        ClassSyntax_ProcessLine$ = rawLine$
        EXIT FUNCTION
    END IF

    'Statement extensions (ENUM, FOR EACH, TRY) lower to plain lines first (syntax_ext.bas)
    IF classSyntaxActive = 0 THEN
        syntaxExtLine$ = SyntaxExt_ProcessLine$(rawLine$)
        IF syntaxExtLine$ <> rawLine$ THEN
            'a rewritten SUB/FUNCTION header (OPTIONAL parameters) still goes through the class scope tracking
            IF LEFT$(UCASE$(LTRIM$(syntaxExtLine$)), 4) = "SUB " OR LEFT$(UCASE$(LTRIM$(syntaxExtLine$)), 9) = "FUNCTION " THEN
                rawLine$ = syntaxExtLine$
            ELSE
                ClassSyntax_ProcessLine$ = syntaxExtLine$
                EXIT FUNCTION
            END IF
        END IF
    END IF

    trimmedLine$ = LTRIM$(RTRIM$(rawLine$))
    upperLine$ = UCASE$(trimmedLine$)

    IF classSyntaxActive = 0 THEN
        IF upperLine$ = "END SUB" OR upperLine$ = "END FUNCTION" THEN
            ClassSyntax_ExitScope
            ClassSyntax_ProcessLine$ = rawLine$
            EXIT FUNCTION
        END IF

        IF LEFT$(upperLine$, 4) = "SUB " OR LEFT$(upperLine$, 9) = "FUNCTION " THEN
            ClassSyntax_RegisterProcedureHeader rawLine$
            ClassSyntax_ProcessLine$ = rawLine$
            EXIT FUNCTION
        END IF

        ClassSyntax_RegisterDeclarationLine rawLine$

        IF LEFT$(upperLine$, 6) <> "CLASS " THEN
            ClassSyntax_ProcessLine$ = ClassSyntax_RewriteDispatch$(rawLine$, "")
            EXIT FUNCTION
        END IF

        headerText$ = LTRIM$(MID$(trimmedLine$, 6))
        className$ = ClassSyntax_FirstToken$(headerText$)
        IF LEN(className$) = 0 THEN
            Give_Error "Expected class name after CLASS"
            ClassSyntax_ProcessLine$ = rawLine$
            EXIT FUNCTION
        END IF

        ClassSyntax_Reset
        classSyntaxActive = -1
        classSyntaxTypeOpen = -1
        classSyntaxClassName$ = className$

        remainderText$ = LTRIM$(MID$(headerText$, LEN(className$) + 1))
        IF LEN(remainderText$) THEN
            IF LEFT$(UCASE$(remainderText$), 8) = "EXTENDS " THEN
                remainderText$ = LTRIM$(MID$(remainderText$, 9))
                classSyntaxBaseName$ = ClassSyntax_FirstToken$(remainderText$)
                remainderText$ = LTRIM$(MID$(remainderText$, LEN(classSyntaxBaseName$) + 1))
            END IF
            IF LEFT$(UCASE$(remainderText$), 11) = "IMPLEMENTS " THEN
                classSyntaxInterfaces$ = LTRIM$(MID$(remainderText$, 12))
            END IF
        END IF

        ClassSyntax_QueueLine "TYPE " + classSyntaxClassName$
        ClassSyntax_QueueLine "    Header AS QBNex_ObjectHeader"
        IF LEN(classSyntaxBaseName$) THEN
            IF ClassSyntax_FindRegistryClass&(classSyntaxBaseName$) = 0 THEN
                Give_Error "Base class must be declared before derived class"
                EXIT FUNCTION
            END IF
            ClassSyntax_QueueFieldLines ClassSyntax_FlatFieldLines$(classSyntaxBaseName$)
        END IF

        ClassSyntax_ProcessLine$ = ClassSyntax_DequeueLine$
        EXIT FUNCTION
    END IF

    IF classSyntaxInMethod THEN
        IF upperLine$ = "END METHOD" OR upperLine$ = "END CONSTRUCTOR" OR upperLine$ = "END SUB" OR upperLine$ = "END FUNCTION" THEN
            IF classSyntaxMethodKind$ = "FUNCTION" THEN
                ClassSyntax_QueueDeferredLine "END FUNCTION"
            ELSE
                ClassSyntax_QueueDeferredLine "END SUB"
            END IF
            classSyntaxInMethod = 0
            classSyntaxMethodKind$ = ""
            classSyntaxMethodAlias$ = ""
            classSyntaxGeneratedName$ = ""
            ClassSyntax_ProcessLine$ = ""
            EXIT FUNCTION
        END IF

        IF upperLine$ = "END CLASS" THEN
            Give_Error "Expected END METHOD before END CLASS"
            EXIT FUNCTION
        END IF

        ClassSyntax_QueueDeferredLine ClassSyntax_TransformBody$(rawLine$)
        ClassSyntax_ProcessLine$ = ""
        EXIT FUNCTION
    END IF

    IF upperLine$ = "END CLASS" THEN
        IF classSyntaxTypeOpen THEN
            ClassSyntax_QueueLine "END TYPE"
            classSyntaxTypeOpen = 0
        END IF
        ClassSyntax_RegisterCurrentClass
        IF Error_Happened THEN EXIT FUNCTION
        ClassSyntax_EmitHelper
        ClassSyntax_EmitInherited
        savedQueue$ = classSyntaxQueue$
        ClassSyntax_Reset
        classSyntaxQueue$ = savedQueue$
        ClassSyntax_ProcessLine$ = ClassSyntax_DequeueLine$
        EXIT FUNCTION
    END IF

    IF upperLine$ = "END METHOD" OR upperLine$ = "END CONSTRUCTOR" OR upperLine$ = "END SUB" OR upperLine$ = "END FUNCTION" THEN
        Give_Error "END METHOD without METHOD"
        EXIT FUNCTION
    END IF

    IF LEFT$(upperLine$, 7) = "METHOD " OR LEFT$(upperLine$, 4) = "SUB " OR LEFT$(upperLine$, 9) = "FUNCTION " OR upperLine$ = "CONSTRUCTOR" OR LEFT$(upperLine$, 12) = "CONSTRUCTOR " THEN
        IF classSyntaxTypeOpen THEN
            ClassSyntax_QueueLine "END TYPE"
            classSyntaxTypeOpen = 0
        END IF
        ClassSyntax_RegisterCurrentClass
        IF Error_Happened THEN EXIT FUNCTION
        ClassSyntax_EmitHelper
        methodHeader$ = ClassSyntax_BuildMethodHeader$(trimmedLine$)
        ClassSyntax_RegisterMethod classSyntaxClassName$, classSyntaxMethodAlias$, classSyntaxGeneratedName$
        ClassSyntax_RegisterHeader classSyntaxClassName$, classSyntaxMethodAlias$, methodHeader$
        'the class has this method of its own: the inherited one is not wanted
        classSyntaxInheritPending$ = ClassSyntax_WithoutKey$(classSyntaxInheritPending$, UCASE$(RTRIM$(classSyntaxMethodAlias$)))
        classSyntaxInMethod = -1
        ClassSyntax_QueueDeferredLine methodHeader$
        IF classSyntaxMethodAlias$ = "CONSTRUCTOR" THEN
            ClassSyntax_QueueDeferredLine "    self.Header.ClassID = __QBNEX_CLASSID_" + ClassSyntax_SafeIdentifier$(classSyntaxClassName$) + "&"
            ClassSyntax_QueueDeferredLine "    self.Header.Flags = 0"
        ELSE
            ClassSyntax_QueueDeferredLine "    IF self.Header.ClassID = 0 THEN"
            ClassSyntax_QueueDeferredLine "        self.Header.ClassID = __QBNEX_CLASSID_" + ClassSyntax_SafeIdentifier$(classSyntaxClassName$) + "&"
            ClassSyntax_QueueDeferredLine "        self.Header.Flags = 0"
            ClassSyntax_QueueDeferredLine "    END IF"
        END IF
        ClassSyntax_ProcessLine$ = ""
        EXIT FUNCTION
    END IF

    IF classSyntaxHelperEmitted THEN
        IF LEN(trimmedLine$) AND LEFT$(trimmedLine$, 1) <> "'" AND LEFT$(upperLine$, 4) <> "REM " THEN
            Give_Error "Class fields must be declared before methods"
            EXIT FUNCTION
        END IF
    END IF

    IF LEN(trimmedLine$) THEN
        IF LEFT$(trimmedLine$, 1) <> "'" AND LEFT$(upperLine$, 4) <> "REM " THEN
            ClassSyntax_AppendLine classSyntaxOwnFieldLines$, rawLine$
        END IF
    END IF

    ClassSyntax_ProcessLine$ = rawLine$
END FUNCTION
SUB ClassSyntax_Reset
    classSyntaxQueue$ = ""
    classSyntaxActive = 0
    classSyntaxTypeOpen = 0
    classSyntaxInMethod = 0
    classSyntaxHelperEmitted = 0
    classSyntaxClassName$ = ""
    classSyntaxBaseName$ = ""
    classSyntaxInterfaces$ = ""
    classSyntaxMethodKind$ = ""
    classSyntaxMethodAlias$ = ""
    classSyntaxGeneratedName$ = ""
    classSyntaxOwnFieldLines$ = ""
    classSyntaxInheritPending$ = ""
    classSyntaxInheritDone = 0
END SUB

SUB ClassSyntax_QueueLine (line$)
    IF LEN(classSyntaxQueue$) THEN classSyntaxQueue$ = classSyntaxQueue$ + CHR$(10)
    classSyntaxQueue$ = classSyntaxQueue$ + line$
END SUB

SUB ClassSyntax_PushFrontLine (line$)
    IF LEN(line$) = 0 THEN EXIT SUB
    IF LEN(classSyntaxQueue$) THEN
        classSyntaxQueue$ = line$ + CHR$(10) + classSyntaxQueue$
    ELSE
        classSyntaxQueue$ = line$
    END IF
END SUB

SUB ClassSyntax_QueueDeferredLine (line$)
    IF LEN(classSyntaxDeferredQueue$) THEN classSyntaxDeferredQueue$ = classSyntaxDeferredQueue$ + CHR$(10)
    classSyntaxDeferredQueue$ = classSyntaxDeferredQueue$ + line$
END SUB

SUB ClassSyntax_ClearRegistry
    DIM i AS LONG

    classSyntaxRegistryCount = 0
    FOR i = 1 TO 256
        classSyntaxRegistryName(i) = ""
        classSyntaxRegistryBase(i) = ""
        classSyntaxRegistryOwnFields(i) = ""
        classSyntaxRegistryFlatFields(i) = ""
        classSyntaxRegistryMethods(i) = ""
        classSyntaxRegistryHeaders(i) = ""
    NEXT
END SUB

SUB ClassSyntax_ClearScopes
    DIM i AS LONG

    classSyntaxScopeDepth = 0
    FOR i = 0 TO 63
        classSyntaxScopeVars(i) = ""
    NEXT
END SUB

FUNCTION ClassSyntax_DequeueLine$ ()
    DIM separatorPos AS LONG

    IF LEN(classSyntaxQueue$) = 0 THEN EXIT FUNCTION

    separatorPos = INSTR(classSyntaxQueue$, CHR$(10))
    IF separatorPos = 0 THEN
        ClassSyntax_DequeueLine$ = classSyntaxQueue$
        classSyntaxQueue$ = ""
    ELSE
        ClassSyntax_DequeueLine$ = LEFT$(classSyntaxQueue$, separatorPos - 1)
        classSyntaxQueue$ = MID$(classSyntaxQueue$, separatorPos + 1)
    END IF
END FUNCTION

FUNCTION ClassSyntax_DequeueDeferredLine$ ()
    DIM separatorPos AS LONG

    IF LEN(classSyntaxDeferredQueue$) = 0 THEN EXIT FUNCTION

    separatorPos = INSTR(classSyntaxDeferredQueue$, CHR$(10))
    IF separatorPos = 0 THEN
        ClassSyntax_DequeueDeferredLine$ = classSyntaxDeferredQueue$
        classSyntaxDeferredQueue$ = ""
    ELSE
        ClassSyntax_DequeueDeferredLine$ = LEFT$(classSyntaxDeferredQueue$, separatorPos - 1)
        classSyntaxDeferredQueue$ = MID$(classSyntaxDeferredQueue$, separatorPos + 1)
    END IF
END FUNCTION

SUB ClassSyntax_AppendLine (target$, line$)
    IF LEN(line$) = 0 THEN EXIT SUB
    IF LEN(target$) THEN target$ = target$ + CHR$(10)
    target$ = target$ + line$
END SUB

FUNCTION ClassSyntax_FindRegistryClass& (className$)
    DIM i AS LONG
    DIM lookupName AS STRING

    lookupName = UCASE$(RTRIM$(className$))
    FOR i = 1 TO classSyntaxRegistryCount
        IF UCASE$(RTRIM$(classSyntaxRegistryName(i))) = lookupName THEN
            ClassSyntax_FindRegistryClass = i
            EXIT FUNCTION
        END IF
    NEXT
END FUNCTION

FUNCTION ClassSyntax_FlatFieldLines$ (className$)
    DIM classIndex AS LONG

    classIndex = ClassSyntax_FindRegistryClass&(className$)
    IF classIndex = 0 THEN EXIT FUNCTION
    ClassSyntax_FlatFieldLines$ = classSyntaxRegistryFlatFields(classIndex)
END FUNCTION

SUB ClassSyntax_QueueFieldLines (fieldLines$)
    DIM remainingLines AS STRING
    DIM nextBreak AS LONG
    DIM nextLine AS STRING

    remainingLines$ = fieldLines$
    DO WHILE LEN(remainingLines$)
        nextBreak = INSTR(remainingLines$, CHR$(10))
        IF nextBreak = 0 THEN
            nextLine$ = remainingLines$
            remainingLines$ = ""
        ELSE
            nextLine$ = LEFT$(remainingLines$, nextBreak - 1)
            remainingLines$ = MID$(remainingLines$, nextBreak + 1)
        END IF
        IF LEN(LTRIM$(RTRIM$(nextLine$))) THEN ClassSyntax_QueueLine nextLine$
    LOOP
END SUB

SUB ClassSyntax_RegisterCurrentClass
    DIM classIndex AS LONG
    DIM baseIndex AS LONG
    DIM flattenedFields AS STRING

    classIndex = ClassSyntax_FindRegistryClass&(classSyntaxClassName$)
    IF classIndex = 0 THEN
        classSyntaxRegistryCount = classSyntaxRegistryCount + 1
        IF classSyntaxRegistryCount > 256 THEN
            Give_Error "Maximum CLASS limit exceeded"
            EXIT SUB
        END IF
        classIndex = classSyntaxRegistryCount
    END IF

    classSyntaxRegistryName(classIndex) = classSyntaxClassName$
    classSyntaxRegistryBase(classIndex) = classSyntaxBaseName$
    classSyntaxRegistryOwnFields(classIndex) = classSyntaxOwnFieldLines$

    flattenedFields$ = ""
    IF LEN(classSyntaxBaseName$) THEN
        baseIndex = ClassSyntax_FindRegistryClass&(classSyntaxBaseName$)
        IF baseIndex = 0 THEN
            Give_Error "Base class must be declared before derived class"
            EXIT SUB
        END IF
        flattenedFields$ = classSyntaxRegistryFlatFields(baseIndex)
    END IF
    IF LEN(flattenedFields$) AND LEN(classSyntaxOwnFieldLines$) THEN flattenedFields$ = flattenedFields$ + CHR$(10)
    flattenedFields$ = flattenedFields$ + classSyntaxOwnFieldLines$
    classSyntaxRegistryFlatFields(classIndex) = flattenedFields$
    IF LEN(classSyntaxBaseName$) > 0 AND classSyntaxInheritDone = 0 THEN ClassSyntax_RegisterInherited classIndex, baseIndex
    classSyntaxInheritDone = -1
END SUB

FUNCTION ClassSyntax_WithoutKey$ (lines$, key$)
    'lines$ ("KEY=value" lines) without the line of key$
    DIM remainingLines AS STRING
    DIM nextBreak AS LONG
    DIM nextLine AS STRING
    DIM kept AS STRING

    remainingLines = lines$
    DO WHILE LEN(remainingLines)
        nextBreak = INSTR(remainingLines, CHR$(10))
        IF nextBreak = 0 THEN
            nextLine = remainingLines
            remainingLines = ""
        ELSE
            nextLine = LEFT$(remainingLines, nextBreak - 1)
            remainingLines = MID$(remainingLines, nextBreak + 1)
        END IF
        IF LEFT$(nextLine, LEN(key$) + 1) <> key$ + "=" THEN ClassSyntax_AppendLine kept, nextLine
    LOOP
    ClassSyntax_WithoutKey$ = kept
END FUNCTION

SUB ClassSyntax_RegisterHeader (className$, methodName$, headerLine$)
    DIM classIndex AS LONG
    DIM methodKey AS STRING
    DIM updatedLines AS STRING

    classIndex = ClassSyntax_FindRegistryClass&(className$)
    IF classIndex = 0 THEN EXIT SUB
    methodKey = UCASE$(RTRIM$(methodName$))
    updatedLines = ClassSyntax_WithoutKey$(classSyntaxRegistryHeaders(classIndex), methodKey)
    ClassSyntax_AppendLine updatedLines, methodKey + "=" + headerLine$
    classSyntaxRegistryHeaders(classIndex) = updatedLines
END SUB

SUB ClassSyntax_SplitHeader (headerLine$, kindText$, generatedName$, otherParams$, returnClause$)
    'parts of "SUB name(self AS Class, a AS LONG) ..." made by ClassSyntax_BuildMethodHeader$
    DIM restText AS STRING
    DIM openPos AS LONG
    DIM closePos AS LONG
    DIM commaPos AS LONG

    generatedName$ = ""
    kindText$ = ClassSyntax_FirstToken$(headerLine$)
    restText = LTRIM$(MID$(LTRIM$(headerLine$), LEN(kindText$) + 1))
    openPos = INSTR(restText, "(")
    IF openPos = 0 THEN EXIT SUB
    closePos = ClassSyntax_FindMatchingCloseParen&(restText, openPos)
    IF closePos = 0 THEN EXIT SUB
    generatedName$ = RTRIM$(LEFT$(restText, openPos - 1))
    otherParams$ = MID$(restText, openPos + 1, closePos - openPos - 1)
    returnClause$ = LTRIM$(MID$(restText, closePos + 1))
    commaPos = INSTR(otherParams$, ",")
    IF commaPos THEN otherParams$ = LTRIM$(RTRIM$(MID$(otherParams$, commaPos + 1))) ELSE otherParams$ = ""
END SUB

SUB ClassSyntax_RegisterInherited (classIndex AS LONG, baseIndex AS LONG)
    'A method of the base class takes "self AS Base": an object of the derived class is
    'another TYPE and cannot be passed to it. Every inherited method therefore gets a
    'procedure of the derived class (ClassSyntax_EmitInherited writes them at END CLASS).
    'They are registered here, before the class's own methods are read, so that those
    'can call them; a method the class defines itself takes the place of the inherited one.
    DIM remainingLines AS STRING
    DIM nextBreak AS LONG
    DIM nextLine AS STRING
    DIM eqPos AS LONG
    DIM methodKey AS STRING
    DIM kindText AS STRING
    DIM baseGenerated AS STRING
    DIM otherParams AS STRING
    DIM returnClause AS STRING
    DIM newGenerated AS STRING
    DIM newHeader AS STRING
    DIM basePrefix AS STRING

    basePrefix = "__QBNEX_" + ClassSyntax_SafeIdentifier$(RTRIM$(classSyntaxRegistryName(baseIndex))) + "_"
    remainingLines = classSyntaxRegistryHeaders(baseIndex)
    DO WHILE LEN(remainingLines)
        nextBreak = INSTR(remainingLines, CHR$(10))
        IF nextBreak = 0 THEN
            nextLine = remainingLines
            remainingLines = ""
        ELSE
            nextLine = LEFT$(remainingLines, nextBreak - 1)
            remainingLines = MID$(remainingLines, nextBreak + 1)
        END IF
        eqPos = INSTR(nextLine, "=")
        IF eqPos > 1 THEN
            methodKey = LEFT$(nextLine, eqPos - 1)
            ClassSyntax_SplitHeader MID$(nextLine, eqPos + 1), kindText, baseGenerated, otherParams, returnClause
            IF methodKey <> "CONSTRUCTOR" AND LEFT$(baseGenerated, LEN(basePrefix)) = basePrefix THEN
                newGenerated = "__QBNEX_" + ClassSyntax_SafeIdentifier$(classSyntaxClassName$) + "_" + MID$(baseGenerated, LEN(basePrefix) + 1)
                newHeader = kindText + " " + newGenerated + "(self AS " + classSyntaxClassName$
                IF LEN(otherParams) THEN newHeader = newHeader + ", " + otherParams
                newHeader = newHeader + ")"
                IF LEN(returnClause) THEN newHeader = newHeader + " " + returnClause
                classSyntaxRegistryMethods(classIndex) = ClassSyntax_WithoutKey$(classSyntaxRegistryMethods(classIndex), methodKey)
                ClassSyntax_AppendLine classSyntaxRegistryMethods(classIndex), methodKey + "=" + newGenerated
                classSyntaxRegistryHeaders(classIndex) = ClassSyntax_WithoutKey$(classSyntaxRegistryHeaders(classIndex), methodKey)
                ClassSyntax_AppendLine classSyntaxRegistryHeaders(classIndex), methodKey + "=" + newHeader
                'KEY=the base procedure it calls
                ClassSyntax_AppendLine classSyntaxInheritPending$, methodKey + "=" + baseGenerated
            END IF
        END IF
    LOOP
END SUB

SUB ClassSyntax_QueueFieldCopies (fieldLines$, toName$, fromName$)
    'one assignment a field: "name AS type" lines of a class
    DIM remainingLines AS STRING
    DIM nextBreak AS LONG
    DIM nextLine AS STRING
    DIM fieldName AS STRING

    remainingLines = fieldLines$
    DO WHILE LEN(remainingLines)
        nextBreak = INSTR(remainingLines, CHR$(10))
        IF nextBreak = 0 THEN
            nextLine = remainingLines
            remainingLines = ""
        ELSE
            nextLine = LEFT$(remainingLines, nextBreak - 1)
            remainingLines = MID$(remainingLines, nextBreak + 1)
        END IF
        fieldName = ClassSyntax_FirstIdentifier$(nextLine)
        IF LEN(fieldName) > 0 AND UCASE$(fieldName) <> "AS" THEN ClassSyntax_QueueDeferredLine "    " + toName$ + "." + fieldName + " = " + fromName$ + "." + fieldName
    LOOP
END SUB

SUB ClassSyntax_EmitInherited
    'the procedures registered by ClassSyntax_RegisterInherited that the class did not
    'replace: copy the object into one of the base class, call the base method, copy back
    DIM classIndex AS LONG
    DIM baseIndex AS LONG
    DIM remainingLines AS STRING
    DIM nextBreak AS LONG
    DIM nextLine AS STRING
    DIM eqPos AS LONG
    DIM methodKey AS STRING
    DIM kindText AS STRING
    DIM generatedName AS STRING
    DIM otherParams AS STRING
    DIM returnClause AS STRING
    DIM baseGenerated AS STRING
    DIM headerLine AS STRING
    DIM argList AS STRING
    DIM paramText AS STRING
    DIM argName AS STRING
    DIM depth AS LONG
    DIM i AS LONG
    DIM c AS STRING

    IF LEN(classSyntaxBaseName$) = 0 OR LEN(classSyntaxInheritPending$) = 0 THEN EXIT SUB
    classIndex = ClassSyntax_FindRegistryClass&(classSyntaxClassName$)
    baseIndex = ClassSyntax_FindRegistryClass&(classSyntaxBaseName$)
    IF classIndex = 0 OR baseIndex = 0 THEN EXIT SUB

    remainingLines = classSyntaxInheritPending$
    DO WHILE LEN(remainingLines)
        nextBreak = INSTR(remainingLines, CHR$(10))
        IF nextBreak = 0 THEN
            nextLine = remainingLines
            remainingLines = ""
        ELSE
            nextLine = LEFT$(remainingLines, nextBreak - 1)
            remainingLines = MID$(remainingLines, nextBreak + 1)
        END IF
        eqPos = INSTR(nextLine, "=")
        methodKey = LEFT$(nextLine, eqPos - 1)
        baseGenerated = MID$(nextLine, eqPos + 1)
        headerLine = ""
        i = INSTR(CHR$(10) + classSyntaxRegistryHeaders(classIndex), CHR$(10) + methodKey + "=")
        IF i THEN
            headerLine = MID$(classSyntaxRegistryHeaders(classIndex), i + LEN(methodKey) + 1)
            IF INSTR(headerLine, CHR$(10)) THEN headerLine = LEFT$(headerLine, INSTR(headerLine, CHR$(10)) - 1)
        END IF
        IF LEN(headerLine) THEN
            ClassSyntax_SplitHeader headerLine, kindText, generatedName, otherParams, returnClause
            'the names of the parameters, to pass them on (an array as name())
            argList = ""
            paramText = ""
            depth = 0
            FOR i = 1 TO LEN(otherParams) + 1
                c = MID$(otherParams, i, 1)
                IF c = "(" THEN depth = depth + 1
                IF c = ")" THEN depth = depth - 1
                IF (c = "," AND depth = 0) OR i > LEN(otherParams) THEN
                    paramText = LTRIM$(RTRIM$(paramText))
                    IF UCASE$(LEFT$(paramText, 6)) = "BYVAL " THEN paramText = LTRIM$(MID$(paramText, 7))
                    argName = ClassSyntax_FirstIdentifier$(paramText)
                    IF LEN(argName) THEN
                        IF INSTR(paramText, "()") THEN argName = argName + "()"
                        argList = argList + ", " + argName
                    END IF
                    paramText = ""
                ELSE
                    paramText = paramText + c
                END IF
            NEXT
            ClassSyntax_QueueDeferredLine headerLine
            ClassSyntax_QueueDeferredLine "    DIM qbnexBase AS " + classSyntaxBaseName$
            ClassSyntax_QueueDeferredLine "    qbnexBase.Header = self.Header"
            ClassSyntax_QueueFieldCopies classSyntaxRegistryFlatFields(baseIndex), "qbnexBase", "self"
            IF kindText = "FUNCTION" THEN
                ClassSyntax_QueueDeferredLine "    " + generatedName + " = " + baseGenerated + "(qbnexBase" + argList + ")"
            ELSE
                ClassSyntax_QueueDeferredLine "    " + baseGenerated + " qbnexBase" + argList
            END IF
            ClassSyntax_QueueFieldCopies classSyntaxRegistryFlatFields(baseIndex), "self", "qbnexBase"
            ClassSyntax_QueueDeferredLine "    self.Header = qbnexBase.Header"
            ClassSyntax_QueueDeferredLine "END " + kindText
        END IF
    LOOP
    classSyntaxInheritPending$ = ""
END SUB

SUB ClassSyntax_RegisterMethod (className$, methodName$, generatedName$)
    DIM classIndex AS LONG
    DIM methodKey AS STRING
    DIM entryLine AS STRING
    DIM remainingLines AS STRING
    DIM nextBreak AS LONG
    DIM nextLine AS STRING
    DIM updatedLines AS STRING

    classIndex = ClassSyntax_FindRegistryClass&(className$)
    IF classIndex = 0 THEN EXIT SUB

    methodKey$ = UCASE$(RTRIM$(methodName$))
    entryLine$ = methodKey$ + "=" + generatedName$
    remainingLines$ = classSyntaxRegistryMethods(classIndex)

    DO WHILE LEN(remainingLines$)
        nextBreak = INSTR(remainingLines$, CHR$(10))
        IF nextBreak = 0 THEN
            nextLine$ = remainingLines$
            remainingLines$ = ""
        ELSE
            nextLine$ = LEFT$(remainingLines$, nextBreak - 1)
            remainingLines$ = MID$(remainingLines$, nextBreak + 1)
        END IF
        IF LEFT$(nextLine$, LEN(methodKey$) + 1) <> methodKey$ + "=" THEN
            ClassSyntax_AppendLine updatedLines$, nextLine$
        END IF
    LOOP

    ClassSyntax_AppendLine updatedLines$, entryLine$
    classSyntaxRegistryMethods(classIndex) = updatedLines$
END SUB

FUNCTION ClassSyntax_FindGeneratedMethod$ (className$, methodName$)
    DIM classIndex AS LONG
    DIM remainingLines AS STRING
    DIM nextBreak AS LONG
    DIM nextLine AS STRING
    DIM methodKey AS STRING

    classIndex = ClassSyntax_FindRegistryClass&(className$)
    IF classIndex = 0 THEN EXIT FUNCTION

    methodKey$ = UCASE$(RTRIM$(methodName$)) + "="
    remainingLines$ = classSyntaxRegistryMethods(classIndex)
    DO WHILE LEN(remainingLines$)
        nextBreak = INSTR(remainingLines$, CHR$(10))
        IF nextBreak = 0 THEN
            nextLine$ = remainingLines$
            remainingLines$ = ""
        ELSE
            nextLine$ = LEFT$(remainingLines$, nextBreak - 1)
            remainingLines$ = MID$(remainingLines$, nextBreak + 1)
        END IF
        IF LEFT$(nextLine$, LEN(methodKey$)) = methodKey$ THEN
            ClassSyntax_FindGeneratedMethod$ = MID$(nextLine$, LEN(methodKey$) + 1)
            EXIT FUNCTION
        END IF
    LOOP

    IF LEN(classSyntaxRegistryBase(classIndex)) THEN
        ClassSyntax_FindGeneratedMethod$ = ClassSyntax_FindGeneratedMethod$(classSyntaxRegistryBase(classIndex), methodName$)
    END IF
END FUNCTION

FUNCTION ClassSyntax_RewriteMethodCall$ (tokens$, objectStart AS LONG, methodElement AS LONG, generatedName$)
    DIM n AS LONG
    DIM i AS LONG
    DIM depth AS LONG
    DIM closeElement AS LONG
    DIM prefixText AS STRING
    DIM objectExpr AS STRING
    DIM argsText AS STRING
    DIM suffixText AS STRING
    DIM resultText AS STRING

    n = numelements(tokens$)
    IF methodElement + 1 > n THEN EXIT FUNCTION
    IF getelement$(tokens$, methodElement + 1) <> "(" THEN EXIT FUNCTION

    depth = 0
    FOR i = methodElement + 1 TO n
        IF getelement$(tokens$, i) = "(" THEN depth = depth + 1
        IF getelement$(tokens$, i) = ")" THEN
            depth = depth - 1
            IF depth = 0 THEN closeElement = i: EXIT FOR
        END IF
    NEXT
    IF closeElement = 0 THEN EXIT FUNCTION

    IF objectStart > 1 THEN prefixText = getelements$(tokens$, 1, objectStart - 1)
    objectExpr = getelements$(tokens$, objectStart, methodElement - 2)
    IF closeElement > methodElement + 2 THEN argsText = getelements$(tokens$, methodElement + 2, closeElement - 1)
    IF closeElement < n THEN suffixText = getelements$(tokens$, closeElement + 1, n)

    resultText = generatedName$ + sp + "(" + sp + objectExpr
    IF LEN(argsText) THEN resultText = resultText + sp + "," + sp + argsText
    resultText = resultText + sp + ")"

    IF LEN(prefixText) THEN resultText = prefixText + sp + resultText
    IF LEN(suffixText) THEN resultText = resultText + sp + suffixText
    ClassSyntax_RewriteMethodCall$ = resultText
END FUNCTION

SUB ClassSyntax_EnterScope
    IF classSyntaxScopeDepth < 63 THEN classSyntaxScopeDepth = classSyntaxScopeDepth + 1
    classSyntaxScopeVars(classSyntaxScopeDepth) = ""
END SUB

SUB ClassSyntax_ExitScope
    IF classSyntaxScopeDepth < 0 THEN classSyntaxScopeDepth = 0
    classSyntaxScopeVars(classSyntaxScopeDepth) = ""
    IF classSyntaxScopeDepth > 0 THEN classSyntaxScopeDepth = classSyntaxScopeDepth - 1
END SUB

SUB ClassSyntax_RegisterScopeVar (varName$, typeName$)
    DIM entryLine AS STRING
    DIM scopeText AS STRING
    DIM remainingLines AS STRING
    DIM nextBreak AS LONG
    DIM nextLine AS STRING
    DIM updatedLines AS STRING
    DIM lookupKey AS STRING

    varName$ = LTRIM$(RTRIM$(varName$))
    typeName$ = LTRIM$(RTRIM$(typeName$))
    IF LEN(varName$) = 0 OR LEN(typeName$) = 0 THEN EXIT SUB
    IF ClassSyntax_FindRegistryClass&(typeName$) = 0 THEN EXIT SUB

    lookupKey$ = UCASE$(varName$) + "="
    entryLine$ = lookupKey$ + typeName$
    scopeText$ = classSyntaxScopeVars(classSyntaxScopeDepth)
    remainingLines$ = scopeText$

    DO WHILE LEN(remainingLines$)
        nextBreak = INSTR(remainingLines$, CHR$(10))
        IF nextBreak = 0 THEN
            nextLine$ = remainingLines$
            remainingLines$ = ""
        ELSE
            nextLine$ = LEFT$(remainingLines$, nextBreak - 1)
            remainingLines$ = MID$(remainingLines$, nextBreak + 1)
        END IF
        IF LEFT$(nextLine$, LEN(lookupKey$)) <> lookupKey$ THEN
            ClassSyntax_AppendLine updatedLines$, nextLine$
        END IF
    LOOP

    ClassSyntax_AppendLine updatedLines$, entryLine$
    classSyntaxScopeVars(classSyntaxScopeDepth) = updatedLines$
END SUB

FUNCTION ClassSyntax_LookupVarType$ (varName$)
    DIM scopeIndex AS LONG
    DIM remainingLines AS STRING
    DIM nextBreak AS LONG
    DIM nextLine AS STRING
    DIM lookupKey AS STRING

    lookupKey$ = UCASE$(LTRIM$(RTRIM$(varName$))) + "="
    FOR scopeIndex = classSyntaxScopeDepth TO 0 STEP -1
        remainingLines$ = classSyntaxScopeVars(scopeIndex)
        DO WHILE LEN(remainingLines$)
            nextBreak = INSTR(remainingLines$, CHR$(10))
            IF nextBreak = 0 THEN
                nextLine$ = remainingLines$
                remainingLines$ = ""
            ELSE
                nextLine$ = LEFT$(remainingLines$, nextBreak - 1)
                remainingLines$ = MID$(remainingLines$, nextBreak + 1)
            END IF
            IF LEFT$(nextLine$, LEN(lookupKey$)) = lookupKey$ THEN
                ClassSyntax_LookupVarType$ = MID$(nextLine$, LEN(lookupKey$) + 1)
                EXIT FUNCTION
            END IF
        LOOP
    NEXT
END FUNCTION

FUNCTION ClassSyntax_FirstIdentifier$ (text$)
    DIM i AS LONG
    DIM c$

    text$ = LTRIM$(RTRIM$(text$))
    FOR i = 1 TO LEN(text$)
        c$ = MID$(text$, i, 1)
        IF c$ = " " OR c$ = CHR$(9) OR c$ = "(" THEN
            ClassSyntax_FirstIdentifier$ = LEFT$(text$, i - 1)
            EXIT FUNCTION
        END IF
    NEXT
    ClassSyntax_FirstIdentifier$ = text$
END FUNCTION

SUB ClassSyntax_RegisterTypedList (leftText$, typeText$)
    DIM remainingText AS STRING
    DIM nextComma AS LONG
    DIM nextPart AS STRING
    DIM varName AS STRING

    remainingText$ = leftText$
    DO WHILE LEN(remainingText$)
        nextComma = INSTR(remainingText$, ",")
        IF nextComma = 0 THEN
            nextPart$ = remainingText$
            remainingText$ = ""
        ELSE
            nextPart$ = LEFT$(remainingText$, nextComma - 1)
            remainingText$ = MID$(remainingText$, nextComma + 1)
        END IF
        varName$ = ClassSyntax_FirstIdentifier$(nextPart$)
        IF LEN(varName$) THEN ClassSyntax_RegisterScopeVar varName$, typeText$
    LOOP
END SUB

SUB ClassSyntax_RegisterDeclarationLine (rawLine$)
    DIM trimmedLine AS STRING
    DIM upperLine AS STRING
    DIM asPos AS LONG
    DIM leftText AS STRING
    DIM typeText AS STRING

    trimmedLine = LTRIM$(RTRIM$(rawLine$))
    upperLine = UCASE$(trimmedLine)

    IF LEFT$(upperLine, 11) = "DIM SHARED " THEN
        leftText = MID$(trimmedLine, 12)
    ELSEIF LEFT$(upperLine, 4) = "DIM " THEN
        leftText = MID$(trimmedLine, 5)
    ELSEIF LEFT$(upperLine, 7) = "STATIC " THEN
        leftText = MID$(trimmedLine, 8)
    ELSE
        EXIT SUB
    END IF

    asPos = INSTR(UCASE$(leftText), " AS ")
    IF asPos = 0 THEN EXIT SUB
    typeText = ClassSyntax_FirstToken$(MID$(leftText, asPos + 4))
    leftText = LEFT$(leftText, asPos - 1)
    ClassSyntax_RegisterTypedList leftText, typeText
END SUB

SUB ClassSyntax_RegisterProcedureHeader (rawLine$)
    DIM trimmedLine AS STRING
    DIM upperLine AS STRING
    DIM signatureText AS STRING
    DIM openPos AS LONG
    DIM closePos AS LONG
    DIM paramsText AS STRING
    DIM remainingText AS STRING
    DIM nextComma AS LONG
    DIM nextParam AS STRING
    DIM asPos AS LONG
    DIM paramName AS STRING
    DIM typeText AS STRING

    trimmedLine = LTRIM$(RTRIM$(rawLine$))
    upperLine = UCASE$(trimmedLine)
    IF LEFT$(upperLine, 4) = "SUB " THEN
        signatureText = MID$(trimmedLine, 4)
    ELSEIF LEFT$(upperLine, 9) = "FUNCTION " THEN
        signatureText = MID$(trimmedLine, 9)
    ELSE
        EXIT SUB
    END IF

    ClassSyntax_EnterScope
    openPos = INSTR(signatureText, "(")
    IF openPos = 0 THEN EXIT SUB
    closePos = INSTR(signatureText, ")")
    IF closePos = 0 THEN EXIT SUB
    paramsText = MID$(signatureText, openPos + 1, closePos - openPos - 1)
    remainingText = paramsText

    DO WHILE LEN(remainingText)
        nextComma = INSTR(remainingText, ",")
        IF nextComma = 0 THEN
            nextParam = remainingText
            remainingText = ""
        ELSE
            nextParam = LEFT$(remainingText, nextComma - 1)
            remainingText = MID$(remainingText, nextComma + 1)
        END IF
        asPos = INSTR(UCASE$(nextParam), " AS ")
        IF asPos THEN
            paramName = ClassSyntax_FirstIdentifier$(LEFT$(nextParam, asPos - 1))
            typeText = ClassSyntax_FirstToken$(MID$(nextParam, asPos + 4))
            ClassSyntax_RegisterScopeVar paramName, typeText
        END IF
    LOOP
END SUB

FUNCTION ClassSyntax_AtStatementStart% (textBefore$)
    'nothing but a statement boundary precedes: start of the line, ":", THEN or ELSE
    DIM t AS STRING

    t = UCASE$(RTRIM$(textBefore$))
    IF LEN(LTRIM$(t)) = 0 THEN ClassSyntax_AtStatementStart% = -1: EXIT FUNCTION
    IF RIGHT$(t, 1) = ":" THEN ClassSyntax_AtStatementStart% = -1: EXIT FUNCTION
    IF RIGHT$(t, 5) = " THEN" OR RIGHT$(t, 5) = " ELSE" OR LTRIM$(t) = "ELSE" THEN ClassSyntax_AtStatementStart% = -1
END FUNCTION

FUNCTION ClassSyntax_RewriteDispatch$ (sourceLine$, selfType$)
    DIM outputText AS STRING
    DIM i AS LONG
    DIM inString AS LONG
    DIM flushPos AS LONG
    DIM objectEnd AS LONG
    DIM objectStart AS LONG
    DIM methodStart AS LONG
    DIM j AS LONG
    DIM methodName AS STRING
    DIM openPos AS LONG
    DIM closePos AS LONG
    DIM currentChar AS STRING
    DIM className AS STRING
    DIM generatedName AS STRING
    DIM objectExpr AS STRING
    DIM emittedObjectExpr AS STRING
    DIM argsText AS STRING

    'a dispatch needs "object.method(" with a registered class: skip the character scan
    'of every source line when there is no class or no "."
    IF classSyntaxRegistryCount = 0 OR INSTR(sourceLine$, ".") = 0 THEN
        ClassSyntax_RewriteDispatch$ = sourceLine$
        EXIT FUNCTION
    END IF

    flushPos = 1
    i = 1
    DO WHILE i <= LEN(sourceLine$)
        currentChar = MID$(sourceLine$, i, 1)

        IF currentChar = CHR$(34) THEN
            IF inString THEN inString = 0 ELSE inString = -1
            i = i + 1
            GOTO ClassSyntax_RewriteDispatch_Next
        END IF

        IF inString = 0 THEN
            IF currentChar = "'" THEN
                EXIT DO
            END IF

            IF currentChar = "." THEN
                methodStart = i + 1
                DO WHILE methodStart <= LEN(sourceLine$) AND (MID$(sourceLine$, methodStart, 1) = " " OR MID$(sourceLine$, methodStart, 1) = CHR$(9))
                    methodStart = methodStart + 1
                LOOP
                j = methodStart
                DO WHILE j <= LEN(sourceLine$) AND (MID$(sourceLine$, j, 1) = " " OR MID$(sourceLine$, j, 1) = CHR$(9))
                    j = j + 1
                LOOP

                DO WHILE j <= LEN(sourceLine$)
                    currentChar = MID$(sourceLine$, j, 1)
                    IF ClassSyntax_IsIdentifierChar%(currentChar) OR ClassSyntax_IsTypeSuffixChar%(currentChar) THEN
                        j = j + 1
                    ELSE
                        EXIT DO
                    END IF
                LOOP
                methodName = MID$(sourceLine$, methodStart, j - methodStart)

                DO WHILE j <= LEN(sourceLine$) AND (MID$(sourceLine$, j, 1) = " " OR MID$(sourceLine$, j, 1) = CHR$(9))
                    j = j + 1
                LOOP

                IF LEN(methodName) AND j <= LEN(sourceLine$) AND MID$(sourceLine$, j, 1) = "(" THEN
                    objectEnd = ClassSyntax_SkipLeftSpaces&(sourceLine$, i - 1)
                    objectStart = ClassSyntax_FindDispatchObjectStart&(sourceLine$, objectEnd)
                    IF objectStart > 0 AND objectStart >= flushPos THEN
                        objectExpr = MID$(sourceLine$, objectStart, objectEnd - objectStart + 1)
                        emittedObjectExpr = LTRIM$(RTRIM$(objectExpr))
                        DO WHILE ClassSyntax_IsWrappedExpression%(emittedObjectExpr)
                            emittedObjectExpr = LTRIM$(RTRIM$(MID$(emittedObjectExpr, 2, LEN(emittedObjectExpr) - 2)))
                        LOOP
                        className = ClassSyntax_ResolveDispatchType$(objectExpr, selfType$)
                        IF LEN(className) THEN
                            generatedName = ClassSyntax_FindGeneratedMethod$(className, methodName)
                            IF LEN(generatedName) THEN
                                openPos = j
                                closePos = ClassSyntax_FindMatchingCloseParen&(sourceLine$, openPos)
                                IF closePos > 0 THEN
                                    argsText = MID$(sourceLine$, openPos + 1, closePos - openPos - 1)
                                    IF objectStart > flushPos THEN outputText = outputText + MID$(sourceLine$, flushPos, objectStart - flushPos)
                                    'object.method(...) as a statement: with CALL, or the object in
                                    'brackets would be an expression (a TYPE cannot be one)
                                    IF ClassSyntax_AtStatementStart%(outputText) THEN outputText = outputText + "CALL "
                                    outputText = outputText + generatedName + "(" + emittedObjectExpr
                                    IF LEN(LTRIM$(RTRIM$(argsText))) THEN outputText = outputText + ", " + argsText
                                    outputText = outputText + ")"
                                    i = closePos + 1
                                    flushPos = i
                                    GOTO ClassSyntax_RewriteDispatch_Next
                                END IF
                            END IF
                        END IF
                    END IF
                END IF
            END IF
        END IF

        i = i + 1
        ClassSyntax_RewriteDispatch_Next:
    LOOP

    IF flushPos <= LEN(sourceLine$) THEN outputText = outputText + MID$(sourceLine$, flushPos)
    ClassSyntax_RewriteDispatch$ = outputText
END FUNCTION
