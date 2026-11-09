' ============================================================================
' QBNex Standard Library - Error: Result
' ============================================================================

TYPE QBNex_Result
    Ok AS LONG
    Code AS LONG
    Value AS STRING
    Message AS STRING
    Context AS STRING
    Source AS STRING
    Cause AS STRING
END TYPE

CONST QBNEX_RESULT_CODE_OK = 0
CONST QBNEX_RESULT_CODE_ERROR = 1

FUNCTION Result_TrimText$ (text AS STRING)
    Result_TrimText$ = LTRIM$(RTRIM$(text))
END FUNCTION

FUNCTION Result_PrependDetail$ (existingText AS STRING, newText AS STRING)
    DIM normalizedExisting AS STRING
    DIM normalizedNew AS STRING

    normalizedExisting = Result_TrimText$(existingText)
    normalizedNew = Result_TrimText$(newText)

    IF normalizedNew = "" THEN
        Result_PrependDetail$ = normalizedExisting
    ELSEIF normalizedExisting = "" THEN
        Result_PrependDetail$ = normalizedNew
    ELSEIF UCASE$(normalizedExisting) = UCASE$(normalizedNew) THEN
        Result_PrependDetail$ = normalizedExisting
    ELSEIF INSTR(UCASE$(normalizedExisting), UCASE$(normalizedNew)) > 0 THEN
        Result_PrependDetail$ = normalizedExisting
    ELSE
        Result_PrependDetail$ = normalizedNew + " -> " + normalizedExisting
    END IF
END FUNCTION

SUB Result_Clear (resultRef AS QBNex_Result)
    resultRef.Ok = 0
    resultRef.Code = QBNEX_RESULT_CODE_OK
    resultRef.Value = ""
    resultRef.Message = ""
    resultRef.Context = ""
    resultRef.Source = ""
    resultRef.Cause = ""
END SUB

SUB Result_Copy (resultRef AS QBNex_Result, sourceResult AS QBNex_Result)
    resultRef.Ok = sourceResult.Ok
    resultRef.Code = sourceResult.Code
    resultRef.Value = sourceResult.Value
    resultRef.Message = sourceResult.Message
    resultRef.Context = sourceResult.Context
    resultRef.Source = sourceResult.Source
    resultRef.Cause = sourceResult.Cause
END SUB

SUB Result_Ok (resultRef AS QBNex_Result, valueText AS STRING)
    Result_Clear resultRef
    resultRef.Ok = -1
    resultRef.Value = valueText
END SUB

SUB Result_Fail (resultRef AS QBNex_Result, messageText AS STRING)
    Result_FailCode resultRef, QBNEX_RESULT_CODE_ERROR, messageText
END SUB

SUB Result_FailCode (resultRef AS QBNex_Result, errorCode AS LONG, messageText AS STRING)
    resultRef.Ok = 0
    resultRef.Code = errorCode
    resultRef.Value = ""
    resultRef.Message = Result_TrimText$(messageText)
    resultRef.Context = ""
    resultRef.Source = ""
    resultRef.Cause = ""
END SUB

SUB Result_FailWithContext (resultRef AS QBNex_Result, errorCode AS LONG, messageText AS STRING, contextText AS STRING, sourceText AS STRING)
    Result_FailCode resultRef, errorCode, messageText
    resultRef.Context = Result_TrimText$(contextText)
    resultRef.Source = Result_TrimText$(sourceText)
END SUB

SUB Result_AddContext (resultRef AS QBNex_Result, contextText AS STRING)
    IF resultRef.Ok THEN EXIT SUB
    resultRef.Context = Result_PrependDetail$(resultRef.Context, contextText)
END SUB

SUB Result_SetSource (resultRef AS QBNex_Result, sourceText AS STRING)
    IF resultRef.Ok THEN EXIT SUB
    resultRef.Source = Result_TrimText$(sourceText)
END SUB

SUB Result_SetCause (resultRef AS QBNex_Result, causeText AS STRING)
    IF resultRef.Ok THEN EXIT SUB
    resultRef.Cause = Result_TrimText$(causeText)
END SUB

