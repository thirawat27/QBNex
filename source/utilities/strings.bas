' String Extensions

' Please reserve this module for only string methods that can be 
' utilized outside of QB64. Meaning, anyone could add this to their
' personal project and leverage the same functionality.

' Removes a string pattern from an existing string (case-insensitive).
FUNCTION StrRemove$ (myString$, whatToRemove$)
    DIM a$, b$
    DIM AS LONG i
    a$ = myString$
    b$ = LCASE$(whatToRemove$)
    i = INSTR(LCASE$(a$), b$)
    DO WHILE i
        a$ = LEFT$(a$, i - 1) + RIGHT$(a$, LEN(a$) - i - LEN(b$) + 1)
        i = INSTR(LCASE$(a$), b$)
    LOOP
    StrRemove$ = a$
END FUNCTION

' Replaces a string pattern within an existing string (case-insensitive).
FUNCTION StrReplace$ (myString$, find$, replaceWith$)
    DIM a$, b$
    DIM AS LONG basei, i
    IF LEN(myString$) = 0 THEN EXIT FUNCTION
    a$ = myString$
    b$ = LCASE$(find$)
    basei = 1
    i = INSTR(basei, LCASE$(a$), b$)
    DO WHILE i
        a$ = LEFT$(a$, i - 1) + replaceWith$ + RIGHT$(a$, LEN(a$) - i - LEN(b$) + 1)
        basei = i + LEN(replaceWith$)
        i = INSTR(basei, LCASE$(a$), b$)
    LOOP
    StrReplace$ = a$
END FUNCTION

' Adds quotation (ASCII 034) marks around a string.
FUNCTION AddQuotes$ (s$)
    AddQuotes$ = CHR$(34) + s$ + CHR$(34)
END FUNCTION