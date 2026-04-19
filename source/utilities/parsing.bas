FUNCTION fixoperationorder$ (savea$)
    STATIC uboundlbound AS _BYTE

    a$ = savea$
    IF Debug THEN PRINT #9, "fixoperationorder:in:" + a$

    fooindwel = fooindwel + 1

    n = numelements(a$) 'n is maintained throughout function

    IF fooindwel = 1 THEN 'actions to take on initial call only
    uboundlbound = 0

    'Quick check for duplicate binary operations
    uppercasea$ = UCASE$(a$) 'capitalize it once to reduce calls to ucase over and over
    FOR i = 1 TO n - 1
        temp1$ = getelement(uppercasea$, i)
        temp2$ = getelement(uppercasea$, i + 1)
        IF temp1$ = "AND" AND temp2$ = "AND" THEN Give_Error "Error: AND AND": EXIT FUNCTION
        IF temp1$ = "OR" AND temp2$ = "OR" THEN Give_Error "Error: OR OR": EXIT FUNCTION
        IF temp1$ = "XOR" AND temp2$ = "XOR" THEN Give_Error "Error: XOR XOR": EXIT FUNCTION
        IF temp1$ = "IMP" AND temp2$ = "IMP" THEN Give_Error "Error: IMP IMP": EXIT FUNCTION
        IF temp1$ = "EQV" AND temp2$ = "EQV" THEN Give_Error "Error: EQV EQV": EXIT FUNCTION
    NEXT

    '----------------A. 'Quick' mismatched brackets check----------------
    b = 0
    a2$ = sp + a$ + sp
    b1$ = sp + "(" + sp
    b2$ = sp + ")" + sp
    i = 1
    findmmb:
    i1 = INSTR(i, a2$, b1$)
    i2 = INSTR(i, a2$, b2$)
    i3 = i1
    IF i2 THEN
        IF i1 = 0 THEN
            i3 = i2
        ELSE
            IF i2 < i1 THEN i3 = i2
        END IF
    END IF
    IF i3 THEN
        IF i3 = i1 THEN b = b + 1
        IF i3 = i2 THEN b = b - 1
        i = i3 + 2
        IF b < 0 THEN Give_Error "Missing (": EXIT FUNCTION
        GOTO findmmb
    END IF
    IF b > 0 THEN Give_Error "Missing )": EXIT FUNCTION

    '----------------B. 'Quick' correction of over-use of +,- ----------------
    'note: the results of this change are beneficial to foolayout
    a2$ = sp + a$ + sp

    'rule 1: change ++ to +
    rule1:
    i = INSTR(a2$, sp + "+" + sp + "+" + sp)
    IF i THEN
        a2$ = LEFT$(a2$, i + 2) + RIGHT$(a2$, LEN(a2$) - i - 4)
        a$ = MID$(a2$, 2, LEN(a2$) - 2)
        n = n - 1
        IF Debug THEN PRINT #9, "fixoperationorder:+/-:" + a$
        GOTO rule1
    END IF

    'rule 2: change -+ to -
    rule2:
    i = INSTR(a2$, sp + "-" + sp + "+" + sp)
    IF i THEN
        a2$ = LEFT$(a2$, i + 2) + RIGHT$(a2$, LEN(a2$) - i - 4)
        a$ = MID$(a2$, 2, LEN(a2$) - 2)
        n = n - 1
        IF Debug THEN PRINT #9, "fixoperationorder:+/-:" + a$
        GOTO rule2
    END IF

    'rule 3: change anyoperator-- to anyoperator
    rule3:
    IF INSTR(a2$, sp + "-" + sp + "-" + sp) THEN
        FOR i = 1 TO n - 2
            IF isoperator(getelement(a$, i)) THEN
                IF getelement(a$, i + 1) = "-" THEN
                    IF getelement(a$, i + 2) = "-" THEN
                        removeelements a$, i + 1, i + 2, 0
                        a2$ = sp + a$ + sp
                        n = n - 2
                        IF Debug THEN PRINT #9, "fixoperationorder:+/-:" + a$
                        GOTO rule3
                    END IF
                END IF
            END IF
        NEXT
    END IF 'rule 3



    '----------------C. 'Quick' location of negation----------------
    'note: the results of this change are beneficial to foolayout

    'for numbers...
    'before: anyoperator,-,number
    'after:  anyoperator,-number

    'for variables...
    'before: anyoperator,-,variable
    'after:  anyoperator,CHR$(241),variable

    'exception for numbers followed by ^... (they will be bracketed up along with the ^ later)
    'before: anyoperator,-,number,^
    'after:  anyoperator,CHR$(241),number,^

    FOR i = 1 TO n - 1
        IF i > n - 1 THEN EXIT FOR 'n changes, so manually exit if required

        IF ASC(getelement(a$, i)) = 45 THEN '-

        neg = 0
        IF i = 1 THEN
            neg = 1
        ELSE
            a2$ = getelement(a$, i - 1)
            c = ASC(a2$)
            IF c = 40 OR c = 44 THEN '(,
            neg = 1
        ELSE
            IF isoperator(a2$) THEN neg = 1
        END IF '()
    END IF 'i=1
    IF neg = 1 THEN

        a2$ = getelement(a$, i + 1)
        c = ASC(a2$)
        IF c >= 48 AND c <= 57 THEN
            c2 = 0: IF i < n - 1 THEN c2 = ASC(getelement(a$, i + 2))
            IF c2 <> 94 THEN 'not ^
            'number...
            i2 = INSTR(a2$, ",")
            IF i2 AND ASC(a2$, i2 + 1) <> 38 THEN '&H/&O/&B values don't need the assumed negation
            a2$ = "-" + LEFT$(a2$, i2) + "-" + RIGHT$(a2$, LEN(a2$) - i2)
        ELSE
            a2$ = "-" + a2$
        END IF
        removeelements a$, i, i + 1, 0
        insertelements a$, i - 1, a2$
        n = n - 1
        IF Debug THEN PRINT #9, "fixoperationorder:negation:" + a$

        GOTO negdone

    END IF
END IF


'not a number (or for exceptions)...
removeelements a$, i, i, 0
insertelements a$, i - 1, CHR$(241)
IF Debug THEN PRINT #9, "fixoperationorder:negation:" + a$

END IF 'isoperator
END IF '-
negdone:
NEXT



END IF 'fooindwel=1



'----------------D. 'Quick' Add 'power of' with negation {}bracketing to bottom bracket level----------------
pownegused = 0
powneg:
IF INSTR(a$, "^" + sp + CHR$(241)) THEN 'quick check
b = 0
b1 = 0
FOR i = 1 TO n
    a2$ = getelement(a$, i)
    c = ASC(a2$)
    IF c = 40 THEN b = b + 1
    IF c = 41 THEN b = b - 1
    IF b = 0 THEN
        IF b1 THEN
            IF isoperator(a2$) THEN
                IF a2$ <> "^" AND a2$ <> CHR$(241) THEN
                    insertelements a$, i - 1, "}"
                    insertelements a$, b1, "{"
                    n = n + 2
                    IF Debug THEN PRINT #9, "fixoperationorder:^-:" + a$
                    GOTO powneg
                    pownegused = 1
                END IF
            END IF
        END IF
        IF c = 94 THEN '^
        IF getelement$(a$, i + 1) = CHR$(241) THEN b1 = i: i = i + 1
    END IF
END IF 'b=0
NEXT i
IF b1 THEN
    insertelements a$, b1, "{"
    a$ = a$ + sp + "}"
    n = n + 2
    IF Debug THEN PRINT #9, "fixoperationorder:^-:" + a$
    pownegused = 1
    GOTO powneg
END IF

END IF 'quick check


'----------------E. Find lowest & highest operator level in bottom bracket level----------------
NOT_recheck:
lco = 255
hco = 0
b = 0
FOR i = 1 TO n
    a2$ = getelement(a$, i)
    c = ASC(a2$)
    IF c = 40 OR c = 123 THEN b = b + 1
    IF c = 41 OR c = 125 THEN b = b - 1
    IF b = 0 THEN
        op = isoperator(a2$)
        IF op THEN
            IF op < lco THEN lco = op
            IF op > hco THEN hco = op
        END IF
    END IF
NEXT

'----------------F. Add operator {}bracketting----------------
'apply bracketting only if required
IF hco <> 0 THEN 'operators were used
IF lco <> hco THEN
    'brackets needed

    IF lco = 6 THEN 'NOT exception
    'Step 1: Add brackets as follows ~~~ ( NOT ( ~~~ NOT ~~~ NOT ~~~ NOT ~~~ ))
    'Step 2: Recheck line from beginning
    IF n = 1 THEN Give_Error "Expected NOT ...": EXIT FUNCTION
    b = 0
    FOR i = 1 TO n
        a2$ = getelement(a$, i)
        c = ASC(a2$)
        IF c = 40 OR c = 123 THEN b = b + 1
        IF c = 41 OR c = 125 THEN b = b - 1
        IF b = 0 THEN
            IF UCASE$(a2$) = "NOT" THEN
                IF i = n THEN Give_Error "Expected NOT ...": EXIT FUNCTION
                IF i = 1 THEN a$ = "NOT" + sp + "{" + sp + getelements$(a$, 2, n) + sp + "}": n = n + 2: GOTO lco_bracketting_done
                a$ = getelements$(a$, 1, i - 1) + sp + "{" + sp + "NOT" + sp + "{" + sp + getelements$(a$, i + 1, n) + sp + "}" + sp + "}"
                n = n + 4
                GOTO NOT_recheck
            END IF 'not
        END IF 'b=0
    NEXT
END IF 'NOT exception

n2 = n
b = 0
a3$ = "{"
n = 1
FOR i = 1 TO n2
    a2$ = getelement(a$, i)
    c = ASC(a2$)
    IF c = 40 OR c = 123 THEN b = b + 1
    IF c = 41 OR c = 125 THEN b = b - 1
    IF b = 0 THEN
        op = isoperator(a2$)
        IF op = lco THEN
            IF i = 1 THEN
                a3$ = a2$ + sp + "{"
                n = 2
            ELSE
                IF i = n2 THEN Give_Error "Expected variable/value after '" + UCASE$(a2$) + "'": EXIT FUNCTION
                a3$ = a3$ + sp + "}" + sp + a2$ + sp + "{"
                n = n + 3
            END IF
            GOTO fixop0
        END IF

    END IF 'b=0
    a3$ = a3$ + sp + a2$
    n = n + 1
    fixop0:
NEXT
a3$ = a3$ + sp + "}"
n = n + 1
a$ = a3$

lco_bracketting_done:
IF Debug THEN PRINT #9, "fixoperationorder:lco bracketing["; lco; ","; hco; "]:" + a$

'--------(F)G. Remove indwelling {}bracketting from power-negation--------
IF pownegused THEN
    b = 0
    i = 0
    DO
        i = i + 1
        IF i > n THEN EXIT DO
        c = ASC(getelement(a$, i))
        IF c = 41 OR c = 125 THEN b = b - 1
        IF (c = 123 OR c = 125) AND b <> 0 THEN
            removeelements a$, i, i, 0
            n = n - 1
            i = i - 1
            IF Debug THEN PRINT #9, "fixoperationorder:^- {} removed:" + a$
        END IF
        IF c = 40 OR c = 123 THEN b = b + 1
    LOOP
