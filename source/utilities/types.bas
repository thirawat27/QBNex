FUNCTION symboltype (s$) 'returns type or 0(not a valid symbol)
    'note: sets symboltype_size for fixed length strings
    'created: 2011 (fast & comprehensive)
    IF LEN(s$) = 0 THEN EXIT FUNCTION
    'treat common cases first
    a = ASC(s$)
    l = LEN(s$)
    IF a = 37 THEN '%
    IF l = 1 THEN symboltype = 16: EXIT FUNCTION
    IF l > 2 THEN EXIT FUNCTION
    IF ASC(s$, 2) = 37 THEN symboltype = 8: EXIT FUNCTION
    IF ASC(s$, 2) = 38 THEN symboltype = OFFSETTYPE - ISPOINTER: EXIT FUNCTION '%&
    EXIT FUNCTION
END IF
IF a = 38 THEN '&
IF l = 1 THEN symboltype = 32: EXIT FUNCTION
IF l > 2 THEN EXIT FUNCTION
IF ASC(s$, 2) = 38 THEN symboltype = 64: EXIT FUNCTION
EXIT FUNCTION
END IF
IF a = 33 THEN '!
IF l = 1 THEN symboltype = 32 + ISFLOAT: EXIT FUNCTION
EXIT FUNCTION
END IF
IF a = 35 THEN '#
IF l = 1 THEN symboltype = 64 + ISFLOAT: EXIT FUNCTION
IF l > 2 THEN EXIT FUNCTION
IF ASC(s$, 2) = 35 THEN symboltype = 64 + ISFLOAT: EXIT FUNCTION
EXIT FUNCTION
END IF
IF a = 36 THEN '$
IF l = 1 THEN symboltype = ISSTRING: EXIT FUNCTION
IF isuinteger(RIGHT$(s$, l - 1)) THEN
    IF l >= (1 + 10) THEN
        IF l > (1 + 10) THEN EXIT FUNCTION
        IF s$ > "$2147483647" THEN EXIT FUNCTION
    END IF
    symboltype_size = VAL(RIGHT$(s$, l - 1))
    symboltype = ISSTRING + ISFIXEDLENGTH
    EXIT FUNCTION
END IF
EXIT FUNCTION
END IF
IF a = 96 THEN '`
IF l = 1 THEN symboltype = 1 + ISOFFSETINBITS: EXIT FUNCTION
IF isuinteger(RIGHT$(s$, l - 1)) THEN
    IF l > 3 THEN EXIT FUNCTION
    n = VAL(RIGHT$(s$, l - 1))
    IF n > 64 THEN EXIT FUNCTION
    symboltype = n + ISOFFSETINBITS: EXIT FUNCTION
END IF
EXIT FUNCTION
END IF
IF a = 126 THEN '~
IF l = 1 THEN EXIT FUNCTION
a = ASC(s$, 2)
IF a = 37 THEN '%
IF l = 2 THEN symboltype = 16 + ISUNSIGNED: EXIT FUNCTION
IF l > 3 THEN EXIT FUNCTION
IF ASC(s$, 3) = 37 THEN symboltype = 8 + ISUNSIGNED: EXIT FUNCTION
IF ASC(s$, 3) = 38 THEN symboltype = UOFFSETTYPE - ISPOINTER: EXIT FUNCTION '~%&
EXIT FUNCTION
END IF
IF a = 38 THEN '&
IF l = 2 THEN symboltype = 32 + ISUNSIGNED: EXIT FUNCTION
IF l > 3 THEN EXIT FUNCTION
IF ASC(s$, 3) = 38 THEN symboltype = 64 + ISUNSIGNED: EXIT FUNCTION
EXIT FUNCTION
END IF
IF a = 96 THEN '`
IF l = 2 THEN symboltype = 1 + ISOFFSETINBITS + ISUNSIGNED: EXIT FUNCTION
IF isuinteger(RIGHT$(s$, l - 2)) THEN
    IF l > 4 THEN EXIT FUNCTION
    n = VAL(RIGHT$(s$, l - 2))
    IF n > 64 THEN EXIT FUNCTION
    symboltype = n + ISOFFSETINBITS + ISUNSIGNED: EXIT FUNCTION
