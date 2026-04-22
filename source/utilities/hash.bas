FUNCTION HashValue& (a$)
    l = LEN(a$)
    IF l = 0 THEN EXIT FUNCTION

    a = ASC(a$)
    IF a <> 95 THEN
        SELECT CASE l
        CASE 1
            HashValue& = hash1char(a) + 1048576
            EXIT FUNCTION
        CASE 2
            HashValue& = hash2char(CVI(a$)) + 2097152
            EXIT FUNCTION
        CASE 3
            HashValue& = hash2char(CVI(a$)) + hash1char(ASC(a$, 3)) * 1024 + 3145728
            EXIT FUNCTION
        CASE ELSE
            HashValue& = hash2char(CVI(a$)) + hash2char(ASC(a$, l) + ASC(a$, l - 1) * 256) * 1024 + (l AND 7) * 1048576
            EXIT FUNCTION
        END SELECT
    ELSE
        SELECT CASE l
        CASE 1
            HashValue& = 1048576 + 8388608
            EXIT FUNCTION
        CASE 2
            HashValue& = hash1char(ASC(a$, 2)) + 2097152 + 8388608
            EXIT FUNCTION
        CASE 3
            HashValue& = hash2char(ASC(a$, 2) + ASC(a$, 3) * 256) + 3145728 + 8388608
            EXIT FUNCTION
        CASE 4
            HashValue& = hash2char((CVL(a$) AND &HFFFF00) \ 256) + hash1char(ASC(a$, 4)) * 1024 + 4194304 + 8388608
            EXIT FUNCTION
        CASE ELSE
            HashValue& = hash2char((CVL(a$) AND &HFFFF00) \ 256) + hash2char(ASC(a$, l) + ASC(a$, l - 1) * 256) * 1024 + (l AND 7) * 1048576 + 8388608
            EXIT FUNCTION
        END SELECT
    END IF
END FUNCTION

SUB HashAdd (a$, flags, reference)
    IF HashListFreeLast > 0 THEN
        i = HashListFree(HashListFreeLast)
        HashListFreeLast = HashListFreeLast - 1
    ELSE
        IF HashListNext > HashListSize THEN
            HashListSize = HashListSize * 2
            REDIM _PRESERVE HashList(1 TO HashListSize) AS HashListItem
            REDIM _PRESERVE HashListName(1 TO HashListSize) AS STRING * 256
        END IF
        i = HashListNext
        HashListNext = HashListNext + 1
    END IF

    x = HashValue(a$) AND HASH_TABLE_MASK
    i2 = HashTable(x)
    IF i2 THEN
        i3 = HashList(i2).LastItem
        HashList(i2).LastItem = i
        HashList(i3).NextItem = i
        HashList(i).PrevItem = i3
    ELSE
        HashTable(x) = i
        HashList(i).PrevItem = 0
        HashList(i).LastItem = i
    END IF
    HashList(i).NextItem = 0

    HashList(i).Flags = flags
    HashList(i).Reference = reference
    HashListName(i) = UCASE$(a$)
END SUB

FUNCTION HashFind (a$, searchflags, resultflags, resultreference)
    i = HashTable(HashValue(a$) AND HASH_TABLE_MASK)
    IF i THEN
        ua$ = UCASE$(a$) + SPACE$(256 - LEN(a$))
        hashfind_next:
        f = HashList(i).Flags
        IF searchflags AND f THEN
            IF HashListName(i) = ua$ THEN
                resultflags = f
                resultreference = HashList(i).Reference
                i2 = HashList(i).NextItem
                IF i2 THEN
                    HashFind = 2
                    HashFind_NextListItem = i2
                    HashFind_Reverse = 0
                    HashFind_SearchFlags = searchflags
                    HashFind_Name = ua$
                    EXIT FUNCTION
                ELSE
                    HashFind = 1
                    EXIT FUNCTION
                END IF
            END IF
        END IF
        i = HashList(i).NextItem
        IF i THEN GOTO hashfind_next
    END IF
END FUNCTION

FUNCTION HashFindRev (a$, searchflags, resultflags, resultreference)
    i = HashTable(HashValue(a$) AND HASH_TABLE_MASK)
    IF i THEN
        i = HashList(i).LastItem
        ua$ = UCASE$(a$) + SPACE$(256 - LEN(a$))
        hashfindrev_next:
        f = HashList(i).Flags
        IF searchflags AND f THEN
            IF HashListName(i) = ua$ THEN
                resultflags = f
                resultreference = HashList(i).Reference
                i2 = HashList(i).PrevItem
                IF i2 THEN
                    HashFindRev = 2
                    HashFind_NextListItem = i2
                    HashFind_Reverse = 1
                    HashFind_SearchFlags = searchflags
                    HashFind_Name = ua$
                    EXIT FUNCTION
                ELSE
                    HashFindRev = 1
                    EXIT FUNCTION
                END IF
            END IF
        END IF
        i = HashList(i).PrevItem
        IF i THEN GOTO hashfindrev_next
    END IF
