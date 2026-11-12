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
    'Look for the quote inside this element only (a search to the end of
    'the line on every call was quadratic for long lines without quotes)
    i2 = 0
    IF i THEN i2 = INSTR(MID$(a$, p, i - p), CHR$(34)): IF i2 THEN i2 = i2 + p - 1
    IF i2 < i AND i2 <> 0 THEN
        i3 = INSTR(i2 + 1, a$, CHR$(34)): IF i3 = 0 THEN Give_Error "Expected " + CHR$(34): EXIT FUNCTION
        i = INSTR(i3, a$, sp)
    END IF

    IF elenum = n THEN
        'cacheSource(slot) already holds a$ (matched or set above); copying
        'the whole line again on every call made long lines quadratic
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
        'Jump to the element with the shared position cache (seeded by the
        'SUBs that edit lines) instead of scanning from the start
        p = elementpos(a$, elenum, 0)
        IF p = 0 THEN EXIT FUNCTION 'no such element
        n = elenum
    END IF

    getelementnext:
    i = INSTR(p, a$, sp)

    IF elenum = n THEN
        'cacheSource(slot) already holds a$ (matched or set above); copying
        'the whole line again on every call made long lines quadratic
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

    'Find element i (position cache), then a single splice. Rebuilding
    'the line element by element made every call O(line length * elements),
    'and long expressions call this once per number (cubic compile time).
    IF a$ = "" THEN EXIT SUB
    'look up element i + 1: removeelements leaves exactly that one cached
    'when a caller replaces element i + 1 (remove, then insert after i)
    p = elementpos(a$, i + 1, 0)
    IF p THEN
        a$ = LEFT$(a$, p - 1) + elements$ + sp + MID$(a$, p)
        p = elementpos(a$, i + 1, p) 'the first inserted element starts here
    ELSE
        IF elementpos(a$, i, 0) = 0 THEN EXIT SUB 'i past the last element: nothing inserted (as before)
        a$ = a$ + sp + elements$
    END IF
END SUB

FUNCTION elementpos (a$, k, setpos)
    'Position of the first character of element k in a$ (0 if there is
    'no element k). The last answer is remembered, and the SUBs that splice a
    'line store the position they know, so walking a long line element by
    'element while editing it does not rescan it from the start each time.
    STATIC cacheText AS STRING, cacheK AS LONG, cachePos AS LONG
    IF setpos > 0 THEN
        cacheText = a$: cacheK = k: cachePos = setpos
        elementpos = setpos
        EXIT FUNCTION
    END IF
    IF k < 1 OR a$ = "" THEN EXIT FUNCTION

    n = 1: p = 1: cachehit = 0
    IF cacheK > 0 THEN
        IF LEN(cacheText) = LEN(a$) THEN
            IF cacheText = a$ THEN cachehit = -1
        END IF
    END IF
    IF cachehit THEN
        IF k >= cacheK THEN n = cacheK: p = cachePos
    END IF
    DO WHILE n < k
        i = INSTR(p, a$, sp)
        IF i = 0 THEN EXIT FUNCTION 'no element k
        p = i + 1
        n = n + 1
    LOOP
    IF cachehit = 0 THEN cacheText = a$
    cacheK = k: cachePos = p
    elementpos = p
END FUNCTION

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
    'note: first and last MUST be valid
    '      keepindexing means the number of elements will stay the same
    '       but some elements will be equal to ""
    'Cut at the element boundaries (position cache, see insertelements)

    firstpos = elementpos(a$, first, 0) 'first character of element first
    IF firstpos = 0 THEN EXIT SUB 'nothing to remove
    lastn = last
    lastpos = elementpos(a$, lastn, 0)
    IF lastpos = 0 THEN lastn = numelements(a$): lastpos = elementpos(a$, lastn, 0)
    lastend = INSTR(lastpos, a$, sp) 'separator after element last
    IF lastend = 0 THEN lastend = LEN(a$) + 1

    IF keepindexing THEN
        a$ = LEFT$(a$, firstpos - 1) + STRING$(lastn - first, sp) + MID$(a$, lastend)
        p = elementpos(a$, first, firstpos)
        EXIT SUB
    END IF

    IF lastend > LEN(a$) THEN 'up to the last element
        IF first > 1 THEN a$ = LEFT$(a$, firstpos - 2) ELSE a$ = ""
    ELSE
        IF first > 1 THEN
            a$ = LEFT$(a$, firstpos - 2) + MID$(a$, lastend)
        ELSE
            a$ = MID$(a$, lastend + 1)
        END IF
        p = elementpos(a$, first, firstpos) 'the next element now starts here
    END IF
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
