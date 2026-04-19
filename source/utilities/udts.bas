SUB initialise_udt_varstrings (n$, udt, file, base_offset)
    IF NOT udtxvariable(udt) THEN EXIT SUB
    element = udtxnext(udt)
    offset = 0
    DO WHILE element
        IF udtetype(element) AND ISSTRING THEN
            IF (udtetype(element) AND ISFIXEDLENGTH) = 0 THEN
                PRINT #file, "*(qbs**)(((char*)" + n$ + ")+" + STR$(base_offset + offset) + ") = qbs_new(0,0);"
            END IF
        ELSEIF udtetype(element) AND ISUDT THEN
            initialise_udt_varstrings n$, udtetype(element) AND 511, file, offset
        END IF
        offset = offset + udtesize(element) \ 8
        element = udtenext(element)
    LOOP
END SUB

SUB free_udt_varstrings (n$, udt, file, base_offset)
    IF NOT udtxvariable(udt) THEN EXIT SUB
    element = udtxnext(udt)
    offset = 0
    DO WHILE element
        IF udtetype(element) AND ISSTRING THEN
            IF (udtetype(element) AND ISFIXEDLENGTH) = 0 THEN
                PRINT #file, "qbs_free(*((qbs**)(((char*)" + n$ + ")+" + STR$(base_offset + offset) + ")));"
            END IF
        ELSEIF udtetype(element) AND ISUDT THEN
            initialise_udt_varstrings n$, udtetype(element) AND 511, file, offset
        END IF
        offset = offset + udtesize(element) \ 8
        element = udtenext(element)
    LOOP
END SUB

SUB clear_udt_with_varstrings (n$, udt, file, base_offset)
    IF NOT udtxvariable(udt) THEN EXIT SUB
    element = udtxnext(udt)
    offset = 0
    DO WHILE element
        IF udtetype(element) AND ISSTRING THEN
            IF (udtetype(element) AND ISFIXEDLENGTH) = 0 THEN
                PRINT #file, "(*(qbs**)(((char*)" + n$ + ")+" + STR$(base_offset + offset) + "))->len=0;"
            ELSE
                PRINT #file, "memset((char*)" + n$ + "+" + STR$(base_offset + offset) + ",0," + STR$(udtesize(element) \ 8) + ");"
            END IF
        ELSE
            IF udtetype(element) AND ISUDT THEN
                clear_udt_with_varstrings n$, udtetype(element) AND 511, file, base_offset + offset
            ELSE
                PRINT #file, "memset((char*)" + n$ + "+" + STR$(base_offset + offset) + ",0," + STR$(udtesize(element) \ 8) + ");"
            END IF
        END IF
        offset = offset + udtesize(element) \ 8
        element = udtenext(element)
    LOOP
END SUB

SUB initialise_array_udt_varstrings (n$, udt, base_offset, bytesperelement$, acc$)
    IF NOT udtxvariable(udt) THEN EXIT SUB
    offset = base_offset
    element = udtxnext(udt)
    DO WHILE element
        IF udtetype(element) AND ISSTRING THEN
            IF (udtetype(element) AND ISFIXEDLENGTH) = 0 THEN
                acc$ = acc$ + CHR$(13) + CHR$(10) + "*(qbs**)(" + n$ + "[0]+(" + bytesperelement$ + "-1)*tmp_long+" + STR$(offset) + ")=qbs_new(0,0);"
            END IF
        ELSEIF udtetype(element) AND ISUDT THEN
            initialise_array_udt_varstrings n$, udtetype(element) AND 511, offset, bytesperelement$, acc$
        END IF
        offset = offset + udtesize(element) \ 8
        element = udtenext(element)
    LOOP
END SUB