SUB Result_Propagate (resultRef AS QBNex_Result, sourceResult AS QBNex_Result, contextText AS STRING, sourceText AS STRING)
    Result_Copy resultRef, sourceResult

    IF sourceResult.Ok THEN EXIT SUB

    Result_AddContext resultRef, contextText

    IF Result_TrimText$(sourceText) <> "" THEN resultRef.Source = Result_TrimText$(sourceText)
    IF Result_TrimText$(resultRef.Cause) = "" THEN resultRef.Cause = Result_ErrorChain$(sourceResult)
END SUB

FUNCTION Result_IsOk& (resultRef AS QBNex_Result)
    Result_IsOk = resultRef.Ok
END FUNCTION

FUNCTION Result_IsError& (resultRef AS QBNex_Result)
    IF resultRef.Ok THEN
        Result_IsError = 0
    ELSE
        Result_IsError = -1
    END IF
END FUNCTION

FUNCTION Result_Code& (resultRef AS QBNex_Result)
    Result_Code = resultRef.Code
END FUNCTION

FUNCTION Result_Value$ (resultRef AS QBNex_Result, defaultValue AS STRING)
    IF resultRef.Ok THEN
        Result_Value = resultRef.Value
    ELSE
        Result_Value = defaultValue
    END IF
END FUNCTION

FUNCTION Result_Message$ (resultRef AS QBNex_Result)
    Result_Message = resultRef.Message
END FUNCTION

FUNCTION Result_Context$ (resultRef AS QBNex_Result)
    Result_Context = resultRef.Context
END FUNCTION

FUNCTION Result_Source$ (resultRef AS QBNex_Result)
    Result_Source = resultRef.Source
END FUNCTION

FUNCTION Result_Cause$ (resultRef AS QBNex_Result)
    Result_Cause = resultRef.Cause
END FUNCTION

FUNCTION Result_UnwrapOr$ (resultRef AS QBNex_Result, defaultValue AS STRING)
    Result_UnwrapOr$ = Result_Value$(resultRef, defaultValue)
END FUNCTION

FUNCTION Result_ErrorChain$ (resultRef AS QBNex_Result)
    DIM description AS STRING
    DIM codeText AS STRING

    description = Result_TrimText$(resultRef.Message)
    codeText = ""

    IF resultRef.Code <> QBNEX_RESULT_CODE_OK THEN codeText = "E" + LTRIM$(STR$(resultRef.Code))

    IF codeText <> "" THEN
        IF description = "" THEN
            description = "[" + codeText + "]"
        ELSE
            description = "[" + codeText + "] " + description
        END IF
    END IF

    IF Result_TrimText$(resultRef.Context) <> "" THEN
        IF description = "" THEN
            description = Result_TrimText$(resultRef.Context)
        ELSE
            description = Result_TrimText$(resultRef.Context) + ": " + description
        END IF
    END IF

    IF Result_TrimText$(resultRef.Source) <> "" THEN
        IF description = "" THEN
            description = "source=" + Result_TrimText$(resultRef.Source)
        ELSE
            description = description + " [source=" + Result_TrimText$(resultRef.Source) + "]"
        END IF
    END IF

    IF Result_TrimText$(resultRef.Cause) <> "" THEN
        IF description = "" THEN
            description = "cause: " + Result_TrimText$(resultRef.Cause)
        ELSE
            description = description + " | cause: " + Result_TrimText$(resultRef.Cause)
        END IF
    END IF

    Result_ErrorChain$ = description
END FUNCTION

FUNCTION Result_Describe$ (resultRef AS QBNex_Result)
    IF resultRef.Ok THEN
        Result_Describe$ = "ok: " + resultRef.Value
    ELSE
        Result_Describe$ = Result_ErrorChain$(resultRef)
    END IF
END FUNCTION

FUNCTION Result_Expect$ (resultRef AS QBNex_Result, expectationText AS STRING)
    IF resultRef.Ok THEN
        Result_Expect$ = resultRef.Value
        EXIT FUNCTION
    END IF

    expectationText = Result_TrimText$(expectationText)
    IF expectationText = "" THEN expectationText = "called Result_Expect on an error"

    PRINT "panic: "; expectationText
    PRINT "error: "; Result_ErrorChain$(resultRef)
    SYSTEM 1
END FUNCTION
