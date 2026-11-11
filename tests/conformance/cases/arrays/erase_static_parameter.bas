$CONSOLE:ONLY
' ERASE of a static array passed to a SUB as a() clears it; the caller's
' array keeps its dimensions.
DIM t(3, 3)
t(1, 1) = 5
clr t()
t(1, 1) = t(1, 1) + 1
PRINT t(1, 1)
SYSTEM
SUB clr (a())
ERASE a
END SUB
