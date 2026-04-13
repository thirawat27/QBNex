' ============================================================================
' QBNex Method Chain Regression
' ============================================================================

'$IMPORT:'qbnex'

CLASS Dog
    Name AS STRING * 32

    CONSTRUCTOR (petName AS STRING)
        ME.Name = petName
    END CONSTRUCTOR

    FUNCTION Describe$ ()
        Describe$ = RTRIM$(ME.Name)
    END FUNCTION
END CLASS

SUB MethodChainRegression ()
    DIM pet AS Dog

    __QBNEX_Dog_CTOR pet, "Buddy"
    PRINT (pet).Describe$()
END SUB

MethodChainRegression
