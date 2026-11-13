$CONSOLE:ONLY
' _DIR$ ends with the path separator and falls back to the desktop (the current
' folder off Windows) for a name it does not know; _ENVIRONCOUNT counts ENVIRON$(n).
d$ = _DIR$("desktop")
PRINT LEN(d$) > 0; RIGHT$(d$, 1) = "\" OR RIGHT$(d$, 1) = "/"
PRINT _DIR$("no such folder") = d$
PRINT _DIR$("DeskTop") = d$
t$ = _DIR$("temp")
PRINT RIGHT$(t$, 1) = "\" OR RIGHT$(t$, 1) = "/"
n = _ENVIRONCOUNT
PRINT n > 0
PRINT LEN(ENVIRON$(n)) > 0; LEN(ENVIRON$(n + 1))
SYSTEM