END FUNCTION

FUNCTION HashFindCont (resultflags, resultreference)
    IF HashFind_Reverse THEN
        i = HashFind_NextListItem
        hashfindrevc_next:
        f = HashList(i).Flags
        IF HashFind_SearchFlags AND f THEN
            IF HashListName(i) = HashFind_Name THEN
                resultflags = f
                resultreference = HashList(i).Reference
                i2 = HashList(i).PrevItem
                IF i2 THEN
                    HashFindCont = 2
                    HashFind_NextListItem = i2
                    EXIT FUNCTION
                ELSE
                    HashFindCont = 1
                    EXIT FUNCTION
                END IF
            END IF
        END IF
        i = HashList(i).PrevItem
        IF i THEN GOTO hashfindrevc_next
        EXIT FUNCTION
    ELSE
        i = HashFind_NextListItem
        hashfindc_next:
        f = HashList(i).Flags
        IF HashFind_SearchFlags AND f THEN
            IF HashListName(i) = HashFind_Name THEN
                resultflags = f
                resultreference = HashList(i).Reference
                i2 = HashList(i).NextItem
                IF i2 THEN
                    HashFindCont = 2
                    HashFind_NextListItem = i2
                    EXIT FUNCTION
                ELSE
                    HashFindCont = 1
                    EXIT FUNCTION
                END IF
            END IF
        END IF
        i = HashList(i).NextItem
        IF i THEN GOTO hashfindc_next
        EXIT FUNCTION
    END IF
END FUNCTION

SUB HashDump
    fh = FREEFILE
    OPEN "hashdump.txt" FOR OUTPUT AS #fh
    b$ = "12345678901234567890123456789012}"

    FOR x = 0 TO HASH_TABLE_MASK
        IF HashTable(x) THEN
            PRINT #fh, "START HashTable("; x; "):"
            i = HashTable(x)

            lasti = HashList(i).LastItem
            IF HashList(i).LastItem = 0 OR HashList(i).PrevItem <> 0 OR (HashValue(HashListName(i)) AND HASH_TABLE_MASK) <> x THEN GOTO corrupt

            PRINT #fh, "  HashList("; i; ").LastItem="; HashList(i).LastItem
            hashdumpnextitem:
            x$ = "  [" + STR$(i) + "]" + HashListName(i)

            f = HashList(i).Flags
            x$ = x$ + ",.Flags=" + STR$(f) + "{"
            FOR z = 1 TO 32
                ASC(b$, z) = (f AND 1) + 48
                f = f \ 2
            NEXT
            x$ = x$ + b$
            x$ = x$ + ",.Reference=" + STR$(HashList(i).Reference)

            PRINT #fh, x$

            i1 = HashList(i).PrevItem
            i2 = HashList(i).NextItem
            IF i1 THEN
                IF HashList(i1).NextItem <> i THEN GOTO corrupt
            END IF
            IF i2 THEN
                IF HashList(i2).PrevItem <> i THEN GOTO corrupt
            END IF
            IF i2 = 0 THEN
                IF lasti <> i THEN GOTO corrupt
            END IF

            i = HashList(i).NextItem
            IF i THEN GOTO hashdumpnextitem

            PRINT #fh, "END HashTable("; x; ")"
        END IF
    NEXT
    CLOSE #fh
    EXIT SUB

    corrupt:
    PRINT #fh, "HASH TABLE CORRUPT!"
    CLOSE #fh
END SUB

SUB HashClear
    DIM usedItems AS LONG
    DIM clearLimit AS LONG

    usedItems = HashListNext - 1
    IF usedItems < 0 THEN usedItems = 0

    clearLimit = HashListFreeLast
    IF clearLimit > HashListFreeSize THEN clearLimit = HashListFreeSize

    FOR i = 1 TO usedItems
        HashList(i).Flags = 0
        HashList(i).Reference = 0
        HashList(i).NextItem = 0
        HashList(i).PrevItem = 0
        HashList(i).LastItem = 0
        HashListName(i) = ""
    NEXT

    FOR i = 1 TO clearLimit
        HashListFree(i) = 0
    NEXT

    FOR i = 0 TO HASH_TABLE_MASK
        HashTable(i) = 0
    NEXT

    HashListNext = 1
    HashListFreeLast = 0

    HashFind_NextListItem = 0
    HashFind_Reverse = 0
    HashFind_SearchFlags = 0
    HashFind_Name = ""
END SUB