SUB free_array_udt_varstrings (n$, udt, base_offset, bytesperelement$, acc$)
    IF NOT udtxvariable(udt) THEN EXIT SUB
    offset = base_offset
    element = udtxnext(udt)
    DO WHILE element
        IF udtetype(element) AND ISSTRING THEN
            IF (udtetype(element) AND ISFIXEDLENGTH) = 0 THEN
                acc$ = acc$ + CHR$(13) + CHR$(10) + "qbs_free(*(qbs**)(" + n$ + "[0]+(" + bytesperelement$ + "-1)*tmp_long+" + STR$(offset) + "));"
            END IF
        ELSEIF udtetype(element) AND ISUDT THEN
            free_array_udt_varstrings n$, udtetype(element) AND 511, offset, bytesperelement$, acc$
        END IF
        offset = offset + udtesize(element) \ 8
        element = udtenext(element)
    LOOP
END SUB

SUB copy_full_udt (dst$, src$, file, base_offset, udt)
    IF NOT udtxvariable(udt) THEN
        PRINT #file, "memcpy(" + dst$ + "+" + STR$(base_offset) + "," + src$ + "+" + STR$(base_offset) + "," + STR$(udtxsize(udt) \ 8) + ");"
        EXIT SUB
    END IF
    offset = base_offset
    element = udtxnext(udt)
    DO WHILE element
        IF ((udtetype(element) AND ISSTRING) > 0) AND (udtetype(element) AND ISFIXEDLENGTH) = 0 THEN
            PRINT #file, "qbs_set(*(qbs**)(" + dst$ + "+" + STR$(offset) + "), *(qbs**)(" + src$ + "+" + STR$(offset) + "));"
        ELSEIF ((udtetype(element) AND ISUDT) > 0) THEN
            copy_full_udt dst$, src$, 12, offset, udtetype(element) AND 511
        ELSE
            PRINT #file, "memcpy((" + dst$ + "+" + STR$(offset) + "),(" + src$ + "+" + STR$(offset) + ")," + STR$(udtesize(element) \ 8) + ");"
        END IF
        offset = offset + udtesize(element) \ 8
        element = udtenext(element)
    LOOP
END SUB

SUB dump_udts
    f = FREEFILE
    OPEN "types.txt" FOR OUTPUT AS #f
    PRINT #f, "Name   Size   Align? Next   Var?"
    FOR i = 1 TO lasttype
        PRINT #f, RTRIM$(udtxname(i)), udtxsize(i), udtxbytealign(i), udtxnext(i), udtxvariable(i)
    NEXT i
    PRINT #f, "Name   Size   Align? Next   Type   Tsize  Arr"
    FOR i = 1 TO lasttypeelement
        PRINT #f, RTRIM$(udtename(i)), udtesize(i), udtebytealign(i), udtenext(i), udtetype(i), udtetypesize(i), udtearrayelements(i)
    NEXT i
    CLOSE #f
END SUB

SUB increaseUDTArrays
    x = UBOUND(udtxname)
    REDIM _PRESERVE udtxname(x + 1000) AS STRING * 256
    REDIM _PRESERVE udtxcname(x + 1000) AS STRING * 256
    REDIM _PRESERVE udtxsize(x + 1000) AS LONG
    REDIM _PRESERVE udtxbytealign(x + 1000) AS INTEGER 'first element MUST be on a byte alignment & size is a multiple of 8
    REDIM _PRESERVE udtxnext(x + 1000) AS LONG
    REDIM _PRESERVE udtxvariable(x + 1000) AS INTEGER 'true if the udt contains variable length elements
    'elements
    REDIM _PRESERVE udtename(x + 1000) AS STRING * 256
    REDIM _PRESERVE udtecname(x + 1000) AS STRING * 256
    REDIM _PRESERVE udtebytealign(x + 1000) AS INTEGER
    REDIM _PRESERVE udtesize(x + 1000) AS LONG
    REDIM _PRESERVE udtetype(x + 1000) AS LONG
    REDIM _PRESERVE udtetypesize(x + 1000) AS LONG
    REDIM _PRESERVE udtearrayelements(x + 1000) AS LONG
    REDIM _PRESERVE udtenext(x + 1000) AS LONG
END SUB
