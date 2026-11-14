$CONSOLE:ONLY
' TRY keeps the handler that was set before it: the program's ON ERROR GOTO works
' again after END TRY, an inner TRY...FINALLY passes its error to the outer TRY, a
' TRY left with EXIT FOR or EXIT SUB does not go on catching errors, and line
' numbers after the block are the source lines.
ON ERROR GOTO h
TRY
    PRINT "body"
CATCH e
END TRY
ERROR 10
PRINT "after"
FOR i = 1 TO 2
    TRY
        TRY
            IF i = 1 THEN ERROR 5
            PRINT "inner ok"; i
        FINALLY
            PRINT "fin"; i
        END TRY
        PRINT "outer ok"; i
    CATCH e
        PRINT "caught"; e; "i ="; i
    END TRY
NEXT
FOR i = 1 TO 3
    TRY
        IF i = 2 THEN EXIT FOR
    CATCH e
    END TRY
NEXT
ERROR 11
work
ERROR 12
PRINT "line"; _ERRORLINE
SYSTEM
h:
PRINT "handler"; ERR
RESUME NEXT

SUB work
    TRY
        EXIT SUB
    CATCH e
    END TRY
END SUB
