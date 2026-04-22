FUNCTION getelementspecial$ (savea$, elenum)
    STATIC cacheSource(1 TO 2) AS STRING
    STATIC cacheElement(1 TO 2) AS LONG
    STATIC cacheStart(1 TO 2) AS LONG
    STATIC cacheNext(1 TO 2) AS LONG
    STATIC cacheVictim AS INTEGER

    a$ = savea$
    IF a$ = "" THEN EXIT FUNCTION 'no elements!

    slot = 0
    FOR s = 1 TO 2
        IF cacheSource(s) = a$ THEN slot = s: EXIT FOR
    NEXT
    IF slot = 0 THEN
        cacheVictim = cacheVictim + 1
        IF cacheVictim > 2 THEN cacheVictim = 1
        slot = cacheVictim
        cacheSource(slot) = a$
        cacheElement(slot) = 0
        cacheStart(slot) = 0
        cacheNext(slot) = 0
    END IF

    IF elenum = cacheElement(slot) AND cacheStart(slot) > 0 THEN
        p = cacheStart(slot)
        i = cacheNext(slot)
        GOTO getelementspecialreturn
    END IF

    IF elenum > cacheElement(slot) AND cacheNext(slot) <> 0 THEN
        n = cacheElement(slot) + 1
        p = cacheNext(slot) + 1
    ELSE
        n = 1
        p = 1
    END IF

    getelementspecialnext:
    i = INSTR(p, a$, sp)

    'avoid sp inside "..."
    i2 = INSTR(p, a$, CHR$(34))
    IF i2 < i AND i2 <> 0 THEN
        i3 = INSTR(i2 + 1, a$, CHR$(34)): IF i3 = 0 THEN Give_Error "Expected " + CHR$(34): EXIT FUNCTION
        i = INSTR(i3, a$, sp)
    END IF

    IF elenum = n THEN
        cacheSource(slot) = a$
        cacheElement(slot) = elenum
        cacheStart(slot) = p
        cacheNext(slot) = i
        getelementspecialreturn:
        IF i THEN getelementspecial$ = MID$(a$, p, i - p) ELSE getelementspecial$ = RIGHT$(a$, LEN(a$) - p + 1)
        EXIT FUNCTION
    END IF

    IF i = 0 THEN EXIT FUNCTION 'no more elements!
    n = n + 1
    p = i + 1
    GOTO getelementspecialnext
END FUNCTION

FUNCTION getelement$ (a$, elenum)
    STATIC cacheSource(1 TO 2) AS STRING
    STATIC cacheElement(1 TO 2) AS LONG
    STATIC cacheStart(1 TO 2) AS LONG
    STATIC cacheNext(1 TO 2) AS LONG
    STATIC cacheVictim AS INTEGER

    IF a$ = "" THEN EXIT FUNCTION 'no elements!

    slot = 0
    FOR s = 1 TO 2
        IF cacheSource(s) = a$ THEN slot = s: EXIT FOR
    NEXT
    IF slot = 0 THEN
        cacheVictim = cacheVictim + 1
        IF cacheVictim > 2 THEN cacheVictim = 1
        slot = cacheVictim
        cacheSource(slot) = a$
        cacheElement(slot) = 0
        cacheStart(slot) = 0
        cacheNext(slot) = 0
    END IF

    IF elenum = cacheElement(slot) AND cacheStart(slot) > 0 THEN
        p = cacheStart(slot)
        i = cacheNext(slot)
        GOTO getelementreturn
    END IF

    IF elenum > cacheElement(slot) AND cacheNext(slot) <> 0 THEN
        n = cacheElement(slot) + 1
        p = cacheNext(slot) + 1
    ELSE
        n = 1
        p = 1
    END IF

    getelementnext:
    i = INSTR(p, a$, sp)

    IF elenum = n THEN
        cacheSource(slot) = a$
        cacheElement(slot) = elenum
        cacheStart(slot) = p
        cacheNext(slot) = i
        getelementreturn:
        IF i THEN getelement$ = MID$(a$, p, i - p) ELSE getelement$ = RIGHT$(a$, LEN(a$) - p + 1)
        EXIT FUNCTION
    END IF

    IF i = 0 THEN EXIT FUNCTION 'no more elements!
    n = n + 1
    p = i + 1
    GOTO getelementnext
END FUNCTION

