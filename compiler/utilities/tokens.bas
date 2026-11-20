FUNCTION getelementspecial$ (savea$, elenum)
    'two cached lines in scalars, as getelement$
    STATIC gsSrc1 AS STRING, gsEl1 AS LONG, gsSt1 AS LONG, gsNx1 AS LONG
    STATIC gsSrc2 AS STRING, gsEl2 AS LONG, gsSt2 AS LONG, gsNx2 AS LONG
    STATIC gsVictim AS LONG
    DIM slot AS LONG, ce AS LONG, cs AS LONG, cn AS LONG

    a$ = savea$
    IF a$ = "" THEN EXIT FUNCTION 'no elements!

    IF a$ = gsSrc1 THEN
        slot = 1: ce = gsEl1: cs = gsSt1: cn = gsNx1
    ELSEIF a$ = gsSrc2 THEN
        slot = 2: ce = gsEl2: cs = gsSt2: cn = gsNx2
    ELSE
        IF gsVictim = 1 THEN gsVictim = 2 ELSE gsVictim = 1
        slot = gsVictim
        IF slot = 1 THEN
            gsSrc1 = a$: gsEl1 = 0: gsSt1 = 0: gsNx1 = 0
        ELSE
            gsSrc2 = a$: gsEl2 = 0: gsSt2 = 0: gsNx2 = 0
        END IF
    END IF

    IF elenum = ce AND cs > 0 THEN
        p = cs
        i = cn
        GOTO getelementspecialreturn
    END IF

    IF elenum > ce AND cn <> 0 THEN
        n = ce + 1
        p = cn + 1
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
        'the slot already holds a$ (matched or set above); copying the whole
        'line again on every call made long lines quadratic
        IF slot = 1 THEN
            gsEl1 = elenum: gsSt1 = p: gsNx1 = i
        ELSE
            gsEl2 = elenum: gsSt2 = p: gsNx2 = i
        END IF
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
    'element elenum of a$ ("" when there is none), from the token index (state.bas)
    DIM b AS LONG, p AS LONG
    IF a$ = tkSrc1 THEN
        b = 0
    ELSEIF a$ = tkSrc2 THEN
        b = tkStride
    ELSEIF a$ = tkSrc3 THEN
        b = 2 * tkStride
    ELSEIF a$ = tkSrc4 THEN
        b = 3 * tkStride
    ELSE
        b = tkIndex&(a$)
    END IF
    IF elenum < 1 THEN EXIT FUNCTION
    IF elenum > tkPos(b) THEN EXIT FUNCTION
    p = tkPos(b + elenum)
    getelement$ = MID$(a$, p, tkPos(b + elenum + 1) - p - 1)
END FUNCTION

FUNCTION tkIndex& (a$)
    'indexes the elements of a$ in the next slot of the token index; returns the
    'slot's base in tkPos (tkPos(base) is the number of elements)
    DIM b AS LONG, n AS LONG, p AS LONG, i AS LONG
    tkNext = (tkNext + 1) AND 3
    SELECT CASE tkNext
        CASE 0: tkSrc1 = a$
        CASE 1: tkSrc2 = a$
        CASE 2: tkSrc3 = a$
        CASE ELSE: tkSrc4 = a$
    END SELECT
    tkindexagain:
    b = tkNext * tkStride
    IF LEN(a$) = 0 THEN tkPos(b) = 0: tkIndex& = b: EXIT FUNCTION
    n = 0: p = 1
    DO
        n = n + 1
        IF n + 1 >= tkStride THEN tkGrow: GOTO tkindexagain
        tkPos(b + n) = p
        i = INSTR(p, a$, sp)
        IF i = 0 THEN EXIT DO
        p = i + 1
    LOOP
    tkPos(b + n + 1) = LEN(a$) + 2
    tkPos(b) = n
    tkIndex& = b
END FUNCTION

