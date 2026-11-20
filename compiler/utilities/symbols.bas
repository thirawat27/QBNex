FUNCTION findid& (n2$)
    n$ = UCASE$(n2$)

    IF ASC(n$) = 34 THEN GOTO noid

    secondarg$ = findidsecondarg: findidsecondarg = ""

    findanother = findanotherid: findanotherid = 0
    IF findanother <> 0 AND findidinternal <> 2 THEN Give_Error "FINDID() ERROR: Invalid repeat search requested!": EXIT FUNCTION
    IF Error_Happened THEN EXIT FUNCTION
    findid& = 2

    i = 0
    i = INSTR(n$, "~"): IF i THEN GOTO gotsc
    i = INSTR(n$, "`"): IF i THEN GOTO gotsc
    i = INSTR(n$, "%"): IF i THEN GOTO gotsc
    i = INSTR(n$, "&"): IF i THEN GOTO gotsc
    i = INSTR(n$, "!"): IF i THEN GOTO gotsc
    i = INSTR(n$, "#"): IF i THEN GOTO gotsc
    i = INSTR(n$, "$"): IF i THEN GOTO gotsc
gotsc:
    IF i THEN
        sc$ = RIGHT$(n$, LEN(n$) - i + 1): n$ = LEFT$(n$, i - 1)
        IF sc$ = "`" OR sc$ = "~`" THEN sc$ = sc$ + "1"
    END IF

    insf$ = subfunc + SPACE$(256 - LEN(subfunc))
    'secondarg$ is only compared trimmed (RTRIM$, LEFT$ 1): no padding needed
    IF LEN(sc$) THEN scpassed = 1: sc$ = sc$ + SPACE$(8 - LEN(sc$)) ELSE scpassed = 0

    n$ = RTRIM$(n$)
    'The hash chain of n$ is walked here, with the results and the HashFind_* state of
    'HashFind (first search) and HashFindCont (findanotherid): most candidates are
    'locals of other procedures (the I of every SUB), and a call per candidate was
    'most of the time of findid.
    IF findanother THEN
        hi = HashFind_NextListItem: hfFlags = HashFind_SearchFlags: hfName$ = HashFind_Name: hfRev = HashFind_Reverse
    ELSE
        hn$ = n$ 'HashValue takes its argument BYREF: n$ itself stays a plain local
        hi = HashTable(HashValue(hn$) AND HASH_TABLE_MASK)
        hfFlags = 1: hfName$ = n$: hfRev = 0
    END IF
findidnext:
    z = 0
    DO WHILE hi
        IF hfFlags AND HashList(hi).Flags THEN
            IF HashListName(hi) = hfName$ THEN
                IF hfRev THEN hfNext = HashList(hi).PrevItem ELSE hfNext = HashList(hi).NextItem
                IF hfNext THEN
                    z = 2
                    HashFind_NextListItem = hfNext
                    HashFind_Reverse = hfRev: HashFind_SearchFlags = hfFlags: HashFind_Name = hfName$
                ELSE
                    z = 1
                END IF
                EXIT DO
            END IF
        END IF
        IF hfRev THEN hi = HashList(hi).PrevItem ELSE hi = HashList(hi).NextItem
    LOOP
    findidinternal = z
    IF z = 0 THEN GOTO noid
    findid = z
    i = HashList(hi).Reference

    IF ids(i).subfunc = 0 AND ids(i).share = 0 THEN
        IF ids(i).insubfunc <> insf$ THEN GOTO findidnomatch
    END IF

    IF ids(i).subfunc = 2 THEN
        IF ASC(ids(i).secondargmustbe) <> 32 THEN
            IF RTRIM$(secondarg$) = UCASE$(RTRIM$(ids(i).secondargmustbe)) THEN
            ELSEIF qbnexprefix_set = 1 AND LEFT$(ids(i).secondargmustbe, 1) = "_" AND LEFT$(secondarg$, 1) <> "_" AND RTRIM$(secondarg$) = UCASE$(MID$(RTRIM$(ids(i).secondargmustbe), 2)) THEN
            ELSE
                GOTO findidnomatch
            END IF
        END IF
        IF ASC(ids(i).secondargcantbe) <> 32 THEN
            IF RTRIM$(secondarg$) <> UCASE$(RTRIM$(ids(i).secondargcantbe)) THEN
            ELSEIF qbnexprefix_set = 1 AND LEFT$(ids(i).secondargcantbe, 1) = "_" AND LEFT$(secondarg$, 1) <> "_" AND RTRIM$(secondarg$) <> UCASE$(MID$(RTRIM$(ids(i).secondargcantbe), 2)) THEN
            ELSE
                GOTO findidnomatch
            END IF
        END IF
    END IF

    imusthave = CVI(ids(i).musthave)
    amusthave = imusthave AND 255
    IF amusthave <> 32 THEN
        IF scpassed THEN
            IF sc$ = ids(i).musthave THEN GOTO findidok
        END IF
        GOTO findidnomatch
    END IF

    IF scpassed THEN
        imayhave = CVI(ids(i).mayhave)
        amayhave = imayhave AND 255
        IF amayhave = 32 THEN GOTO findidnomatch

        IF amayhave = 36 THEN
            IF imayhave <> 8228 THEN
                IF CVI(sc$) = 8228 THEN GOTO findidok
            END IF
        END IF
        IF sc$ <> ids(i).mayhave THEN GOTO findidnomatch
    END IF