END IF 'pownegused

END IF 'lco <> hco
END IF 'hco <> 0

'--------Bracketting of multiple NOT/negation unary operators--------
IF LEFT$(a$, 4) = CHR$(241) + sp + CHR$(241) + sp THEN
    a$ = CHR$(241) + sp + "{" + sp + getelements$(a$, 2, n) + sp + "}": n = n + 2
END IF
IF UCASE$(LEFT$(a$, 8)) = "NOT" + sp + "NOT" + sp THEN
    a$ = "NOT" + sp + "{" + sp + getelements$(a$, 2, n) + sp + "}": n = n + 2
END IF

'----------------H. Identification/conversion of elements within bottom bracket level----------------
'actions performed:
'   ->builds f$(tlayout)
'   ->adds symbols to all numbers
'   ->evaluates constants to numbers

f$ = ""
b = 0
c = 0
udtMethodObjectStart = 0
lastt = 0: lastti = 0
FOR i = 1 TO n
    f2$ = getelement(a$, i)
    lastc = c
    c = ASC(f2$)

    IF c = 40 OR c = 123 THEN
        IF c <> 40 OR b <> 0 THEN f2$ = "" 'skip temporary & indwelling  brackets
        b = b + 1
        GOTO classdone
    END IF
    IF c = 41 OR c = 125 THEN

        b = b - 1

        'check for "("+sp+")" after literal-string, operator, number or nothing
        IF b = 0 THEN 'must be within the lowest level
        IF c = 41 THEN
            IF lastc = 40 THEN
                IF lastti = i - 2 OR lastti = 0 THEN
                    IF lastt >= 0 AND lastt <= 3 THEN
                        Give_Error "Unexpected (": EXIT FUNCTION
                    END IF
                END IF
            END IF
        END IF
    END IF

    IF c <> 41 OR b <> 0 THEN f2$ = "" 'skip temporary & indwelling  brackets
    GOTO classdone
END IF