SUB tkGrow
    'a line with more elements than a slot holds: larger slots; the lines of the other
    'slots are forgotten (an empty slot has no elements, as the empty line)
    tkStride = tkStride * 2
    REDIM tkPos(0 TO 4 * tkStride - 1) AS LONG
    IF tkNext <> 0 THEN tkSrc1 = ""
    IF tkNext <> 1 THEN tkSrc2 = ""
    IF tkNext <> 2 THEN tkSrc3 = ""
    IF tkNext <> 3 THEN tkSrc4 = ""
END SUB

FUNCTION getelements$ (a$, i1, i2)
    DIM b AS LONG, p AS LONG
    IF i2 < i1 THEN getelements$ = "": EXIT FUNCTION
    IF i1 >= 1 THEN
        IF a$ = tkSrc1 THEN
            b = 0
        ELSEIF a$ = tkSrc2 THEN
            b = tkStride
        ELSEIF a$ = tkSrc3 THEN
            b = 2 * tkStride
        ELSEIF a$ = tkSrc4 THEN
            b = 3 * tkStride
        ELSE
            b = tkIndex&(a$)
        END IF
        IF i2 <= tkPos(b) THEN
            p = tkPos(b + i1)
            getelements$ = MID$(a$, p, tkPos(b + i2 + 1) - p - 1)
            EXIT FUNCTION
        END IF
    END IF
    'not all of i1 to i2 are elements of a$: the result of the former scan
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

FUNCTION joinelements$ (a$, i1, i2)
    'elements i1 to i2 of a$ joined with sp as appending them one by one does
    '(IF r$ = "" THEN r$ = e$ ELSE r$ = r$ + sp + e$: empty elements at the start are
    'dropped): the text from element i1 to element i2 when none of them is empty
    DIM r AS STRING, k AS LONG, n AS LONG
    IF i2 < i1 THEN EXIT FUNCTION
    n = numelements(a$)
    IF i1 >= 1 AND i2 <= n THEN
        r = getelements$(a$, i1, i2)
        IF INSTR(r, sp + sp) = 0 THEN
            IF LEN(r) THEN
                IF LEFT$(r, 1) <> sp AND RIGHT$(r, 1) <> sp THEN joinelements$ = r: EXIT FUNCTION
            END IF
        END IF
        r = ""
    END IF
    FOR k = i1 TO i2
        IF r = "" THEN r = getelement$(a$, k) ELSE r = r + sp + getelement$(a$, k)
    NEXT
    joinelements$ = r
END FUNCTION

FUNCTION elementpos (a$, k, setpos)
    'Position of the first character of element k in a$ (0 if there is no element k),
    'from the token index. The SUBs that splice a line pass the position they know as
    'setpos: it is the answer (the index of the new line is made when it is read).
    DIM b AS LONG
    IF setpos > 0 THEN elementpos = setpos: EXIT FUNCTION
    IF k < 1 OR a$ = "" THEN EXIT FUNCTION
    IF a$ = tkSrc1 THEN
        b = 0
    ELSEIF a$ = tkSrc2 THEN
        b = tkStride
    ELSEIF a$ = tkSrc3 THEN
        b = 2 * tkStride
    ELSEIF a$ = tkSrc4 THEN
        b = 3 * tkStride
    ELSE
        b = tkIndex&(a$)
    END IF
    IF k <= tkPos(b) THEN elementpos = tkPos(b + k)
END FUNCTION

FUNCTION numelements (a$)
    'the number of elements of a$, from the token index
    IF a$ = tkSrc1 THEN
        numelements = tkPos(0)
    ELSEIF a$ = tkSrc2 THEN
        numelements = tkPos(tkStride)
    ELSEIF a$ = tkSrc3 THEN
        numelements = tkPos(2 * tkStride)
    ELSEIF a$ = tkSrc4 THEN
        numelements = tkPos(3 * tkStride)
    ELSE
        numelements = tkPos(tkIndex&(a$))
    END IF
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
