FUNCTION arrayreference$ (indexes$, typ)
    arrayprocessinghappened = 1
    '*returns an array reference: idnumber | index$
    '*does not take into consideration the type of the array

    '*expects array id to be passed in the global id structure





    idnumber$ = str2(currentid)

    DIM id2 AS idstruct

    id2 = id

    a$ = indexes$
    typ = id2.arraytype + ISARRAY + ISREFERENCE
    n$ = RTRIM$(id2.callname)

    IF a$ = "" THEN 'no indexes passed eg. a()
    r$ = "0"
    GOTO gotarrayindex
END IF

n = numelements(a$)

'find number of elements supplied
elements = 1
b = 0
FOR i = 1 TO n
    a = ASC(getelement(a$, i))
    IF a = 40 THEN b = b + 1
    IF a = 41 THEN b = b - 1
    IF a = 44 AND b = 0 THEN elements = elements + 1
NEXT

IF id2.arrayelements = -1 THEN
    IF arrayelementslist(currentid) <> 0 AND elements <> arrayelementslist(currentid) THEN Give_Error "Cannot change the number of elements an array has!": EXIT FUNCTION
    IF elements = 1 THEN id2.arrayelements = 1: ids(currentid).arrayelements = 1 'lucky guess
    arrayelementslist(currentid) = elements
ELSE
    IF elements <> id2.arrayelements THEN Give_Error "Cannot change the number of elements an array has!": EXIT FUNCTION
END IF

curarg = 1
firsti = 1
FOR i = 1 TO n
    l$ = getelement(a$, i)
    IF l$ = "(" THEN b = b + 1
    IF l$ = ")" THEN b = b - 1
    IF (l$ = "," AND b = 0) OR (i = n) THEN
        IF i = n THEN
            IF l$ = "," THEN Give_Error "Array index missing": EXIT FUNCTION
            e$ = evaluatetotyp(getelements$(a$, firsti, i), 64&)
            IF Error_Happened THEN EXIT FUNCTION
        ELSE
            e$ = evaluatetotyp(getelements$(a$, firsti, i - 1), 64&)
            IF Error_Happened THEN EXIT FUNCTION
        END IF
        IF e$ = "" THEN Give_Error "Array index missing": EXIT FUNCTION
        argi = (elements - curarg) * 4 + 4
        IF curarg = 1 THEN
            IF NoChecks = 0 THEN
                r$ = r$ + "array_check((" + e$ + ")-" + n$ + "[" + str2(argi) + "]," + n$ + "[" + str2(argi + 1) + "])+"
            ELSE
                r$ = r$ + "(" + e$ + ")-" + n$ + "[" + str2(argi) + "]+"
            END IF

        ELSE
            IF NoChecks = 0 THEN
                r$ = r$ + "array_check((" + e$ + ")-" + n$ + "[" + str2(argi) + "]," + n$ + "[" + str2(argi + 1) + "])*" + n$ + "[" + str2(argi + 2) + "]+"
            ELSE
                r$ = r$ + "((" + e$ + ")-" + n$ + "[" + str2(argi) + "])*" + n$ + "[" + str2(argi + 2) + "]+"
            END IF
        END IF
        firsti = i + 1
        curarg = curarg + 1
    END IF
NEXT
r$ = LEFT$(r$, LEN(r$) - 1) 'remove trailing +
gotarrayindex:

r$ = idnumber$ + sp3 + r$
arrayreference$ = r$
'PRINT "arrayreference returning:" + r$

END FUNCTION

FUNCTION countelements (a$)
    n = numelements(a$)
    c = 1
    FOR i = 1 TO n
        e$ = getelement$(a$, i)
        IF e$ = "(" THEN b = b + 1
        IF e$ = ")" THEN b = b - 1
        IF b < 0 THEN Give_Error "Unexpected ) encountered": EXIT FUNCTION
        IF e$ = "," AND b = 0 THEN c = c + 1
    NEXT
    countelements = c
END FUNCTION

FUNCTION udtreference$ (o$, a$, typ AS LONG)
    'UDT REFERENCE FORMAT
    'idno|udtno|udtelementno|byteoffset
    '     ^udt of the element, not of the id

    obak$ = o$

    'PRINT "called udtreference!"


    r$ = str2$(currentid) + sp3


    o = 0 'the fixed/known part of the offset

    incmem = 0
    IF id.t THEN
        u = id.t AND 511
        IF id.t AND ISINCONVENTIONALMEMORY THEN incmem = 1
    ELSE
        u = id.arraytype AND 511
        IF id.arraytype AND ISINCONVENTIONALMEMORY THEN incmem = 1
    END IF
    E = 0

    n = numelements(a$)
    IF n = 0 THEN GOTO fulludt

    i = 1
    udtfindelenext:
    IF getelement$(a$, i) <> "." THEN Give_Error "Expected .": EXIT FUNCTION
    i = i + 1
    n$ = getelement$(a$, i)
    nsym$ = removesymbol(n$): IF LEN(nsym$) THEN ntyp = typname2typ(nsym$): ntypsize = typname2typsize
    IF Error_Happened THEN EXIT FUNCTION

    IF n$ = "" THEN Give_Error "Expected .elementname": EXIT FUNCTION
    udtfindele:
    IF E = 0 THEN E = udtxnext(u) ELSE E = udtenext(E)
    IF E = 0 THEN Give_Error "Element not defined": EXIT FUNCTION
    n2$ = RTRIM$(udtename(E))
    IF udtebytealign(E) THEN
        IF o MOD 8 THEN o = o + (8 - (o MOD 8))
    END IF

    IF n$ <> n2$ THEN
        'increment fixed offset
        o = o + udtesize(E)
        GOTO udtfindele
    END IF

    'check symbol after element's name (if given) is correct
    IF LEN(nsym$) THEN

        IF udtetype(E) AND ISUDT THEN Give_Error "Invalid symbol after user defined type": EXIT FUNCTION
        IF ntyp <> udtetype(E) OR ntypsize <> udtetypesize(E) THEN
            IF nsym$ = "$" AND ((udtetype(E) AND ISFIXEDLENGTH) <> 0) THEN GOTO correctsymbol
            Give_Error "Incorrect symbol after element name": EXIT FUNCTION
        END IF
    END IF
    correctsymbol:

    'Move into another UDT structure?
    IF i <> n THEN
        IF (udtetype(E) AND ISUDT) = 0 THEN Give_Error "Expected user defined type": EXIT FUNCTION
        u = udtetype(E) AND 511
        E = 0
        i = i + 1
        GOTO udtfindelenext
    END IF

    'Change e reference to u | 0 reference?
    IF udtetype(E) AND ISUDT THEN
        u = udtetype(E) AND 511
        E = 0
    END IF

    fulludt:

    r$ = r$ + str2$(u) + sp3 + str2$(E) + sp3

    IF o MOD 8 THEN Give_Error "QBNex cannot handle bit offsets within user defined types": EXIT FUNCTION
    o = o \ 8

    IF o$ <> "" THEN
        IF o <> 0 THEN 'dont add an unnecessary 0
        o$ = o$ + "+" + str2$(o)
    END IF
ELSE
    o$ = str2$(o)
END IF

r$ = r$ + o$

udtreference$ = r$
typ = udtetype(E) + ISUDT + ISREFERENCE

'full udt override:
IF E = 0 THEN
    typ = u + ISUDT + ISREFERENCE
END IF

IF obak$ <> "" THEN typ = typ + ISARRAY
IF incmem THEN typ = typ + ISINCONVENTIONALMEMORY

'print "UDTREF:"+r$+","+str2$(typ)

END FUNCTION

