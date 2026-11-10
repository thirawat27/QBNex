$CONSOLE:ONLY
' Integer division by zero (\ and MOD) is error 11 and can be trapped, as in
' QBasic 4.5: ON ERROR GOTO, RESUME NEXT, and TRY/CATCH. Also -32768 \ -1 style
' overflows on the widest integer type are error 6 instead of a crash.
ON ERROR GOTO handler
a% = 7: z% = 0: r% = 99
r% = a% \ z%
PRINT "after \ r%="; r%
r% = 99
r% = a% MOD z%
PRINT "after MOD r%="; r%
DIM big AS _INTEGER64, mone AS _INTEGER64, q AS _INTEGER64
big = -9223372036854775807 - 1: mone = -1
q = big \ mone
PRINT "after min\-1"
l& = 100& \ 7&: m& = -7 MOD 3: u~& = 4000000000 \ 3
PRINT l&; m&; u~&
ON ERROR GOTO 0

TRY
    PRINT 10 \ (z% * 2)
    PRINT "not here"
CATCH e
    PRINT "caught"; e
END TRY
TRY
    x& = 5 MOD z%
CATCH e2
    PRINT "caught MOD"; e2; _ERRORMESSAGE$(e2)
END TRY
PRINT Safe&(9, 3); Safe&(9, 0)
PRINT "done"
SYSTEM

handler:
PRINT "ERR"; ERR; _ERRORMESSAGE$
RESUME NEXT

FUNCTION Safe& (a AS LONG, b AS LONG)
    TRY
        Safe& = a \ b
    CATCH
        Safe& = -1
    END TRY
END FUNCTION