IF b = 0 THEN

    'classifications/conversions:
    '1. quoted string ("....)
    '2. number
    '3. operator
    '4. constant
    '5. variable/array/udt/function (note: nothing can share the same name as a function except a label)


    'quoted string?
    IF c = 34 THEN '"
    lastt = 1: lastti = i

    'convert \\ to \
    'convert \??? to CHR$(&O???)
    x2 = 1
    x = INSTR(x2, f2$, "\")
    DO WHILE x
        c2 = ASC(f2$, x + 1)
        IF c2 = 92 THEN '\\
        f2$ = LEFT$(f2$, x) + RIGHT$(f2$, LEN(f2$) - x - 1) 'remove second \
        x2 = x + 1
    ELSE
        'octal triplet value
        c3 = (ASC(f2$, x + 3) - 48) + (ASC(f2$, x + 2) - 48) * 8 + (ASC(f2$, x + 1) - 48) * 64
        f2$ = LEFT$(f2$, x - 1) + CHR$(c3) + RIGHT$(f2$, LEN(f2$) - x - 3)
        x2 = x + 1
    END IF
    x = INSTR(x2, f2$, "\")
LOOP
'remove ',len' (if it exists)
x = INSTR(2, f2$, CHR$(34) + ","): IF x THEN f2$ = LEFT$(f2$, x)
GOTO classdone
END IF

'number?
IF (c >= 48 AND c <= 57) OR c = 45 THEN
    lastt = 2: lastti = i

    x = INSTR(f2$, ",")
    IF x THEN
        removeelements a$, i, i, 0: insertelements a$, i - 1, LEFT$(f2$, x - 1)
        f2$ = RIGHT$(f2$, LEN(f2$) - x)
    END IF

    IF x = 0 THEN
        c2 = ASC(f2$, LEN(f2$))
        IF c2 < 48 OR c2 > 57 THEN
            x = 1 'extension given
        ELSE
            x = INSTR(f2$, "`")
        END IF
    END IF

    'add appropriate integer symbol if none present
    IF x = 0 THEN
        f3$ = f2$
        s$ = ""
        IF c = 45 THEN
            s$ = "&&"
            IF (f3$ < "-2147483648" AND LEN(f3$) = 11) OR LEN(f3$) < 11 THEN s$ = "&"
            IF (f3$ <= "-32768" AND LEN(f3$) = 6) OR LEN(f3$) < 6 THEN s$ = "%"
        ELSE
            s$ = "~&&"
            IF (f3$ <= "9223372036854775807" AND LEN(f3$) = 19) OR LEN(f3$) < 19 THEN s$ = "&&"
            IF (f3$ <= "2147483647" AND LEN(f3$) = 10) OR LEN(f3$) < 10 THEN s$ = "&"
            IF (f3$ <= "32767" AND LEN(f3$) = 5) OR LEN(f3$) < 5 THEN s$ = "%"
        END IF
        f3$ = f3$ + s$
        removeelements a$, i, i, 0: insertelements a$, i - 1, f3$
    END IF 'x=0

    GOTO classdone
END IF

'operator?
IF isoperator(f2$) THEN
    lastt = 3: lastti = i
    IF LEN(f2$) > 1 THEN
        IF f2$ <> SCase2$(f2$) THEN
            f2$ = SCase2$(f2$)
            removeelements a$, i, i, 0
            insertelements a$, i - 1, f2$
        END IF
    END IF
    'append negation
    IF f2$ = CHR$(241) THEN f$ = f$ + sp + "-": GOTO classdone_special
    GOTO classdone
END IF

IF alphanumeric(c) THEN
    lastt = 4: lastti = i

    IF i < n THEN nextc = ASC(getelement(a$, i + 1)) ELSE nextc = 0

    ' a constant?
    IF nextc <> 40 THEN '<>"(" (not an array)
    IF lastc <> 46 THEN '<>"." (not an element of a UDT)

    e$ = UCASE$(f2$)
    es$ = removesymbol$(e$)
    IF Error_Happened THEN EXIT FUNCTION

    hashfound = 0
    hashname$ = e$
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
        'FOR i2 = constlast TO 0 STEP -1
        'IF e$ = constname(i2) THEN





        'is a STATIC variable overriding this constant?
        staticvariable = 0
        try = findid(e$ + es$)
        IF Error_Happened THEN EXIT FUNCTION
        DO WHILE try
            IF id.arraytype = 0 THEN staticvariable = 1: EXIT DO 'if it's not an array, it's probably a static variable
            IF try = 2 THEN findanotherid = 1: try = findid(e$ + es$) ELSE try = 0
            IF Error_Happened THEN EXIT FUNCTION
        LOOP
        'add symbol and try again
        IF staticvariable = 0 THEN
            IF LEN(es$) = 0 THEN
                a = ASC(UCASE$(e$)): IF a = 95 THEN a = 91
                a = a - 64 'so A=1, Z=27 and _=28
                es2$ = defineextaz(a)
                try = findid(e$ + es2$)
                IF Error_Happened THEN EXIT FUNCTION
                DO WHILE try
                    IF id.arraytype = 0 THEN staticvariable = 1: EXIT DO 'if it's not an array, it's probably a static variable
                    IF try = 2 THEN findanotherid = 1: try = findid(e$ + es2$) ELSE try = 0
                    IF Error_Happened THEN EXIT FUNCTION
                LOOP
            END IF
        END IF

        IF staticvariable = 0 THEN

            t = consttype(i2)
            IF t AND ISSTRING THEN
                IF LEN(es$) > 0 AND es$ <> "$" THEN Give_Error "Type mismatch": EXIT FUNCTION
                e$ = conststring(i2)
            ELSE 'not a string
                IF LEN(es$) THEN et = typname2typ(es$) ELSE et = 0
                IF Error_Happened THEN EXIT FUNCTION
                IF et AND ISSTRING THEN Give_Error "Type mismatch": EXIT FUNCTION
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
                'apply type conversion if necessary
                IF et THEN t = et
                '(todo: range checking)
                'convert value into string for returning
                IF t AND ISFLOAT THEN
                    e$ = LTRIM$(RTRIM$(STR$(v##)))
                ELSE
                    IF t AND ISUNSIGNED THEN
                        e$ = LTRIM$(RTRIM$(STR$(v~&&)))
                    ELSE
                        e$ = LTRIM$(RTRIM$(STR$(v&&)))
                    END IF
                END IF

                'floats returned by str$ must be converted to qbnex standard format
                IF t AND ISFLOAT THEN
                    t2 = t AND 511
                    'find E,D or F
                    s$ = ""
                    IF INSTR(e$, "E") THEN s$ = "E"
                    IF INSTR(e$, "D") THEN s$ = "D"
                    IF INSTR(e$, "F") THEN s$ = "F"
                    IF LEN(s$) THEN
                        'E,D,F found
                        x = INSTR(e$, s$)
                        'as incorrect type letter may have been returned by STR$, override it
                        IF t2 = 32 THEN s$ = "E"
                        IF t2 = 64 THEN s$ = "D"
                        IF t2 = 256 THEN s$ = "F"
                        MID$(e$, x, 1) = s$
                        IF INSTR(e$, ".") = 0 THEN e$ = LEFT$(e$, x - 1) + ".0" + RIGHT$(e$, LEN(e$) - x + 1): x = x + 2
                        IF LEFT$(e$, 1) = "." THEN e$ = "0" + e$
                        IF LEFT$(e$, 2) = "-." THEN e$ = "-0" + RIGHT$(e$, LEN(e$) - 1)
                        IF INSTR(e$, "+") = 0 AND INSTR(e$, "-") = 0 THEN
                            e$ = LEFT$(e$, x) + "+" + RIGHT$(e$, LEN(e$) - x)
                        END IF
                    ELSE
                        'E,D,F not found
                        IF INSTR(e$, ".") = 0 THEN e$ = e$ + ".0"
                        IF LEFT$(e$, 1) = "." THEN e$ = "0" + e$
                        IF LEFT$(e$, 2) = "-." THEN e$ = "-0" + RIGHT$(e$, LEN(e$) - 1)
                        IF t2 = 32 THEN e$ = e$ + "E+0"
                        IF t2 = 64 THEN e$ = e$ + "D+0"
                        IF t2 = 256 THEN e$ = e$ + "F+0"
                    END IF
                ELSE
                    s$ = typevalue2symbol$(t)
                    IF Error_Happened THEN EXIT FUNCTION
                    e$ = e$ + s$ 'simply append symbol to integer
                END IF

            END IF 'not a string

            removeelements a$, i, i, 0
            insertelements a$, i - 1, e$
            'alter f2$ here to original casing
            f2$ = constcname(i2) + es$
            GOTO classdone

        END IF 'not static
        'END IF 'same name
        'NEXT
    END IF 'hashfound
END IF 'not udt element
END IF 'not array

'variable/array/udt?
u$ = f2$

try_string$ = f2$
try_string2$ = try_string$ 'pure version of try_string$

FOR try_method = 1 TO 4
    try_string$ = try_string2$
    IF try_method = 2 OR try_method = 4 THEN
        dtyp$ = removesymbol(try_string$)
        IF LEN(dtyp$) = 0 THEN
            IF isoperator(try_string$) = 0 THEN
                IF isvalidvariable(try_string$) THEN
                    IF LEFT$(try_string$, 1) = "_" THEN v = 27 ELSE v = ASC(UCASE$(try_string$)) - 64
                    try_string$ = try_string$ + defineextaz(v)
                END IF
            END IF
        ELSE
            try_string$ = try_string2$
        END IF
    END IF
    try = findid(try_string$)
    IF Error_Happened THEN EXIT FUNCTION
    DO WHILE try
        IF (subfuncn = id.insubfuncn AND try_method <= 2) OR try_method >= 3 THEN

            IF Debug THEN PRINT #9, "found id matching " + f2$

            IF nextc = 40 OR uboundlbound <> 0 THEN '(

            uboundlbound = 0

            'function or array?
            IF id.arraytype <> 0 OR id.subfunc = 1 THEN
                'note: even if it's an array of UDTs, the bracketted index will follow immediately

                'correct name
                f3$ = f2$
                s$ = removesymbol$(f3$)
                IF Error_Happened THEN EXIT FUNCTION
                IF id.internal_subfunc THEN
                    f2$ = SCase$(RTRIM$(id.cn)) + s$
                    IF (UCASE$(f2$) = "UBOUND" OR UCASE$(f2$) = "LBOUND") THEN
                        uboundlbound = 2
                    END IF
                ELSE
                    f2$ = RTRIM$(id.cn) + s$
                END IF
                removeelements a$, i, i, 0
                insertelements a$, i - 1, UCASE$(f2$)
                f$ = f$ + f2$ + sp + "(" + sp

                'skip (but record with nothing inside them) brackets
                b2 = 1 'already in first bracket
                FOR i2 = i + 2 TO n
                    c2 = ASC(getelement(a$, i2))
                    IF c2 = 40 THEN b2 = b2 + 1
                    IF c2 = 41 THEN b2 = b2 - 1
                    IF b2 = 0 THEN EXIT FOR 'note: mismatched brackets check ensures this always succeeds
                    f$ = f$ + sp
                NEXT

                'adjust i accordingly
                i = i2

                f$ = f$ + ")"

                'jump to UDT section if array is of UDT type (and elements are referenced)
                IF id.arraytype AND ISUDT THEN
                    IF i < n THEN nextc = ASC(getelement(a$, i + 1)) ELSE nextc = 0
                    IF nextc = 46 THEN udtMethodObjectStart = lastti: t = id.arraytype: GOTO fooudt
                END IF

                f$ = f$ + sp
                GOTO classdone_special
            END IF 'id.arraytype
        END IF 'nextc "("

        IF nextc <> 40 THEN 'not "(" (this avoids confusing simple variables with arrays)
        IF id.t <> 0 OR id.subfunc = 1 THEN 'simple variable or function (without parameters)

        IF id.t AND ISUDT THEN
            'note: it may or may not be followed by a period (eg. if whole udt is being referred to)
            'check if next item is a period

            'correct name
            IF id.internal_subfunc THEN
                f2$ = SCase$(RTRIM$(id.cn)) + removesymbol$(f2$)
            ELSE
                f2$ = RTRIM$(id.cn) + removesymbol$(f2$)
            END IF
            IF Error_Happened THEN EXIT FUNCTION
            removeelements a$, i, i, 0
            insertelements a$, i - 1, UCASE$(f2$)
            f$ = f$ + f2$



            IF nextc <> 46 THEN f$ = f$ + sp: GOTO classdone_special 'no sub-elements referenced
            udtMethodObjectStart = lastti
            t = id.t

            fooudt:

            f$ = f$ + sp + "." + sp
            E = udtxnext(t AND 511) 'next element to check
            i = i + 2

            'loop

            '"." encountered, i must be an element
            IF i > n THEN Give_Error "Expected .element": EXIT FUNCTION
            f2$ = getelement(a$, i)
            s$ = removesymbol$(f2$)
            IF Error_Happened THEN EXIT FUNCTION
            u$ = UCASE$(f2$) + SPACE$(256 - LEN(f2$)) 'fast scanning

            'is f$ the same as element e?
            fooudtnexte:
            IF udtename(E) = u$ THEN
                'match found
                'todo: check symbol(s$) matches element's type

                'correct name
                f2$ = RTRIM$(udtecname(E)) + s$
                removeelements a$, i, i, 0
                insertelements a$, i - 1, UCASE$(f2$)
                f$ = f$ + f2$

                IF i = n THEN f$ = f$ + sp: GOTO classdone_special
                nextc = ASC(getelement(a$, i + 1))
                IF nextc <> 46 THEN f$ = f$ + sp: GOTO classdone_special 'no sub-elements referenced
                'sub-element exists
                t = udtetype(E)
                IF (t AND ISUDT) = 0 THEN Give_Error "Invalid . after element": EXIT FUNCTION
                GOTO fooudt

            END IF 'match found

            'no, so check next element
            IF i < n THEN nextc = ASC(getelement(a$, i + 1)) ELSE nextc = 0
            IF nextc = 40 THEN
                generatedMethod$ = ClassSyntax_FindGeneratedMethod$(RTRIM$(udtxname(t AND 511)), f2$)
                IF LEN(generatedMethod$) THEN
                    a$ = ClassSyntax_RewriteMethodCall$(a$, udtMethodObjectStart, i, generatedMethod$)
                    fixoperationorder$ = fixoperationorder$(a$)
                    EXIT FUNCTION
                END IF
            END IF
            E = udtenext(E)
            IF E = 0 THEN Give_Error "Element not defined": EXIT FUNCTION
            GOTO fooudtnexte

        END IF 'udt

        'non array/udt based variable
        f3$ = f2$
        s$ = removesymbol$(f3$)
        IF Error_Happened THEN EXIT FUNCTION
        IF id.internal_subfunc THEN
            f2$ = SCase$(RTRIM$(id.cn)) + s$
        ELSE
            f2$ = RTRIM$(id.cn) + s$
        END IF
        'change was is returned to uppercase
        removeelements a$, i, i, 0
        insertelements a$, i - 1, UCASE$(f2$)
        GOTO CouldNotClassify
    END IF 'id.t

END IF 'nextc not "("

END IF
IF try = 2 THEN findanotherid = 1: try = findid(try_string$) ELSE try = 0
IF Error_Happened THEN EXIT FUNCTION
LOOP
NEXT 'try method (1-4)
CouldNotClassify:

'alphanumeric, but item name is unknown... is it an internal type? if so, use capitals
f3$ = UCASE$(f2$)
internaltype = 0
IF f3$ = "STRING" THEN internaltype = 1
IF f3$ = "_UNSIGNED" OR (f3$ = "UNSIGNED" AND qbnexprefix_set = 1) THEN internaltype = 1
IF f3$ = "_BIT" OR (f3$ = "BIT" AND qbnexprefix_set = 1) THEN internaltype = 1
IF f3$ = "_BYTE" OR (f3$ = "BYTE" AND qbnexprefix_set = 1) THEN internaltype = 1
IF f3$ = "INTEGER" THEN internaltype = 1
IF f3$ = "LONG" THEN internaltype = 1
IF f3$ = "_INTEGER64" OR (f3$ = "INTEGER64" AND qbnexprefix_set = 1) THEN internaltype = 1
IF f3$ = "SINGLE" THEN internaltype = 1
IF f3$ = "DOUBLE" THEN internaltype = 1
IF f3$ = "_FLOAT" OR (f3$ = "FLOAT" AND qbnexprefix_set = 1) THEN internaltype = 1
IF f3$ = "_OFFSET" OR (f3$ = "OFFSET" AND qbnexprefix_set = 1) THEN internaltype = 1
IF internaltype = 1 THEN
    f2$ = SCase2$(f3$)
    removeelements a$, i, i, 0
    insertelements a$, i - 1, f3$
    GOTO classdone
END IF

GOTO classdone
END IF 'alphanumeric

classdone:
f$ = f$ + f2$
END IF 'b=0
f$ = f$ + sp
classdone_special:
NEXT

IF LEN(f$) THEN f$ = LEFT$(f$, LEN(f$) - 1) 'remove trailing 'sp'

IF Debug THEN PRINT #9, "fixoperationorder:identification:" + a$, n
IF Debug THEN PRINT #9, "fixoperationorder:identification(layout):" + f$, n


'----------------I. Pass (){}bracketed items (if any) to fixoperationorder & build return----------------
'note: items seperated by commas are done seperately

ff$ = ""
b = 0
b2 = 0
p1 = 0 'where level 1 began
aa$ = ""
n = numelements(a$)
FOR i = 1 TO n

    openbracket = 0

    a2$ = getelement(a$, i)

    c = ASC(a2$)



    IF c = 40 OR c = 123 THEN '({
    b = b + 1

    IF b = 1 THEN




        p1 = i + 1
        aa$ = aa$ + "(" + sp

    END IF

    openbracket = 1

    GOTO foopass

END IF '({

IF c = 44 THEN ',
IF b = 1 THEN
    GOTO foopassit
END IF
END IF

IF c = 41 OR c = 125 THEN ')}
IF uboundlbound THEN uboundlbound = uboundlbound - 1
b = b - 1

IF b = 0 THEN
    foopassit:
    IF p1 <> i THEN
        foo$ = fixoperationorder(getelements(a$, p1, i - 1))
        IF Error_Happened THEN EXIT FUNCTION
        IF LEN(foo$) THEN
            aa$ = aa$ + foo$ + sp
            IF c = 125 THEN ff$ = ff$ + tlayout$ + sp ELSE ff$ = ff$ + tlayout$ + sp2 'spacing between ) } , varies
        END IF
    END IF
    IF c = 44 THEN aa$ = aa$ + "," + sp: ff$ = ff$ + "," + sp ELSE aa$ = aa$ + ")" + sp
    p1 = i + 1
END IF

GOTO foopass
END IF ')}




IF b = 0 THEN aa$ = aa$ + a2$ + sp


foopass:

f2$ = getelementspecial(f$, i)
IF Error_Happened THEN EXIT FUNCTION
IF LEN(f2$) THEN

    'use sp2 to join items connected by a period
    IF c = 46 THEN '"."
    IF i > 1 AND i < n THEN 'stupidity check
    IF LEN(ff$) THEN MID$(ff$, LEN(ff$), 1) = sp2 'convert last spacer to a sp2
    ff$ = ff$ + "." + sp2
    GOTO fooloopnxt
END IF
END IF

'spacing just before (
IF openbracket THEN

    'convert last spacer?
    IF i <> 1 THEN
        IF isoperator(getelement$(a$, i - 1)) = 0 THEN
            MID$(ff$, LEN(ff$), 1) = sp2
        END IF
    END IF
    ff$ = ff$ + f2$ + sp2
ELSE 'not openbracket
    ff$ = ff$ + f2$ + sp
END IF

END IF 'len(f2$)

fooloopnxt:

NEXT

IF LEN(aa$) THEN aa$ = LEFT$(aa$, LEN(aa$) - 1)
IF LEN(ff$) THEN ff$ = LEFT$(ff$, LEN(ff$) - 1)

IF Debug THEN PRINT #9, "fixoperationorder:return:" + aa$
IF Debug THEN PRINT #9, "fixoperationorder:layout:" + ff$
tlayout$ = ff$
fixoperationorder$ = aa$

fooindwel = fooindwel - 1
END FUNCTION

FUNCTION lineformat$ (a$)
    a2$ = ""
    linecontinuation = 0

    continueline:

    a$ = a$ + "  " 'add 2 extra spaces to make reading next char easier

    ca$ = a$
    a$ = UCASE$(a$)

    n = LEN(a$)
    i = 1
    lineformatnext:
    IF i >= n THEN GOTO lineformatdone

    c = ASC(a$, i)
    c$ = CHR$(c) '***remove later***

    '----------------quoted string----------------
    IF c = 34 THEN '"
    'Emit one token "content",len so getelement/evaluatefunc do not split the
    'opening quote from the literal (avoids Illegal string-number conversion).
    escaped_output = 0
    p1 = i + 1
    FOR i2 = i + 1 TO n - 2
        c2 = ASC(a$, i2)

        IF c2 = 34 THEN
            IF escaped_output = 0 THEN
                a2$ = a2$ + sp + CHR$(34) + MID$(ca$, p1, i2 - p1 + 1) + "," + str2$(i2 - (i + 1))
            ELSE
                a2$ = a2$ + MID$(ca$, p1, i2 - p1 + 1) + "," + str2$(i2 - (i + 1))
            END IF
            i = i2 + 1
            EXIT FOR
        END IF

        IF c2 = 92 THEN '\
        IF escaped_output = 0 THEN
            a2$ = a2$ + sp + CHR$(34)
            escaped_output = 1
        END IF
        a2$ = a2$ + MID$(ca$, p1, i2 - p1) + "\\"
        p1 = i2 + 1
    END IF

    IF c2 < 32 OR c2 > 126 THEN
        IF escaped_output = 0 THEN
            a2$ = a2$ + sp + CHR$(34)
            escaped_output = 1
        END IF
        o$ = OCT$(c2)
        IF LEN(o$) < 3 THEN
            o$ = "0" + o$
            IF LEN(o$) < 3 THEN o$ = "0" + o$
        END IF
        a2$ = a2$ + MID$(ca$, p1, i2 - p1) + "\" + o$
        p1 = i2 + 1
    END IF

NEXT

IF i2 = n - 1 THEN 'no closing "
IF escaped_output = 0 THEN
    a2$ = a2$ + sp + CHR$(34) + MID$(ca$, p1, (n - 2) - p1 + 1) + CHR$(34) + "," + str2$((n - 2) - (i + 1) + 1)
ELSE
    a2$ = a2$ + MID$(ca$, p1, (n - 2) - p1 + 1) + CHR$(34) + "," + str2$((n - 2) - (i + 1) + 1)
END IF
i = n - 1
END IF

GOTO lineformatnext

END IF

'----------------number----------------
firsti = i
IF c = 46 THEN
    c2$ = MID$(a$, i + 1, 1): c2 = ASC(c2$)
    IF (c2 >= 48 AND c2 <= 57) THEN GOTO lfnumber
END IF
IF (c >= 48 AND c <= 57) THEN '0-9
lfnumber:

'handle 'IF a=1 THEN a=2 ELSE 100' by assuming numeric after ELSE to be a
IF RIGHT$(a2$, 5) = sp + "ELSE" THEN
    a2$ = a2$ + sp + "GOTO"
END IF

'Number will be converted to the following format:
' 999999  .        99999  E        +         999
'[whole$][dp(0/1)][frac$][ed(1/2)][pm(1/-1)][ex$]
' 0                1               2         3    <-mode

mode = 0
whole$ = ""
dp = 0
frac$ = ""
ed = 0 'E=1, D=2, F=3
pm = 1
ex$ = ""




lfreadnumber:
valid = 0

IF c = 46 THEN
    IF mode = 0 THEN valid = 1: dp = 1: mode = 1
END IF

IF c >= 48 AND c <= 57 THEN '0-9
valid = 1
IF mode = 0 THEN whole$ = whole$ + c$
IF mode = 1 THEN frac$ = frac$ + c$
IF mode = 2 THEN mode = 3
IF mode = 3 THEN ex$ = ex$ + c$
END IF

IF c = 69 OR c = 68 OR c = 70 THEN 'E,D,F
IF mode < 2 THEN
    valid = 1
    IF c = 69 THEN ed = 1
    IF c = 68 THEN ed = 2
    IF c = 70 THEN ed = 3
    mode = 2
END IF
END IF

IF c = 43 OR c = 45 THEN '+,-
IF mode = 2 THEN
    valid = 1
    IF c = 45 THEN pm = -1
    mode = 3
END IF
END IF

IF valid THEN
    IF i <= n THEN i = i + 1: c$ = MID$(a$, i, 1): c = ASC(c$): GOTO lfreadnumber
END IF



'cull leading 0s off whole$
DO WHILE LEFT$(whole$, 1) = "0": whole$ = RIGHT$(whole$, LEN(whole$) - 1): LOOP
    'cull trailing 0s off frac$
    DO WHILE RIGHT$(frac$, 1) = "0": frac$ = LEFT$(frac$, LEN(frac$) - 1): LOOP
        'cull leading 0s off ex$
        DO WHILE LEFT$(ex$, 1) = "0": ex$ = RIGHT$(ex$, LEN(ex$) - 1): LOOP

            IF dp <> 0 OR ed <> 0 THEN float = 1 ELSE float = 0

            extused = 1

            IF ed THEN e$ = "": GOTO lffoundext 'no extensions valid after E/D/F specified

            '3-character extensions
            IF i <= n - 2 THEN
                e$ = MID$(a$, i, 3)
                IF e$ = "~%%" AND float = 0 THEN i = i + 3: GOTO lffoundext
                IF e$ = "~&&" AND float = 0 THEN i = i + 3: GOTO lffoundext
                IF e$ = "~%&" AND float = 0 THEN Give_Error "Cannot use _OFFSET symbols after numbers": EXIT FUNCTION
            END IF
            '2-character extensions
            IF i <= n - 1 THEN
                e$ = MID$(a$, i, 2)
                IF e$ = "%%" AND float = 0 THEN i = i + 2: GOTO lffoundext
                IF e$ = "~%" AND float = 0 THEN i = i + 2: GOTO lffoundext
                IF e$ = "&&" AND float = 0 THEN i = i + 2: GOTO lffoundext
                IF e$ = "~&" AND float = 0 THEN i = i + 2: GOTO lffoundext
                IF e$ = "%&" AND float = 0 THEN Give_Error "Cannot use _OFFSET symbols after numbers": EXIT FUNCTION
                IF e$ = "##" THEN
                    i = i + 2
                    ed = 3
                    e$ = ""
                    GOTO lffoundext
                END IF
                IF e$ = "~`" THEN
                    i = i + 2
                    GOTO lffoundbitext
                END IF
            END IF
            '1-character extensions
            IF i <= n THEN
                e$ = MID$(a$, i, 1)
                IF e$ = "%" AND float = 0 THEN i = i + 1: GOTO lffoundext
                IF e$ = "&" AND float = 0 THEN i = i + 1: GOTO lffoundext
                IF e$ = "!" THEN
                    i = i + 1
                    ed = 1
                    e$ = ""
                    GOTO lffoundext
                END IF
                IF e$ = "#" THEN
                    i = i + 1
                    ed = 2
                    e$ = ""
                    GOTO lffoundext
                END IF
                IF e$ = "`" THEN
                    i = i + 1
                    lffoundbitext:
                    bitn$ = ""
                    DO WHILE i <= n
                        c2 = ASC(MID$(a$, i, 1))
                        IF c2 >= 48 AND c2 <= 57 THEN
                            bitn$ = bitn$ + CHR$(c2)
                            i = i + 1
                        ELSE
                            EXIT DO
                        END IF
                    LOOP
                    IF bitn$ = "" THEN bitn$ = "1"
                    'cull leading 0s off bitn$
                    DO WHILE LEFT$(bitn$, 1) = "0": bitn$ = RIGHT$(bitn$, LEN(bitn$) - 1): LOOP
                        e$ = e$ + bitn$
                        GOTO lffoundext
                    END IF
                END IF

                IF float THEN 'floating point types CAN be assumed
                'calculate first significant digit offset & number of significant digits
                IF whole$ <> "" THEN
                    offset = LEN(whole$) - 1
                    sigdigits = LEN(whole$) + LEN(frac$)
                ELSE
                    IF frac$ <> "" THEN
                        offset = -1
                        sigdigits = LEN(frac$)
                        FOR i2 = 1 TO LEN(frac$)
                            IF MID$(frac$, i2, 1) <> "0" THEN EXIT FOR
                            offset = offset - 1
                            sigdigits = sigdigits - 1
                        NEXT
                    ELSE
                        'number is 0
                        offset = 0
                        sigdigits = 0
                    END IF
                END IF
                sigdig$ = RIGHT$(whole$ + frac$, sigdigits)
                'SINGLE?
                IF sigdigits <= 7 THEN 'QBASIC interprets anything with more than 7 sig. digits as a DOUBLE
                IF offset <= 38 AND offset >= -38 THEN 'anything outside this range cannot be represented as a SINGLE
                IF offset = 38 THEN
                    IF sigdig$ > "3402823" THEN GOTO lfxsingle
                END IF
                IF offset = -38 THEN
                    IF sigdig$ < "1175494" THEN GOTO lfxsingle
                END IF
                ed = 1
                e$ = ""
                GOTO lffoundext
            END IF
        END IF
        lfxsingle:
        'DOUBLE?
        IF sigdigits <= 16 THEN 'QBNex handles DOUBLES with 16-digit precision
        IF offset <= 308 AND offset >= -308 THEN 'anything outside this range cannot be represented as a DOUBLE
        IF offset = 308 THEN
            IF sigdig$ > "1797693134862315" THEN GOTO lfxdouble
        END IF
        IF offset = -308 THEN
            IF sigdig$ < "2225073858507201" THEN GOTO lfxdouble
        END IF
        ed = 2
        e$ = ""
        GOTO lffoundext
    END IF
END IF
lfxdouble:
'assume _FLOAT
ed = 3
e$ = "": GOTO lffoundext
END IF

extused = 0
e$ = ""
lffoundext:

'make sure a leading numberic character exists
IF whole$ = "" THEN whole$ = "0"
'if a float, ensure frac$<>"" and dp=1
IF float THEN
    dp = 1
    IF frac$ = "" THEN frac$ = "0"
END IF
'if ed is specified, make sure ex$ exists
IF ed <> 0 AND ex$ = "" THEN ex$ = "0"

a2$ = a2$ + sp
a2$ = a2$ + whole$
IF dp THEN a2$ = a2$ + "." + frac$
IF ed THEN
    IF ed = 1 THEN a2$ = a2$ + "E"
    IF ed = 2 THEN a2$ = a2$ + "D"
    IF ed = 3 THEN a2$ = a2$ + "F"
    IF pm = -1 AND ex$ <> "0" THEN a2$ = a2$ + "-" ELSE a2$ = a2$ + "+"
    a2$ = a2$ + ex$
END IF
a2$ = a2$ + e$

IF extused THEN a2$ = a2$ + "," + MID$(a$, firsti, i - firsti)

GOTO lineformatnext
END IF

'----------------(number)&H...----------------
'note: the final value, not the number of hex characters, sets the default type
IF c = 38 THEN '&
IF MID$(a$, i + 1, 1) = "H" THEN
    i = i + 2
    hx$ = ""
    lfreadhex:
    IF i <= n THEN
        c$ = MID$(a$, i, 1): c = ASC(c$)
        IF (c >= 48 AND c <= 57) OR (c >= 65 AND c <= 70) THEN hx$ = hx$ + c$: i = i + 1: GOTO lfreadhex
    END IF
    fullhx$ = "&H" + hx$

    'cull leading 0s off hx$
    DO WHILE LEFT$(hx$, 1) = "0": hx$ = RIGHT$(hx$, LEN(hx$) - 1): LOOP
        IF hx$ = "" THEN hx$ = "0"

        bitn$ = ""
        '3-character extensions
        IF i <= n - 2 THEN
            e$ = MID$(a$, i, 3)
            IF e$ = "~%%" THEN i = i + 3: GOTO lfhxext
            IF e$ = "~&&" THEN i = i + 3: GOTO lfhxext
            IF e$ = "~%&" THEN Give_Error "Cannot use _OFFSET symbols after numbers": EXIT FUNCTION
        END IF
        '2-character extensions
        IF i <= n - 1 THEN
            e$ = MID$(a$, i, 2)
            IF e$ = "%%" THEN i = i + 2: GOTO lfhxext
            IF e$ = "~%" THEN i = i + 2: GOTO lfhxext
            IF e$ = "&&" THEN i = i + 2: GOTO lfhxext
            IF e$ = "%&" THEN Give_Error "Cannot use _OFFSET symbols after numbers": EXIT FUNCTION
            IF e$ = "~&" THEN i = i + 2: GOTO lfhxext
            IF e$ = "~`" THEN
                i = i + 2
                GOTO lfhxbitext
            END IF
        END IF
        '1-character extensions
        IF i <= n THEN
            e$ = MID$(a$, i, 1)
            IF e$ = "%" THEN i = i + 1: GOTO lfhxext
            IF e$ = "&" THEN i = i + 1: GOTO lfhxext
            IF e$ = "`" THEN
                i = i + 1
                lfhxbitext:
                DO WHILE i <= n
                    c2 = ASC(MID$(a$, i, 1))
                    IF c2 >= 48 AND c2 <= 57 THEN
                        bitn$ = bitn$ + CHR$(c2)
                        i = i + 1
                    ELSE
                        EXIT DO
                    END IF
                LOOP
                IF bitn$ = "" THEN bitn$ = "1"
                'cull leading 0s off bitn$
                DO WHILE LEFT$(bitn$, 1) = "0": bitn$ = RIGHT$(bitn$, LEN(bitn$) - 1): LOOP
                    GOTO lfhxext
                END IF
            END IF
            'if no valid extension context was given, assume one
            'note: leading 0s have been culled, so LEN(hx$) reflects its values size
            e$ = "&&"
            IF LEN(hx$) <= 8 THEN e$ = "&" 'as in QBASIC, signed values must be used
            IF LEN(hx$) <= 4 THEN e$ = "%" 'as in QBASIC, signed values must be used
            GOTO lfhxext2
            lfhxext:
            fullhx$ = fullhx$ + e$ + bitn$
            lfhxext2:

            'build 8-byte unsigned integer rep. of hx$
            IF LEN(hx$) > 16 THEN Give_Error "Overflow": EXIT FUNCTION
            v~&& = 0
            FOR i2 = 1 TO LEN(hx$)
                v2 = ASC(MID$(hx$, i2, 1))
                IF v2 <= 57 THEN v2 = v2 - 48 ELSE v2 = v2 - 65 + 10
                v~&& = v~&& * 16 + v2
            NEXT

            finishhexoctbin:
            num$ = str2u64$(v~&&) 'correct for unsigned values (overflow of unsigned can be checked later)
            IF LEFT$(e$, 1) <> "~" THEN 'note: range checking will be performed later in fixop.order
            'signed

            IF e$ = "%%" THEN
                IF v~&& > 127 THEN
                    IF v~&& > 255 THEN Give_Error "Overflow": EXIT FUNCTION
                    v~&& = ((NOT v~&&) AND 255) + 1
                    num$ = "-" + sp + str2u64$(v~&&)
                END IF
            END IF

            IF e$ = "%" THEN
                IF v~&& > 32767 THEN
                    IF v~&& > 65535 THEN Give_Error "Overflow": EXIT FUNCTION
                    v~&& = ((NOT v~&&) AND 65535) + 1
                    num$ = "-" + sp + str2u64$(v~&&)
                END IF
            END IF

            IF e$ = "&" THEN
                IF v~&& > 2147483647 THEN
                    IF v~&& > 4294967295 THEN Give_Error "Overflow": EXIT FUNCTION
                    v~&& = ((NOT v~&&) AND 4294967295) + 1
                    num$ = "-" + sp + str2u64$(v~&&)
                END IF
            END IF

            IF e$ = "&&" THEN
                IF v~&& > 9223372036854775807 THEN
                    'note: no error checking necessary
                    v~&& = (NOT v~&&) + 1
                    num$ = "-" + sp + str2u64$(v~&&)
                END IF
            END IF

            IF e$ = "`" THEN
                vbitn = VAL(bitn$)
                h~&& = 1: FOR i2 = 1 TO vbitn - 1: h~&& = h~&& * 2: NEXT: h~&& = h~&& - 1 'build h~&&
                IF v~&& > h~&& THEN
                    h~&& = 1: FOR i2 = 1 TO vbitn: h~&& = h~&& * 2: NEXT: h~&& = h~&& - 1 'build h~&&
                    IF v~&& > h~&& THEN Give_Error "Overflow": EXIT FUNCTION
                    v~&& = ((NOT v~&&) AND h~&&) + 1
                    num$ = "-" + sp + str2u64$(v~&&)
                END IF
            END IF

        END IF '<>"~"

        a2$ = a2$ + sp + num$ + e$ + bitn$ + "," + fullhx$

        GOTO lineformatnext
    END IF
END IF

'----------------(number)&O...----------------
'note: the final value, not the number of oct characters, sets the default type
IF c = 38 THEN '&
IF MID$(a$, i + 1, 1) = "O" THEN
    i = i + 2
    'note: to avoid mistakes, hx$ is used instead of 'ot$'
    hx$ = ""
    lfreadoct:
    IF i <= n THEN
        c$ = MID$(a$, i, 1): c = ASC(c$)
        IF c >= 48 AND c <= 55 THEN hx$ = hx$ + c$: i = i + 1: GOTO lfreadoct
    END IF
    fullhx$ = "&O" + hx$

    'cull leading 0s off hx$
    DO WHILE LEFT$(hx$, 1) = "0": hx$ = RIGHT$(hx$, LEN(hx$) - 1): LOOP
        IF hx$ = "" THEN hx$ = "0"

        bitn$ = ""
        '3-character extensions
        IF i <= n - 2 THEN
            e$ = MID$(a$, i, 3)
            IF e$ = "~%%" THEN i = i + 3: GOTO lfotext
            IF e$ = "~&&" THEN i = i + 3: GOTO lfotext
            IF e$ = "~%&" THEN Give_Error "Cannot use _OFFSET symbols after numbers": EXIT FUNCTION
        END IF
        '2-character extensions
        IF i <= n - 1 THEN
            e$ = MID$(a$, i, 2)
            IF e$ = "%%" THEN i = i + 2: GOTO lfotext
            IF e$ = "~%" THEN i = i + 2: GOTO lfotext
            IF e$ = "&&" THEN i = i + 2: GOTO lfotext
            IF e$ = "%&" THEN Give_Error "Cannot use _OFFSET symbols after numbers": EXIT FUNCTION
            IF e$ = "~&" THEN i = i + 2: GOTO lfotext
            IF e$ = "~`" THEN
                i = i + 2
                GOTO lfotbitext
            END IF
        END IF
        '1-character extensions
        IF i <= n THEN
            e$ = MID$(a$, i, 1)
            IF e$ = "%" THEN i = i + 1: GOTO lfotext
            IF e$ = "&" THEN i = i + 1: GOTO lfotext
            IF e$ = "`" THEN
                i = i + 1
                lfotbitext:
                bitn$ = ""
                DO WHILE i <= n
                    c2 = ASC(MID$(a$, i, 1))
                    IF c2 >= 48 AND c2 <= 57 THEN
                        bitn$ = bitn$ + CHR$(c2)
                        i = i + 1
                    ELSE
                        EXIT DO
                    END IF
                LOOP
                IF bitn$ = "" THEN bitn$ = "1"
                'cull leading 0s off bitn$
                DO WHILE LEFT$(bitn$, 1) = "0": bitn$ = RIGHT$(bitn$, LEN(bitn$) - 1): LOOP
                    GOTO lfotext
                END IF
            END IF
            'if no valid extension context was given, assume one
            'note: leading 0s have been culled, so LEN(hx$) reflects its values size
            e$ = "&&"
            '37777777777
            IF LEN(hx$) <= 11 THEN
                IF LEN(hx$) < 11 OR ASC(LEFT$(hx$, 1)) <= 51 THEN e$ = "&"
            END IF
            '177777
            IF LEN(hx$) <= 6 THEN
                IF LEN(hx$) < 6 OR LEFT$(hx$, 1) = "1" THEN e$ = "%"
            END IF

            GOTO lfotext2
            lfotext:
            fullhx$ = fullhx$ + e$ + bitn$
            lfotext2:

            'build 8-byte unsigned integer rep. of hx$
            '1777777777777777777777 (22 digits)
            IF LEN(hx$) > 22 THEN Give_Error "Overflow": EXIT FUNCTION
            IF LEN(hx$) = 22 THEN
                IF LEFT$(hx$, 1) <> "1" THEN Give_Error "Overflow": EXIT FUNCTION
            END IF
            '********change v& to v~&&********
            v~&& = 0
            FOR i2 = 1 TO LEN(hx$)
                v2 = ASC(MID$(hx$, i2, 1))
                v2 = v2 - 48
                v~&& = v~&& * 8 + v2
            NEXT

            GOTO finishhexoctbin
        END IF
    END IF

    '----------------(number)&B...----------------
    'note: the final value, not the number of bin characters, sets the default type
    IF c = 38 THEN '&
    IF MID$(a$, i + 1, 1) = "B" THEN
        i = i + 2
        'note: to avoid mistakes, hx$ is used instead of 'bi$'
        hx$ = ""
        lfreadbin:
        IF i <= n THEN
            c$ = MID$(a$, i, 1): c = ASC(c$)
            IF c >= 48 AND c <= 49 THEN hx$ = hx$ + c$: i = i + 1: GOTO lfreadbin
        END IF
        fullhx$ = "&B" + hx$

        'cull leading 0s off hx$
        DO WHILE LEFT$(hx$, 1) = "0": hx$ = RIGHT$(hx$, LEN(hx$) - 1): LOOP
            IF hx$ = "" THEN hx$ = "0"

            bitn$ = ""
            '3-character extensions
            IF i <= n - 2 THEN
                e$ = MID$(a$, i, 3)
                IF e$ = "~%%" THEN i = i + 3: GOTO lfbiext
                IF e$ = "~&&" THEN i = i + 3: GOTO lfbiext
                IF e$ = "~%&" THEN Give_Error "Cannot use _OFFSET symbols after numbers": EXIT FUNCTION
            END IF
            '2-character extensions
            IF i <= n - 1 THEN
                e$ = MID$(a$, i, 2)
                IF e$ = "%%" THEN i = i + 2: GOTO lfbiext
                IF e$ = "~%" THEN i = i + 2: GOTO lfbiext
                IF e$ = "&&" THEN i = i + 2: GOTO lfbiext
                IF e$ = "%&" THEN Give_Error "Cannot use _OFFSET symbols after numbers": EXIT FUNCTION
                IF e$ = "~&" THEN i = i + 2: GOTO lfbiext
                IF e$ = "~`" THEN
                    i = i + 2
                    GOTO lfbibitext
                END IF
            END IF


            '1-character extensions
            IF i <= n THEN
                e$ = MID$(a$, i, 1)
                IF e$ = "%" THEN i = i + 1: GOTO lfbiext
                IF e$ = "&" THEN i = i + 1: GOTO lfbiext
                IF e$ = "`" THEN
                    i = i + 1
                    lfbibitext:
                    bitn$ = ""
                    DO WHILE i <= n
                        c2 = ASC(MID$(a$, i, 1))
                        IF c2 >= 48 AND c2 <= 57 THEN
                            bitn$ = bitn$ + CHR$(c2)
                            i = i + 1
                        ELSE
                            EXIT DO
                        END IF
                    LOOP
                    IF bitn$ = "" THEN bitn$ = "1"
                    'cull leading 0s off bitn$
                    DO WHILE LEFT$(bitn$, 1) = "0": bitn$ = RIGHT$(bitn$, LEN(bitn$) - 1): LOOP
                        GOTO lfbiext
                    END IF
                END IF
                'if no valid extension context was given, assume one
                'note: leading 0s have been culled, so LEN(hx$) reflects its values size
                e$ = "&&"
                IF LEN(hx$) <= 32 THEN e$ = "&"
                IF LEN(hx$) <= 16 THEN e$ = "%"

                GOTO lfbiext2
                lfbiext:
                fullhx$ = fullhx$ + e$ + bitn$
                lfbiext2:

                'build 8-byte unsigned integer rep. of hx$
                IF LEN(hx$) > 64 THEN Give_Error "Overflow": EXIT FUNCTION

                v~&& = 0
                FOR i2 = 1 TO LEN(hx$)
                    v2 = ASC(MID$(hx$, i2, 1))
                    v2 = v2 - 48
                    v~&& = v~&& * 2 + v2
                NEXT

                GOTO finishhexoctbin
            END IF
        END IF


        '----------------(number)&H??? error----------------
        IF c = 38 THEN Give_Error "Expected &H... or &O...": EXIT FUNCTION

        '----------------variable/name----------------
        '*trailing _ is treated as a seperate line extension*
        IF (c >= 65 AND c <= 90) OR c = 95 THEN 'A-Z(a-z) or _
        IF c = 95 THEN p2 = 0 ELSE p2 = i
        FOR i2 = i + 1 TO n
            c2 = ASC(a$, i2)
            IF NOT alphanumeric(c2) THEN EXIT FOR
            IF c2 <> 95 THEN p2 = i2
        NEXT
        IF p2 THEN 'not just underscores!
        'char is from i to p2
        n2 = p2 - i + 1
        a3$ = MID$(a$, i, n2)

        '----(variable/name)rem----
        IF n2 = 3 THEN
            IF a3$ = "REM" THEN
                i = i + n2
                IF i < n THEN
                    c = ASC(a$, i)
                    IF c = 46 THEN a2$ = a2$ + sp + MID$(ca$, i - n2, n2): GOTO extcheck 'rem.Variable is a valid variable name in QB45
                END IF

                'note: In QBASIC 'IF cond THEN REM comment' counts as a single line IF statement, however use of ' instead of REM does not
                IF UCASE$(RIGHT$(a2$, 5)) = sp + "THEN" THEN a2$ = a2$ + sp + "'" 'add nop
                layoutcomment = SCase$("Rem")
                GOTO comment
            END IF
        END IF

        '----(variable/name)data----
        IF n2 = 4 THEN
            IF a3$ = "DATA" THEN
                x$ = ""
                i = i + n2
                IF i < n THEN
                    c = ASC(a$, i)
                    IF c = 46 THEN a2$ = a2$ + sp + MID$(ca$, i - n2, n2): GOTO extcheck 'data.Variable is a valid variable name in QB45
                END IF

                scan = 0
                speechmarks = 0
                commanext = 0
                finaldata = 0
                e$ = ""
                p1 = 0
                p2 = 0
                nextdatachr:
                IF i < n THEN
                    c = ASC(a$, i)
                    IF c = 9 OR c = 32 THEN
                        IF scan = 0 THEN GOTO skipwhitespace
                    END IF

                    IF c = 58 THEN '":"
                    IF speechmarks = 0 THEN finaldata = 1: GOTO adddata
                END IF

                IF c = 44 THEN '","
                IF speechmarks = 0 THEN
                    adddata:
                    IF prepass = 0 THEN
                        IF p1 THEN
                            'FOR i2 = p1 TO p2
                            '    DATA_add ASC(ca$, i2)
                            'NEXT
                            x$ = x$ + MID$(ca$, p1, p2 - p1 + 1)
                        END IF
                        'assume closing "
                        IF speechmarks THEN
                            'DATA_add 34
                            x$ = x$ + CHR$(34)
                        END IF
                        'append comma
                        'DATA_add 44
                        x$ = x$ + CHR$(44)
                    END IF
                    IF finaldata = 1 THEN GOTO finisheddata
                    e$ = ""
                    p1 = 0
                    p2 = 0
                    speechmarks = 0
                    scan = 0
                    commanext = 0
                    i = i + 1
                    GOTO nextdatachr
                END IF
            END IF '","

            IF commanext = 1 THEN
                IF c <> 32 AND c <> 9 THEN Give_Error "Expected , after quoted string in DATA statement": EXIT FUNCTION
            END IF

            IF c = 34 THEN
                IF speechmarks = 1 THEN
                    commanext = 1
                    speechmarks = 0
                END IF
                IF scan = 0 THEN speechmarks = 1
            END IF

            scan = 1

            IF p1 = 0 THEN p1 = i: p2 = i
            IF c <> 9 AND c <> 32 THEN p2 = i

            skipwhitespace:
            i = i + 1: GOTO nextdatachr
        END IF 'i<n
        finaldata = 1: GOTO adddata
        finisheddata:
        e$ = ""
        IF prepass = 0 THEN
            PUT #16, , x$
            DataOffset = DataOffset + LEN(x$)

            e$ = SPACE$((LEN(x$) - 1) * 2)
            FOR ec = 1 TO LEN(x$) - 1
                '2 chr hex encode each character
                v1 = ASC(x$, ec)
                v2 = v1 \ 16: IF v2 <= 9 THEN v2 = v2 + 48 ELSE v2 = v2 + 55
                v1 = v1 AND 15: IF v1 <= 9 THEN v1 = v1 + 48 ELSE v1 = v1 + 55
                ASC(e$, ec * 2 - 1) = v1
                ASC(e$, ec * 2) = v2
            NEXT

        END IF

        a2$ = a2$ + sp + "DATA": IF LEN(e$) THEN a2$ = a2$ + sp + "_" + e$
        GOTO lineformatnext
    END IF
END IF

a2$ = a2$ + sp + MID$(ca$, i, n2)
i = i + n2

'----(variable/name)extensions----
extcheck:
IF n2 > 40 THEN Give_Error "Identifier longer than 40 character limit": EXIT FUNCTION
c3 = ASC(a$, i)
m = 0
IF c3 = 126 THEN '"~"
e2$ = MID$(a$, i + 1, 2)
IF e2$ = "&&" THEN e2$ = "~&&": GOTO lfgetve
IF e2$ = "%%" THEN e2$ = "~%%": GOTO lfgetve
IF e2$ = "%&" THEN e2$ = "~%&": GOTO lfgetve
e2$ = CHR$(ASC(e2$))
IF e2$ = "&" THEN e2$ = "~&": GOTO lfgetve
IF e2$ = "%" THEN e2$ = "~%": GOTO lfgetve
IF e2$ = "`" THEN m = 1: e2$ = "~`": GOTO lfgetve
END IF
IF c3 = 37 THEN
    c4 = ASC(a$, i + 1)
    IF c4 = 37 THEN e2$ = "%%": GOTO lfgetve
    IF c4 = 38 THEN e2$ = "%&": GOTO lfgetve
    e2$ = "%": GOTO lfgetve
END IF
IF c3 = 38 THEN
    c4 = ASC(a$, i + 1)
    IF c4 = 38 THEN e2$ = "&&": GOTO lfgetve
    e2$ = "&": GOTO lfgetve
END IF
IF c3 = 33 THEN e2$ = "!": GOTO lfgetve
IF c3 = 35 THEN
    c4 = ASC(a$, i + 1)
    IF c4 = 35 THEN e2$ = "##": GOTO lfgetve
    e2$ = "#": GOTO lfgetve
END IF
IF c3 = 36 THEN m = 1: e2$ = "$": GOTO lfgetve
IF c3 = 96 THEN m = 1: e2$ = "`": GOTO lfgetve
'(no symbol)

'cater for unusual names/labels (eg a.0b%)
IF ASC(a$, i) = 46 THEN '"."
c2 = ASC(a$, i + 1)
IF c2 >= 48 AND c2 <= 57 THEN
    'scan until no further alphanumerics
    p2 = i + 1
    FOR i2 = i + 2 TO n
        c = ASC(a$, i2)

        IF NOT alphanumeric(c) THEN EXIT FOR
        IF c <> 95 THEN p2 = i2 'don't including trailing _
    NEXT
    a2$ = a2$ + sp + "." + sp + MID$(ca$, i + 1, p2 - (i + 1) + 1) 'case sensitive
    n2 = n2 + 1 + (p2 - (i + 1) + 1)
    i = p2 + 1
    GOTO extcheck 'it may have an extension or be continued with another "."
END IF
END IF

GOTO lineformatnext

lfgetve:
i = i + LEN(e2$)
a2$ = a2$ + e2$
IF m THEN 'allow digits after symbol
lfgetvd:
IF i < n THEN
    c = ASC(a$, i)
    IF c >= 48 AND c <= 57 THEN a2$ = a2$ + CHR$(c): i = i + 1: GOTO lfgetvd
END IF
END IF 'm

GOTO lineformatnext

END IF 'p2
END IF 'variable/name
'----------------variable/name end----------------

'----------------spacing----------------
IF c = 32 OR c = 9 THEN i = i + 1: GOTO lineformatnext

'----------------symbols----------------
'--------single characters--------
IF lfsinglechar(c) THEN
    IF (c = 60) OR (c = 61) OR (c = 62) THEN
        count = 0
        DO
            count = count + 1
            IF i + count >= LEN(a$) - 2 THEN EXIT DO
        LOOP UNTIL ASC(a$, i + count) <> 32
        c2 = ASC(a$, i + count)
        IF c = 60 THEN '<
        IF c2 = 61 THEN a2$ = a2$ + sp + "<=": i = i + count + 1: GOTO lineformatnext
        IF c2 = 62 THEN a2$ = a2$ + sp + "<>": i = i + count + 1: GOTO lineformatnext
    ELSEIF c = 62 THEN '>
        IF c2 = 61 THEN a2$ = a2$ + sp + ">=": i = i + count + 1: GOTO lineformatnext
        IF c2 = 60 THEN a2$ = a2$ + sp + "<>": i = i + count + 1: GOTO lineformatnext '>< to <>
    ELSEIF c = 61 THEN '=
        IF c2 = 62 THEN a2$ = a2$ + sp + ">=": i = i + count + 1: GOTO lineformatnext '=> to >=
        IF c2 = 60 THEN a2$ = a2$ + sp + "<=": i = i + count + 1: GOTO lineformatnext '=< to <=
    END IF
END IF

IF c = 36 AND LEN(a2$) THEN GOTO badusage '$


a2$ = a2$ + sp + CHR$(c)
i = i + 1
GOTO lineformatnext
END IF
badusage:

IF c <> 39 THEN Give_Error "Unexpected character on line": EXIT FUNCTION 'invalid symbol encountered

'----------------comment(')----------------
layoutcomment = "'"
i = i + 1
comment:
IF i >= n THEN GOTO lineformatdone2
c$ = RIGHT$(a$, LEN(a$) - i + 1)
cc$ = RIGHT$(ca$, LEN(ca$) - i + 1)
IF LEN(c$) = 0 THEN GOTO lineformatdone2
layoutcomment$ = RTRIM$(layoutcomment$ + cc$)

c$ = LTRIM$(c$)
IF LEN(c$) = 0 THEN GOTO lineformatdone2
ac = ASC(c$)
'note: any non-whitespace character between the comment leader and the
'      first '$' renders this a plain comment
'    : the leading '$' does NOT have to be part of a valid metacommand.
'      E.g., REM $FOO $DYNAMIC is a valid metacommand line
IF ac <> 36 THEN GOTO lineformatdone2
nocasec$ = LTRIM$(RIGHT$(ca$, LEN(ca$) - i + 1))
memmode = 0
x = 1
DO
    'note: metacommands may appear on a line any number of times but only
    '      the last appearance of $INCLUDE, and either $STATIC or $DYNAMIC,
    '      is processed
    '    : metacommands do not need to be terminated by word boundaries.
    '      E.g., $STATICanychars$DYNAMIC is valid

    IF MID$(c$, x, 7) = "$STATIC" THEN
        memmode = 1
    ELSEIF MID$(c$, x, 8) = "$DYNAMIC" THEN
        memmode = 2
    ELSEIF MID$(c$, x, 8) = "$INCLUDE" THEN
        'note: INCLUDE adds the file AFTER the line it is on has been processed
        'skip spaces until :
        FOR xx = x + 8 TO LEN(c$)
            ac = ASC(MID$(c$, xx, 1))
            IF ac = 58 THEN EXIT FOR ':
            IF ac <> 32 AND ac <> 9 THEN Give_Error "Expected $INCLUDE:'filename'": EXIT FUNCTION
        NEXT
        x = xx
        'skip spaces until '
        FOR xx = x + 1 TO LEN(c$)
            ac = ASC(MID$(c$, xx, 1))
            IF ac = 39 THEN EXIT FOR 'character:'
            IF ac <> 32 AND ac <> 9 THEN Give_Error "Expected $INCLUDE:'filename'": EXIT FUNCTION
        NEXT
        x = xx
        xx = INSTR(x + 1, c$, "'")
        IF xx = 0 THEN Give_Error "Expected $INCLUDE:'filename'": EXIT FUNCTION
        addmetainclude$ = MID$(nocasec$, x + 1, xx - x - 1)
        IF addmetainclude$ = "" THEN Give_Error "Expected $INCLUDE:'filename'": EXIT FUNCTION
    ELSEIF MID$(c$, x, 7) = "$IMPORT" THEN
        FOR xx = x + 7 TO LEN(c$)
            ac = ASC(MID$(c$, xx, 1))
            IF ac = 58 THEN EXIT FOR ':
            IF ac <> 32 AND ac <> 9 THEN Give_Error "Expected $IMPORT:'module.name'": EXIT FUNCTION
        NEXT
        x = xx
        FOR xx = x + 1 TO LEN(c$)
            ac = ASC(MID$(c$, xx, 1))
            IF ac = 39 THEN EXIT FOR 'character:'
            IF ac <> 32 AND ac <> 9 THEN Give_Error "Expected $IMPORT:'module.name'": EXIT FUNCTION
        NEXT
        x = xx
        xx = INSTR(x + 1, c$, "'")
        IF xx = 0 THEN Give_Error "Expected $IMPORT:'module.name'": EXIT FUNCTION
        addmetainclude$ = StdLib_QueueImport$(MID$(nocasec$, x + 1, xx - x - 1))
        IF Error_Happened THEN EXIT FUNCTION
    END IF

    x = INSTR(x + 1, c$, "$")
LOOP WHILE x <> 0

IF memmode = 1 THEN addmetastatic = 1
IF memmode = 2 THEN addmetadynamic = 1

GOTO lineformatdone2



lineformatdone:

'line continuation?
IF LEN(a2$) THEN
    IF RIGHT$(a2$, 1) = "_" THEN

        linecontinuation = 1 'avoids auto-format glitches
        layout$ = ""

        'remove _ from the end of the building string
        IF LEN(a2$) >= 2 THEN
            IF RIGHT$(a2$, 2) = sp + "_" THEN a2$ = LEFT$(a2$, LEN(a2$) - 1)
        END IF
        a2$ = LEFT$(a2$, LEN(a2$) - 1)

        IF inclevel THEN
            fh = 99 + inclevel
            IF EOF(fh) THEN GOTO lineformatdone2
            LINE INPUT #fh, a$
            inclinenumber(inclevel) = inclinenumber(inclevel) + 1
            GOTO includecont 'note: should not increase linenumber
        END IF

        a$ = lineinput3$
        IF a$ = CHR$(13) THEN GOTO lineformatdone2

        linenumber = linenumber + 1

        includecont:

        contline = 1
        GOTO continueline
    END IF
END IF

lineformatdone2:
IF LEFT$(a2$, 1) = sp THEN a2$ = RIGHT$(a2$, LEN(a2$) - 1)

'fix for trailing : error
IF RIGHT$(a2$, 1) = ":" THEN a2$ = a2$ + sp + "'" 'add nop

IF Debug THEN PRINT #9, "lineformat():return:" + a2$
IF Error_Happened THEN EXIT FUNCTION
lineformat$ = a2$

END FUNCTION

FUNCTION evaluateconst$ (a2$, t AS LONG)
    a$ = a2$
    IF Debug THEN PRINT #9, "evaluateconst:in:" + a$


    DIM block(1000) AS STRING
    DIM status(1000) AS INTEGER
    '0=unprocessed (can be "")
    '1=processed
    DIM btype(1000) AS LONG 'for status=1 blocks

    'put a$ into blocks
    n = numelements(a$)
    FOR i = 1 TO n
        block(i) = getelement$(a$, i)
    NEXT

    evalconstevalbrack:

    'find highest bracket level
    l = 0
    b = 0
    FOR i = 1 TO n
        IF block(i) = "(" THEN b = b + 1
        IF block(i) = ")" THEN b = b - 1
        IF b > l THEN l = b
    NEXT

    'if brackets exist, evaluate that item first
    IF l THEN

        b = 0
        e$ = ""
        FOR i = 1 TO n

            IF block(i) = ")" THEN
                IF b = l THEN block(i) = "": EXIT FOR
                b = b - 1
            END IF

            IF b >= l THEN
                IF LEN(e$) = 0 THEN e$ = block(i) ELSE e$ = e$ + sp + block(i)
                block(i) = ""
            END IF

            IF block(i) = "(" THEN
                b = b + 1
                IF b = l THEN i2 = i: block(i) = ""
            END IF

        NEXT i

        status(i) = 1
        block(i) = evaluateconst$(e$, btype(i))
        IF Error_Happened THEN EXIT FUNCTION
        GOTO evalconstevalbrack

    END IF 'l

    'linear equation remains with some pre-calculated & non-pre-calc blocks

    'problem: type QBASIC assumes and type required to store calc. value may
    '         differ dramatically. in qbasic, this would have caused an overflow,
    '         but in qbnex it MUST work. eg. 32767% * 32767%
    'solution: all interger calc. will be performed using a signed _INTEGER64
    '          all float calc. will be performed using a _FLOAT

    'convert non-calc block numbers into binary form with QBASIC-like type
    FOR i = 1 TO n
        IF status(i) = 0 THEN
            IF LEN(block(i)) THEN

                a = ASC(block(i))
                IF (a = 45 AND LEN(block(i)) > 1) OR (a >= 48 AND a <= 57) THEN 'number?

                'integers
                e$ = RIGHT$(block(i), 3)
                IF e$ = "~&&" THEN btype(i) = UINTEGER64TYPE - ISPOINTER: GOTO gotconstblkityp
                IF e$ = "~%%" THEN btype(i) = UBYTETYPE - ISPOINTER: GOTO gotconstblkityp
                e$ = RIGHT$(block(i), 2)
                IF e$ = "&&" THEN btype(i) = INTEGER64TYPE - ISPOINTER: GOTO gotconstblkityp
                IF e$ = "%%" THEN btype(i) = BYTETYPE - ISPOINTER: GOTO gotconstblkityp
                IF e$ = "~%" THEN btype(i) = UINTEGERTYPE - ISPOINTER: GOTO gotconstblkityp
                IF e$ = "~&" THEN btype(i) = ULONGTYPE - ISPOINTER: GOTO gotconstblkityp
                e$ = RIGHT$(block(i), 1)
                IF e$ = "%" THEN btype(i) = INTEGERTYPE - ISPOINTER: GOTO gotconstblkityp
                IF e$ = "&" THEN btype(i) = LONGTYPE - ISPOINTER: GOTO gotconstblkityp

                'ubit-type?
                IF INSTR(block(i), "~`") THEN
                    x = INSTR(block(i), "~`")
                    IF x = LEN(block(i)) - 1 THEN block(i) = block(i) + "1"
                    btype(i) = UBITTYPE - ISPOINTER - 1 + VAL(RIGHT$(block(i), LEN(block(i)) - x - 1))
                    block(i) = _MK$(_INTEGER64, VAL(LEFT$(block(i), x - 1)))
                    status(i) = 1
                    GOTO gotconstblktyp
                END IF

                'bit-type?
                IF INSTR(block(i), "`") THEN
                    x = INSTR(block(i), "`")
                    IF x = LEN(block(i)) THEN block(i) = block(i) + "1"
                    btype(i) = BITTYPE - ISPOINTER - 1 + VAL(RIGHT$(block(i), LEN(block(i)) - x))
                    block(i) = _MK$(_INTEGER64, VAL(LEFT$(block(i), x - 1)))
                    status(i) = 1
                    GOTO gotconstblktyp
                END IF

                'floats
                IF INSTR(block(i), "E") THEN
                    block(i) = _MK$(_FLOAT, VAL(block(i)))
                    btype(i) = SINGLETYPE - ISPOINTER
                    status(i) = 1
                    GOTO gotconstblktyp
                END IF
                IF INSTR(block(i), "D") THEN
                    block(i) = _MK$(_FLOAT, VAL(block(i)))
                    btype(i) = DOUBLETYPE - ISPOINTER
                    status(i) = 1
                    GOTO gotconstblktyp
                END IF
                IF INSTR(block(i), "F") THEN
                    block(i) = _MK$(_FLOAT, VAL(block(i)))
                    btype(i) = FLOATTYPE - ISPOINTER
                    status(i) = 1
                    GOTO gotconstblktyp
                END IF

                Give_Error "Invalid CONST expression.1": EXIT FUNCTION

                gotconstblkityp:
                block(i) = LEFT$(block(i), LEN(block(i)) - LEN(e$))
                block(i) = _MK$(_INTEGER64, VAL(block(i)))
                status(i) = 1
                gotconstblktyp:

            END IF 'a

            IF a = 34 THEN 'string?
            'no changes need to be made to block(i) which is of format "CHARACTERS",size
            btype(i) = STRINGTYPE - ISPOINTER
            status(i) = 1
        END IF

    END IF 'len<>0
END IF 'status
NEXT

'remove NULL blocks
n2 = 0
FOR i = 1 TO n
    IF block(i) <> "" THEN
        n2 = n2 + 1
        block(n2) = block(i)
        status(n2) = status(i)
        btype(n2) = btype(i)
    END IF
NEXT
n = n2

'only one block?
IF n = 1 THEN
    IF status(1) = 0 THEN Give_Error "Invalid CONST expression.2": EXIT FUNCTION
    t = btype(1)
    evaluateconst$ = block(1)
    EXIT FUNCTION
END IF 'n=1

'evaluate equation (equation cannot contain any STRINGs)

'[negation/not][variable]
e$ = block(1)
IF status(1) = 0 THEN
    IF n <> 2 THEN Give_Error "Invalid CONST expression.4": EXIT FUNCTION
    IF status(2) = 0 THEN Give_Error "Invalid CONST expression.5": EXIT FUNCTION
    IF btype(2) AND ISSTRING THEN Give_Error "Invalid CONST expression.6": EXIT FUNCTION
    o$ = block(1)

    IF o$ = CHR$(241) THEN
        IF btype(2) AND ISFLOAT THEN
            r## = -_CV(_FLOAT, block(2))
            evaluateconst$ = _MK$(_FLOAT, r##)
        ELSE
            r&& = -_CV(_INTEGER64, block(2))
            evaluateconst$ = _MK$(_INTEGER64, r&&)
        END IF
        t = btype(2)
        EXIT FUNCTION
    END IF

    IF UCASE$(o$) = "NOT" THEN
        IF btype(2) AND ISFLOAT THEN
            r&& = _CV(_FLOAT, block(2))
        ELSE
            r&& = _CV(_INTEGER64, block(2))
        END IF
        r&& = NOT r&&
        t = btype(2)
        IF t AND ISFLOAT THEN t = LONGTYPE - ISPOINTER 'markdown to LONG
        evaluateconst$ = _MK$(_INTEGER64, r&&)
        EXIT FUNCTION
    END IF

    Give_Error "Invalid CONST expression.7": EXIT FUNCTION
END IF

'[variable][bool-operator][variable]...

'get first variable
et = btype(1)
ev$ = block(1)

i = 2

evalconstequ:

'get operator
IF i >= n THEN Give_Error "Invalid CONST expression.8": EXIT FUNCTION
o$ = UCASE$(block(i))
i = i + 1
IF isoperator(o$) = 0 THEN Give_Error "Invalid CONST expression.9": EXIT FUNCTION
IF i > n THEN Give_Error "Invalid CONST expression.10": EXIT FUNCTION

'string/numeric mismatch?
IF (btype(i) AND ISSTRING) <> (et AND ISSTRING) THEN Give_Error "Invalid CONST expression.11": EXIT FUNCTION

IF et AND ISSTRING THEN
    IF o$ <> "+" THEN Give_Error "Invalid CONST expression.12": EXIT FUNCTION
    'concat strings
    s1$ = RIGHT$(ev$, LEN(ev$) - 1)
    s1$ = LEFT$(s1$, INSTR(s1$, CHR$(34)) - 1)
    s1size = VAL(RIGHT$(ev$, LEN(ev$) - LEN(s1$) - 3))
    s2$ = RIGHT$(block(i), LEN(block(i)) - 1)
    s2$ = LEFT$(s2$, INSTR(s2$, CHR$(34)) - 1)
    s2size = VAL(RIGHT$(block(i), LEN(block(i)) - LEN(s2$) - 3))
    ev$ = CHR$(34) + s1$ + s2$ + CHR$(34) + "," + str2$(s1size + s2size)
    GOTO econstmarkedup
END IF

'prepare left and right values
IF et AND ISFLOAT THEN
    linteger = 0
    l## = _CV(_FLOAT, ev$)
    l&& = l##
ELSE
    linteger = 1
    l&& = _CV(_INTEGER64, ev$)
    l## = l&&
END IF
IF btype(i) AND ISFLOAT THEN
    rinteger = 0
    r## = _CV(_FLOAT, block(i))
    r&& = r##
ELSE
    rinteger = 1
    r&& = _CV(_INTEGER64, block(i))
    r## = r&&
END IF

IF linteger = 1 AND rinteger = 1 THEN
    IF o$ = "+" THEN r&& = l&& + r&&: GOTO econstmarkupi
    IF o$ = "-" THEN r&& = l&& - r&&: GOTO econstmarkupi
    IF o$ = "*" THEN r&& = l&& * r&&: GOTO econstmarkupi
    IF o$ = "^" THEN r## = l&& ^ r&&: GOTO econstmarkupf
    IF o$ = "/" THEN r## = l&& / r&&: GOTO econstmarkupf
    IF o$ = "\" THEN r&& = l&& \ r&&: GOTO econstmarkupi
    IF o$ = "MOD" THEN r&& = l&& MOD r&&: GOTO econstmarkupi
    IF o$ = "=" THEN r&& = l&& = r&&: GOTO econstmarkupi16
    IF o$ = ">" THEN r&& = l&& > r&&: GOTO econstmarkupi16
    IF o$ = "<" THEN r&& = l&& < r&&: GOTO econstmarkupi16
    IF o$ = ">=" THEN r&& = l&& >= r&&: GOTO econstmarkupi16
    IF o$ = "<=" THEN r&& = l&& <= r&&: GOTO econstmarkupi16
    IF o$ = "<>" THEN r&& = l&& <> r&&: GOTO econstmarkupi16
    IF o$ = "IMP" THEN r&& = l&& IMP r&&: GOTO econstmarkupi
    IF o$ = "EQV" THEN r&& = l&& EQV r&&: GOTO econstmarkupi
    IF o$ = "XOR" THEN r&& = l&& XOR r&&: GOTO econstmarkupi
    IF o$ = "OR" THEN r&& = l&& OR r&&: GOTO econstmarkupi
    IF o$ = "AND" THEN r&& = l&& AND r&&: GOTO econstmarkupi
END IF

IF o$ = "+" THEN r## = l## + r##: GOTO econstmarkupf
IF o$ = "-" THEN r## = l## - r##: GOTO econstmarkupf
IF o$ = "*" THEN r## = l## * r##: GOTO econstmarkupf
IF o$ = "^" THEN r## = l## ^ r##: GOTO econstmarkupf
IF o$ = "/" THEN r## = l## / r##: GOTO econstmarkupf
IF o$ = "\" THEN r&& = l## \ r##: GOTO econstmarkupi32
IF o$ = "MOD" THEN r&& = l## MOD r##: GOTO econstmarkupi32
IF o$ = "=" THEN r&& = l## = r##: GOTO econstmarkupi16
IF o$ = ">" THEN r&& = l## > r##: GOTO econstmarkupi16
IF o$ = "<" THEN r&& = l## < r##: GOTO econstmarkupi16
IF o$ = ">=" THEN r&& = l## >= r##: GOTO econstmarkupi16
IF o$ = "<=" THEN r&& = l## <= r##: GOTO econstmarkupi16
IF o$ = "<>" THEN r&& = l## <> r##: GOTO econstmarkupi16
IF o$ = "IMP" THEN r&& = l## IMP r##: GOTO econstmarkupi32
IF o$ = "EQV" THEN r&& = l## EQV r##: GOTO econstmarkupi32
IF o$ = "XOR" THEN r&& = l## XOR r##: GOTO econstmarkupi32
IF o$ = "OR" THEN r&& = l## OR r##: GOTO econstmarkupi32
IF o$ = "AND" THEN r&& = l## AND r##: GOTO econstmarkupi32

Give_Error "Invalid CONST expression.13": EXIT FUNCTION

econstmarkupi16:
et = INTEGERTYPE - ISPOINTER
ev$ = _MK$(_INTEGER64, r&&)
GOTO econstmarkedup

econstmarkupi32:
et = LONGTYPE - ISPOINTER
ev$ = _MK$(_INTEGER64, r&&)
GOTO econstmarkedup

econstmarkupi:
IF et <> btype(i) THEN
    'keep unsigned?
    u = 0: IF (et AND ISUNSIGNED) <> 0 AND (btype(i) AND ISUNSIGNED) <> 0 THEN u = 1
    lb = et AND 511: rb = btype(i) AND 511
    ob = 0
    IF lb = rb THEN
        IF (et AND ISOFFSETINBITS) <> 0 AND (btype(i) AND ISOFFSETINBITS) <> 0 THEN ob = 1
        b = lb
    END IF
    IF lb > rb THEN
        IF (et AND ISOFFSETINBITS) <> 0 THEN ob = 1
        b = lb
    END IF
    IF lb < rb THEN
        IF (btype(i) AND ISOFFSETINBITS) <> 0 THEN ob = 1
        b = rb
    END IF
    et = b
    IF ob THEN et = et + ISOFFSETINBITS
    IF u THEN et = et + ISUNSIGNED
END IF
ev$ = _MK$(_INTEGER64, r&&)
GOTO econstmarkedup

econstmarkupf:
lfb = 0: rfb = 0
lib = 0: rib = 0
IF et AND ISFLOAT THEN lfb = et AND 511 ELSE lib = et AND 511
IF btype(i) AND ISFLOAT THEN rfb = btype(i) AND 511 ELSE rib = btype(i) AND 511
f = 32
IF lib > 16 OR rib > 16 THEN f = 64
IF lfb > 32 OR rfb > 32 THEN f = 64
IF lib > 32 OR rib > 32 THEN f = 256
IF lfb > 64 OR rfb > 64 THEN f = 256
et = ISFLOAT + f
ev$ = _MK$(_FLOAT, r##)

econstmarkedup:

i = i + 1

IF i <= n THEN GOTO evalconstequ

t = et
evaluateconst$ = ev$

END FUNCTION