END IF
EXIT FUNCTION
END IF
END IF '~
END FUNCTION

FUNCTION removesymbol$ (varname$)
    i = INSTR(varname$, "~"): IF i THEN GOTO foundsymbol
    i = INSTR(varname$, "`"): IF i THEN GOTO foundsymbol
    i = INSTR(varname$, "%"): IF i THEN GOTO foundsymbol
    i = INSTR(varname$, "&"): IF i THEN GOTO foundsymbol
    i = INSTR(varname$, "!"): IF i THEN GOTO foundsymbol
    i = INSTR(varname$, "#"): IF i THEN GOTO foundsymbol
    i = INSTR(varname$, "$"): IF i THEN GOTO foundsymbol
    EXIT FUNCTION
    foundsymbol:
    IF i = 1 THEN Give_Error "Expected variable name before symbol": EXIT FUNCTION
    symbol$ = RIGHT$(varname$, LEN(varname$) - i + 1)
    IF symboltype(symbol$) = 0 THEN Give_Error "Invalid symbol": EXIT FUNCTION
    removesymbol$ = symbol$
    varname$ = LEFT$(varname$, i - 1)
END FUNCTION

FUNCTION scope$
    IF id.share THEN scope$ = module$ + "__": EXIT FUNCTION
    scope$ = module$ + "_" + subfunc$ + "_"
END FUNCTION

FUNCTION typ2ctyp$ (t AS LONG, tstr AS STRING)
    ctyp$ = ""
    'typ can be passed as either: (the unused value is ignored)
    'i. as a typ value in t
    'ii. as a typ symbol (eg. "~%") in tstr
    'iii. as a typ name (eg. _UNSIGNED INTEGER) in tstr
    IF tstr$ = "" THEN
        IF (t AND ISARRAY) THEN EXIT FUNCTION 'cannot return array types
        IF (t AND ISSTRING) THEN typ2ctyp$ = "qbs": EXIT FUNCTION
        b = t AND 511
        IF (t AND ISUDT) THEN typ2ctyp$ = "void": EXIT FUNCTION
        IF (t AND ISOFFSETINBITS) THEN
            IF b <= 32 THEN ctyp$ = "int32" ELSE ctyp$ = "int64"
            IF (t AND ISUNSIGNED) THEN ctyp$ = "u" + ctyp$
            typ2ctyp$ = ctyp$: EXIT FUNCTION
        END IF
        IF (t AND ISFLOAT) THEN
            IF b = 32 THEN ctyp$ = "float"
            IF b = 64 THEN ctyp$ = "double"
            IF b = 256 THEN ctyp$ = "long double"
        ELSE
            IF b = 8 THEN ctyp$ = "int8"
            IF b = 16 THEN ctyp$ = "int16"
            IF b = 32 THEN ctyp$ = "int32"
            IF b = 64 THEN ctyp$ = "int64"
            IF t AND ISOFFSET THEN ctyp$ = "ptrszint"
            IF (t AND ISUNSIGNED) THEN ctyp$ = "u" + ctyp$
        END IF
        IF t AND ISOFFSET THEN
            ctyp$ = "ptrszint": IF (t AND ISUNSIGNED) THEN ctyp$ = "uptrszint"
        END IF
        typ2ctyp$ = ctyp$: EXIT FUNCTION
    END IF

    ts$ = tstr$
    'is ts$ a symbol?
    IF ts$ = "$" THEN ctyp$ = "qbs"
    IF ts$ = "!" THEN ctyp$ = "float"
    IF ts$ = "#" THEN ctyp$ = "double"
    IF ts$ = "##" THEN ctyp$ = "long double"
    IF LEFT$(ts$, 1) = "~" THEN unsgn = 1: ts$ = RIGHT$(ts$, LEN(ts$) - 1)
    IF LEFT$(ts$, 1) = "`" THEN
        n$ = RIGHT$(ts$, LEN(ts$) - 1)
        b = 1
        IF n$ <> "" THEN
            IF isuinteger(n$) = 0 THEN Give_Error "Invalid index after _BIT type": EXIT FUNCTION
            b = VAL(n$)
            IF b > 64 THEN Give_Error "Invalid index after _BIT type": EXIT FUNCTION
        END IF
        IF b <= 32 THEN ctyp$ = "int32" ELSE ctyp$ = "int64"
        IF unsgn THEN ctyp$ = "u" + ctyp$
        typ2ctyp$ = ctyp$: EXIT FUNCTION
    END IF
    IF ts$ = "%&" THEN
        typ2ctyp$ = "ptrszint": IF (t AND ISUNSIGNED) THEN typ2ctyp$ = "uptrszint"
        EXIT FUNCTION
    END IF
    IF ts$ = "%%" THEN ctyp$ = "int8"
    IF ts$ = "%" THEN ctyp$ = "int16"
    IF ts$ = "&" THEN ctyp$ = "int32"
    IF ts$ = "&&" THEN ctyp$ = "int64"
    IF ctyp$ <> "" THEN
        IF unsgn THEN ctyp$ = "u" + ctyp$
        typ2ctyp$ = ctyp$: EXIT FUNCTION
    END IF
    'is tstr$ a named type? (eg. 'LONG')
    s$ = type2symbol$(tstr$)
    IF Error_Happened THEN EXIT FUNCTION
    IF LEN(s$) THEN
        typ2ctyp$ = typ2ctyp$(0, s$)
        IF Error_Happened THEN EXIT FUNCTION
        EXIT FUNCTION
    END IF

    Give_Error "Invalid type": EXIT FUNCTION