findidok:
    'id = ids(i): refer$ makes this copy (getid) before anything else
    t = ids(i).t
    ri = i 'a copy for the BYREF parameter: i stays a plain local
    temp$ = refer$(str2$(ri), t, 1)
    manageVariableList "", temp$, 0, 1
    currentid = i
    EXIT FUNCTION

findidnomatch:
    IF z = 2 THEN hi = hfNext: GOTO findidnext

noid:
    findid& = 0
    currentid = -1
END FUNCTION

FUNCTION FindArray (secure$)
    FindArray = -1
    n$ = secure$
    IF alphanumeric(ASC(n$)) = 0 THEN FindArray = 0: EXIT FUNCTION

    i = INSTR(n$, "~"): IF i THEN sc$ = RIGHT$(n$, LEN(n$) - i + 1): n$ = LEFT$(n$, i - 1): GOTO gotsc2
    i = INSTR(n$, "`"): IF i THEN sc$ = RIGHT$(n$, LEN(n$) - i + 1): n$ = LEFT$(n$, i - 1): GOTO gotsc2
    i = INSTR(n$, "%"): IF i THEN sc$ = RIGHT$(n$, LEN(n$) - i + 1): n$ = LEFT$(n$, i - 1): GOTO gotsc2
    i = INSTR(n$, "&"): IF i THEN sc$ = RIGHT$(n$, LEN(n$) - i + 1): n$ = LEFT$(n$, i - 1): GOTO gotsc2
    i = INSTR(n$, "!"): IF i THEN sc$ = RIGHT$(n$, LEN(n$) - i + 1): n$ = LEFT$(n$, i - 1): GOTO gotsc2
    i = INSTR(n$, "#"): IF i THEN sc$ = RIGHT$(n$, LEN(n$) - i + 1): n$ = LEFT$(n$, i - 1): GOTO gotsc2
    i = INSTR(n$, "$"): IF i THEN sc$ = RIGHT$(n$, LEN(n$) - i + 1): n$ = LEFT$(n$, i - 1): GOTO gotsc2
gotsc2:
    n2$ = n$ + sc$

    IF sc$ <> "" THEN
        try = findid(n2$): IF Error_Happened THEN EXIT FUNCTION
        DO WHILE try
            IF id.arraytype THEN EXIT FUNCTION
            IF try = 2 THEN findanotherid = 1: try = findid(n2$) ELSE try = 0
            IF Error_Happened THEN EXIT FUNCTION
        LOOP
    ELSE
        try = findid(n2$): IF Error_Happened THEN EXIT FUNCTION
        DO WHILE try
            IF id.arraytype THEN
                IF subfuncn = 0 THEN EXIT FUNCTION
                IF id.insubfuncn = subfuncn THEN EXIT FUNCTION
            END IF
            IF try = 2 THEN findanotherid = 1: try = findid(n2$) ELSE try = 0
            IF Error_Happened THEN EXIT FUNCTION
        LOOP

        a = ASC(UCASE$(n$)): IF a = 95 THEN a = 91
        a = a - 64
        n2$ = n$ + defineextaz(a)
        try = findid(n2$): IF Error_Happened THEN EXIT FUNCTION
        DO WHILE try
            IF id.arraytype THEN
                IF subfuncn = 0 THEN EXIT FUNCTION
                IF id.insubfuncn = subfuncn THEN EXIT FUNCTION
                EXIT FUNCTION
            END IF
            IF try = 2 THEN findanotherid = 1: try = findid(n2$) ELSE try = 0
            IF Error_Happened THEN EXIT FUNCTION
        LOOP

        n2$ = n$
        try = findid(n2$): IF Error_Happened THEN EXIT FUNCTION
        DO WHILE try
            IF id.arraytype THEN EXIT FUNCTION
            IF try = 2 THEN findanotherid = 1: try = findid(n2$) ELSE try = 0
            IF Error_Happened THEN EXIT FUNCTION
        LOOP

        a = ASC(UCASE$(n$)): IF a = 95 THEN a = 91
        a = a - 64
        n2$ = n$ + defineextaz(a)
        try = findid(n2$): IF Error_Happened THEN EXIT FUNCTION
        DO WHILE try
            IF id.arraytype THEN EXIT FUNCTION
            IF try = 2 THEN findanotherid = 1: try = findid(n2$) ELSE try = 0
            IF Error_Happened THEN EXIT FUNCTION
        LOOP
    END IF

    FindArray = 0
