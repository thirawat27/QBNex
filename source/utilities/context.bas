FUNCTION IsOctalText% (text AS STRING)
    DIM i AS LONG

    IF LEN(text) <> 3 THEN
        IsOctalText% = 0
        EXIT FUNCTION
    END IF

    FOR i = 1 TO 3
        IF ASC(text, i) < 48 OR ASC(text, i) > 55 THEN
            IsOctalText% = 0
            EXIT FUNCTION
        END IF
    NEXT

    IsOctalText% = -1
END FUNCTION

FUNCTION OctalTextValue% (text AS STRING)
    DIM i AS LONG
    DIM value AS INTEGER

    value = 0
    FOR i = 1 TO LEN(text)
        value = value * 8 + (ASC(text, i) - 48)
    NEXT

    OctalTextValue% = value
END FUNCTION

FUNCTION CleanDiagnosticContext$ (text$)
    DIM result AS STRING
    DIM i AS LONG
    DIM j AS LONG
    DIM ch AS STRING
    DIM octal AS STRING
    DIM hadCompilerEscapes AS _BYTE

    result = ""
    i = 1

    DO WHILE i <= LEN(text$)
        ch = MID$(text$, i, 1)

        IF ch = sp$ THEN
            result = result + " "
            i = i + 1
        ELSEIF ch = CHR$(34) THEN
            result = result + ch
            i = i + 1
            hadCompilerEscapes = 0

            DO WHILE i <= LEN(text$)
                ch = MID$(text$, i, 1)

                IF ch = CHR$(34) THEN
                    result = result + ch
                    i = i + 1
                    EXIT DO
                ELSEIF ch = "\" AND i + 3 <= LEN(text$) THEN
                    octal = MID$(text$, i + 1, 3)
                    IF IsOctalText%(octal) THEN
                        result = result + CHR$(OctalTextValue%(octal))
                        hadCompilerEscapes = -1
                        i = i + 4
                    ELSEIF MID$(text$, i + 1, 1) = "\" THEN
                        result = result + "\"
                        hadCompilerEscapes = -1
                        i = i + 2
                    ELSE
                        result = result + ch
                        i = i + 1
                    END IF
                ELSE
                    IF ch = sp$ THEN ch = " "
                    result = result + ch
                    i = i + 1
                END IF
            LOOP

            IF hadCompilerEscapes THEN
                IF i <= LEN(text$) AND MID$(text$, i, 1) = "," THEN
                    j = i + 1
                    DO WHILE j <= LEN(text$)
                        IF IsDigitChar%(MID$(text$, j, 1)) = 0 THEN EXIT DO
                        j = j + 1
                    LOOP
                    IF j > i + 1 THEN i = j
                END IF
            END IF
        ELSE
            result = result + ch
            i = i + 1
        END IF
    LOOP

    CleanDiagnosticContext$ = RTRIM$(result)
END FUNCTION