FUNCTION evaluate$ (a2$, typ AS LONG)
    DIM block(1000) AS STRING
    DIM evaledblock(1000) AS INTEGER
    DIM blocktype(1000) AS LONG
    'typ IS A RETURN VALUE
    '''DIM cli(15) AS INTEGER
    a$ = a2$
    typ = -1

    IF Debug THEN PRINT #9, "evaluating:[" + a2$ + "]"
    IF a2$ = "" THEN Give_Error "Syntax error": EXIT FUNCTION






    '''cl$ = classify(a$)

    blockn = 0
    n = numelements(a$)
    b = 0 'bracketting level
    FOR i = 1 TO n

        reevaluate:




        l$ = getelement(a$, i)


        IF Debug THEN PRINT #9, "#*#*#* reevaluating:" + l$, i


        IF i <> n THEN nextl$ = getelement(a$, i + 1) ELSE nextl$ = ""

        '''getclass cl$, i, cli()

        IF b = 0 THEN 'don't evaluate anything within brackets

        IF Debug THEN PRINT #9, l$

        l2$ = l$ 'pure version of l$
        FOR try_method = 1 TO 4
            l$ = l2$
            IF try_method = 2 OR try_method = 4 THEN
                IF Error_Happened THEN EXIT FUNCTION
                dtyp$ = removesymbol(l$): IF Error_Happened THEN dtyp$ = "": Error_Happened = 0
                IF LEN(dtyp$) = 0 THEN
                    IF isoperator(l$) = 0 THEN
                        IF isvalidvariable(l$) THEN
                            IF LEFT$(l$, 1) = "_" THEN v = 27 ELSE v = ASC(UCASE$(l$)) - 64
                            l$ = l$ + defineextaz(v)
                        END IF
                    END IF
                ELSE
                    l$ = l2$
                END IF
            END IF
            try = findid(l$)
            IF Error_Happened THEN EXIT FUNCTION
            DO WHILE try

                IF Debug THEN PRINT #9, try

                'is l$ an array?
                IF nextl$ = "(" THEN
                    IF id.arraytype THEN
                        IF (subfuncn = id.insubfuncn AND try_method <= 2) OR try_method >= 3 THEN
                            arrayid = currentid
                            constequation = 0
                            i2 = i + 2
                            b2 = 0
                            evalnextele3:
                            l2$ = getelement(a$, i2)
                            IF l2$ = "(" THEN b2 = b2 + 1
                            IF l2$ = ")" THEN
                                b2 = b2 - 1
                                IF b2 = -1 THEN
                                    c$ = arrayreference(getelements$(a$, i + 2, i2 - 1), typ2)
                                    IF Error_Happened THEN EXIT FUNCTION
                                    i = i2

                                    'UDT
                                    IF typ2 AND ISUDT THEN
                                        'print "arrayref returned:"+c$
                                        getid arrayid
                                        IF Error_Happened THEN EXIT FUNCTION
                                        o$ = RIGHT$(c$, LEN(c$) - INSTR(c$, sp3))
                                        'change o$ to a byte offset if necessary
                                        u = typ2 AND 511
                                        s = udtxsize(u)
                                        IF udtxbytealign(u) THEN
                                            IF s MOD 8 THEN s = s + (8 - (s MOD 8)) 'round up to nearest byte
                                            s = s \ 8
                                        END IF
                                        o$ = "(" + o$ + ")*" + str2$(s)
                                        'print "calling evaludt with o$:"+o$
                                        GOTO evaludt
                                    END IF

                                    GOTO evalednextele3
                                END IF
                            END IF
                            i2 = i2 + 1
                            GOTO evalnextele3
                            evalednextele3:
                            blockn = blockn + 1
                            block(blockn) = c$
                            evaledblock(blockn) = 2
                            blocktype(blockn) = typ2
                            IF (typ2 AND ISSTRING) THEN stringprocessinghappened = 1
                            GOTO evaled
                        END IF
                    END IF

                ELSE
                    'not followed by "("

                    'is l$ a simple variable?
                    IF id.t <> 0 AND (id.t AND ISUDT) = 0 THEN
                        IF (subfuncn = id.insubfuncn AND try_method <= 2) OR try_method >= 3 THEN
                            constequation = 0
                            blockn = blockn + 1
                            makeidrefer block(blockn), blocktype(blockn)
                            IF (blocktype(blockn) AND ISSTRING) THEN stringprocessinghappened = 1
                            evaledblock(blockn) = 2
                            GOTO evaled
                        END IF
                    END IF

                    'is l$ a UDT?
                    IF id.t AND ISUDT THEN
                        IF (subfuncn = id.insubfuncn AND try_method <= 2) OR try_method >= 3 THEN
                            constequation = 0
                            o$ = ""
                            evaludt:
                            b2 = 0
                            i3 = i + 1
                            FOR i2 = i3 TO n
                                e2$ = getelement(a$, i2)
                                IF e2$ = "(" THEN b2 = b2 + 1
                                IF b2 = 0 THEN
                                    IF e2$ = ")" OR isoperator(e2$) THEN
                                        i4 = i2 - 1
                                        GOTO gotudt
                                    END IF
                                END IF
                                IF e2$ = ")" THEN b2 = b2 - 1
                            NEXT
                            i4 = n
                            gotudt:
                            IF i4 < i3 THEN e$ = "" ELSE e$ = getelements$(a$, i3, i4)
                            'PRINT "UDTREFERENCE:";l$; e$
                            e$ = udtreference(o$, e$, typ2)
                            IF Error_Happened THEN EXIT FUNCTION
                            i = i4
                            blockn = blockn + 1
                            block(blockn) = e$
                            evaledblock(blockn) = 2
                            blocktype(blockn) = typ2
                            'is the following next necessary?
                            'IF (typ2 AND ISSTRING) THEN stringprocessinghappened = 1
                            GOTO evaled
                        END IF
                    END IF

                END IF '"(" or no "("

                'is l$ a function?
                IF id.subfunc = 1 THEN
                    constequation = 0
                    IF getelement(a$, i + 1) = "(" THEN
                        i2 = i + 2
                        b2 = 0
                        args = 1
                        evalnextele:
                        l2$ = getelement(a$, i2)
                        IF l2$ = "(" THEN b2 = b2 + 1
                        IF l2$ = ")" THEN
                            b2 = b2 - 1
                            IF b2 = -1 THEN
                                IF i2 = i + 2 THEN Give_Error "Expected (...)": EXIT FUNCTION
                                c$ = evaluatefunc(getelements$(a$, i + 2, i2 - 1), args, typ2)
                                IF Error_Happened THEN EXIT FUNCTION
                                i = i2
                                GOTO evalednextele
                            END IF
                        END IF
                        IF l2$ = "," AND b2 = 0 THEN args = args + 1
                        i2 = i2 + 1
                        GOTO evalnextele
                    ELSE
                        'no brackets
                        c$ = evaluatefunc("", 0, typ2)
                        IF Error_Happened THEN EXIT FUNCTION
                    END IF
                    evalednextele:
                    blockn = blockn + 1
                    block(blockn) = c$
                    evaledblock(blockn) = 2
                    blocktype(blockn) = typ2
                    IF (typ2 AND ISSTRING) THEN stringprocessinghappened = 1
                    GOTO evaled
                END IF

                IF try = 2 THEN findanotherid = 1: try = findid(l$) ELSE try = 0
                IF Error_Happened THEN EXIT FUNCTION
            LOOP
        NEXT 'try method (1-4)

        'assume l$ an undefined array?

        IF i <> n THEN
            IF getelement$(a$, i + 1) = "(" THEN
                IF isoperator(l$) = 0 THEN
                    IF isvalidvariable(l$) THEN
                        IF Debug THEN
                            PRINT #9, "**************"
                            PRINT #9, "about to auto-create array:" + l$, i
                            PRINT #9, "**************"
                        END IF
                        dtyp$ = removesymbol(l$)
                        IF Error_Happened THEN EXIT FUNCTION
                        'count the number of elements
                        nume = 1
                        b2 = 0
                        FOR i2 = i + 2 TO n
                            e$ = getelement(a$, i2)
                            IF e$ = "(" THEN b2 = b2 + 1
                            IF b2 = 0 AND e$ = "," THEN nume = nume + 1
                            IF e$ = ")" THEN b2 = b2 - 1
                            IF b2 = -1 THEN EXIT FOR
                        NEXT
                        fakee$ = "10": FOR i2 = 2 TO nume: fakee$ = fakee$ + sp + "," + sp + "10": NEXT
                        IF Debug THEN PRINT #9, "evaluate:creating undefined array using dim2(" + l$ + "," + dtyp$ + ",1," + fakee$ + ")"
                        IF optionexplicit OR optionexplicitarray THEN Give_Error "Array '" + l$ + "' (" + symbol2fulltypename$(dtyp$) + ") not defined": EXIT FUNCTION
                        IF Error_Happened THEN EXIT FUNCTION
                        olddimstatic = dimstatic
                        method = 1
                        IF subfuncn THEN
                            autoarray = 1 'move dimensioning of auto array to data???.txt from inline
                            'static array declared by STATIC name()?
                            'check if varname is on the static list
                            xi = 1
                            FOR x = 1 TO staticarraylistn
                                varname2$ = getelement$(staticarraylist, xi): xi = xi + 1
                                typ2$ = getelement$(staticarraylist, xi): xi = xi + 1
                                dimmethod2 = VAL(getelement$(staticarraylist, xi)): xi = xi + 1
                                'check if they are similar
                                IF UCASE$(l$) = UCASE$(varname2$) THEN
                                    l3$ = l2$: s$ = removesymbol(l3$)
                                    IF symbol2fulltypename$(dtyp$) = typ2$ OR (dimmethod2 = 0 AND s$ = "") THEN
                                        IF Error_Happened THEN EXIT FUNCTION
                                        'adopt properties
                                        l$ = varname2$
                                        dtyp$ = typ2$
                                        method = dimmethod2
                                        dimstatic = 3
                                    END IF 'typ
                                    IF Error_Happened THEN EXIT FUNCTION
                                END IF 'varname
                            NEXT
                        END IF 'subfuncn
                        bypassNextVariable = -1
                        ignore = dim2(l$, dtyp$, method, fakee$)
                        IF Error_Happened THEN EXIT FUNCTION
                        dimstatic = olddimstatic
                        IF Debug THEN PRINT #9, "#*#*#* dim2 has returned!!!"
                        GOTO reevaluate
                    END IF
                END IF
            END IF
        END IF

        l$ = l2$ 'restore l$

    END IF 'b=0

    IF l$ = "(" THEN
        IF b = 0 THEN i1 = i + 1
        b = b + 1
    END IF

    IF b = 0 THEN
        blockn = blockn + 1
        block(blockn) = l$
        evaledblock(blockn) = 0
    END IF

    IF l$ = ")" THEN
        b = b - 1
        IF b = 0 THEN
            c$ = evaluate(getelements$(a$, i1, i - 1), typ2)
            IF Error_Happened THEN EXIT FUNCTION
            IF (typ2 AND ISSTRING) THEN stringprocessinghappened = 1
            blockn = blockn + 1
            IF (typ2 AND ISPOINTER) THEN
                block(blockn) = c$
            ELSE
                block(blockn) = "(" + c$ + ")"
            END IF
            evaledblock(blockn) = 1
            blocktype(blockn) = typ2
        END IF
    END IF
    evaled:
NEXT

r$ = "" 'return value

IF Debug THEN PRINT #9, "evaluated blocks:";
FOR i = 1 TO blockn
    IF i <> blockn THEN
        IF Debug THEN PRINT #9, block(i) + CHR$(219);
    ELSE
        IF Debug THEN PRINT #9, block(i)
    END IF
NEXT



'identify any referencable values
FOR i = 1 TO blockn
    IF isoperator(block(i)) = 0 THEN
        IF evaledblock(i) = 0 THEN

            'a number?
            c = ASC(LEFT$(block(i), 1))
            IF c = 45 OR (c >= 48 AND c <= 57) THEN
                num$ = block(i)
                'a float?
                f = 0
                x = INSTR(num$, "E")
                IF x THEN
                    f = 1: blocktype(i) = SINGLETYPE - ISPOINTER
                ELSE
                    x = INSTR(num$, "D")
                    IF x THEN
                        f = 2: blocktype(i) = DOUBLETYPE - ISPOINTER
                    ELSE
                        x = INSTR(num$, "F")
                        IF x THEN
                            f = 3: blocktype(i) = FLOATTYPE - ISPOINTER
                        END IF
                    END IF
                END IF
                IF f THEN
                    'float
                    IF f = 2 OR f = 3 THEN MID$(num$, x, 1) = "E" 'D,F invalid in C++
                    IF f = 3 THEN num$ = num$ + "L" 'otherwise number is rounded to a double
                ELSE
                    'integer
                    blocktype(i) = typname2typ(removesymbol$(num$))
                    IF Error_Happened THEN EXIT FUNCTION
                    IF blocktype(i) AND ISPOINTER THEN blocktype(i) = blocktype(i) - ISPOINTER
                    IF (blocktype(i) AND 511) > 32 THEN
                        IF blocktype(i) AND ISUNSIGNED THEN num$ = num$ + "ull" ELSE num$ = num$ + "ll"
                    END IF
                END IF
                block(i) = " " + num$ + " " 'pad with spaces to avoid C++ computation errors
                evaledblock(i) = 1
                GOTO evaledblock
            END IF

            'number?
            'fc = ASC(LEFT$(block(i), 1))
            'IF fc = 45 OR (fc >= 48 AND fc <= 57) THEN '- or 0-9
            ''it's a number
            ''check for an extension, if none, assume integer
            'blocktype(i) = INTEGER64TYPE - ISPOINTER
            'tblock$ = " " + block(i)
            'IF RIGHT$(tblock$, 2) = "##" THEN blocktype(i) = FLOATTYPE - ISPOINTER: block(i) = LEFT$(block(i), LEN(block$(i)) - 2): GOTO evfltnum
            'IF RIGHT$(tblock$, 1) = "#" THEN blocktype(i) = DOUBLETYPE - ISPOINTER: block(i) = LEFT$(block(i), LEN(block$(i)) - 1): GOTO evfltnum
            'IF RIGHT$(tblock$, 1) = "!" THEN blocktype(i) = SINGLETYPE - ISPOINTER: block(i) = LEFT$(block(i), LEN(block$(i)) - 1): GOTO evfltnum
            '
            ''C++ 32bit unsigned to signed 64bit
            'IF INSTR(block(i),".")=0 THEN
            '
            'negated=0
            'if left$(block(i),1)="-" then block(i)=right$(block(i),len(block(i))-1):negated=1
            '
            'if left$(block(i),2)="0x" then 'hex
            'if len(block(i))=10 then
            'if block(i)>="0x80000000" and block(i)<="0xFFFFFFFF" then block(i)="(int64)"+block(i): goto evnum
            'end if
            'if len(block(i))>10 then block(i)=block(i)+"ll": goto evnum
            'goto evnum
            'end if
            '
            'if left$(block(i),1)="0" then 'octal
            'if len(block(i))=12 then
            'if block(i)>="020000000000" and block(i)<="037777777777" then block(i)="(int64)"+block(i): goto evnum
            'if block(i)>"037777777777" then block(i)=block(i)+"ll": goto evnum
            'end if
            'if len(block(i))>12 then block(i)=block(i)+"ll": goto evnum
            'goto evnum
            'end if
            '
            ''decimal
            'if len(block(i))=10 then
            'if block(i)>="2147483648" and block(i)<="4294967295" then block(i)="(int64)"+block(i): goto evnum
            'if block(i)>"4294967295" then block(i)=block(i)+"ll": goto evnum
            'end if
            'if len(block(i))>10 then block(i)=block(i)+"ll"
            '
            'evnum:
            '
            'if negated=1 then block(i)="-"+block(i)
            '
            'END IF
            '
            'evfltnum:
            '
            'block(i) = " " + block(i)+" "
            'evaledblock(i) = 1
            'GOTO evaledblock
            'END IF

            'a typed string in ""
            IF LEFT$(block(i), 1) = CHR$(34) THEN
                IF RIGHT$(block(i), 1) <> CHR$(34) THEN
                    block(i) = "qbs_new_txt_len(" + block(i) + ")"
                ELSE
                    block(i) = "qbs_new_txt(" + block(i) + ")"
                END IF
                blocktype(i) = ISSTRING
                evaledblock(i) = 1
                stringprocessinghappened = 1
                GOTO evaledblock
            END IF

            'create variable
            IF isvalidvariable(block(i)) THEN
                x$ = block(i)

                typ$ = removesymbol$(x$)
                IF Error_Happened THEN EXIT FUNCTION

                'add symbol extension if none given
                IF LEN(typ$) = 0 THEN
                    IF LEFT$(x$, 1) = "_" THEN v = 27 ELSE v = ASC(UCASE$(x$)) - 64
                    typ$ = defineextaz(v)
                END IF

                'check that it hasn't just been created within this loop (a=b+b)
                try = findid(x$ + typ$)
                IF Error_Happened THEN EXIT FUNCTION
                DO WHILE try
                    IF Debug THEN PRINT #9, try
                    IF id.t <> 0 AND (id.t AND ISUDT) = 0 THEN 'is x$ a simple variable?
                    GOTO simplevarfound
                END IF
                IF try = 2 THEN findanotherid = 1: try = findid(x$ + typ$) ELSE try = 0
                IF Error_Happened THEN EXIT FUNCTION
            LOOP

            IF Debug THEN PRINT #9, "CREATING VARIABLE:" + x$
            IF optionexplicit THEN Give_Error "Variable '" + x$ + "' (" + symbol2fulltypename$(typ$) + ") not defined": EXIT FUNCTION
            bypassNextVariable = -1
            retval = dim2(x$, typ$, 1, "")
            manageVariableList "", vWatchNewVariable$, 0, 3
            IF Error_Happened THEN EXIT FUNCTION

            simplevarfound:
            constequation = 0
            makeidrefer block(i), blocktype(i)
            IF (blocktype(i) AND ISSTRING) THEN stringprocessinghappened = 1
            IF blockn = 1 THEN
                IF (blocktype(i) AND ISREFERENCE) THEN GOTO returnpointer
            END IF
            'reference value
            block(i) = refer(block(i), blocktype(i), 0): IF Error_Happened THEN EXIT FUNCTION
            evaledblock(i) = 1
            GOTO evaledblock
        END IF
        Give_Error "Invalid expression": EXIT FUNCTION

    ELSE
        IF (blocktype(i) AND ISREFERENCE) THEN
            IF blockn = 1 THEN GOTO returnpointer

            'if blocktype(i) and ISUDT then PRINT "UDT passed to refer by evaluate"

            block(i) = refer(block(i), blocktype(i), 0)
            IF Error_Happened THEN EXIT FUNCTION

        END IF

    END IF
END IF
evaledblock:
NEXT


'return a POINTER if possible
IF blockn = 1 THEN
    IF evaledblock(1) THEN
        IF (blocktype(1) AND ISREFERENCE) THEN
            returnpointer:
            IF (blocktype(1) AND ISSTRING) THEN stringprocessinghappened = 1
            IF Debug THEN PRINT #9, "evaluated reference:" + block(1)
            typ = blocktype(1)
            evaluate$ = block(1)
            EXIT FUNCTION
        END IF
    END IF
END IF
'it cannot be returned as a pointer








IF Debug THEN PRINT #9, "applying operators:";


IF typ = -1 THEN
    typ = blocktype(1) 'init typ with first blocktype


    IF isoperator(block(1)) THEN 'but what if it starts with a UNARY operator?
    typ = blocktype(2) 'init typ with second blocktype
END IF
END IF

nonop = 0
FOR i = 1 TO blockn

    IF evaledblock(i) = 0 THEN
        isop = isoperator(block(i))
        IF isop THEN
            nonop = 0

            constequation = 0

            'operator found
            o$ = block(i)
            u = operatorusage(o$, typ, i$, lhstyp, rhstyp, result)

            IF u <> 5 THEN 'not unary
            nonop = 1
            IF i = 1 OR evaledblock(i - 1) = 0 THEN
                IF i = 1 AND blockn = 1 AND o$ = "-" THEN Give_Error "Expected variable/value after '" + UCASE$(o$) + "'": EXIT FUNCTION 'guess - is neg in this case
                Give_Error "Expected variable/value before '" + UCASE$(o$) + "'": EXIT FUNCTION
            END IF
        END IF
        IF i = blockn OR evaledblock(i + 1) = 0 THEN Give_Error "Expected variable/value after '" + UCASE$(o$) + "'": EXIT FUNCTION

        'lhstyp & rhstyp bit-field values
        '1=integeral
        '2=floating point
        '4=string
        '8=bool *only used for result

        oldtyp = typ
        newtyp = blocktype(i + 1)

        'IF block(i - 1) = "6" THEN
        'PRINT o$
        'PRINT oldtyp AND ISFLOAT
        'PRINT blocktype(i - 1) AND ISFLOAT
        'END
        'END IF



        'numeric->string is illegal!
        IF (typ AND ISSTRING) = 0 AND (newtyp AND ISSTRING) <> 0 THEN
            Give_Error "Cannot convert number to string": EXIT FUNCTION
        END IF

        'Offset protection: Override conversion rules for operator as necessary
        offsetmode = 0
        offsetcvi = 0
        IF (oldtyp AND ISOFFSET) <> 0 OR (newtyp AND ISOFFSET) <> 0 THEN
            offsetmode = 2
            IF newtyp AND ISOFFSET THEN
                IF (newtyp AND ISUNSIGNED) = 0 THEN offsetmode = 1
            END IF
            IF oldtyp AND ISOFFSET THEN
                IF (oldtyp AND ISUNSIGNED) = 0 THEN offsetmode = 1
            END IF

            'depending on the operater we may do things differently
            'the default method is convert both sides to integer first
            'but these operators are different: * / ^
            IF o$ = "*" OR o$ = "/" OR o$ = "^" THEN
                IF o$ = "*" OR o$ = "^" THEN
                    'for mult, if either side is a float cast integers to 'long double's first
                    IF (newtyp AND ISFLOAT) <> 0 OR (oldtyp AND ISFLOAT) <> 0 THEN
                        offsetcvi = 1
                        IF (oldtyp AND ISFLOAT) = 0 THEN lhstyp = 2
                        IF (newtyp AND ISFLOAT) = 0 THEN rhstyp = 2
                    END IF
                END IF
                IF o$ = "/" OR o$ = "^" THEN
                    'for division or exponentials, to prevent integer division cast integers to 'long double's
                    offsetcvi = 1
                    IF (oldtyp AND ISFLOAT) = 0 THEN lhstyp = 2
                    IF (newtyp AND ISFLOAT) = 0 THEN rhstyp = 2
                END IF
            ELSE
                IF lhstyp AND 2 THEN lhstyp = 1 'force lhs and rhs to be integer values
                IF rhstyp AND 2 THEN rhstyp = 1
            END IF

            IF result = 2 THEN result = 1 'force integer result
            'note: result=1 just sets typ&=64 if typ is a float

        END IF

        'STEP 1: convert oldtyp and/or newtyp if required for the operator
        'convert lhs
        IF (oldtyp AND ISSTRING) THEN
            IF (lhstyp AND 4) = 0 THEN Give_Error "Cannot convert string to number": EXIT FUNCTION
        ELSE
            'oldtyp is numeric
            IF lhstyp = 4 THEN Give_Error "Cannot convert number to string": EXIT FUNCTION
            IF (oldtyp AND ISFLOAT) THEN
                IF (lhstyp AND 2) = 0 THEN
                    'convert float to int
                    block(i - 1) = "qbr(" + block(i - 1) + ")"
                    oldtyp = 64&
                END IF
            ELSE
                'oldtyp is an int
                IF (lhstyp AND 1) = 0 THEN
                    'convert int to float
                    block(i - 1) = "((long double)(" + block(i - 1) + "))"
                    oldtyp = 256& + ISFLOAT
                END IF
            END IF
        END IF
        'convert rhs
        IF (newtyp AND ISSTRING) THEN
            IF (rhstyp AND 4) = 0 THEN Give_Error "Cannot convert string to number": EXIT FUNCTION
        ELSE
            'newtyp is numeric
            IF rhstyp = 4 THEN Give_Error "Cannot convert number to string": EXIT FUNCTION
            IF (newtyp AND ISFLOAT) THEN
                IF (rhstyp AND 2) = 0 THEN
                    'convert float to int
                    block(i + 1) = "qbr(" + block(i + 1) + ")"
                    newtyp = 64&
                END IF
            ELSE
                'newtyp is an int
                IF (rhstyp AND 1) = 0 THEN
                    'convert int to float
                    block(i + 1) = "((long double)(" + block(i + 1) + "))"
                    newtyp = 256& + ISFLOAT
                END IF
            END IF
        END IF

        'Reduce floating point values to common base for comparison?
        IF isop = 7 THEN 'comparitive operator
        'Corrects problems encountered such as:
        '    S = 2.1
        '    IF S = 2.1 THEN PRINT "OK" ELSE PRINT "ERROR S PRINTS AS"; S; "BUT IS SEEN BY QBNex AS..."
        '    IF S < 2.1 THEN PRINT "LESS THAN 2.1"
        'concerns:
        '1. Return value from TIMER will be reduced to a SINGLE in direct comparisons
        'solution: assess, and only apply to SINGLE variables/arrays
        '2. Comparison of a double higher/lower than single range may fail
        'solution: out of range values convert to +/-1.#INF, making comparison still possible
        IF (oldtyp AND ISFLOAT) <> 0 AND (newtyp AND ISFLOAT) <> 0 THEN 'both floating point
        s1 = oldtyp AND 511: s2 = newtyp AND 511
        IF s2 < s1 THEN s1 = s2
        IF s1 = 32 THEN
            block(i - 1) = "((float)(" + block(i - 1) + "))": oldtyp = 32& + ISFLOAT
            block(i + 1) = "((float)(" + block(i + 1) + "))": newtyp = 32& + ISFLOAT
        END IF
        IF s1 = 64 THEN
            block(i - 1) = "((double)(" + block(i - 1) + "))": oldtyp = 64& + ISFLOAT
            block(i + 1) = "((double)(" + block(i + 1) + "))": newtyp = 64& + ISFLOAT
        END IF
    END IF 'both floating point
END IF 'comparitive operator

typ = newtyp

'STEP 2: markup typ
'        if either side is a float, markup typ to largest float
'        if either side is integer, markup typ
'Note: A markup is a GUESS of what the return type will be,
'      'result' can override this markup
IF (oldtyp AND ISSTRING) = 0 AND (newtyp AND ISSTRING) = 0 THEN
    IF (oldtyp AND ISFLOAT) <> 0 OR (newtyp AND ISFLOAT) <> 0 THEN
        'float
        b = 0: IF (oldtyp AND ISFLOAT) THEN b = oldtyp AND 511
        IF (newtyp AND ISFLOAT) THEN
            b2 = newtyp AND 511: IF b2 > b THEN b = b2
        END IF
        typ = ISFLOAT + b
    ELSE
        'integer
        '***THIS IS THE IDEAL MARKUP FOR A 64-BIT SYSTEM***
        'In reality 32-bit C++ only marks-up to 32-bit integers
        b = oldtyp AND 511: b2 = newtyp AND 511: IF b2 > b THEN b = b2
        typ = 64&
        IF b = 64 THEN
            IF (oldtyp AND ISUNSIGNED) <> 0 AND (newtyp AND ISUNSIGNED) <> 0 THEN typ = 64& + ISUNSIGNED
        END IF
    END IF
END IF

IF result = 1 THEN
    IF (typ AND ISFLOAT) <> 0 OR (typ AND ISSTRING) <> 0 THEN typ = 64 'otherwise keep markuped integer type
END IF
IF result = 2 THEN
    IF (typ AND ISFLOAT) = 0 THEN typ = ISFLOAT + 256
END IF
IF result = 4 THEN
    typ = ISSTRING
END IF
IF result = 8 THEN 'bool
typ = 32
END IF

'Offset protection: Force result to be an offset type with correct signage
IF offsetmode THEN
    IF result <> 8 THEN 'boolean comparison results are allowed
    typ = OFFSETTYPE - ISPOINTER: IF offsetmode = 2 THEN typ = typ + ISUNSIGNED
END IF
END IF

'override typ=ISFLOAT+256 to typ=ISFLOAT+64 for ^ operator's result
IF u = 2 THEN
    IF i$ = "pow2" THEN

        IF offsetmode THEN Give_Error "Operator '^' cannot be used with an _OFFSET": EXIT FUNCTION

        'QB-like conversion of math functions returning floating point values
        'reassess oldtype & newtype
        b = oldtyp AND 511
        IF oldtyp AND ISFLOAT THEN
            'no change to b
        ELSE
            IF b > 16 THEN b = 64 'larger than INTEGER? return DOUBLE
            IF b > 32 THEN b = 256 'larger than LONG? return FLOAT
            IF b <= 16 THEN b = 32
        END IF
        b2 = newtyp AND 511
        IF newtyp AND ISFLOAT THEN
            IF b2 > b THEN b = b2
        ELSE
            b3 = 32
            IF b2 > 16 THEN b3 = 64 'larger than INTEGER? return DOUBLE
            IF b2 > 32 THEN b3 = 256 'larger than LONG? return FLOAT
            IF b3 > b THEN b = b3
        END IF
        typ = ISFLOAT + b

    END IF 'pow2
END IF 'u=2

'STEP 3: apply operator appropriately

IF u = 5 THEN
    block(i + 1) = i$ + "(" + block(i + 1) + ")"
    block(i) = "": i = i + 1: GOTO operatorapplied
END IF

'binary operators

IF u = 1 THEN
    block(i + 1) = block(i - 1) + i$ + block(i + 1)
    block(i - 1) = "": block(i) = "": i = i + 1: GOTO operatorapplied
END IF

IF u = 2 THEN
    block(i + 1) = i$ + "(" + block(i - 1) + "," + block(i + 1) + ")"
    block(i - 1) = "": block(i) = "": i = i + 1: GOTO operatorapplied
END IF

IF u = 3 THEN
    block(i + 1) = "-(" + block(i - 1) + i$ + block(i + 1) + ")"
    block(i - 1) = "": block(i) = "": i = i + 1: GOTO operatorapplied
END IF

IF u = 4 THEN
    block(i + 1) = "~" + block(i - 1) + i$ + block(i + 1)
    block(i - 1) = "": block(i) = "": i = i + 1: GOTO operatorapplied
END IF

'...more?...

Give_Error "ERROR: Operator could not be applied correctly!": EXIT FUNCTION '<--should never happen!
operatorapplied:

IF offsetcvi THEN block(i) = "qbr(" + block(i) + ")": offsetcvi = 0
offsetmode = 0

ELSE
    nonop = nonop + 1
END IF
ELSE
    nonop = nonop + 1
END IF
IF nonop > 1 THEN Give_Error "Expected operator in equation": EXIT FUNCTION
NEXT
IF Debug THEN PRINT #9, ""

'join blocks
FOR i = 1 TO blockn
    r$ = r$ + block(i)
NEXT

IF Debug THEN
    PRINT #9, "evaluated:" + r$ + " AS TYPE:";
    IF (typ AND ISSTRING) THEN PRINT #9, "[ISSTRING]";
    IF (typ AND ISFLOAT) THEN PRINT #9, "[ISFLOAT]";
    IF (typ AND ISUNSIGNED) THEN PRINT #9, "[ISUNSIGNED]";
    IF (typ AND ISPOINTER) THEN PRINT #9, "[ISPOINTER]";
    IF (typ AND ISFIXEDLENGTH) THEN PRINT #9, "[ISFIXEDLENGTH]";
    IF (typ AND ISINCONVENTIONALMEMORY) THEN PRINT #9, "[ISINCONVENTIONALMEMORY]";
    PRINT #9, "(size in bits=" + str2$(typ AND 511) + ")"
END IF


evaluate$ = r$



END FUNCTION

FUNCTION evaluatefunc$ (a2$, args AS LONG, typ AS LONG)
    a$ = a2$

    IF Debug THEN PRINT #9, "evaluatingfunction:" + RTRIM$(id.n) + ":" + a$

    DIM id2 AS idstruct

    id2 = id
    n$ = RTRIM$(id2.n)
    typ = id2.ret
    targetid = currentid

    IF RequiresGuiCore%(n$) THEN
        DEPENDENCY(DEPENDENCY_GUI_CORE) = 1
        IF AutoConsoleOnlyEligible THEN AutoConsoleOnlyEligible = 0
    END IF

    IF RTRIM$(id2.callname) = "func_stub" THEN Give_Error "Command not implemented": EXIT FUNCTION
    IF RTRIM$(id2.callname) = "func_input" AND args = 1 AND inputfunctioncalled = 0 THEN
        inputfunctioncalled = -1
        IF vWatchOn = 1 THEN
            PRINT #12, "*__LONG_VWATCH_LINENUMBER= -4; SUB_VWATCH((ptrszint*)vwatch_global_vars,(ptrszint*)vwatch_local_vars);"
        END IF
    END IF

    SetDependency id2.Dependency

    passomit = 0
    omitarg_first = 0: omitarg_last = 0

    f$ = RTRIM$(id2.specialformat)
    IF LEN(f$) THEN 'special format given

    'count omittable args
    sqb = 0
    a = 0
    FOR fi = 1 TO LEN(f$)
        fa = ASC(f$, fi)
        IF fa = ASC_QUESTIONMARK THEN
            a = a + 1
            IF sqb <> 0 AND omitarg_first = 0 THEN omitarg_first = a
        END IF
        IF fa = ASC_LEFTSQUAREBRACKET THEN sqb = 1
        IF fa = ASC_RIGHTSQUAREBRACKET THEN sqb = 0: omitarg_last = a
    NEXT
    omitargs = omitarg_last - omitarg_first + 1

    IF args <> id2.args - omitargs AND args <> id2.args THEN
        IF LEN(id2.hr_syntax) > 0 THEN
            Give_Error "Incorrect number of arguments - Reference: " + id2.hr_syntax
        ELSE
            Give_Error "Incorrect number of arguments passed to function"
        END IF
        EXIT FUNCTION
    END IF

    passomit = 1 'pass omit flags param to function

    IF id2.args = args THEN omitarg_first = 0: omitarg_last = 0 'all arguments were passed!

ELSE 'no special format given

    IF n$ = "ASC" AND args = 2 THEN GOTO skipargnumchk
    IF id2.overloaded = -1 AND (args >= id2.minargs AND args <= id2.args) THEN GOTO skipargnumchk

    IF id2.args <> args THEN
        IF LEN(id2.hr_syntax) > 0 THEN
            Give_Error "Incorrect number of arguments - Reference: " + id2.hr_syntax
        ELSE
            Give_Error "Incorrect number of arguments passed to function"
        END IF
        EXIT FUNCTION
    END IF

END IF

skipargnumchk:

r$ = RTRIM$(id2.callname) + "("


IF id2.args <> 0 THEN

    curarg = 1
    firsti = 1

    n = numelements(a$)
    IF n = 0 THEN i = 0: GOTO noargs

    FOR i = 1 TO n



        IF curarg >= omitarg_first AND curarg <= omitarg_last THEN
            noargs:
            targettyp = CVL(MID$(id2.arg, curarg * 4 - 4 + 1, 4))

            'IF (targettyp AND ISSTRING) THEN Give_Error "QBNex doesn't support optional string arguments for functions yet!": EXIT FUNCTION

            FOR fi = 1 TO omitargs - 1: r$ = r$ + "NULL,": NEXT: r$ = r$ + "NULL"
                curarg = curarg + omitargs
                IF i = n THEN EXIT FOR
                r$ = r$ + ","
            END IF

            l$ = getelement(a$, i)
            IF l$ = "(" THEN b = b + 1
            IF l$ = ")" THEN b = b - 1
            IF (l$ = "," AND b = 0) OR (i = n) THEN

                targettyp = CVL(MID$(id2.arg, curarg * 4 - 4 + 1, 4))
                nele = ASC(MID$(id2.nele, curarg, 1))
                nelereq = ASC(MID$(id2.nelereq, curarg, 1))

                IF i = n THEN
                    e$ = getelements$(a$, firsti, i)
                ELSE
                    e$ = getelements$(a$, firsti, i - 1)
                END IF

                IF LEFT$(e$, 2) = "(" + sp THEN dereference = 1 ELSE dereference = 0



                '*special case CVI,CVL,CVS,CVD,_CV (part #1)
                IF n$ = "_CV" OR (n$ = "CV" AND qbnexprefix_set = 1) THEN
                    IF curarg = 1 THEN
                        cvtype$ = type2symbol$(e$)
                        IF Error_Happened THEN EXIT FUNCTION
                        e$ = ""
                        GOTO dontevaluate
                    END IF
                END IF

                '*special case MKI,MKL,MKS,MKD,_MK (part #1)

                IF n$ = "_MK" OR (n$ = "MK" AND qbnexprefix_set = 1) THEN
                    IF RTRIM$(id2.musthave) = "$" THEN
                        IF curarg = 1 THEN
                            mktype$ = type2symbol$(e$)
                            IF Error_Happened THEN EXIT FUNCTION
                            IF Debug THEN PRINT #9, "_MK:[" + e$ + "]:[" + mktype$ + "]"
                            e$ = ""
                            GOTO dontevaluate
                        END IF
                    END IF
                END IF

                IF n$ = "UBOUND" OR n$ = "LBOUND" THEN
                    IF curarg = 1 THEN
                        'perform a "fake" evaluation of the array
                        e$ = e$ + sp + "(" + sp + ")"
                        e$ = evaluate(e$, sourcetyp)
                        IF Error_Happened THEN EXIT FUNCTION
                        IF (sourcetyp AND ISREFERENCE) = 0 THEN Give_Error "Expected array-name": EXIT FUNCTION
                        IF (sourcetyp AND ISARRAY) = 0 THEN Give_Error "Expected array-name": EXIT FUNCTION
                        'make a note of the array's index for later
                        ulboundarray$ = e$
                        ulboundarraytyp = sourcetyp
                        e$ = ""
                        r$ = ""
                        GOTO dontevaluate
                    END IF
                END IF


                '*special case: INPUT$ function
                IF n$ = "INPUT" THEN
                    IF RTRIM$(id2.musthave) = "$" THEN
                        IF curarg = 2 THEN
                            IF LEFT$(e$, 2) = "#" + sp THEN e$ = RIGHT$(e$, LEN(e$) - 2)
                        END IF
                    END IF
                END IF


                '*special case*
                IF n$ = "ASC" THEN
                    IF curarg = 2 THEN
                        e$ = evaluatetotyp$(e$, 32&)
                        IF Error_Happened THEN EXIT FUNCTION
                        typ& = LONGTYPE - ISPOINTER
                        r$ = r$ + e$ + ")"
                        GOTO evalfuncspecial
                    END IF
                END IF


                'PRINT #12, "n$="; n$
                'PRINT #12, "curarg="; curarg
                'PRINT #12, "e$="; e$
                'PRINT #12, "r$="; r$

                '*special case*
                IF n$ = "_MEMGET" OR (n$ = "MEMGET" AND qbnexprefix_set = 1) THEN
                    IF curarg = 1 THEN
                        memget_blk$ = e$
                    END IF
                    IF curarg = 2 THEN
                        memget_offs$ = e$
                    END IF
                    IF curarg = 3 THEN
                        e$ = UCASE$(e$)
                        IF INSTR(e$, sp + "*" + sp) THEN 'multiplier will have an appended %,& or && symbol
                        IF RIGHT$(e$, 2) = "&&" THEN
                            e$ = LEFT$(e$, LEN(e$) - 2)
                        ELSE
                            IF RIGHT$(e$, 1) = "&" OR RIGHT$(e$, 1) = "%" THEN e$ = LEFT$(e$, LEN(e$) - 1)
                        END IF
                    END IF
                    t = typname2typ(e$)
                    IF t = 0 THEN Give_Error "Invalid TYPE name": EXIT FUNCTION
                    IF t AND ISOFFSETINBITS THEN Give_Error qbnexprefix$ + "BIT TYPE unsupported": EXIT FUNCTION
                    memget_size = typname2typsize
                    IF t AND ISSTRING THEN
                        IF (t AND ISFIXEDLENGTH) = 0 THEN Give_Error "Expected STRING * ...": EXIT FUNCTION
                        memget_ctyp$ = "qbs*"
                    ELSE
                        IF t AND ISUDT THEN
                            memget_size = udtxsize(t AND 511) \ 8
                            memget_ctyp$ = "void*"
                        ELSE
                            memget_size = (t AND 511) \ 8
                            memget_ctyp$ = typ2ctyp$(t, "")
                        END IF
                    END IF





                    'assume checking off
                    offs$ = evaluatetotyp(memget_offs$, OFFSETTYPE - ISPOINTER)
                    blkoffs$ = evaluatetotyp(memget_blk$, -6)
                    IF NoChecks = 0 THEN
                        'change offs$ to be the return of the safe version
                        offs$ = "func__memget((mem_block*)" + blkoffs$ + "," + offs$ + "," + str2(memget_size) + ")"
                    END IF
                    IF t AND ISSTRING THEN
                        r$ = "qbs_new_txt_len((char*)" + offs$ + "," + str2(memget_size) + ")"
                    ELSE
                        IF t AND ISUDT THEN
                            r$ = "((void*)+" + offs$ + ")"
                            t = ISUDT + ISPOINTER + (t AND 511)
                        ELSE
                            r$ = "*(" + memget_ctyp$ + "*)(" + offs$ + ")"
                            IF t AND ISPOINTER THEN t = t - ISPOINTER
                        END IF
                    END IF







                    typ& = t


                    GOTO evalfuncspecial
                END IF
            END IF

            '------------------------------------------------------------------------------------------------------------
            e2$ = e$
            e$ = evaluate(e$, sourcetyp)
            IF Error_Happened THEN EXIT FUNCTION
            '------------------------------------------------------------------------------------------------------------

            '***special case***
            IF n$ = "_MEM" OR (n$ = "MEM" AND qbnexprefix_set = 1) THEN
                IF curarg = 1 THEN
                    IF args = 1 THEN
                        targettyp = -7
                    END IF
                    IF args = 2 THEN
                        r$ = RTRIM$(id2.callname) + "_at_offset" + RIGHT$(r$, LEN(r$) - LEN(RTRIM$(id2.callname)))
                        IF (sourcetyp AND ISOFFSET) = 0 THEN Give_Error "Expected _MEM(_OFFSET-value,...)": EXIT FUNCTION
                    END IF
                END IF
            END IF

            '*special case*
            IF n$ = "_OFFSET" OR (n$ = "OFFSET" AND qbnexprefix_set = 1) THEN
                IF (sourcetyp AND ISREFERENCE) = 0 THEN
                    Give_Error qbnexprefix$ + "OFFSET expects the name of a variable/array": EXIT FUNCTION
                END IF
                IF (sourcetyp AND ISARRAY) THEN
                    IF (sourcetyp AND ISOFFSETINBITS) THEN Give_Error qbnexprefix$ + "OFFSET cannot reference _BIT type arrays": EXIT FUNCTION
                END IF
                r$ = "((uptrszint)(" + evaluatetotyp$(e2$, -6) + "))"
                IF Error_Happened THEN EXIT FUNCTION
                typ& = UOFFSETTYPE - ISPOINTER
                GOTO evalfuncspecial
            END IF '_OFFSET

            '*_OFFSET exceptions*
            IF sourcetyp AND ISOFFSET THEN
                IF n$ = "MKSMBF" AND RTRIM$(id2.musthave) = "$" THEN Give_Error "Cannot convert _OFFSET type to other types": EXIT FUNCTION
                IF n$ = "MKDMBF" AND RTRIM$(id2.musthave) = "$" THEN Give_Error "Cannot convert _OFFSET type to other types": EXIT FUNCTION
            END IF

            '*special case*
            IF n$ = "ENVIRON" THEN
                IF sourcetyp AND ISSTRING THEN
                    IF sourcetyp AND ISREFERENCE THEN e$ = refer(e$, sourcetyp, 0)
                    IF Error_Happened THEN EXIT FUNCTION
                    GOTO dontevaluate
                END IF
            END IF

            '*special case*
            IF n$ = "LEN" THEN
                typ& = LONGTYPE - ISPOINTER
                IF (sourcetyp AND ISREFERENCE) = 0 THEN
                    'could be a string expression
                    IF sourcetyp AND ISSTRING THEN
                        r$ = "((int32)(" + e$ + ")->len)"
                        GOTO evalfuncspecial
                    END IF
                    Give_Error "String expression or variable name required in LEN statement": EXIT FUNCTION
                END IF
                r$ = evaluatetotyp$(e2$, -5) 'use evaluatetotyp to get 'element' size
                IF Error_Happened THEN EXIT FUNCTION
                GOTO evalfuncspecial
            END IF


            '*special case*
            IF n$ = "_BIN" OR (n$ = "BIN" AND qbnexprefix_set = 1) THEN
                IF RTRIM$(id2.musthave) = "$" THEN
                    bits = sourcetyp AND 511

                    IF (sourcetyp AND ISSTRING) THEN Give_Error "Expected numeric value": EXIT FUNCTION
                    wasref = 0
                    IF (sourcetyp AND ISREFERENCE) THEN e$ = refer(e$, sourcetyp, 0): wasref = 1
                    IF Error_Happened THEN EXIT FUNCTION
                    bits = sourcetyp AND 511
                    IF (sourcetyp AND ISOFFSETINBITS) THEN
                        e$ = "func__bin(" + e$ + "," + str2$(bits) + ")"
                    ELSE
                        IF (sourcetyp AND ISFLOAT) THEN
                            e$ = "func__bin_float(" + e$ + ")"
                        ELSE
                            IF bits = 64 THEN
                                IF wasref = 0 THEN bits = 0
                            END IF
                            e$ = "func__bin(" + e$ + "," + str2$(bits) + ")"
                        END IF
                    END IF
                    typ& = STRINGTYPE - ISPOINTER
                    r$ = e$
                    GOTO evalfuncspecial
                END IF
            END IF

            '*special case*
            IF n$ = "OCT" THEN
                IF RTRIM$(id2.musthave) = "$" THEN
                    bits = sourcetyp AND 511

                    IF (sourcetyp AND ISSTRING) THEN Give_Error "Expected numeric value": EXIT FUNCTION
                    wasref = 0
                    IF (sourcetyp AND ISREFERENCE) THEN e$ = refer(e$, sourcetyp, 0): wasref = 1
                    IF Error_Happened THEN EXIT FUNCTION
                    bits = sourcetyp AND 511
                    IF (sourcetyp AND ISOFFSETINBITS) THEN
                        e$ = "func_oct(" + e$ + "," + str2$(bits) + ")"
                    ELSE
                        IF (sourcetyp AND ISFLOAT) THEN
                            e$ = "func_oct_float(" + e$ + ")"
                        ELSE
                            IF bits = 64 THEN
                                IF wasref = 0 THEN bits = 0
                            END IF
                            e$ = "func_oct(" + e$ + "," + str2$(bits) + ")"
                        END IF
                    END IF
                    typ& = STRINGTYPE - ISPOINTER
                    r$ = e$
                    GOTO evalfuncspecial
                END IF
            END IF

            '*special case*
            IF n$ = "HEX" THEN
                IF RTRIM$(id2.musthave) = "$" THEN
                    bits = sourcetyp AND 511
                    IF (sourcetyp AND ISSTRING) THEN Give_Error "Expected numeric value": EXIT FUNCTION
                    wasref = 0
                    IF (sourcetyp AND ISREFERENCE) THEN e$ = refer(e$, sourcetyp, 0): wasref = 1
                    IF Error_Happened THEN EXIT FUNCTION
                    bits = sourcetyp AND 511
                    IF (sourcetyp AND ISOFFSETINBITS) THEN
                        chars = (bits + 3) \ 4
                        e$ = "func_hex(" + e$ + "," + str2$(chars) + ")"
                    ELSE
                        IF (sourcetyp AND ISFLOAT) THEN
                            e$ = "func_hex_float(" + e$ + ")"
                        ELSE
                            IF bits = 8 THEN chars = 2
                            IF bits = 16 THEN chars = 4
                            IF bits = 32 THEN chars = 8
                            IF bits = 64 THEN
                                IF wasref = 1 THEN chars = 16 ELSE chars = 0
                            END IF
                            e$ = "func_hex(" + e$ + "," + str2$(chars) + ")"
                        END IF
                    END IF
                    typ& = STRINGTYPE - ISPOINTER
                    r$ = e$
                    GOTO evalfuncspecial
                END IF
            END IF


            '*special case*
            IF n$ = "EXP" THEN
                bits = sourcetyp AND 511
                IF (sourcetyp AND ISSTRING) THEN Give_Error "Expected numeric value": EXIT FUNCTION
                IF (sourcetyp AND ISREFERENCE) THEN e$ = refer(e$, sourcetyp, 0)
                IF Error_Happened THEN EXIT FUNCTION
                bits = sourcetyp AND 511
                typ& = SINGLETYPE - ISPOINTER
                IF (sourcetyp AND ISFLOAT) THEN
                    IF bits = 32 THEN e$ = "func_exp_single(" + e$ + ")" ELSE e$ = "func_exp_float(" + e$ + ")": typ& = FLOATTYPE - ISPOINTER
                ELSE
                    IF (sourcetyp AND ISOFFSETINBITS) THEN
                        e$ = "func_exp_float(" + e$ + ")": typ& = FLOATTYPE - ISPOINTER
                    ELSE
                        IF bits <= 16 THEN e$ = "func_exp_single(" + e$ + ")" ELSE e$ = "func_exp_float(" + e$ + ")": typ& = FLOATTYPE - ISPOINTER
                    END IF
                END IF
                r$ = e$
                GOTO evalfuncspecial
            END IF

            '*special case*
            IF n$ = "INT" THEN
                IF (sourcetyp AND ISSTRING) THEN Give_Error "Expected numeric value": EXIT FUNCTION
                IF (sourcetyp AND ISREFERENCE) THEN e$ = refer(e$, sourcetyp, 0)
                IF Error_Happened THEN EXIT FUNCTION
                'establish which function (if any!) should be used
                IF (sourcetyp AND ISFLOAT) THEN e$ = "floor(" + e$ + ")" ELSE e$ = "(" + e$ + ")"
                r$ = e$
                typ& = sourcetyp
                GOTO evalfuncspecial
            END IF

            '*special case*
            IF n$ = "FIX" THEN
                IF (sourcetyp AND ISSTRING) THEN Give_Error "Expected numeric value": EXIT FUNCTION
                IF (sourcetyp AND ISREFERENCE) THEN e$ = refer(e$, sourcetyp, 0)
                IF Error_Happened THEN EXIT FUNCTION
                'establish which function (if any!) should be used
                bits = sourcetyp AND 511
                IF (sourcetyp AND ISFLOAT) THEN
                    IF bits > 64 THEN e$ = "func_fix_float(" + e$ + ")" ELSE e$ = "func_fix_double(" + e$ + ")"
                ELSE
                    e$ = "(" + e$ + ")"
                END IF
                r$ = e$
                typ& = sourcetyp
                GOTO evalfuncspecial
            END IF

            '*special case*
            IF n$ = "_ROUND" OR (n$ = "ROUND" AND qbnexprefix_set = 1) THEN
                IF (sourcetyp AND ISSTRING) THEN Give_Error "Expected numeric value": EXIT FUNCTION
                IF (sourcetyp AND ISREFERENCE) THEN e$ = refer(e$, sourcetyp, 0)
                IF Error_Happened THEN EXIT FUNCTION
                'establish which function (if any!) should be used
                IF (sourcetyp AND ISFLOAT) THEN
                    bits = sourcetyp AND 511
                    IF bits > 64 THEN e$ = "func_round_float(" + e$ + ")" ELSE e$ = "func_round_double(" + e$ + ")"
                ELSE
                    e$ = "(" + e$ + ")"
                END IF
                r$ = e$
                typ& = 64&
                IF (sourcetyp AND ISOFFSET) THEN
                    IF sourcetyp AND ISUNSIGNED THEN typ& = UOFFSETTYPE - ISPOINTER ELSE typ& = OFFSETTYPE - ISPOINTER
                END IF
                GOTO evalfuncspecial
            END IF


            '*special case*
            IF n$ = "CDBL" THEN
                IF (sourcetyp AND ISOFFSET) THEN Give_Error "Cannot convert _OFFSET type to other types": EXIT FUNCTION
                IF (sourcetyp AND ISSTRING) THEN Give_Error "Expected numeric value": EXIT FUNCTION
                IF (sourcetyp AND ISREFERENCE) THEN e$ = refer(e$, sourcetyp, 0)
                IF Error_Happened THEN EXIT FUNCTION
                'establish which function (if any!) should be used
                bits = sourcetyp AND 511
                IF (sourcetyp AND ISFLOAT) THEN
                    IF bits > 64 THEN e$ = "func_cdbl_float(" + e$ + ")"
                ELSE
                    e$ = "((double)(" + e$ + "))"
                END IF
                r$ = e$
                typ& = DOUBLETYPE - ISPOINTER
                GOTO evalfuncspecial
            END IF

            '*special case*
            IF n$ = "CSNG" THEN
                IF (sourcetyp AND ISOFFSET) THEN Give_Error "Cannot convert _OFFSET type to other types": EXIT FUNCTION
                IF (sourcetyp AND ISSTRING) THEN Give_Error "Expected numeric value": EXIT FUNCTION
                IF (sourcetyp AND ISREFERENCE) THEN e$ = refer(e$, sourcetyp, 0)
                IF Error_Happened THEN EXIT FUNCTION
                'establish which function (if any!) should be used
                bits = sourcetyp AND 511
                IF (sourcetyp AND ISFLOAT) THEN
                    IF bits = 64 THEN e$ = "func_csng_double(" + e$ + ")"
                    IF bits > 64 THEN e$ = "func_csng_float(" + e$ + ")"
                ELSE
                    e$ = "((double)(" + e$ + "))"
                END IF
                r$ = e$
                typ& = SINGLETYPE - ISPOINTER
                GOTO evalfuncspecial
            END IF


            '*special case*
            IF n$ = "CLNG" THEN
                IF (sourcetyp AND ISOFFSET) THEN Give_Error "Cannot convert _OFFSET type to other types": EXIT FUNCTION
                IF (sourcetyp AND ISSTRING) THEN Give_Error "Expected numeric value": EXIT FUNCTION
                IF (sourcetyp AND ISREFERENCE) THEN e$ = refer(e$, sourcetyp, 0)
                IF Error_Happened THEN EXIT FUNCTION
                'establish which function (if any!) should be used
                bits = sourcetyp AND 511
                IF (sourcetyp AND ISFLOAT) THEN
                    IF bits > 64 THEN e$ = "func_clng_float(" + e$ + ")" ELSE e$ = "func_clng_double(" + e$ + ")"
                ELSE 'integer
                    IF (sourcetyp AND ISUNSIGNED) THEN
                        IF bits = 32 THEN e$ = "func_clng_ulong(" + e$ + ")"
                        IF bits > 32 THEN e$ = "func_clng_uint64(" + e$ + ")"
                    ELSE 'signed
                        IF bits > 32 THEN e$ = "func_clng_int64(" + e$ + ")"
                    END IF
                END IF
                r$ = e$
                typ& = 32&
                GOTO evalfuncspecial
            END IF

            '*special case*
            IF n$ = "CINT" THEN
                IF (sourcetyp AND ISOFFSET) THEN Give_Error "Cannot convert _OFFSET type to other types": EXIT FUNCTION
                IF (sourcetyp AND ISSTRING) THEN Give_Error "Expected numeric value": EXIT FUNCTION
                IF (sourcetyp AND ISREFERENCE) THEN e$ = refer(e$, sourcetyp, 0)
                IF Error_Happened THEN EXIT FUNCTION
                'establish which function (if any!) should be used
                bits = sourcetyp AND 511
                IF (sourcetyp AND ISFLOAT) THEN
                    IF bits > 64 THEN e$ = "func_cint_float(" + e$ + ")" ELSE e$ = "func_cint_double(" + e$ + ")"
                ELSE 'integer
                    IF (sourcetyp AND ISUNSIGNED) THEN
                        IF bits > 15 AND bits <= 32 THEN e$ = "func_cint_ulong(" + e$ + ")"
                        IF bits > 32 THEN e$ = "func_cint_uint64(" + e$ + ")"
                    ELSE 'signed
                        IF bits > 16 AND bits <= 32 THEN e$ = "func_cint_long(" + e$ + ")"
                        IF bits > 32 THEN e$ = "func_cint_int64(" + e$ + ")"
                    END IF
                END IF
                r$ = e$
                typ& = 16&
                GOTO evalfuncspecial
            END IF

            '*special case MKI,MKL,MKS,MKD,_MK (part #2)
            mktype = 0
            size = 0
            IF n$ = "MKI" THEN mktype = 1: mktype$ = "%"
            IF n$ = "MKL" THEN mktype = 2: mktype$ = "&"
            IF n$ = "MKS" THEN mktype = 3: mktype$ = "!"
            IF n$ = "MKD" THEN mktype = 4: mktype$ = "#"
            IF n$ = "_MK" OR (n$ = "MK" AND qbnexprefix_set = 1) THEN mktype = -1
            IF mktype THEN
                IF mktype <> -1 OR curarg = 2 THEN
                    'IF (sourcetyp AND ISOFFSET) THEN Give_Error "Cannot convert " + qbnexprefix$ + "OFFSET type to other types": EXIT FUNCTION
                    'both _MK and trad. process the following
                    qtyp& = 0
                    IF mktype$ = "%%" THEN ctype$ = "b": qtyp& = BYTETYPE - ISPOINTER
                    IF mktype$ = "~%%" THEN ctype$ = "ub": qtyp& = UBYTETYPE - ISPOINTER
                    IF mktype$ = "%" THEN ctype$ = "i": qtyp& = INTEGERTYPE - ISPOINTER
                    IF mktype$ = "~%" THEN ctype$ = "ui": qtyp& = UINTEGERTYPE - ISPOINTER
                    IF mktype$ = "&" THEN ctype$ = "l": qtyp& = LONGTYPE - ISPOINTER
                    IF mktype$ = "~&" THEN ctype$ = "ul": qtyp& = ULONGTYPE - ISPOINTER
                    IF mktype$ = "&&" THEN ctype$ = "i64": qtyp& = INTEGER64TYPE - ISPOINTER
                    IF mktype$ = "~&&" THEN ctype$ = "ui64": qtyp& = UINTEGER64TYPE - ISPOINTER
                    IF mktype$ = "!" THEN ctype$ = "s": qtyp& = SINGLETYPE - ISPOINTER
                    IF mktype$ = "#" THEN ctype$ = "d": qtyp& = DOUBLETYPE - ISPOINTER
                    IF mktype$ = "##" THEN ctype$ = "f": qtyp& = FLOATTYPE - ISPOINTER
                    IF mktype$ = "%&" THEN ctype$ = "o": qtyp& = OFFSETTYPE - ISPOINTER
                    IF mktype$ = "~%&" THEN ctype$ = "uo": qtyp& = UOFFSETTYPE - ISPOINTER
                    IF LEFT$(mktype$, 2) = "~`" THEN ctype$ = "ubit": qtyp& = UINTEGER64TYPE - ISPOINTER: size = VAL(RIGHT$(mktype$, LEN(mktype$) - 2))
                    IF LEFT$(mktype$, 1) = "`" THEN ctype$ = "bit": qtyp& = INTEGER64TYPE - ISPOINTER: size = VAL(RIGHT$(mktype$, LEN(mktype$) - 1))
                    IF qtyp& = 0 THEN Give_Error qbnexprefix$ + "MK only accepts numeric types": EXIT FUNCTION
                    IF size THEN
                        r$ = ctype$ + "2string(" + str2(size) + ","
                    ELSE
                        r$ = ctype$ + "2string("
                    END IF
                    nocomma = 1
                    targettyp = qtyp&
                END IF
            END IF

            '*special case CVI,CVL,CVS,CVD,_CV (part #2)
            cvtype = 0
            IF n$ = "CVI" THEN cvtype = 1: cvtype$ = "%"
            IF n$ = "CVL" THEN cvtype = 2: cvtype$ = "&"
            IF n$ = "CVS" THEN cvtype = 3: cvtype$ = "!"
            IF n$ = "CVD" THEN cvtype = 4: cvtype$ = "#"
            IF n$ = "_CV" OR (n$ = "CV" AND qbnexprefix_set = 1) THEN cvtype = -1
            IF cvtype THEN
                IF cvtype <> -1 OR curarg = 2 THEN
                    IF (sourcetyp AND ISSTRING) = 0 THEN Give_Error n$ + " requires a STRING argument": EXIT FUNCTION
                    IF (sourcetyp AND ISREFERENCE) THEN e$ = refer(e$, sourcetyp, 0)
                    IF Error_Happened THEN EXIT FUNCTION
                    typ& = 0
                    IF cvtype$ = "%%" THEN ctype$ = "b": typ& = BYTETYPE - ISPOINTER
                    IF cvtype$ = "~%%" THEN ctype$ = "ub": typ& = UBYTETYPE - ISPOINTER
                    IF cvtype$ = "%" THEN ctype$ = "i": typ& = INTEGERTYPE - ISPOINTER
                    IF cvtype$ = "~%" THEN ctype$ = "ui": typ& = UINTEGERTYPE - ISPOINTER
                    IF cvtype$ = "&" THEN ctype$ = "l": typ& = LONGTYPE - ISPOINTER
                    IF cvtype$ = "~&" THEN ctype$ = "ul": typ& = ULONGTYPE - ISPOINTER
                    IF cvtype$ = "&&" THEN ctype$ = "i64": typ& = INTEGER64TYPE - ISPOINTER
                    IF cvtype$ = "~&&" THEN ctype$ = "ui64": typ& = UINTEGER64TYPE - ISPOINTER
                    IF cvtype$ = "!" THEN ctype$ = "s": typ& = SINGLETYPE - ISPOINTER
                    IF cvtype$ = "#" THEN ctype$ = "d": typ& = DOUBLETYPE - ISPOINTER
                    IF cvtype$ = "##" THEN ctype$ = "f": typ& = FLOATTYPE - ISPOINTER
                    IF cvtype$ = "%&" THEN ctype$ = "o": typ& = OFFSETTYPE - ISPOINTER
                    IF cvtype$ = "~%&" THEN ctype$ = "uo": typ& = UOFFSETTYPE - ISPOINTER
                    IF LEFT$(cvtype$, 2) = "~`" THEN ctype$ = "ubit": typ& = UINTEGER64TYPE - ISPOINTER: size = VAL(RIGHT$(cvtype$, LEN(cvtype$) - 2))
                    IF LEFT$(cvtype$, 1) = "`" THEN ctype$ = "bit": typ& = INTEGER64TYPE - ISPOINTER: size = VAL(RIGHT$(cvtype$, LEN(cvtype$) - 1))
                    IF typ& = 0 THEN Give_Error qbnexprefix$ + "CV cannot return STRING type!": EXIT FUNCTION
                    IF ctype$ = "bit" OR ctype$ = "ubit" THEN
                        r$ = "string2" + ctype$ + "(" + e$ + "," + str2(size) + ")"
                    ELSE
                        r$ = "string2" + ctype$ + "(" + e$ + ")"
                    END IF
                    GOTO evalfuncspecial
                END IF
            END IF

            '*special case
            IF RTRIM$(id2.n) = "STRING" THEN
                IF curarg = 2 THEN
                    IF (sourcetyp AND ISSTRING) THEN
                        IF (sourcetyp AND ISREFERENCE) THEN e$ = refer(e$, sourcetyp, 0)
                        IF Error_Happened THEN EXIT FUNCTION
                        sourcetyp = 64&
                        e$ = "(" + e$ + "->chr[0])"
                    END IF
                END IF
            END IF

            '*special case
            IF RTRIM$(id2.n) = "SADD" THEN
                IF (sourcetyp AND ISREFERENCE) = 0 THEN
                    Give_Error "SADD only accepts variable-length string variables": EXIT FUNCTION
                END IF
                IF (sourcetyp AND ISFIXEDLENGTH) THEN
                    Give_Error "SADD only accepts variable-length string variables": EXIT FUNCTION
                END IF
                IF (sourcetyp AND ISINCONVENTIONALMEMORY) = 0 THEN
                    recompile = 1
                    cmemlist(VAL(e$)) = 1
                    r$ = "[CONVENTIONAL_MEMORY_REQUIRED]"
                    typ& = 64&
                    GOTO evalfuncspecial
                END IF
                r$ = refer(e$, sourcetyp, 0)
                IF Error_Happened THEN EXIT FUNCTION
                r$ = "((unsigned short)(" + r$ + "->chr-&cmem[1280]))"
                typ& = 64&
                GOTO evalfuncspecial
            END IF

            '*special case
            IF RTRIM$(id2.n) = "VARPTR" THEN
                IF (sourcetyp AND ISREFERENCE) = 0 THEN
                    Give_Error "Expected reference to a variable/array": EXIT FUNCTION
                END IF

                IF RTRIM$(id2.musthave) = "$" THEN
                    IF (sourcetyp AND ISINCONVENTIONALMEMORY) = 0 THEN
                        recompile = 1
                        cmemlist(VAL(e$)) = 1
                        r$ = "[CONVENTIONAL_MEMORY_REQUIRED]"
                        typ& = ISSTRING
                        GOTO evalfuncspecial
                    END IF

                    IF (sourcetyp AND ISARRAY) THEN
                        IF (sourcetyp AND ISSTRING) = 0 THEN Give_Error "VARPTR$ only accepts variable-length string arrays": EXIT FUNCTION
                        IF (sourcetyp AND ISFIXEDLENGTH) THEN Give_Error "VARPTR$ only accepts variable-length string arrays": EXIT FUNCTION
                    END IF

                    'must be a simple variable
                    '!assuming it is in cmem in DBLOCK
                    r$ = refer(e$, sourcetyp, 1)
                    IF Error_Happened THEN EXIT FUNCTION
                    IF (sourcetyp AND ISSTRING) THEN
                        IF (sourcetyp AND ISARRAY) THEN r$ = refer(e$, sourcetyp, 0)
                        IF Error_Happened THEN EXIT FUNCTION
                        r$ = r$ + "->cmem_descriptor_offset"
                        t = 3
                    ELSE
                        r$ = "((unsigned short)(((uint8*)" + r$ + ")-&cmem[1280]))"
                        '*top bit on=unsigned
                        '*second top bit on=bit-value (lower bits indicate the size)
                        'BYTE=1
                        'INTEGER=2
                        'STRING=3
                        'SINGLE=4
                        'INT64=5
                        'FLOAT=6
                        'DOUBLE=8
                        'LONG=20
                        'BIT=64+n
                        t = 0
                        IF (sourcetyp AND ISUNSIGNED) THEN t = t + 128
                        IF (sourcetyp AND ISOFFSETINBITS) THEN
                            t = t + 64
                            t = t + (sourcetyp AND 63)
                        ELSE
                            bits = sourcetyp AND 511
                            IF (sourcetyp AND ISFLOAT) THEN
                                IF bits = 32 THEN t = t + 4
                                IF bits = 64 THEN t = t + 8
                                IF bits = 256 THEN t = t + 6
                            ELSE
                                IF bits = 8 THEN t = t + 1
                                IF bits = 16 THEN t = t + 2
                                IF bits = 32 THEN t = t + 20
                                IF bits = 64 THEN t = t + 5
                            END IF
                        END IF
                    END IF
                    r$ = "func_varptr_helper(" + str2(t) + "," + r$ + ")"
                    typ& = ISSTRING
                    GOTO evalfuncspecial
                END IF 'end of varptr$











                'VARPTR
                IF (sourcetyp AND ISINCONVENTIONALMEMORY) = 0 THEN
                    recompile = 1
                    cmemlist(VAL(e$)) = 1
                    r$ = "[CONVENTIONAL_MEMORY_REQUIRED]"
                    typ& = 64&
                    GOTO evalfuncspecial
                END IF

                IF (sourcetyp AND ISARRAY) THEN
                    IF (sourcetyp AND ISOFFSETINBITS) THEN Give_Error "VARPTR cannot reference _BIT type arrays": EXIT FUNCTION

                    'string array?
                    IF (sourcetyp AND ISSTRING) THEN
                        IF (sourcetyp AND ISFIXEDLENGTH) THEN
                            getid VAL(e$)
                            IF Error_Happened THEN EXIT FUNCTION
                            m = id.tsize
                            index$ = RIGHT$(e$, LEN(e$) - INSTR(e$, sp3))
                            typ = 64&
                            r$ = "((" + index$ + ")*" + str2(m) + ")"
                            GOTO evalfuncspecial
                        ELSE
                            'return the offset of the string's descriptor
                            r$ = refer(e$, sourcetyp, 0)
                            IF Error_Happened THEN EXIT FUNCTION
                            r$ = r$ + "->cmem_descriptor_offset"
                            typ = 64&
                            GOTO evalfuncspecial
                        END IF
                    END IF

                    IF sourcetyp AND ISUDT THEN
                        e$ = RIGHT$(e$, LEN(e$) - INSTR(e$, sp3)) 'skip idnumber
                        e$ = RIGHT$(e$, LEN(e$) - INSTR(e$, sp3)) 'skip u
                        o$ = RIGHT$(e$, LEN(e$) - INSTR(e$, sp3)) 'skip e
                        typ = 64&
                        r$ = "(" + o$ + ")"
                        GOTO evalfuncspecial
                    END IF

                    'non-UDT array
                    m = (sourcetyp AND 511) \ 8 'calculate size multiplier
                    index$ = RIGHT$(e$, LEN(e$) - INSTR(e$, sp3))
                    typ = 64&
                    r$ = "((" + index$ + ")*" + str2(m) + ")"
                    GOTO evalfuncspecial

                END IF

                'not an array

                IF sourcetyp AND ISUDT THEN
                    r$ = refer(e$, sourcetyp, 1)
                    IF Error_Happened THEN EXIT FUNCTION
                    e$ = RIGHT$(e$, LEN(e$) - INSTR(e$, sp3)) 'skip idnumber
                    e$ = RIGHT$(e$, LEN(e$) - INSTR(e$, sp3)) 'skip u
                    o$ = RIGHT$(e$, LEN(e$) - INSTR(e$, sp3)) 'skip e
                    typ = 64&

                    'if sub/func arg, may not be in DBLOCK
                    getid VAL(e$)
                    IF Error_Happened THEN EXIT FUNCTION
                    IF id.sfarg THEN 'could be in DBLOCK
                    'note: segment could be the closest segment to UDT element or the base of DBLOCK
                    r$ = "varptr_dblock_check(((uint8*)" + r$ + ")+(" + o$ + "))"
                ELSE 'definitely in DBLOCK
                    'give offset relative to DBLOCK
                    r$ = "((unsigned short)(((uint8*)" + r$ + ") - &cmem[1280] + (" + o$ + ") ))"
                END IF

                GOTO evalfuncspecial
            END IF

            typ = 64&
            r$ = refer(e$, sourcetyp, 1)
            IF Error_Happened THEN EXIT FUNCTION
            IF (sourcetyp AND ISSTRING) THEN
                IF (sourcetyp AND ISFIXEDLENGTH) THEN

                    'if sub/func arg, may not be in DBLOCK
                    getid VAL(e$)
                    IF Error_Happened THEN EXIT FUNCTION
                    IF id.sfarg THEN 'could be in DBLOCK
                    r$ = "varptr_dblock_check(" + r$ + "->chr)"
                ELSE 'definitely in DBLOCK
                    r$ = "((unsigned short)(" + r$ + "->chr-&cmem[1280]))"
                END IF

            ELSE
                r$ = r$ + "->cmem_descriptor_offset"
            END IF
            GOTO evalfuncspecial
        END IF

        'single, simple variable
        'if sub/func arg, may not be in DBLOCK
        getid VAL(e$)
        IF Error_Happened THEN EXIT FUNCTION
        IF id.sfarg THEN 'could be in DBLOCK
        r$ = "varptr_dblock_check((uint8*)" + r$ + ")"
    ELSE 'definitely in DBLOCK
        r$ = "((unsigned short)(((uint8*)" + r$ + ")-&cmem[1280]))"
    END IF

    GOTO evalfuncspecial
END IF

'*special case*
IF RTRIM$(id2.n) = "VARSEG" THEN
    IF (sourcetyp AND ISREFERENCE) = 0 THEN
        Give_Error "Expected reference to a variable/array": EXIT FUNCTION
    END IF
    IF (sourcetyp AND ISINCONVENTIONALMEMORY) = 0 THEN
        recompile = 1
        cmemlist(VAL(e$)) = 1
        r$ = "[CONVENTIONAL_MEMORY_REQUIRED]"
        typ& = 64&
        GOTO evalfuncspecial
    END IF
    'array?
    IF (sourcetyp AND ISARRAY) THEN
        IF (sourcetyp AND ISFIXEDLENGTH) = 0 THEN
            IF (sourcetyp AND ISSTRING) THEN
                r$ = "80"
                typ = 64&
                GOTO evalfuncspecial
            END IF
        END IF
        typ = 64&
        r$ = "( ( ((ptrszint)(" + refer(e$, sourcetyp, 1) + "[0])) - ((ptrszint)(&cmem[0])) ) /16)"
        IF Error_Happened THEN EXIT FUNCTION
        GOTO evalfuncspecial
    END IF

    'single variable/(var-len)string/udt? (usually stored in DBLOCK)
    typ = 64&
    'if sub/func arg, may not be in DBLOCK
    getid VAL(e$)
    IF Error_Happened THEN EXIT FUNCTION
    IF id.sfarg <> 0 AND (sourcetyp AND ISSTRING) = 0 THEN
        IF sourcetyp AND ISUDT THEN
            r$ = refer(e$, sourcetyp, 1)
            IF Error_Happened THEN EXIT FUNCTION
            e$ = RIGHT$(e$, LEN(e$) - INSTR(e$, sp3)) 'skip idnumber
            e$ = RIGHT$(e$, LEN(e$) - INSTR(e$, sp3)) 'skip u
            o$ = RIGHT$(e$, LEN(e$) - INSTR(e$, sp3)) 'skip e
            r$ = "varseg_dblock_check(((uint8*)" + r$ + ")+(" + o$ + "))"
        ELSE
            r$ = "varseg_dblock_check((uint8*)" + refer(e$, sourcetyp, 1) + ")"
            IF Error_Happened THEN EXIT FUNCTION
        END IF
    ELSE
        'can be assumed to be in DBLOCK
        r$ = "80"
    END IF
    GOTO evalfuncspecial
END IF 'varseg















'note: this code has already been called...
'------------------------------------------------------------------------------------------------------------
'e2$ = e$
'e$ = evaluate(e$, sourcetyp)
'------------------------------------------------------------------------------------------------------------

'note: this comment makes no sense...
'any numeric variable, but it must be type-speficied

IF targettyp = -2 THEN
    e$ = evaluatetotyp(e2$, -2)
    IF Error_Happened THEN EXIT FUNCTION
    GOTO dontevaluate
END IF '-2

IF targettyp = -7 THEN
    e$ = evaluatetotyp(e2$, -7)
    IF Error_Happened THEN EXIT FUNCTION
    GOTO dontevaluate
END IF '-7

IF targettyp = -8 THEN
    e$ = evaluatetotyp(e2$, -8)
    IF Error_Happened THEN EXIT FUNCTION
    GOTO dontevaluate
END IF '-8

IF sourcetyp AND ISOFFSET THEN
    IF (targettyp AND ISOFFSET) = 0 THEN
        IF id2.internal_subfunc = 0 THEN Give_Error "Cannot convert _OFFSET type to other types": EXIT FUNCTION
    END IF
END IF

'note: this is used for functions like STR(...) which accept all types...
explicitreference = 0
IF targettyp = -1 THEN
    explicitreference = 1
    IF (sourcetyp AND ISSTRING) THEN Give_Error "Number required for function": EXIT FUNCTION
    targettyp = sourcetyp
    IF (targettyp AND ISPOINTER) THEN targettyp = targettyp - ISPOINTER
END IF

'pointer?
IF (targettyp AND ISPOINTER) THEN
    IF dereference = 0 THEN 'check deferencing wasn't used



    'note: array pointer
    IF (targettyp AND ISARRAY) THEN
        IF (sourcetyp AND ISREFERENCE) = 0 THEN Give_Error "Expected arrayname()": EXIT FUNCTION
        IF (sourcetyp AND ISARRAY) = 0 THEN Give_Error "Expected arrayname()": EXIT FUNCTION
        IF Debug THEN PRINT #9, "evaluatefunc:array reference:[" + e$ + "]"

        'check arrays are of same type
        targettyp2 = targettyp: sourcetyp2 = sourcetyp
        targettyp2 = targettyp2 AND (511 + ISOFFSETINBITS + ISUDT + ISSTRING + ISFIXEDLENGTH + ISFLOAT)
        sourcetyp2 = sourcetyp2 AND (511 + ISOFFSETINBITS + ISUDT + ISSTRING + ISFIXEDLENGTH + ISFLOAT)
        IF sourcetyp2 <> targettyp2 THEN Give_Error "Incorrect array type passed to function": EXIT FUNCTION

        'check arrayname was followed by '()'
        IF targettyp AND ISUDT THEN
            IF Debug THEN PRINT #9, "evaluatefunc:array reference:udt reference:[" + e$ + "]"
            'get UDT info
            udtrefid = VAL(e$)
            getid udtrefid
            IF Error_Happened THEN EXIT FUNCTION
            udtrefi = INSTR(e$, sp3) 'end of id
            udtrefi2 = INSTR(udtrefi + 1, e$, sp3) 'end of u
            udtrefu = VAL(MID$(e$, udtrefi + 1, udtrefi2 - udtrefi - 1))
            udtrefi3 = INSTR(udtrefi2 + 1, e$, sp3) 'skip e
            udtrefe = VAL(MID$(e$, udtrefi2 + 1, udtrefi3 - udtrefi2 - 1))
            o$ = RIGHT$(e$, LEN(e$) - udtrefi3)
            'note: most of the UDT info above is not required
            IF LEFT$(o$, 4) <> "(0)*" THEN Give_Error "Expected arrayname()": EXIT FUNCTION
        ELSE
            IF RIGHT$(e$, 2) <> sp3 + "0" THEN Give_Error "Expected arrayname()": EXIT FUNCTION
        END IF


        idnum = VAL(LEFT$(e$, INSTR(e$, sp3) - 1))
        getid idnum
        IF Error_Happened THEN EXIT FUNCTION

        IF targettyp AND ISFIXEDLENGTH THEN
            targettypsize = CVL(MID$(id2.argsize, curarg * 4 - 4 + 1, 4))
            IF id.tsize <> targettypsize THEN Give_Error "Incorrect array type passed to function": EXIT FUNCTION
        END IF

        IF MID$(sfcmemargs(targetid), curarg, 1) = CHR$(1) THEN 'cmem required?
        IF cmemlist(idnum) = 0 THEN
            cmemlist(idnum) = 1

            recompile = 1
        END IF
    END IF



    IF id.linkid = 0 THEN
        'if id.linkid is 0, it means the number of array elements is definietly
        'known of the array being passed, this is not some "fake"/unknown array.
        'using the numer of array elements of a fake array would be dangerous!

        IF nelereq = 0 THEN
            'only continue if the number of array elements required is unknown
            'and it needs to be set

            IF id.arrayelements <> -1 THEN
                nelereq = id.arrayelements
                MID$(id2.nelereq, curarg, 1) = CHR$(nelereq)
            END IF

            ids(targetid) = id2

        ELSE

            'the number of array elements required is known AND
            'the number of elements in the array to be passed is known



            'REMOVE FOR TESTING PURPOSES ONLY!!! SHOULD BE UNREM'd!
            'print id.arrayelements,nelereq
            '             1       ,  2

            IF id.arrayelements <> nelereq THEN Give_Error "Passing arrays with a differing number of elements to a SUB/FUNCTION is not supported": EXIT FUNCTION



        END IF
    END IF


    e$ = refer(e$, sourcetyp, 1)
    IF Error_Happened THEN EXIT FUNCTION
    GOTO dontevaluate
END IF












'note: not an array...

'target is not an array

IF (targettyp AND ISSTRING) = 0 THEN
    IF (sourcetyp AND ISREFERENCE) THEN
        idnum = VAL(LEFT$(e$, INSTR(e$, sp3) - 1)) 'id# of sourcetyp

        targettyp2 = targettyp: sourcetyp2 = sourcetyp

        'get info about source/target
        arr = 0: IF (sourcetyp2 AND ISARRAY) THEN arr = 1
        passudtelement = 0: IF (targettyp2 AND ISUDT) = 0 AND (sourcetyp2 AND ISUDT) <> 0 THEN passudtelement = 1: sourcetyp2 = sourcetyp2 - ISUDT

        'remove flags irrelevant for comparison... ISPOINTER,ISREFERENCE,ISINCONVENTIONALMEMORY,ISARRAY
        targettyp2 = targettyp2 AND (511 + ISOFFSETINBITS + ISUDT + ISFLOAT + ISSTRING)
        sourcetyp2 = sourcetyp2 AND (511 + ISOFFSETINBITS + ISUDT + ISFLOAT + ISSTRING)

        'compare types
        IF sourcetyp2 = targettyp2 THEN

            IF sourcetyp AND ISUDT THEN
                'udt/udt array

                'get info
                udtrefid = VAL(e$)
                getid udtrefid
                IF Error_Happened THEN EXIT FUNCTION
                udtrefi = INSTR(e$, sp3) 'end of id
                udtrefi2 = INSTR(udtrefi + 1, e$, sp3) 'end of u
                udtrefu = VAL(MID$(e$, udtrefi + 1, udtrefi2 - udtrefi - 1))
                udtrefi3 = INSTR(udtrefi2 + 1, e$, sp3) 'skip e
                udtrefe = VAL(MID$(e$, udtrefi2 + 1, udtrefi3 - udtrefi2 - 1))
                o$ = RIGHT$(e$, LEN(e$) - udtrefi3)
                'note: most of the UDT info above is not required

                IF arr THEN
                    n2$ = scope$ + "ARRAY_UDT_" + RTRIM$(id.n) + "[0]"
                ELSE
                    n2$ = scope$ + "UDT_" + RTRIM$(id.n)
                END IF

                e$ = "(void*)( ((char*)(" + n2$ + ")) + (" + o$ + ") )"

                'convert void* to target type*
                IF passudtelement THEN e$ = "(" + typ2ctyp$(targettyp2 + (targettyp AND ISUNSIGNED), "") + "*)" + e$
                IF Error_Happened THEN EXIT FUNCTION

            ELSE
                'not a udt
                IF arr THEN
                    IF (sourcetyp2 AND ISOFFSETINBITS) THEN Give_Error "Cannot pass BIT array offsets": EXIT FUNCTION
                    e$ = "(&(" + refer(e$, sourcetyp, 0) + "))"
                    IF Error_Happened THEN EXIT FUNCTION
                ELSE
                    e$ = refer(e$, sourcetyp, 1)
                    IF Error_Happened THEN EXIT FUNCTION
                END IF

                'note: signed/unsigned mismatch requires casting
                IF (sourcetyp AND ISUNSIGNED) <> (targettyp AND ISUNSIGNED) THEN
                    e$ = "(" + typ2ctyp$(targettyp2 + (targettyp AND ISUNSIGNED), "") + "*)" + e$
                    IF Error_Happened THEN EXIT FUNCTION
                END IF

            END IF 'udt?

            'force recompile if target needs to be in cmem and the source is not
            IF MID$(sfcmemargs(targetid), curarg, 1) = CHR$(1) THEN 'cmem required?
            IF cmemlist(idnum) = 0 THEN
                cmemlist(idnum) = 1
                recompile = 1
            END IF
        END IF

        GOTO dontevaluate
    END IF 'similar

    'IF sourcetyp2 = targettyp2 THEN
    'IF arr THEN
    'IF (sourcetyp2 AND ISOFFSETINBITS) THEN Give_Error "Cannot pass BIT array offsets yet": EXIT FUNCTION
    'e$ = "(&(" + refer(e$, sourcetyp, 0) + "))"
    'ELSE
    'e$ = refer(e$, sourcetyp, 1)
    'END IF
    'GOTO dontevaluate
    'END IF

END IF 'source is a reference

ELSE 'string
    'its a string

    IF (sourcetyp AND ISREFERENCE) THEN
        idnum = VAL(LEFT$(e$, INSTR(e$, sp3) - 1)) 'id# of sourcetyp
        IF MID$(sfcmemargs(targetid), curarg, 1) = CHR$(1) THEN 'cmem required?
        IF cmemlist(idnum) = 0 THEN
            cmemlist(idnum) = 1
            recompile = 1
        END IF
    END IF
END IF 'reference

END IF 'string

END IF 'dereference was not used
END IF 'pointer


'note: Target is not a pointer...

'IF (targettyp AND ISSTRING) = 0 THEN
'IF (sourcetyp AND ISREFERENCE) THEN
'targettyp2 = targettyp: sourcetyp2 = sourcetyp - ISREFERENCE
'IF (sourcetyp2 AND ISINCONVENTIONALMEMORY) THEN sourcetyp2 = sourcetyp2 - ISINCONVENTIONALMEMORY
'IF sourcetyp2 = targettyp2 THEN e$ = refer(e$, sourcetyp, 1): GOTO dontevaluate
'END IF
'END IF
'END IF

'String-numeric mismatch?
IF targettyp AND ISSTRING THEN
    IF (sourcetyp AND ISSTRING) = 0 THEN
        nth = curarg
        IF omitarg_last <> 0 AND nth > omitarg_last THEN nth = nth - 1
        IF ids(targetid).args = 1 THEN Give_Error "String required for function": EXIT FUNCTION
        Give_Error str_nth$(nth) + " function argument requires a string": EXIT FUNCTION
    END IF
END IF
IF (targettyp AND ISSTRING) = 0 THEN
    IF sourcetyp AND ISSTRING THEN
        nth = curarg
        IF omitarg_last <> 0 AND nth > omitarg_last THEN nth = nth - 1
        IF ids(targetid).args = 1 THEN Give_Error "Number required for function": EXIT FUNCTION
        Give_Error str_nth$(nth) + " function argument requires a number": EXIT FUNCTION
    END IF
END IF

'change to "non-pointer" value
IF (sourcetyp AND ISREFERENCE) THEN
    e$ = refer(e$, sourcetyp, 0)
    IF Error_Happened THEN EXIT FUNCTION
END IF

IF explicitreference = 0 THEN
    IF targettyp AND ISUDT THEN
        nth = curarg
        IF omitarg_last <> 0 AND nth > omitarg_last THEN nth = nth - 1
        IF qbnexprefix_set AND udtxcname(targettyp AND 511) = "_MEM" THEN
            x$ = "'" + MID$(RTRIM$(udtxcname(targettyp AND 511)), 2) + "'"
        ELSE
            x$ = "'" + RTRIM$(udtxcname(targettyp AND 511)) + "'"
        END IF
        IF ids(targetid).args = 1 THEN Give_Error "TYPE " + x$ + " required for function": EXIT FUNCTION
        Give_Error str_nth$(nth) + " function argument requires TYPE " + x$: EXIT FUNCTION
    END IF
ELSE
    IF sourcetyp AND ISUDT THEN Give_Error "Number required for function": EXIT FUNCTION
END IF

'round to integer if required
IF (sourcetyp AND ISFLOAT) THEN
    IF (targettyp AND ISFLOAT) = 0 THEN
        '**32 rounding fix
        bits = targettyp AND 511
        IF bits <= 16 THEN e$ = "qbr_float_to_long(" + e$ + ")"
        IF bits > 16 AND bits < 32 THEN e$ = "qbr_double_to_long(" + e$ + ")"
        IF bits >= 32 THEN e$ = "qbr(" + e$ + ")"
    END IF
END IF

IF explicitreference THEN
    IF (targettyp AND ISOFFSETINBITS) THEN
        'integer value can fit inside int64
        e$ = "(int64)(" + e$ + ")"
    ELSE
        IF (targettyp AND ISFLOAT) THEN
            IF (targettyp AND 511) = 32 THEN e$ = "(float)(" + e$ + ")"
            IF (targettyp AND 511) = 64 THEN e$ = "(double)(" + e$ + ")"
            IF (targettyp AND 511) = 256 THEN e$ = "(long double)(" + e$ + ")"
        ELSE
            IF (targettyp AND ISUNSIGNED) THEN
                IF (targettyp AND 511) = 8 THEN e$ = "(uint8)(" + e$ + ")"
                IF (targettyp AND 511) = 16 THEN e$ = "(uint16)(" + e$ + ")"
                IF (targettyp AND 511) = 32 THEN e$ = "(uint32)(" + e$ + ")"
                IF (targettyp AND 511) = 64 THEN e$ = "(uint64)(" + e$ + ")"
            ELSE
                IF (targettyp AND 511) = 8 THEN e$ = "(int8)(" + e$ + ")"
                IF (targettyp AND 511) = 16 THEN e$ = "(int16)(" + e$ + ")"
                IF (targettyp AND 511) = 32 THEN e$ = "(int32)(" + e$ + ")"
                IF (targettyp AND 511) = 64 THEN e$ = "(int64)(" + e$ + ")"
            END IF
        END IF 'float?
    END IF 'offset in bits?
END IF 'explicit?


IF (targettyp AND ISPOINTER) THEN 'pointer required
IF (targettyp AND ISSTRING) THEN GOTO dontevaluate 'no changes required
'20090703
t$ = typ2ctyp$(targettyp, "")
IF Error_Happened THEN EXIT FUNCTION
v$ = "pass" + str2$(uniquenumber)
'assume numeric type
IF MID$(sfcmemargs(targetid), curarg, 1) = CHR$(1) THEN 'cmem required?
bytesreq = ((targettyp AND 511) + 7) \ 8
PRINT #defdatahandle, t$ + " *" + v$ + "=NULL;"
PRINT #13, "if(" + v$ + "==NULL){"
PRINT #13, "cmem_sp-=" + str2(bytesreq) + ";"
PRINT #13, v$ + "=(" + t$ + "*)(dblock+cmem_sp);"
PRINT #13, "if (cmem_sp<qbs_cmem_sp) error(257);"
PRINT #13, "}"
e$ = "&(*" + v$ + "=" + e$ + ")"
ELSE
    PRINT #13, t$ + " " + v$ + ";"
    e$ = "&(" + v$ + "=" + e$ + ")"
END IF
GOTO dontevaluate
END IF

dontevaluate:

IF id2.ccall THEN

    'if a forced cast from a returned ccall function is in e$, remove it
    IF LEFT$(e$, 3) = "(  " THEN
        e$ = removecast$(e$)
    END IF

    IF targettyp AND ISSTRING THEN
        e$ = "(char*)(" + e$ + ")->chr"
    END IF

    IF LTRIM$(RTRIM$(e$)) = "0" THEN e$ = "NULL"

END IF

r$ = r$ + e$

'***special case****
IF n$ = "_MEM" OR (n$ = "MEM" AND qbnexprefix_set = 1) THEN
    IF args = 1 THEN
        IF curarg = 1 THEN r$ = r$ + ")": GOTO evalfuncspecial
    END IF
    IF args = 2 THEN
        IF curarg = 2 THEN r$ = r$ + ")": GOTO evalfuncspecial
    END IF
END IF

IF i <> n AND nocomma = 0 THEN r$ = r$ + ","
nocomma = 0
firsti = i + 1
curarg = curarg + 1
END IF

IF (curarg >= omitarg_first AND curarg <= omitarg_last) AND i = n THEN
    targettyp = CVL(MID$(id2.arg, curarg * 4 - 4 + 1, 4))
    'IF (targettyp AND ISSTRING) THEN Give_Error "QBNex doesn't support optional string arguments for functions yet!": EXIT FUNCTION
    FOR fi = 1 TO omitargs: r$ = r$ + ",NULL": NEXT
        curarg = curarg + omitargs
    END IF

NEXT
END IF

IF n$ = "UBOUND" OR n$ = "LBOUND" THEN
    IF r$ = ",NULL" THEN r$ = ",1"
    IF n$ = "UBOUND" THEN r2$ = "func_ubound(" ELSE r2$ = "func_lbound("
    e$ = refer$(ulboundarray$, sourcetyp, 1)
    IF Error_Happened THEN EXIT FUNCTION
    'note: ID contins refer'ed array info

    arrayelements = id.arrayelements '2009
    IF arrayelements = -1 THEN arrayelements = 1 '2009

    r$ = r2$ + e$ + r$ + "," + str2$(arrayelements) + ")"
    typ& = INTEGER64TYPE - ISPOINTER
    GOTO evalfuncspecial
END IF

IF passomit THEN
    IF omitarg_first THEN r$ = r$ + ",0" ELSE r$ = r$ + ",1"
END IF
r$ = r$ + ")"

evalfuncspecial:

IF n$ = "ABS" THEN typ& = sourcetyp 'ABS Note: ABS() returns argument #1's type

'QB-like conversion of math functions returning floating point values
IF n$ = "SIN" OR n$ = "COS" OR n$ = "TAN" OR n$ = "ATN" OR n$ = "SQR" OR n$ = "LOG" THEN
    b = sourcetyp AND 511
    IF sourcetyp AND ISFLOAT THEN
        'Default is FLOATTYPE
        IF b = 64 THEN typ& = DOUBLETYPE - ISPOINTER
        IF b = 32 THEN typ& = SINGLETYPE - ISPOINTER
    ELSE
        'Default is FLOATTYPE
        IF b <= 32 THEN typ& = DOUBLETYPE - ISPOINTER
        IF b <= 16 THEN typ& = SINGLETYPE - ISPOINTER
    END IF
END IF

IF id2.ret = ISUDT + (1) THEN
    '***special case***
    v$ = "func" + str2$(uniquenumber)
    PRINT #defdatahandle, "mem_block " + v$ + ";"
    r$ = "(" + v$ + "=" + r$ + ")"
END IF

IF id2.ccall THEN
    IF LEFT$(r$, 11) = "(  char*  )" THEN
        r$ = "qbs_new_txt(" + r$ + ")"
    END IF
END IF

IF Debug THEN PRINT #9, "evaluatefunc:out:"; r$
evaluatefunc$ = r$
END FUNCTION

FUNCTION evaluatetotyp$ (a2$, targettyp AS LONG)
    'note: 'evaluatetotyp' no longer performs 'fixoperationorder' on a2$ (in many cases, this has already been done)
    a$ = a2$
    e$ = evaluate(a$, sourcetyp)
    IF Error_Happened THEN EXIT FUNCTION

    'Offset protection:
    IF sourcetyp AND ISOFFSET THEN
        IF (targettyp AND ISOFFSET) = 0 AND targettyp >= 0 THEN
            Give_Error "Cannot convert _OFFSET type to other types": EXIT FUNCTION
        END IF
    END IF

    '-5 size
    '-6 offset
    IF targettyp = -4 OR targettyp = -5 OR targettyp = -6 THEN '? -> byte_element(offset,element size in bytes)
    IF (sourcetyp AND ISREFERENCE) = 0 THEN Give_Error "Expected variable name/array element": EXIT FUNCTION
    IF (sourcetyp AND ISOFFSETINBITS) THEN Give_Error "Variable/element cannot be BIT aligned": EXIT FUNCTION

    ' print "-4: evaluated as ["+e$+"]":sleep 1

    IF (sourcetyp AND ISUDT) THEN 'User Defined Type -> byte_element(offset,bytes)
    IF udtxvariable(sourcetyp AND 511) THEN Give_Error "UDT must have fixed size": EXIT FUNCTION
    idnumber = VAL(e$)
    i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i)
    u = VAL(e$) 'closest parent
    i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i)
    E = VAL(e$)
    i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i)
    o$ = e$
    getid idnumber
    IF Error_Happened THEN EXIT FUNCTION
    n$ = "UDT_" + RTRIM$(id.n)
    IF id.arraytype THEN
        n$ = "ARRAY_" + n$ + "[0]"
        'whole array reference examplename()?
        IF LEFT$(o$, 3) = "(0)" THEN
            'use -2 type method
            GOTO method2usealludt
        END IF
    END IF

    dst$ = "(((char*)" + scope$ + n$ + ")+(" + o$ + "))"

    'determine size of element
    IF E = 0 THEN 'no specific element, use size of entire type
    bytes$ = str2(udtxsize(u) \ 8)
ELSE 'a specific element
    IF (udtetype(E) AND ISSTRING) > 0 AND (udtetype(E) AND ISFIXEDLENGTH) = 0 AND (targettyp = -5) THEN
        evaluatetotyp$ = "(*(qbs**)" + dst$ + ")->len"
        EXIT FUNCTION
    ELSEIF (udtetype(E) AND ISSTRING) > 0 AND (udtetype(E) AND ISFIXEDLENGTH) = 0 AND (targettyp = -4) THEN
        dst$ = "(*((qbs**)((char*)" + scope$ + n$ + "+(" + o$ + "))))->chr"
        bytes$ = "(*((qbs**)((char*)" + scope$ + n$ + "+(" + o$ + "))))->len"
        evaluatetotyp$ = "byte_element((uint64)" + dst$ + "," + bytes$ + "," + NewByteElement$ + ")"
        EXIT FUNCTION
    END IF
    bytes$ = str2(udtesize(E) \ 8)
END IF
evaluatetotyp$ = "byte_element((uint64)" + dst$ + "," + bytes$ + "," + NewByteElement$ + ")"
IF targettyp = -5 THEN evaluatetotyp$ = bytes$
IF targettyp = -6 THEN evaluatetotyp$ = dst$
EXIT FUNCTION
END IF

IF (sourcetyp AND ISARRAY) THEN 'Array reference -> byte_element(offset,bytes)
'whole array reference examplename()?
IF RIGHT$(e$, 2) = sp3 + "0" THEN
    'use -2 type method
    IF sourcetyp AND ISSTRING THEN
        IF (sourcetyp AND ISFIXEDLENGTH) = 0 THEN
            Give_Error "Cannot pass array of variable-length strings": EXIT FUNCTION
        END IF
    END IF
    GOTO method2useall
END IF
'assume a specific element
IF sourcetyp AND ISSTRING THEN
    IF sourcetyp AND ISFIXEDLENGTH THEN
        idnumber = VAL(e$)
        getid idnumber
        IF Error_Happened THEN EXIT FUNCTION
        bytes$ = str2(id.tsize)
        e$ = refer(e$, sourcetyp, 0)
        IF Error_Happened THEN EXIT FUNCTION
        evaluatetotyp$ = "byte_element((uint64)" + e$ + "->chr," + bytes$ + "," + NewByteElement$ + ")"
        IF targettyp = -5 THEN evaluatetotyp$ = bytes$
        IF targettyp = -6 THEN evaluatetotyp$ = e$ + "->chr"
    ELSE
        e$ = refer(e$, sourcetyp, 0)
        IF Error_Happened THEN EXIT FUNCTION

        evaluatetotyp$ = "byte_element((uint64)" + e$ + "->chr," + e$ + "->len," + NewByteElement$ + ")"
        IF targettyp = -5 THEN evaluatetotyp$ = e$ + "->len"
        IF targettyp = -6 THEN evaluatetotyp$ = e$ + "->chr"
    END IF
    EXIT FUNCTION
END IF
e$ = refer(e$, sourcetyp, 0)
IF Error_Happened THEN EXIT FUNCTION
e$ = "(&(" + e$ + "))"
bytes$ = str2((sourcetyp AND 511) \ 8)
evaluatetotyp$ = "byte_element((uint64)" + e$ + "," + bytes$ + "," + NewByteElement$ + ")"
IF targettyp = -5 THEN evaluatetotyp$ = bytes$
IF targettyp = -6 THEN evaluatetotyp$ = e$
EXIT FUNCTION
END IF

IF sourcetyp AND ISSTRING THEN 'String -> byte_element(offset,bytes)
IF sourcetyp AND ISFIXEDLENGTH THEN
    idnumber = VAL(e$)
    getid idnumber
    IF Error_Happened THEN EXIT FUNCTION
    bytes$ = str2(id.tsize)
    e$ = refer(e$, sourcetyp, 0)
    IF Error_Happened THEN EXIT FUNCTION
ELSE
    e$ = refer(e$, sourcetyp, 0)
    IF Error_Happened THEN EXIT FUNCTION
    bytes$ = e$ + "->len"
END IF
evaluatetotyp$ = "byte_element((uint64)" + e$ + "->chr," + bytes$ + "," + NewByteElement$ + ")"
IF targettyp = -5 THEN evaluatetotyp$ = bytes$
IF targettyp = -6 THEN evaluatetotyp$ = e$ + "->chr"
EXIT FUNCTION
END IF

'Standard variable -> byte_element(offset,bytes)
e$ = refer(e$, sourcetyp, 1) 'get the variable's formal name
IF Error_Happened THEN EXIT FUNCTION
size = (sourcetyp AND 511) \ 8 'calculate its size in bytes
evaluatetotyp$ = "byte_element((uint64)" + e$ + "," + str2(size) + "," + NewByteElement$ + ")"
IF targettyp = -5 THEN evaluatetotyp$ = str2(size)
IF targettyp = -6 THEN evaluatetotyp$ = e$
EXIT FUNCTION

END IF '-4, -5, -6




IF targettyp = -8 THEN '? -> _MEM structure helper {offset, fullsize, typeval, elementsize, sf_mem_lock|???}
IF (sourcetyp AND ISREFERENCE) = 0 THEN Give_Error "Expected variable name/array element": EXIT FUNCTION
IF (sourcetyp AND ISOFFSETINBITS) THEN Give_Error "Variable/element cannot be BIT aligned": EXIT FUNCTION


IF (sourcetyp AND ISUDT) THEN 'User Defined Type -> byte_element(offset,bytes)
idnumber = VAL(e$)
i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i)
u = VAL(e$) 'closest parent
i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i)
E = VAL(e$)
i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i)
o$ = e$
getid idnumber
IF Error_Happened THEN EXIT FUNCTION
n$ = "UDT_" + RTRIM$(id.n)
IF id.arraytype THEN
    n$ = "ARRAY_" + n$ + "[0]"
    'whole array reference examplename()?
    IF LEFT$(o$, 3) = "(0)" THEN
        'use -7 type method
        GOTO method2usealludt__7
    END IF
END IF
'determine size of element
IF E = 0 THEN 'no specific element, use size of entire type
bytes$ = str2(udtxsize(u) \ 8)
t1 = ISUDT + udtetype(u)
ELSE 'a specific element
    bytes$ = str2(udtesize(E) \ 8)
    t1 = udtetype(E)
END IF
dst$ = "(((char*)" + scope$ + n$ + ")+(" + o$ + "))"
'evaluatetotyp$ = "byte_element((uint64)" + dst$ + "," + bytes$ + "," + NewByteElement$ + ")"
'IF targettyp = -5 THEN evaluatetotyp$ = bytes$
'IF targettyp = -6 THEN evaluatetotyp$ = dst$

t = Type2MemTypeValue(t1)
evaluatetotyp$ = "(ptrszint)" + dst$ + "," + bytes$ + "," + str2(t) + "," + bytes$ + ",sf_mem_lock"

EXIT FUNCTION
END IF

IF (sourcetyp AND ISARRAY) THEN 'Array reference -> byte_element(offset,bytes)
'whole array reference examplename()?
IF RIGHT$(e$, 2) = sp3 + "0" THEN
    'use -7 type method
    IF sourcetyp AND ISSTRING THEN
        IF (sourcetyp AND ISFIXEDLENGTH) = 0 THEN
            Give_Error "Cannot pass array of variable-length strings": EXIT FUNCTION
        END IF
    END IF
    GOTO method2useall__7
END IF

idnumber = VAL(e$)
getid idnumber
IF Error_Happened THEN EXIT FUNCTION
n$ = RTRIM$(id.callname)
lk$ = "(mem_lock*)((ptrszint*)" + n$ + ")[" + str2(4 * id.arrayelements + 4 + 1 - 1) + "]"

'assume a specific element

IF sourcetyp AND ISSTRING THEN
    IF sourcetyp AND ISFIXEDLENGTH THEN
        bytes$ = str2(id.tsize)
        e$ = refer(e$, sourcetyp, 0)
        IF Error_Happened THEN EXIT FUNCTION
        'evaluatetotyp$ = "byte_element((uint64)" + e$ + "->chr," + bytes$ + "," + NewByteElement$ + ")"
        'IF targettyp = -5 THEN evaluatetotyp$ = bytes$
        'IF targettyp = -6 THEN evaluatetotyp$ = e$ + "->chr"

        t = Type2MemTypeValue(sourcetyp)
        evaluatetotyp$ = "(ptrszint)" + e$ + "->chr," + bytes$ + "," + str2(t) + "," + bytes$ + "," + lk$

    ELSE

        Give_Error qbnexprefix$ + "MEMELEMENT cannot reference variable-length strings": EXIT FUNCTION

    END IF
    EXIT FUNCTION
END IF

e$ = refer(e$, sourcetyp, 0)
IF Error_Happened THEN EXIT FUNCTION
e$ = "(&(" + e$ + "))"
bytes$ = str2((sourcetyp AND 511) \ 8)
'evaluatetotyp$ = "byte_element((uint64)" + e$ + "," + bytes$ + "," + NewByteElement$ + ")"
'IF targettyp = -5 THEN evaluatetotyp$ = bytes$
'IF targettyp = -6 THEN evaluatetotyp$ = e$

t = Type2MemTypeValue(sourcetyp)
evaluatetotyp$ = "(ptrszint)" + e$ + "," + bytes$ + "," + str2(t) + "," + bytes$ + "," + lk$

EXIT FUNCTION
END IF 'isarray

IF sourcetyp AND ISSTRING THEN 'String -> byte_element(offset,bytes)
IF sourcetyp AND ISFIXEDLENGTH THEN
    idnumber = VAL(e$)
    getid idnumber
    IF Error_Happened THEN EXIT FUNCTION
    bytes$ = str2(id.tsize)
    e$ = refer(e$, sourcetyp, 0)
    IF Error_Happened THEN EXIT FUNCTION
ELSE
    Give_Error qbnexprefix$ + "MEMELEMENT cannot reference variable-length strings": EXIT FUNCTION
END IF

'evaluatetotyp$ = "byte_element((uint64)" + e$ + "->chr," + bytes$ + "," + NewByteElement$ + ")"
'IF targettyp = -5 THEN evaluatetotyp$ = bytes$
'IF targettyp = -6 THEN evaluatetotyp$ = e$ + "->chr"

t = Type2MemTypeValue(sourcetyp)
evaluatetotyp$ = "(ptrszint)" + e$ + "->chr," + bytes$ + "," + str2(t) + "," + bytes$ + ",sf_mem_lock"

EXIT FUNCTION
END IF

'Standard variable -> byte_element(offset,bytes)
e$ = refer(e$, sourcetyp, 1) 'get the variable's formal name
IF Error_Happened THEN EXIT FUNCTION
size = (sourcetyp AND 511) \ 8 'calculate its size in bytes
'evaluatetotyp$ = "byte_element((uint64)" + e$ + "," + str2(size) + "," + NewByteElement$ + ")"
'IF targettyp = -5 THEN evaluatetotyp$ = str2(size)
'IF targettyp = -6 THEN evaluatetotyp$ = e$

t = Type2MemTypeValue(sourcetyp)
evaluatetotyp$ = "(ptrszint)" + e$ + "," + str2(size) + "," + str2(t) + "," + str2(size) + ",sf_mem_lock"

EXIT FUNCTION

END IF '-8










IF targettyp = -7 THEN '? -> _MEM structure helper {offset, fullsize, typeval, elementsize, sf_mem_lock|???}
method2useall__7:
IF (sourcetyp AND ISREFERENCE) = 0 THEN Give_Error "Expected variable name/array element": EXIT FUNCTION
IF (sourcetyp AND ISOFFSETINBITS) THEN Give_Error "Variable/element cannot be BIT aligned": EXIT FUNCTION

'User Defined Type
IF (sourcetyp AND ISUDT) THEN
    '           print "CI: -2 type from a UDT":sleep 1
    idnumber = VAL(e$)
    i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i)
    u = VAL(e$) 'closest parent
    i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i)
    E = VAL(e$)
    i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i)

    o$ = e$
    getid idnumber
    IF Error_Happened THEN EXIT FUNCTION
    n$ = "UDT_" + RTRIM$(id.n): IF id.arraytype THEN n$ = "ARRAY_" + n$ + "[0]"
    method2usealludt__7:
    bytes$ = variablesize$(-1) + "-(" + o$ + ")"
    IF Error_Happened THEN EXIT FUNCTION
    dst$ = "(((char*)" + scope$ + n$ + ")+(" + o$ + "))"


    'evaluatetotyp$ = "byte_element((uint64)" + dst$ + "," + bytes$ + "," + NewByteElement$ + ")"

    'note: myudt.myelement results in a size of 1 because it is a continuous run of no consistent granularity
    IF E <> 0 THEN size = 1 ELSE size = udtxsize(u) \ 8

    t = Type2MemTypeValue(sourcetyp)
    evaluatetotyp$ = "(ptrszint)" + dst$ + "," + bytes$ + "," + str2(t) + "," + str2(size) + ",sf_mem_lock"

    EXIT FUNCTION
END IF

'Array reference
IF (sourcetyp AND ISARRAY) THEN
    IF sourcetyp AND ISSTRING THEN
        IF (sourcetyp AND ISFIXEDLENGTH) = 0 THEN
            Give_Error qbnexprefix$ + "MEM cannot reference variable-length strings": EXIT FUNCTION
        END IF
    END IF

    idnumber = VAL(e$)
    getid idnumber
    IF Error_Happened THEN EXIT FUNCTION

    n$ = RTRIM$(id.callname)
    lk$ = "(mem_lock*)((ptrszint*)" + n$ + ")[" + str2(4 * id.arrayelements + 4 + 1 - 1) + "]"

    tsize = id.tsize 'used later to determine element size of fixed length strings
    'note: array references consist of idnumber|unmultiplied-element-index
    index$ = RIGHT$(e$, LEN(e$) - INSTR(e$, sp3)) 'get element index
    bytes$ = variablesize$(-1)
    IF Error_Happened THEN EXIT FUNCTION
    e$ = refer(e$, sourcetyp, 0)
    IF Error_Happened THEN EXIT FUNCTION

    IF sourcetyp AND ISSTRING THEN
        e$ = "((" + e$ + ")->chr)" '[2013] handle fixed string arrays differently because they are already pointers
    ELSE
        e$ = "(&(" + e$ + "))"
    END IF

    '           print "CI: array: e$["+e$+"], bytes$["+bytes$+"]":sleep 1
    'calculate size of elements
    IF sourcetyp AND ISSTRING THEN
        bytes = tsize
    ELSE
        bytes = (sourcetyp AND 511) \ 8
    END IF
    bytes$ = bytes$ + "-(" + str2(bytes) + "*(" + index$ + "))"

    t = Type2MemTypeValue(sourcetyp)
    evaluatetotyp$ = "(ptrszint)" + e$ + "," + bytes$ + "," + str2(t) + "," + str2(bytes) + "," + lk$

    EXIT FUNCTION
END IF

'String
IF sourcetyp AND ISSTRING THEN
    IF (sourcetyp AND ISFIXEDLENGTH) = 0 THEN Give_Error qbnexprefix$ + "MEM cannot reference variable-length strings": EXIT FUNCTION

    idnumber = VAL(e$)
    getid idnumber: IF Error_Happened THEN EXIT FUNCTION
    bytes$ = str2(id.tsize)
    e$ = refer(e$, sourcetyp, 0): IF Error_Happened THEN EXIT FUNCTION

    t = Type2MemTypeValue(sourcetyp)
    evaluatetotyp$ = "(ptrszint)" + e$ + "->chr," + bytes$ + "," + str2(t) + "," + bytes$ + ",sf_mem_lock"

    EXIT FUNCTION
END IF

'Standard variable -> byte_element(offset,bytes)
e$ = refer(e$, sourcetyp, 1) 'get the variable's formal name
IF Error_Happened THEN EXIT FUNCTION
size = (sourcetyp AND 511) \ 8 'calculate its size in bytes

t = Type2MemTypeValue(sourcetyp)
evaluatetotyp$ = "(ptrszint)" + e$ + "," + str2(size) + "," + str2(t) + "," + str2(size) + ",sf_mem_lock"

EXIT FUNCTION

END IF '-7 _MEM structure helper


IF targettyp = -2 THEN '? -> byte_element(offset,max possible bytes)
method2useall:
' print "CI: eval2typ detected target type of -2 for ["+a2$+"] evaluated as ["+e$+"]":sleep 1

IF (sourcetyp AND ISREFERENCE) = 0 THEN Give_Error "Expected variable name/array element": EXIT FUNCTION
IF (sourcetyp AND ISOFFSETINBITS) THEN Give_Error "Variable/element cannot be BIT aligned": EXIT FUNCTION

'User Defined Type -> byte_element(offset,bytes)
IF (sourcetyp AND ISUDT) THEN
    '           print "CI: -2 type from a UDT":sleep 1
    idnumber = VAL(e$)
    i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i)
    u = VAL(e$) 'closest parent
    i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i)
    E = VAL(e$)
    i = INSTR(e$, sp3): e$ = RIGHT$(e$, LEN(e$) - i)
    o$ = e$
    getid idnumber
    IF Error_Happened THEN EXIT FUNCTION
    n$ = "UDT_" + RTRIM$(id.n): IF id.arraytype THEN n$ = "ARRAY_" + n$ + "[0]"
    method2usealludt:
    bytes$ = variablesize$(-1) + "-(" + o$ + ")"
    IF Error_Happened THEN EXIT FUNCTION
    dst$ = "(((char*)" + scope$ + n$ + ")+(" + o$ + "))"
    evaluatetotyp$ = "byte_element((uint64)" + dst$ + "," + bytes$ + "," + NewByteElement$ + ")"
    IF targettyp = -5 THEN evaluatetotyp$ = bytes$
    IF targettyp = -6 THEN evaluatetotyp$ = dst$
    EXIT FUNCTION
END IF

'Array reference -> byte_element(offset,bytes)
IF (sourcetyp AND ISARRAY) THEN
    'array of variable length strings (special case, can only refer to single element)
    IF sourcetyp AND ISSTRING THEN
        IF (sourcetyp AND ISFIXEDLENGTH) = 0 THEN
            e$ = refer(e$, sourcetyp, 0)
            IF Error_Happened THEN EXIT FUNCTION
            evaluatetotyp$ = "byte_element((uint64)" + e$ + "->chr," + e$ + "->len," + NewByteElement$ + ")"
            IF targettyp = -5 THEN evaluatetotyp$ = e$ + "->len"
            IF targettyp = -6 THEN evaluatetotyp$ = e$ + "->chr"
            EXIT FUNCTION
        END IF
    END IF
    idnumber = VAL(e$)
    getid idnumber
    IF Error_Happened THEN EXIT FUNCTION
    tsize = id.tsize 'used later to determine element size of fixed length strings
    'note: array references consist of idnumber|unmultiplied-element-index
    index$ = RIGHT$(e$, LEN(e$) - INSTR(e$, sp3)) 'get element index
    bytes$ = variablesize$(-1)
    IF Error_Happened THEN EXIT FUNCTION
    e$ = refer(e$, sourcetyp, 0)
    IF Error_Happened THEN EXIT FUNCTION
    e$ = "(&(" + e$ + "))"
    '           print "CI: array: e$["+e$+"], bytes$["+bytes$+"]":sleep 1
    'calculate size of elements
    IF sourcetyp AND ISSTRING THEN
        bytes = tsize
    ELSE
        bytes = (sourcetyp AND 511) \ 8
    END IF
    bytes$ = bytes$ + "-(" + str2(bytes) + "*(" + index$ + "))"
    evaluatetotyp$ = "byte_element((uint64)" + e$ + "," + bytes$ + "," + NewByteElement$ + ")"
    IF targettyp = -5 THEN evaluatetotyp$ = bytes$
    IF targettyp = -6 THEN evaluatetotyp$ = e$
    '           print "CI: array ->["+"byte_element((uint64)" + e$ + "," + bytes$+ ","+NewByteElement$+")"+"]":sleep 1
    EXIT FUNCTION
END IF

'String -> byte_element(offset,bytes)
IF sourcetyp AND ISSTRING THEN
    IF sourcetyp AND ISFIXEDLENGTH THEN
        idnumber = VAL(e$)
        getid idnumber
        IF Error_Happened THEN EXIT FUNCTION
        bytes$ = str2(id.tsize)
        e$ = refer(e$, sourcetyp, 0)
        IF Error_Happened THEN EXIT FUNCTION
    ELSE
        e$ = refer(e$, sourcetyp, 0)
        IF Error_Happened THEN EXIT FUNCTION
        bytes$ = e$ + "->len"
    END IF
    evaluatetotyp$ = "byte_element((uint64)" + e$ + "->chr," + bytes$ + "," + NewByteElement$ + ")"
    IF targettyp = -5 THEN evaluatetotyp$ = bytes$
    IF targettyp = -6 THEN evaluatetotyp$ = e$ + "->chr"
    EXIT FUNCTION
END IF

'Standard variable -> byte_element(offset,bytes)
e$ = refer(e$, sourcetyp, 1) 'get the variable's formal name
IF Error_Happened THEN EXIT FUNCTION
size = (sourcetyp AND 511) \ 8 'calculate its size in bytes
evaluatetotyp$ = "byte_element((uint64)" + e$ + "," + str2(size) + "," + NewByteElement$ + ")"
IF targettyp = -5 THEN evaluatetotyp$ = str2(size)
IF targettyp = -6 THEN evaluatetotyp$ = e$
EXIT FUNCTION

END IF '-2 byte_element(offset,bytes)



'string?
IF (sourcetyp AND ISSTRING) <> (targettyp AND ISSTRING) THEN
    Give_Error "Illegal string-number conversion": EXIT FUNCTION
END IF

IF (sourcetyp AND ISSTRING) THEN
    evaluatetotyp$ = e$
    IF (sourcetyp AND ISREFERENCE) THEN
        evaluatetotyp$ = refer(e$, sourcetyp, 0)
        IF Error_Happened THEN EXIT FUNCTION
    END IF
    EXIT FUNCTION
END IF

'pointer required?
IF (targettyp AND ISPOINTER) THEN
    Give_Error "evaluatetotyp received a request for a pointer (unsupported)": EXIT FUNCTION
    '...
    Give_Error "Invalid pointer": EXIT FUNCTION
END IF

'change to "non-pointer" value
IF (sourcetyp AND ISREFERENCE) THEN
    e$ = refer(e$, sourcetyp, 0)
    IF Error_Happened THEN EXIT FUNCTION
END IF
'check if successful
IF (sourcetyp AND ISPOINTER) THEN
    Give_Error "evaluatetotyp couldn't convert pointer type!": EXIT FUNCTION
END IF

'round to integer if required
IF (sourcetyp AND ISFLOAT) THEN
    IF (targettyp AND ISFLOAT) = 0 THEN
        bits = targettyp AND 511
        '**32 rounding fix
        IF bits <= 16 THEN e$ = "qbr_float_to_long(" + e$ + ")"
        IF bits > 16 AND bits < 32 THEN e$ = "qbr_double_to_long(" + e$ + ")"
        IF bits >= 32 THEN e$ = "qbr(" + e$ + ")"
    END IF
END IF

evaluatetotyp$ = e$
END FUNCTION

FUNCTION seperateargs (a$, ca$, pass&)
    pass& = 0

    FOR i = 1 TO OptMax
        separgs$(i) = ""
    NEXT
    FOR i = 1 TO OptMax + 1
        separgslayout$(i) = ""
    NEXT
    FOR i = 1 TO OptMax
                Lev(i) = 0
                EntryLev(i) = 0
                DitchLev(i) = 0
                DontPass(i) = 0
                TempList(i) = 0
                PassRule(i) = 0
                LevelEntered(i) = 0
    NEXT

    DIM id2 AS idstruct

    id2 = id

    IF id2.args = 0 THEN EXIT FUNCTION 'no arguments!


    s$ = id2.specialformat
    s$ = RTRIM$(s$)

    'build a special format if none exists
            IF s$ = "" THEN
                FOR i = 1 TO id2.args
                    IF i <> 1 THEN s$ = s$ + ",?" ELSE s$ = "?"
                NEXT
            END IF

            'note: dim'd arrays moved to global to prevent high recreation cost

            PassFlag = 1
            nextentrylevel = 0
            nextentrylevelset = 1
            level = 0
            lastt = 0
            ditchlevel = 0
            FOR i = 1 TO LEN(s$)
                s2$ = MID$(s$, i, 1)

                IF s2$ = "[" THEN
                    level = level + 1
                    LevelEntered(level) = 0
                    GOTO nextsymbol
                END IF

                IF s2$ = "]" THEN
                    level = level - 1
                    IF level < ditchlevel THEN ditchlevel = level
                    GOTO nextsymbol
                END IF

                IF s2$ = "{" THEN
                    lastt = lastt + 1: Lev(lastt) = level: PassRule(lastt) = 0
                    DitchLev(lastt) = ditchlevel: ditchlevel = level 'store & reset ditch level
                    i = i + 1
                    i2 = INSTR(i, s$, "}")
                    numopts = 0
                    nextopt:
                    numopts = numopts + 1
                    i3 = INSTR(i + 1, s$, "|")
                    IF i3 <> 0 AND i3 < i2 THEN
                        Opt$(lastt, numopts) = MID$(s$, i, i3 - i)
                        i = i3 + 1: GOTO nextopt
                    END IF
                    Opt$(lastt, numopts) = MID$(s$, i, i2 - i)
                    T(lastt) = numopts
                    'calculate words in each option
                    FOR x = 1 TO T(lastt)
                        w = 1
                        x2 = 1
                        newword:
                        IF INSTR(x2, RTRIM$(Opt$(lastt, x)), " ") THEN w = w + 1: x2 = INSTR(x2, RTRIM$(Opt$(lastt, x)), " ") + 1: GOTO newword
                        OptWords(lastt, x) = w
                    NEXT
                    i = i2

                    'set entry level routine
                    EntryLev(lastt) = level 'default level when continuing a previously entered level
                    IF LevelEntered(level) = 0 THEN
                        EntryLev(lastt) = 0
                        FOR i2 = 1 TO level - 1
                            IF LevelEntered(i2) = 1 THEN EntryLev(lastt) = i2
                        NEXT
                    END IF
                    LevelEntered(level) = 1

                    GOTO nextsymbol
                END IF

                IF s2$ = "?" THEN
                    lastt = lastt + 1: Lev(lastt) = level: PassRule(lastt) = 0
                    DitchLev(lastt) = ditchlevel: ditchlevel = level 'store & reset ditch level
                    T(lastt) = 0
                    'set entry level routine
                    EntryLev(lastt) = level 'default level when continuing a previously entered level
                    IF LevelEntered(level) = 0 THEN
                        EntryLev(lastt) = 0
                        FOR i2 = 1 TO level - 1
                            IF LevelEntered(i2) = 1 THEN EntryLev(lastt) = i2
                        NEXT
                    END IF
                    LevelEntered(level) = 1

                    GOTO nextsymbol
                END IF

                'assume "special" character (like ( ) , . - etc.)
                lastt = lastt + 1: Lev(lastt) = level: PassRule(lastt) = 0
                DitchLev(lastt) = ditchlevel: ditchlevel = level 'store & reset ditch level
            T(lastt) = 1: Opt$(lastt, 1) = s2$: OptWords(lastt, 1) = 1: DontPass(lastt) = 1

                'set entry level routine
                EntryLev(lastt) = level 'default level when continuing a previously entered level
                IF LevelEntered(level) = 0 THEN
                    EntryLev(lastt) = 0
                    FOR i2 = 1 TO level - 1
                        IF LevelEntered(i2) = 1 THEN EntryLev(lastt) = i2
                    NEXT
                END IF
                LevelEntered(level) = 1

                GOTO nextsymbol

                nextsymbol:
            NEXT


            IF Debug THEN
                PRINT #9, "--------SEPERATE ARGUMENTS REPORT #1:1--------"
                FOR i = 1 TO lastt
                        PRINT #9, i, "OPT=" + CHR$(34) + RTRIM$(Opt$(i, 1)) + CHR$(34)
                    PRINT #9, i, "OPTWORDS="; OptWords(i, 1)
                    PRINT #9, i, "T="; T(i)
                    PRINT #9, i, "DONTPASS="; DontPass(i)
                    PRINT #9, i, "PASSRULE="; PassRule(i)
                    PRINT #9, i, "LEV="; Lev(i)
                    PRINT #9, i, "ENTRYLEV="; EntryLev(i)
                NEXT
            END IF


            'Any symbols already have dontpass() set to 1
            'This sets any {}blocks with only one option/word (eg. {PRINT}) at the lowest level to dontpass()=1
            'because their content is manadatory and there is no choice as to which word to use
            FOR x = 1 TO lastt
                IF Lev(x) = 0 THEN
                    IF T(x) = 1 THEN DontPass(x) = 1
                END IF
            NEXT

            IF Debug THEN
                PRINT #9, "--------SEPERATE ARGUMENTS REPORT #1:2--------"
                FOR i = 1 TO lastt
                        PRINT #9, i, "OPT=" + CHR$(34) + RTRIM$(Opt$(i, 1)) + CHR$(34)
                    PRINT #9, i, "OPTWORDS="; OptWords(i, 1)
                    PRINT #9, i, "T="; T(i)
                    PRINT #9, i, "DONTPASS="; DontPass(i)
                    PRINT #9, i, "PASSRULE="; PassRule(i)
                    PRINT #9, i, "LEV="; Lev(i)
                    PRINT #9, i, "ENTRYLEV="; EntryLev(i)
                NEXT
            END IF




            x1 = 0 'the 'x' position of the beginning element of the current levelled block
            MustPassOpt = 0 'the 'x' position of the FIRST opt () in the block which must be passed
            MustPassOptNeedsFlag = 0 '{}blocks don't need a flag, ? blocks do

            'Note: For something like [{HELLO}x] a choice between passing 'hello' or passing a flag to signify x was specified
            '      has to be made, in such cases, a flag is preferable to wasting a full new int32 on 'hello'

            templistn = 0
            FOR l = 1 TO 32767
                scannextlevel = 0
                FOR x = 1 TO lastt
                    IF Lev(x) > l THEN scannextlevel = 1

                    IF x1 THEN
                        IF EntryLev(x) < l THEN 'end of block reached
                        IF MustPassOpt THEN
                            'If there's an opt () which must be passed that will be identified,
                            'all the 1 option {}blocks can be assumed...
                            IF MustPassOptNeedsFlag THEN
                                'The MustPassOpt requires a flag, so use the same flag for everything
                                FOR x2 = 1 TO templistn
                                    PassRule(TempList(x2)) = PassFlag
                                NEXT
                                PassFlag = PassFlag * 2
                            ELSE
                                'The MustPassOpt is a {}block which doesn't need a flag, so everything else needs to
                                'reference it
                                FOR x2 = 1 TO templistn
                                    IF TempList(x2) <> MustPassOpt THEN PassRule(TempList(x2)) = -MustPassOpt
                                NEXT
                            END IF
                        ELSE
                            'if not, use a unique flag for everything in this block
                            FOR x2 = 1 TO templistn: PassRule(TempList(x2)) = PassFlag: NEXT
                                IF templistn <> 0 THEN PassFlag = PassFlag * 2
                            END IF
                            x1 = 0
                        END IF
                    END IF


                    IF Lev(x) = l THEN 'on same level
                    IF EntryLev(x) < l THEN 'just (re)entered this level (not continuing along it)
                    x1 = x 'set x1 to the starting element of this level
                    MustPassOpt = 0
                    templistn = 0
                END IF
            END IF

            IF x1 THEN
                IF Lev(x) = l THEN 'same level

                IF T(x) <> 1 THEN
                    'It isn't a symbol or a {}block with only one option therefore this opt () must be passed
                    IF MustPassOpt = 0 THEN
                        MustPassOpt = x 'Only record the first instance (it MAY require a flag)
                        IF T(x) = 0 THEN MustPassOptNeedsFlag = 1 ELSE MustPassOptNeedsFlag = 0
                    ELSE
                        'Update current MustPassOpt to non-flag-based {}block if possible (to save flag usage)
                        '(Consider [{A|B}?], where a flag is not required)
                        IF MustPassOptNeedsFlag = 1 THEN
                            IF T(x) > 1 THEN
                                MustPassOpt = x: MustPassOptNeedsFlag = 0
                            END IF
                        END IF
                    END IF
                    'add to list
                    templistn = templistn + 1: TempList(templistn) = x
                END IF

                IF T(x) = 1 THEN
                    'It is a symbol or a {}block with only one option
                    'a {}block with only one option MAY not need to be passed
                    'depending on if anything else is in this block could make the existance of this opt () assumed
                    'Note: Symbols which are not encapsulated inside a {}block never need to be passed
                    '      Symbols already have dontpass() set to 1
                    IF DontPass(x) = 0 THEN templistn = templistn + 1: TempList(templistn) = x: DontPass(x) = 1
                END IF

            END IF
        END IF

    NEXT

    'scan last run (mostly just a copy of code from above)
    IF x1 THEN
        IF MustPassOpt THEN
            'If there's an opt () which must be passed that will be identified,
            'all the 1 option {}blocks can be assumed...
            IF MustPassOptNeedsFlag THEN
                'The MustPassOpt requires a flag, so use the same flag for everything
                FOR x2 = 1 TO templistn
                    PassRule(TempList(x2)) = PassFlag
                NEXT
                PassFlag = PassFlag * 2
            ELSE
                'The MustPassOpt is a {}block which doesn't need a flag, so everything else needs to
                'reference it
                FOR x2 = 1 TO templistn
                    IF TempList(x2) <> MustPassOpt THEN PassRule(TempList(x2)) = -MustPassOpt
                NEXT
            END IF
        ELSE
            'if not, use a unique flag for everything in this block
            FOR x2 = 1 TO templistn: PassRule(TempList(x2)) = PassFlag: NEXT
                IF templistn <> 0 THEN PassFlag = PassFlag * 2
            END IF
            x1 = 0
        END IF

        IF scannextlevel = 0 THEN EXIT FOR
    NEXT

    IF Debug THEN
        PRINT #9, "--------SEPERATE ARGUMENTS REPORT #1:3--------"
        FOR i = 1 TO lastt
                PRINT #9, i, "OPT=" + CHR$(34) + RTRIM$(Opt$(i, 1)) + CHR$(34)
            PRINT #9, i, "OPTWORDS="; OptWords(i, 1)
            PRINT #9, i, "T="; T(i)
            PRINT #9, i, "DONTPASS="; DontPass(i)
            PRINT #9, i, "PASSRULE="; PassRule(i)
            PRINT #9, i, "LEV="; Lev(i)
            PRINT #9, i, "ENTRYLEV="; EntryLev(i)
        NEXT
    END IF



FOR i = 1 TO lastt
    separgs$(i) = "n-ll"
NEXT




        'Consider: "?,[?]"
        'Notes: The comma is mandatory but the second ? is entirely optional
        'Consider: "[?[{B}?]{A}]?"
        'Notes: As unlikely as the above is, it is still valid, but pivots on the outcome of {A} being present
        'Consider: "[?]{A}"
        'Consider: "[?{A}][?{B}][?{C}]?"
        'Notes: The trick here is to realize {A} has greater priority than {B}, so all lines of enquiry must
        '       be exhausted before considering {B}

        'Use inquiry approach to solve format
        'Each line of inquiry must be exhausted
        'An expression ("?") simply means a branch where you can scan ahead

        Branches = 0
        DIM BranchFormatPos(1 TO 100) AS LONG
        DIM BranchTaken(1 TO 100) AS LONG
        '1=taken (this usually involves moving up a level)
        '0=not taken
        DIM BranchInputPos(1 TO 100) AS LONG
        DIM BranchWithExpression(1 TO 100) AS LONG
        'non-zero=expression expected before next item for format item value represents
        '0=no expression allowed before next item
        DIM BranchLevel(1 TO 100) AS LONG 'Level before this branch was/wasn't taken

        n = numelements(ca$)
        i = 1 'Position within ca$

        level = 0
        Expression = 0
        FOR x = 1 TO lastt

            ContinueScan:

            IF DitchLev(x) < level THEN 'dropping down to a lower level
            'we can only go as low as the 'ditch' will allow us, which will limit our options
            level = DitchLev(x)
        END IF

        IF EntryLev(x) <= level THEN 'possible to enter level

        'But was this optional or were we forced to be on this level?
        IF EntryLev(x) < Lev(x) THEN
            optional = 1
            IF level > EntryLev(x) THEN optional = 0
        ELSE
            'entrylev=lev
            optional = 0
        END IF

        t = T(x)

        IF t = 0 THEN 'A "?" expression
        IF Expression THEN
            '*********backtrack************
            'We are tracking an expression which we assumed would be present but was not
            GOTO Backtrack
            '******************************
        END IF
        IF optional THEN
            Branches = Branches + 1
            BranchFormatPos(Branches) = x
            BranchTaken(Branches) = 1
            BranchInputPos(Branches) = i
            BranchWithExpression(Branches) = 0
            BranchLevel(Branches) = level
            level = Lev(x)
        END IF
        Expression = x
    END IF 'A "?" expression

    IF t THEN

        currentlev = level

        'Add new branch if new level will be entered
        IF optional THEN
            Branches = Branches + 1
            BranchFormatPos(Branches) = x
            BranchTaken(Branches) = 1
            BranchInputPos(Branches) = i
            BranchWithExpression(Branches) = Expression
            BranchLevel(Branches) = level
        END IF

        'Scan for Opt () options
        i1 = i: i2 = i
        IF Expression THEN i2 = n
        'Scan a$ for opt () x
        'Note: Finding the closest opt option is necessary
        'Note: This needs to be bracket sensitive
        OutOfRange = 2147483647
        position = OutOfRange
        which = 0
        removePrefix = 0
        IF i <= n THEN 'Past end of contect check
        FOR o = 1 TO t
            words = OptWords(x, o)
            b = 0
            FOR i3 = i1 TO i2
                IF i3 + words - 1 <= n THEN 'enough elements exist
                c$ = getelement$(a$, i3)
                IF b = 0 THEN
                    'Build comparison string (spacing elements)
                    FOR w = 2 TO words
                        c$ = c$ + " " + getelement$(a$, i3 + w - 1)
                    NEXT w
                    'Compare
                    noPrefixMatch = LEFT$(Opt$(x, o), 1) = "_" AND qbnexprefix_set = 1 AND c$ = UCASE$(MID$(RTRIM$(Opt$(x, o)), 2))
                    IF c$ = UCASE$(RTRIM$(Opt$(x, o))) OR noPrefixMatch THEN
                        'Record Match
                        IF i3 < position THEN
                            position = i3
                            which = o
                            IF noPrefixMatch THEN removePrefix = 1
                            bvalue = b
                            EXIT FOR 'Exit the i3 loop
                        END IF 'position check
                    END IF 'match
                END IF

                IF ASC(c$) = 44 AND b = 0 THEN
                    EXIT FOR 'Expressions cannot contain a "," in their base level
                    'Because this wasn't interceppted by the above code it isn't the Opt either
                END IF
                IF ASC(c$) = 40 THEN
                    b = b + 1
                END IF
                IF ASC(c$) = 41 THEN
                    b = b - 1
                    IF b = -1 THEN EXIT FOR 'Exited current bracketting level, making any following match invalid
                END IF

            END IF 'enough elements exist
        NEXT i3
    NEXT o
END IF 'Past end of contect check

IF position <> OutOfRange THEN 'Found?
'Found...
level = Lev(x) 'Adjust level
IF Expression THEN
    'Found...Expression...
    'Has an expression been provided?
    IF position > i AND bvalue = 0 THEN
        'Found...Expression...Provided...
separgs$(Expression) = getelements$(ca$, i, position - 1)
        Expression = 0
        i = position
    ELSE
        'Found...Expression...Omitted...
        '*********backtrack************
        GOTO OptCheckBacktrack
        '******************************
    END IF
END IF 'Expression
i = i + OptWords(x, which)
                        separgslayout$(x) = CHR$(LEN(RTRIM$(Opt$(x, which))) - removePrefix) + SCase$(MID$(RTRIM$(Opt$(x, which)), removePrefix + 1))
separgs$(x) = CHR$(0) + str2(which)
ELSE
    'Not Found...
    '*********backtrack************
    OptCheckBacktrack:
    'Was this optional?
    IF Lev(x) > EntryLev(x) THEN 'Optional Opt ()?
    'Not Found...Optional...
    'Simply don't enter the optional higher level and continue as normal
    BranchTaken(Branches) = 0
    level = currentlev 'We aren't entering the level after all, so our level should remain at the opt's entrylevel
ELSE
    Backtrack:
    'Not Found...Mandatory...
    '1)Erase previous branches where both options have been tried
    FOR branch = Branches TO 1 STEP -1 'Remove branches until last taken branch is found
        IF BranchTaken(branch) THEN EXIT FOR
        Branches = Branches - 1 'Remove branch (it has already been tried with both possible combinations)
    NEXT
    IF Branches = 0 THEN 'All options have been exhausted
    seperateargs_error = 1
    seperateargs_error_message = "Syntax error"
    IF LEN(id2.hr_syntax) > 0 THEN seperateargs_error_message = seperateargs_error_message + " - Reference: " + id2.hr_syntax
    EXIT FUNCTION
END IF
'2)Toggle taken branch to untaken and revert
BranchTaken(Branches) = 0 'toggle branch to untaken
Expression = BranchWithExpression(Branches)
i = BranchInputPos(Branches)
x = BranchFormatPos(Branches)
level = BranchLevel(Branches)
'3)Erase any content created after revert position
IF Expression THEN separgs$(Expression) = "n-ll"
FOR x2 = x TO lastt
separgs$(x2) = "n-ll"
separgslayout$(x2) = ""
NEXT
END IF 'Optional Opt ()?
'******************************

END IF 'Found?

END IF 't

END IF 'possible to enter level

NEXT x

'Final expression?
IF Expression THEN
    IF i <= n THEN
separgs$(Expression) = getelements$(ca$, i, n)

        'can this be an expression?
        'check it passes bracketting and comma rules
        b = 0
        FOR i2 = i TO n
            c$ = getelement$(a$, i2)
            IF ASC(c$) = 44 AND b = 0 THEN
                GOTO Backtrack
            END IF
            IF ASC(c$) = 40 THEN
                b = b + 1
            END IF
            IF ASC(c$) = 41 THEN
                b = b - 1
                IF b = -1 THEN GOTO Backtrack
            END IF
        NEXT
        IF b <> 0 THEN GOTO Backtrack

        i = n + 1 'So it passes the test below
    ELSE
        GOTO Backtrack
    END IF
END IF 'Expression

IF i <> n + 1 THEN GOTO Backtrack 'Trailing content?

IF Debug THEN
    PRINT #9, "--------SEPERATE ARGUMENTS REPORT #2--------"
    FOR i = 1 TO lastt
PRINT #9, i, separgs$(i)
    NEXT
END IF

'   DIM PassRule(1 TO 100) AS LONG
'   '0 means no pass rule
'   'negative values refer to an opt () element
'   'positive values refer to a flag value
'   PassFlag = 1


IF PassFlag <> 1 THEN seperateargs = 1 'Return whether a 'passed' flags variable is required
pass& = 0 'The 'passed' value (shared by argument reference)

'Note: The separgs() elements will be compacted to the C++ function arguments
x = 1 'The new index to move compacted content to within separgs()

FOR i = 1 TO lastt

    IF DontPass(i) = 0 THEN

        IF PassRule(i) > 0 THEN
IF separgs$(i) <> "n-ll" THEN pass& = pass& OR PassRule(i) 'build 'passed' flags
        END IF

separgs$(x) = separgs$(i)
separgslayout$(x) = separgslayout$(i)

IF LEN(separgs$(x)) THEN
IF ASC(separgs$(x)) = 0 THEN
                'switch omit layout tag from item to layout info
separgs$(x) = RIGHT$(separgs$(x), LEN(separgs$(x)) - 1)
separgslayout$(x) = separgslayout$(x) + CHR$(0)
            END IF
        END IF

IF separgs$(x) = "n-ll" THEN separgs$(x) = "N-LL"
        x = x + 1

    ELSE
        'its gonna be skipped!
        'add layout to the next one to be safe

        'for syntax such as [{HELLO}] which uses a flag instead of being passed
        IF PassRule(i) > 0 THEN
IF separgs$(i) <> "n-ll" THEN pass& = pass& OR PassRule(i) 'build 'passed' flags
        END IF

separgslayout$(i + 1) = separgslayout$(i) + separgslayout$(i + 1)

    END IF
NEXT
separgslayout$(x) = separgslayout$(i) 'set final layout

'x = x - 1
'PRINT "total arguments:"; x
'PRINT "pass omit (0/1):"; omit
'PRINT "pass&="; pass&

END FUNCTION
