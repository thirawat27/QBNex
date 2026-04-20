FUNCTION TopLevelRuntime_ShouldCapture% (trimmedLine$, upperLine$)
    IF LEN(trimmedLine$) = 0 THEN EXIT FUNCTION
    IF LEFT$(trimmedLine$, 1) = "'" THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 4) = "REM " THEN EXIT FUNCTION
    IF LEFT$(trimmedLine$, 1) = "$" THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 7) = "OPTION " THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 6) = "CONST " THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 4) = "DIM " THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 6) = "REDIM " THEN EXIT FUNCTION
    IF LEFT$(upperLine$, 7) = "STATIC " THEN EXIT FUNCTION
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

SUB TopLevelRuntime_InjectMainHook (mainPath$)
    DIM fileText AS STRING
    DIM insertPos AS LONG
    DIM runtimeCall AS STRING
    DIM fh AS LONG

    runtimeCall = "SUB_QBNEX_TOPLEVEL_RUNTIME0();"

    fh = FREEFILE
    OPEN mainPath$ FOR BINARY AS #fh
    IF LOF(fh) THEN fileText = INPUT$(LOF(fh), #fh)
    CLOSE #fh

    IF INSTR(fileText, runtimeCall) THEN EXIT SUB

    insertPos = INSTR(fileText, "sub_end();")
    IF insertPos = 0 THEN EXIT SUB

    fileText = LEFT$(fileText, insertPos - 1) + runtimeCall + CHR$(13) + CHR$(10) + MID$(fileText, insertPos)

    fh = FREEFILE
    OPEN mainPath$ FOR OUTPUT AS #fh
    PRINT #fh, fileText;
    CLOSE #fh
END SUB

FUNCTION TopLevelRuntime_ProcessLine$ (rawLine$)
    DIM trimmedLine AS STRING
    DIM upperLine AS STRING

    IF rawLine$ = CHR$(13) THEN
        TopLevelRuntime_ProcessLine$ = rawLine$
        EXIT FUNCTION
    END IF

    trimmedLine = LTRIM$(RTRIM$(rawLine$))
    upperLine = UCASE$(trimmedLine)

    IF upperLine = "END SUB" OR upperLine = "END FUNCTION" THEN
        IF topLevelRuntimeProcDepth > 0 THEN topLevelRuntimeProcDepth = topLevelRuntimeProcDepth - 1
        TopLevelRuntime_ProcessLine$ = rawLine$
        EXIT FUNCTION
    END IF
    IF upperLine = "END TYPE" THEN
        IF topLevelRuntimeTypeDepth > 0 THEN topLevelRuntimeTypeDepth = topLevelRuntimeTypeDepth - 1
        TopLevelRuntime_ProcessLine$ = rawLine$
        EXIT FUNCTION
    END IF
    IF upperLine = "END DECLARE" THEN
        IF topLevelRuntimeDeclareDepth > 0 THEN topLevelRuntimeDeclareDepth = topLevelRuntimeDeclareDepth - 1
        TopLevelRuntime_ProcessLine$ = rawLine$
        EXIT FUNCTION
    END IF

    IF LEFT$(upperLine, 16) = "DECLARE LIBRARY " OR upperLine = "DECLARE LIBRARY" THEN
        topLevelRuntimeDeclareDepth = topLevelRuntimeDeclareDepth + 1
        TopLevelRuntime_ProcessLine$ = rawLine$
        EXIT FUNCTION
    END IF
    IF LEFT$(upperLine, 5) = "TYPE " THEN
        topLevelRuntimeTypeDepth = topLevelRuntimeTypeDepth + 1
        TopLevelRuntime_ProcessLine$ = rawLine$
        EXIT FUNCTION
    END IF
    IF LEFT$(upperLine, 4) = "SUB " OR LEFT$(upperLine, 9) = "FUNCTION " THEN
        topLevelRuntimeProcDepth = topLevelRuntimeProcDepth + 1
        TopLevelRuntime_ProcessLine$ = rawLine$
        EXIT FUNCTION
    END IF

    IF topLevelRuntimeProcDepth OR topLevelRuntimeTypeDepth OR topLevelRuntimeDeclareDepth THEN
        TopLevelRuntime_ProcessLine$ = rawLine$
        EXIT FUNCTION
    END IF

    IF TopLevelRuntime_ShouldCapture%(trimmedLine, upperLine) THEN
        topLevelRuntimeCallInjected = -1
        ClassSyntax_AppendLine topLevelRuntimeLines, rawLine$
        TopLevelRuntime_ProcessLine$ = ""
        EXIT FUNCTION
    END IF

    TopLevelRuntime_ProcessLine$ = rawLine$
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
