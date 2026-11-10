$CONSOLE:ONLY
' REDIM SHARED after an imported module, and a FUNCTION that grows the shared
' array with REDIM _PRESERVE (it must not become a local array).
'$IMPORT:'collections.list'
REDIM SHARED vals(1 TO 1) AS LONG
vals(1) = 7
PRINT Grow(4); UBOUND(vals); vals(1); vals(4)
SYSTEM

FUNCTION Grow& (n AS LONG)
    REDIM _PRESERVE vals(1 TO n) AS LONG
    vals(n) = n * 10
    Grow = UBOUND(vals)
END FUNCTION