END FUNCTION

FUNCTION type2symbol$ (typ$)
    t$ = typ$
    FOR i = 1 TO LEN(t$)
        IF MID$(t$, i, 1) = sp THEN MID$(t$, i, 1) = " "
    NEXT
    e$ = "Cannot convert type (" + typ$ + ") to symbol"
    t2$ = "_UNSIGNED _BIT": s$ = "~`1": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "_UNSIGNED _BYTE": s$ = "~%%": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "_UNSIGNED LONG": s$ = "~&": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "_UNSIGNED _INTEGER64": s$ = "~&&": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "_UNSIGNED INTEGER": s$ = "~%": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "_UNSIGNED _OFFSET": s$ = "~%&": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "_BIT": s$ = "`1": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "_BYTE": s$ = "%%": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "LONG": s$ = "&": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "_INTEGER64": s$ = "&&": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "_OFFSET": s$ = "%&": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "SINGLE": s$ = "!": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "DOUBLE": s$ = "#": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "_FLOAT": s$ = "##": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "STRING": s$ = "$": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "UNSIGNED BIT": s$ = "~`1": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "UNSIGNED BYTE": s$ = "~%%": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "UNSIGNED LONG": s$ = "~&": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "UNSIGNED INTEGER64": s$ = "~&&": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "UNSIGNED INTEGER": s$ = "~%": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "UNSIGNED OFFSET": s$ = "~%&": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "UNSIGNED _BIT": s$ = "~`1": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "UNSIGNED _BYTE": s$ = "~%%": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "UNSIGNED _INTEGER64": s$ = "~&&": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "UNSIGNED _OFFSET": s$ = "~%&": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "_UNSIGNED BIT": s$ = "~`1": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "_UNSIGNED BYTE": s$ = "~%%": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "_UNSIGNED INTEGER64": s$ = "~&&": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "_UNSIGNED OFFSET": s$ = "~%&": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "BIT": s$ = "`1": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "BYTE": s$ = "%%": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "INTEGER64": s$ = "&&": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "INTEGER": s$ = "%": IF LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "OFFSET": s$ = "%&": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    t2$ = "FLOAT": s$ = "##": IF qbnexprefix_set = 1 AND LEFT$(t$, LEN(t2$)) = t2$ THEN GOTO t2sfound
    Give_Error e$: EXIT FUNCTION
    t2sfound:
    type2symbol$ = s$
    IF LEN(t2$) <> LEN(t$) THEN
        IF s$ <> "$" AND s$ <> "~`1" AND s$ <> "`1" THEN Give_Error e$: EXIT FUNCTION
        t$ = RIGHT$(t$, LEN(t$) - LEN(t2$))
        IF LEFT$(t$, 3) <> " * " THEN Give_Error e$: EXIT FUNCTION
        t$ = RIGHT$(t$, LEN(t$) - 3)
        IF isuinteger(t$) = 0 THEN Give_Error e$: EXIT FUNCTION
        v = VAL(t$)
        IF v = 0 THEN Give_Error e$: EXIT FUNCTION
        IF s$ <> "$" AND v > 64 THEN Give_Error e$: EXIT FUNCTION
        IF s$ = "$" THEN
            s$ = s$ + str2$(v)
        ELSE
            s$ = LEFT$(s$, LEN(s$) - 1) + str2$(v)
        END IF
        type2symbol$ = s$
    END IF