END FUNCTION

FUNCTION uniquenumber&
    uniquenumbern = uniquenumbern + 1
    uniquenumber& = uniquenumbern
END FUNCTION

FUNCTION validlabel (LABEL2$)
    create = CreatingLabel: CreatingLabel = 0
    validlabel = 0
    IF LEN(LABEL2$) = 0 THEN EXIT FUNCTION
    clabel$ = LABEL2$
    label$ = UCASE$(LABEL2$)

    n = numelements(label$)
    IF n = 1 THEN
        hashres = HashFind(label$, HASHFLAG_RESERVED + HASHFLAG_SUB + HASHFLAG_FUNCTION, hashresflags, hashresref)
        DO WHILE hashres
            IF hashresflags AND (HASHFLAG_SUB + HASHFLAG_FUNCTION) THEN
                IF ids(hashresref).internal_subfunc THEN EXIT FUNCTION

                IF hashresflags AND HASHFLAG_SUB THEN
                    IF ASC(ids(hashresref).specialformat) = 32 THEN
                        IF ids(hashresref).args = 0 THEN onecommandsub = 1 ELSE onecommandsub = 0
                    ELSE
                        IF ASC(ids(hashresref).specialformat) <> 91 THEN
                            onecommandsub = 0
                        ELSE
                            onecommandsub = 1
                            a$ = RTRIM$(ids(hashresref).specialformat)
                            b = 1
                            FOR x = 2 TO LEN(a$)
                                a = ASC(a$, x)
                                IF a = 91 THEN b = b + 1
                                IF a = 93 THEN b = b - 1
                                IF b = 0 AND x <> LEN(a$) THEN onecommandsub = 0: EXIT FOR
                            NEXT
                        END IF
                    END IF
                END IF

                IF create <> 0 AND onecommandsub = 1 THEN
                    IF INSTR(SubNameLabels$, sp + UCASE$(label$) + sp) = 0 THEN PossibleSubNameLabels$ = PossibleSubNameLabels$ + UCASE$(label$) + sp: EXIT FUNCTION
                END IF
            ELSE
                EXIT FUNCTION
            END IF

            IF hashres <> 1 THEN hashres = HashFindCont(hashresflags, hashresref) ELSE hashres = 0
        LOOP

        t$ = label$
        a = ASC(t$)
        IF (a >= 48 AND a <= 57) OR a = 46 THEN
            x = INSTR(t$, CHR$(44))
            IF x THEN t$ = RIGHT$(t$, LEN(t$) - x)

            labelSymbol$ = removesymbol$(t$)
            IF Error_Happened THEN EXIT FUNCTION
            IF LEN(labelSymbol$) THEN
                IF INSTR(labelSymbol$, "$") THEN EXIT FUNCTION
                IF labelSymbol$ <> "#" AND labelSymbol$ <> "!" THEN labelSymbol$ = ""
            END IF

            IF a = 46 THEN dp = 1
            FOR x = 2 TO LEN(t$)
                a = ASC(MID$(t$, x, 1))
                IF a = 46 THEN dp = dp + 1
                IF (a < 48 OR a > 57) AND a <> 46 THEN EXIT FUNCTION
            NEXT x
            IF dp > 1 THEN EXIT FUNCTION
            IF dp = 1 AND LEN(t$) = 1 THEN EXIT FUNCTION

            tlayout$ = t$ + labelSymbol$

            i = INSTR(t$, "."): IF i THEN MID$(t$, i, 1) = "p"
            IF labelSymbol$ = "#" THEN t$ = t$ + "d"
            IF labelSymbol$ = "!" THEN t$ = t$ + "s"
            IF LEN(t$) > 40 THEN EXIT FUNCTION

            LABEL2$ = t$
            validlabel = 1
            EXIT FUNCTION
        END IF
    END IF

    IF (n AND 1) = 0 THEN EXIT FUNCTION
    FOR nx = 2 TO n - 1 STEP 2
        a$ = getelement$(LABEL2$, nx)
        IF a$ <> "." THEN EXIT FUNCTION
    NEXT

    c = ASC(clabel$): IF c >= 48 AND c <= 57 THEN EXIT FUNCTION

    label3$ = ""
    FOR nx = 1 TO n STEP 2
        label$ = getelement$(clabel$, nx)
        FOR x = 1 TO LEN(label$)
            IF alphanumeric(ASC(label$, x)) = 0 THEN EXIT FUNCTION
        NEXT

        IF label3$ = "" THEN
            label3$ = UCASE$(label$)
            tlayout$ = label$
        ELSE
            label3$ = label3$ + fix046$ + UCASE$(label$)
            tlayout$ = tlayout$ + "." + label$
        END IF
    NEXT nx

    validlabel = 1
    LABEL2$ = label3$
END FUNCTION