FUNCTION getelements$ (a$, i1, i2)
    IF i2 < i1 THEN getelements$ = "": EXIT FUNCTION
    n = 1
    p = 1
    getelementsnext:
    i = INSTR(p, a$, sp)
    IF n = i1 THEN
        i1pos = p
    END IF
    IF n = i2 THEN
        IF i THEN
            getelements$ = MID$(a$, i1pos, i - i1pos)
        ELSE
            getelements$ = RIGHT$(a$, LEN(a$) - i1pos + 1)
        END IF
        EXIT FUNCTION
    END IF
    n = n + 1
    p = i + 1
    GOTO getelementsnext
END FUNCTION

SUB insertelements (a$, i, elements$)
    IF i = 0 THEN
        IF a$ = "" THEN
            a$ = elements$
            EXIT SUB
        END IF
        a$ = elements$ + sp + a$
        EXIT SUB
    END IF

    a2$ = ""
    n = numelements(a$)

    FOR i2 = 1 TO n
        IF i2 > 1 THEN a2$ = a2$ + sp
        a2$ = a2$ + getelement$(a$, i2)
        IF i = i2 THEN a2$ = a2$ + sp + elements$
    NEXT

    a$ = a2$
END SUB

FUNCTION numelements (a$)
    STATIC cacheSource(1 TO 2) AS STRING
    STATIC cacheCount(1 TO 2) AS LONG
    STATIC cacheVictim AS INTEGER

    IF a$ = "" THEN EXIT FUNCTION
    FOR s = 1 TO 2
        IF cacheSource(s) = a$ THEN numelements = cacheCount(s): EXIT FUNCTION
    NEXT

    n = 1
    p = 1
    numelementsnext:
    i = INSTR(p, a$, sp)
    IF i = 0 THEN
        cacheVictim = cacheVictim + 1
        IF cacheVictim > 2 THEN cacheVictim = 1
        cacheSource(cacheVictim) = a$
        cacheCount(cacheVictim) = n
        numelements = n
        EXIT FUNCTION
    END IF
    n = n + 1
    p = i + 1
    GOTO numelementsnext
END FUNCTION

SUB removeelements (a$, first, last, keepindexing)
    a2$ = ""
    'note: first and last MUST be valid
    '      keepindexing means the number of elements will stay the same
    '       but some elements will be equal to ""

    n = numelements(a$)
    FOR i = 1 TO n
        IF i < first OR i > last THEN
            a2$ = a2$ + sp + getelement(a$, i)
        ELSE
            IF keepindexing THEN a2$ = a2$ + sp
        END IF
    NEXT
    IF LEFT$(a2$, 1) = sp THEN a2$ = RIGHT$(a2$, LEN(a2$) - 1)

    a$ = a2$
END SUB

FUNCTION eleucase$ (a$)
    'this function upper-cases all elements except for quoted strings
    'check first element
    IF LEN(a$) = 0 THEN EXIT FUNCTION
    i = 1
    IF ASC(a$) = 34 THEN
        i2 = INSTR(a$, sp)
        IF i2 = 0 THEN eleucase$ = a$: EXIT FUNCTION
        a2$ = LEFT$(a$, i2 - 1)
        i = i2
    END IF
    'check other elements
    sp34$ = sp + CHR$(34)
    IF i < LEN(a$) THEN
        DO WHILE INSTR(i, a$, sp34$)
            i2 = INSTR(i, a$, sp34$)
            a2$ = a2$ + UCASE$(MID$(a$, i, i2 - i + 1)) 'everything prior including spacer
            i3 = INSTR(i2 + 1, a$, sp): IF i3 = 0 THEN i3 = LEN(a$) ELSE i3 = i3 - 1
            a2$ = a2$ + MID$(a$, i2 + 1, i3 - (i2 + 1) + 1) 'everything from " to before next spacer or end
            i = i3 + 1
            IF i > LEN(a$) THEN EXIT DO
        LOOP
    END IF
    a2$ = a2$ + UCASE$(MID$(a$, i, LEN(a$) - i + 1))
    eleucase$ = a2$
END FUNCTION

FUNCTION removecast$ (a$)
    removecast$ = a$
    IF INSTR(a$, "  )") THEN
        removecast$ = RIGHT$(a$, LEN(a$) - INSTR(a$, "  )") - 2)
    END IF
END FUNCTION

FUNCTION converttabs$ (a2$)
    s = 4
    a$ = a2$
    DO WHILE INSTR(a$, CHR_TAB)
        x = INSTR(a$, CHR_TAB)
        a$ = LEFT$(a$, x - 1) + SPACE$(s - ((x - 1) MOD s)) + RIGHT$(a$, LEN(a$) - x)
    LOOP
    converttabs$ = a$
END FUNCTION