END FUNCTION

'Strips away bits/indentifiers which make locating a variables source difficult
FUNCTION typecomp (typ)
    typ2 = typ
    IF (typ2 AND ISINCONVENTIONALMEMORY) THEN typ2 = typ2 - ISINCONVENTIONALMEMORY
    typecomp = typ2
END FUNCTION

FUNCTION typname2typ& (t2$)
    typname2typsize = 0 'the default

    t$ = t2$

    'symbol?
    ts$ = t$
    IF ts$ = "$" THEN typname2typ& = STRINGTYPE: EXIT FUNCTION
    IF ts$ = "!" THEN typname2typ& = SINGLETYPE: EXIT FUNCTION
    IF ts$ = "#" THEN typname2typ& = DOUBLETYPE: EXIT FUNCTION
    IF ts$ = "##" THEN typname2typ& = FLOATTYPE: EXIT FUNCTION

    'fixed length string?
    IF LEFT$(ts$, 1) = "$" THEN
        n$ = RIGHT$(ts$, LEN(ts$) - 1)
        IF isuinteger(n$) = 0 THEN Give_Error "Invalid index after STRING * type": EXIT FUNCTION
        b = VAL(n$)
        IF b = 0 THEN Give_Error "Invalid index after STRING * type": EXIT FUNCTION
        typname2typsize = b
        typname2typ& = STRINGTYPE + ISFIXEDLENGTH
        EXIT FUNCTION
    END IF

    'unsigned?
    IF LEFT$(ts$, 1) = "~" THEN unsgn = 1: ts$ = RIGHT$(ts$, LEN(ts$) - 1)

    'bit-type?
    IF LEFT$(ts$, 1) = "`" THEN
        n$ = RIGHT$(ts$, LEN(ts$) - 1)
        b = 1
        IF n$ <> "" THEN
            IF isuinteger(n$) = 0 THEN Give_Error "Invalid index after _BIT type": EXIT FUNCTION
            b = VAL(n$)
            IF b > 64 THEN Give_Error "Invalid index after _BIT type": EXIT FUNCTION
        END IF
        IF unsgn THEN typname2typ& = UBITTYPE + (b - 1) ELSE typname2typ& = BITTYPE + (b - 1)
        EXIT FUNCTION
    END IF

    t = 0
    IF ts$ = "%%" THEN t = BYTETYPE
    IF ts$ = "%" THEN t = INTEGERTYPE
    IF ts$ = "&" THEN t = LONGTYPE
    IF ts$ = "&&" THEN t = INTEGER64TYPE
    IF ts$ = "%&" THEN t = OFFSETTYPE

    IF t THEN
        IF unsgn THEN t = t + ISUNSIGNED
        typname2typ& = t: EXIT FUNCTION
    END IF
    'not a valid symbol

    'type name?
    FOR i = 1 TO LEN(t$)
        IF MID$(t$, i, 1) = sp THEN MID$(t$, i, 1) = " "
    NEXT
    IF t$ = "STRING" THEN typname2typ& = STRINGTYPE: EXIT FUNCTION

    IF LEFT$(t$, 9) = "STRING * " THEN

        n$ = RIGHT$(t$, LEN(t$) - 9)

        'constant check 2011
        hashfound = 0
        hashname$ = n$
        hashchkflags = HASHFLAG_CONSTANT
        hashres = HashFindRev(hashname$, hashchkflags, hashresflags, hashresref)
        DO WHILE hashres
            IF constsubfunc(hashresref) = subfuncn OR constsubfunc(hashresref) = 0 THEN
                IF constdefined(hashresref) THEN
                    hashfound = 1
                    EXIT DO
                END IF
            END IF
            IF hashres <> 1 THEN hashres = HashFindCont(hashresflags, hashresref) ELSE hashres = 0
        LOOP
        IF hashfound THEN
            i2 = hashresref
            t = consttype(i2)
            IF t AND ISSTRING THEN Give_Error "Expected STRING * numeric-constant": EXIT FUNCTION
            'convert value to general formats
            IF t AND ISFLOAT THEN
                v## = constfloat(i2)
                v&& = v##
                v~&& = v&&
            ELSE
                IF t AND ISUNSIGNED THEN
                    v~&& = constuinteger(i2)
                    v&& = v~&&
                    v## = v&&
                ELSE
                    v&& = constinteger(i2)
                    v## = v&&
                    v~&& = v&&
                END IF
            END IF
            IF v&& < 1 OR v&& > 9999999999 THEN Give_Error "STRING * out-of-range constant": EXIT FUNCTION
            b = v&&
            GOTO constantlenstr
        END IF

        IF isuinteger(n$) = 0 OR LEN(n$) > 10 THEN Give_Error "Invalid number/constant after STRING * type": EXIT FUNCTION
        b = VAL(n$)
        IF b = 0 OR LEN(n$) > 10 THEN Give_Error "Invalid number after STRING * type": EXIT FUNCTION
        constantlenstr:
        typname2typsize = b
        typname2typ& = STRINGTYPE + ISFIXEDLENGTH
        EXIT FUNCTION
    END IF

    IF t$ = "SINGLE" THEN typname2typ& = SINGLETYPE: EXIT FUNCTION
    IF t$ = "DOUBLE" THEN typname2typ& = DOUBLETYPE: EXIT FUNCTION
    IF t$ = "_FLOAT" OR (t$ = "FLOAT" AND qbnexprefix_set = 1) THEN typname2typ& = FLOATTYPE: EXIT FUNCTION
    IF LEFT$(t$, 10) = "_UNSIGNED " OR (LEFT$(t$, 9) = "UNSIGNED " AND qbnexprefix_set = 1) THEN
        u = 1
        t$ = MID$(t$, INSTR(t$, CHR$(32)) + 1)
    END IF
    IF LEFT$(t$, 4) = "_BIT" OR (LEFT$(t$, 3) = "BIT" AND qbnexprefix_set = 1) THEN
        IF t$ = "_BIT" OR (t$ = "BIT" AND qbnexprefix_set = 1) THEN
            IF u THEN typname2typ& = UBITTYPE ELSE typname2typ& = BITTYPE
            EXIT FUNCTION
        END IF
        IF LEFT$(t$, 7) <> "_BIT * " OR (LEFT$(t$, 6) = "BIT * " AND qbnexprefix_set = 1) THEN Give_Error "Expected _BIT * number": EXIT FUNCTION

        n$ = RIGHT$(t$, LEN(t$) - 7)
        IF isuinteger(n$) = 0 THEN Give_Error "Invalid size after " + qbnexprefix$ + "BIT *": EXIT FUNCTION
        b = VAL(n$)
        IF b = 0 OR b > 64 THEN Give_Error "Invalid size after " + qbnexprefix$ + "BIT *": EXIT FUNCTION
        t = BITTYPE - 1 + b: IF u THEN t = t + ISUNSIGNED
        typname2typ& = t
        EXIT FUNCTION
    END IF

    t = 0
    IF t$ = "_BYTE" OR (t$ = "BYTE" AND qbnexprefix_set = 1) THEN t = BYTETYPE
    IF t$ = "INTEGER" THEN t = INTEGERTYPE
    IF t$ = "LONG" THEN t = LONGTYPE
    IF t$ = "_INTEGER64" OR (t$ = "INTEGER64" AND qbnexprefix_set = 1) THEN t = INTEGER64TYPE
    IF t$ = "_OFFSET" OR (t$ = "OFFSET" AND qbnexprefix_set = 1) THEN t = OFFSETTYPE
    IF t THEN
        IF u THEN t = t + ISUNSIGNED
        typname2typ& = t
        EXIT FUNCTION
    END IF
    IF u THEN EXIT FUNCTION '_UNSIGNED (nothing)

    'UDT?
    FOR i = 1 TO lasttype
        IF t$ = RTRIM$(udtxname(i)) THEN
            typname2typ& = ISUDT + ISPOINTER + i
            EXIT FUNCTION
        ELSEIF RTRIM$(udtxname(i)) = "_MEM" AND t$ = "MEM" AND qbnexprefix_set = 1 THEN
            typname2typ& = ISUDT + ISPOINTER + i
            EXIT FUNCTION
        END IF
    NEXT

    'return 0 (failed)
