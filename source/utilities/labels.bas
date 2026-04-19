FUNCTION CountMatchingLabels& (labelName AS STRING)
    DIM lookupMode AS LONG
    DIM lookupResult AS LONG
    DIM lookupRef AS LONG

    CountMatchingLabels& = 0
    lookupMode = validlabel(labelName)
    lookupResult = HashFind(labelName, HASHFLAG_LABEL, lookupMode, lookupRef)

    DO WHILE lookupResult
        CountMatchingLabels& = CountMatchingLabels& + 1

        IF lookupResult = 2 THEN
            lookupResult = HashFindCont(lookupMode, lookupRef)
        ELSE
            lookupResult = 0
        END IF
    LOOP
END FUNCTION

FUNCTION ValidatePendingLabels% ()
    DIM r AS LONG
    DIM lookupMode AS LONG
    DIM lookupResult AS LONG
    DIM lookupRef AS LONG
    DIM labelName AS STRING
    DIM normalizedLabelName AS STRING

    ValidatePendingLabels% = 0
    IF Debug THEN PRINT #9, "Beginning label check..."

    FOR r = 1 TO nLabels
        IF Labels(r).Scope_Restriction THEN
            labelName = RTRIM$(Labels(r).cn)
            lookupMode = validlabel(labelName)
            lookupResult = HashFind(labelName, HASHFLAG_LABEL, lookupMode, lookupRef)

            DO WHILE lookupResult
                IF Labels(lookupRef).Scope = Labels(r).Scope_Restriction THEN
                    linenumber = Labels(r).Error_Line
                    a$ = "Common label within a SUB/FUNCTION"
                    ValidatePendingLabels% = 2
                    EXIT FUNCTION
                END IF

                IF lookupResult = 2 THEN
                    lookupResult = HashFindCont(lookupMode, lookupRef)
                ELSE
                    lookupResult = 0
                END IF
            LOOP
        END IF

        IF Labels(r).State = 0 THEN
            normalizedLabelName = UCASE$(RTRIM$(Labels(r).cn))

            IF INSTR(PossibleSubNameLabels$, sp + normalizedLabelName + sp) THEN
                IF INSTR(SubNameLabels$, sp + normalizedLabelName + sp) = 0 THEN
                    SubNameLabels$ = SubNameLabels$ + normalizedLabelName + sp
                    IF Debug THEN PRINT #9, "Recompiling to resolve label:"; RTRIM$(Labels(r).cn)
                    ValidatePendingLabels% = 1
                    EXIT FUNCTION
                END IF
            END IF

            linenumber = Labels(r).Error_Line
            a$ = "Label '" + RTRIM$(Labels(r).cn) + "' not defined"
            ValidatePendingLabels% = 2
            EXIT FUNCTION
        END IF

        IF Labels(r).Data_Referenced THEN
            labelName = RTRIM$(Labels(r).cn)
            IF CountMatchingLabels&(labelName) <> 1 THEN
                linenumber = Labels(r).Error_Line
                a$ = "Ambiguous DATA label"
                ValidatePendingLabels% = 2
                EXIT FUNCTION
            END IF

            PRINT #18, "ptrszint data_at_LABEL_" + labelName + "=" + str2(Labels(r).Data_Offset) + ";"
        END IF
    NEXT

    IF Debug THEN PRINT #9, "Finished label check!"
END FUNCTION
