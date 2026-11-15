$CONSOLE:ONLY
' BYREF parameters that refer to a SHARED argument (the SUB also uses the
' variable by name) called from native code, with errors inside them.
DEFLNG A-Z
DIM SHARED px AS LONG, tot AS DOUBLE, a(5) AS LONG
ON ERROR GOTO h
FOR i = 1 TO 3000
    CALL bump(px)
    IF i MOD 1000 = 0 THEN CALL bad(px)
NEXT
PRINT px; tot; e
SYSTEM
h:
e = e + 1
RESUME NEXT
SUB bump (v AS LONG)
    v = v + 1
    tot = tot + px
END SUB
SUB bad (v AS LONG)
    v = v + 10
    a(v) = 1
    v = v + 100
    tot = tot + px
END SUB