END FUNCTION

FUNCTION typevalue2symbol$ (t)

    IF t AND ISSTRING THEN
        IF t AND ISFIXEDLENGTH THEN Give_Error "Cannot convert expression type to symbol": EXIT FUNCTION
        typevalue2symbol$ = "$"
        EXIT FUNCTION
    END IF

    s$ = ""

    IF t AND ISUNSIGNED THEN s$ = "~"

    b = t AND 511

    IF t AND ISOFFSETINBITS THEN
        IF b > 1 THEN s$ = s$ + "`" + str2$(b) ELSE s$ = s$ + "`"
        typevalue2symbol$ = s$
        EXIT FUNCTION
    END IF

    IF t AND ISFLOAT THEN
        IF b = 32 THEN s$ = "!"
        IF b = 64 THEN s$ = "#"
        IF b = 256 THEN s$ = "##"
        typevalue2symbol$ = s$
        EXIT FUNCTION
    END IF

    IF b = 8 THEN s$ = s$ + "%%"
    IF b = 16 THEN s$ = s$ + "%"
    IF b = 32 THEN s$ = s$ + "&"
    IF b = 64 THEN s$ = s$ + "&&"
    typevalue2symbol$ = s$

END FUNCTION

FUNCTION id2fulltypename$
    t = id.t
    IF t = 0 THEN t = id.arraytype
    size = id.tsize
    bits = t AND 511
    IF t AND ISUDT THEN
        a$ = RTRIM$(udtxcname(t AND 511))
        id2fulltypename$ = a$: EXIT FUNCTION
    END IF
    IF t AND ISSTRING THEN
        IF t AND ISFIXEDLENGTH THEN a$ = "STRING * " + str2(size) ELSE a$ = "STRING"
        id2fulltypename$ = a$: EXIT FUNCTION
    END IF
    IF t AND ISOFFSETINBITS THEN
        IF bits > 1 THEN a$ = qbnexprefix$ + "BIT * " + str2(bits) ELSE a$ = qbnexprefix$ + "BIT"
        IF t AND ISUNSIGNED THEN a$ = qbnexprefix$ + "UNSIGNED " + a$
        id2fulltypename$ = a$: EXIT FUNCTION
    END IF
    IF t AND ISFLOAT THEN
        IF bits = 32 THEN a$ = "SINGLE"
        IF bits = 64 THEN a$ = "DOUBLE"
        IF bits = 256 THEN a$ = qbnexprefix$ + "FLOAT"
    ELSE 'integer-based
        IF bits = 8 THEN a$ = qbnexprefix$ + "BYTE"
        IF bits = 16 THEN a$ = "INTEGER"
        IF bits = 32 THEN a$ = "LONG"
        IF bits = 64 THEN a$ = qbnexprefix$ + "INTEGER64"
        IF t AND ISUNSIGNED THEN a$ = qbnexprefix$ + "UNSIGNED " + a$
    END IF
    IF t AND ISOFFSET THEN
        a$ = qbnexprefix$ + "OFFSET"
        IF t AND ISUNSIGNED THEN a$ = qbnexprefix$ + "UNSIGNED " + a$
    END IF
    id2fulltypename$ = a$
