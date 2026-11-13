$CONSOLE
_DEST _CONSOLE
' Memory-mode program (whole-array GET below): an array used without DIM is
' implicitly DIM name(10), as in the register model and in QBasic.
EV(1) = 1: PRINT EV(1)
SYSTEM
DIM b1%(480)
GET (1, 1)-(32, 30), b1%