END FUNCTION

FUNCTION id2shorttypename$
    t = id.t
    IF t = 0 THEN t = id.arraytype
    size = id.tsize
    bits = t AND 511
    IF t AND ISUDT THEN
        a$ = RTRIM$(udtxcname(t AND 511))
        id2shorttypename$ = a$: EXIT FUNCTION
    END IF
    IF t AND ISSTRING THEN
        IF t AND ISFIXEDLENGTH THEN a$ = "STRING" + str2(size) ELSE a$ = "STRING"
        id2shorttypename$ = a$: EXIT FUNCTION
    END IF
    IF t AND ISOFFSETINBITS THEN
        IF t AND ISUNSIGNED THEN a$ = "_U" ELSE a$ = "_"
        IF bits > 1 THEN a$ = a$ + "BIT" + str2(bits) ELSE a$ = a$ + "BIT1"
        id2shorttypename$ = a$: EXIT FUNCTION
    END IF
    IF t AND ISFLOAT THEN
        IF bits = 32 THEN a$ = "SINGLE"
        IF bits = 64 THEN a$ = "DOUBLE"
        IF bits = 256 THEN a$ = "_FLOAT"
    ELSE 'integer-based
        IF bits = 8 THEN
            IF (t AND ISUNSIGNED) THEN a$ = "_UBYTE" ELSE a$ = "_BYTE"
        END IF
        IF bits = 16 THEN
            IF (t AND ISUNSIGNED) THEN a$ = "UINTEGER" ELSE a$ = "INTEGER"
        END IF
        IF bits = 32 THEN
            IF (t AND ISUNSIGNED) THEN a$ = "ULONG" ELSE a$ = "LONG"
        END IF
        IF bits = 64 THEN
            IF (t AND ISUNSIGNED) THEN a$ = "_UINTEGER64" ELSE a$ = "_INTEGER64"
        END IF
    END IF
    id2shorttypename$ = a$
END FUNCTION

FUNCTION symbol2fulltypename$ (s2$)
    'note: accepts both symbols and type names
    s$ = s2$

    IF LEFT$(s$, 1) = "~" THEN
        u = 1
        IF LEN(typ$) = 1 THEN Give_Error "Expected ~...": EXIT FUNCTION
        s$ = RIGHT$(s$, LEN(s$) - 1)
        u$ = qbnexprefix$ + "UNSIGNED "
    END IF

    IF s$ = "%%" THEN t$ = u$ + qbnexprefix$ + "BYTE": GOTO gotsym2typ
    IF s$ = "%" THEN t$ = u$ + "INTEGER": GOTO gotsym2typ
    IF s$ = "&" THEN t$ = u$ + "LONG": GOTO gotsym2typ
    IF s$ = "&&" THEN t$ = u$ + qbnexprefix$ + "INTEGER64": GOTO gotsym2typ
    IF s$ = "%&" THEN t$ = u$ + qbnexprefix$ + "OFFSET": GOTO gotsym2typ

    IF LEFT$(s$, 1) = "`" THEN
        IF LEN(s$) = 1 THEN
            t$ = u$ + qbnexprefix$ + "BIT * 1"
            GOTO gotsym2typ
        END IF
        n$ = RIGHT$(s$, LEN(s$) - 1)
        IF isuinteger(n$) = 0 THEN Give_Error "Expected number after symbol `": EXIT FUNCTION
        t$ = u$ + qbnexprefix$ + "BIT * " + n$
        GOTO gotsym2typ
    END IF

    IF u = 1 THEN Give_Error "Expected type symbol after ~": EXIT FUNCTION

    IF s$ = "!" THEN t$ = "SINGLE": GOTO gotsym2typ
    IF s$ = "#" THEN t$ = "DOUBLE": GOTO gotsym2typ
    IF s$ = "##" THEN t$ = qbnexprefix$ + "FLOAT": GOTO gotsym2typ
    IF s$ = "$" THEN t$ = "STRING": GOTO gotsym2typ

    IF LEFT$(s$, 1) = "$" THEN
        n$ = RIGHT$(s$, LEN(s$) - 1)
        IF isuinteger(n$) = 0 THEN Give_Error "Expected number after symbol $": EXIT FUNCTION
        t$ = "STRING * " + n$
        GOTO gotsym2typ
    END IF

    t$ = s$

    gotsym2typ:

    IF RIGHT$(" " + t$, 5) = " _BIT" THEN t$ = t$ + " * 1" 'clarify (_UNSIGNED) _BIT as (_UNSIGNED) _BIT * 1

    FOR i = 1 TO LEN(t$)
        IF ASC(t$, i) = ASC(sp) THEN ASC(t$, i) = 32
    NEXT

    symbol2fulltypename$ = t$

END FUNCTION
