$CONSOLE:ONLY
' _DEFLATE$ / _INFLATE$ (qb-runtime compress.rs, no system zlib).
' Compressed bytes are not printed: they may differ from zlib's (deviation D-R10-1).
' Expected values follow zlib's compress()/uncompress() contracts.
a$ = "QBNex " + STRING$(200, "x") + "end"
z$ = _DEFLATE$(a$)
PRINT LEN(a$); LEN(z$) < LEN(a$); ASC(z$, 1)
PRINT _INFLATE$(z$) = a$
PRINT _INFLATE$(z$, LEN(a$)) = a$
b$ = _INFLATE$(z$, 10)
PRINT LEN(b$); b$
PRINT LEN(_INFLATE$(z$, 0))
PRINT LEN(_INFLATE$(_DEFLATE$(""))); LEN(_DEFLATE$("")) > 0
c$ = SPACE$(5000)
FOR i = 1 TO 5000: MID$(c$, i, 1) = CHR$((i * 7 + i \ 13) MOD 256): NEXT
d$ = _DEFLATE$(c$)
PRINT _INFLATE$(d$) = c$; _INFLATE$(d$, 6000) = c$; LEN(_INFLATE$(d$, 4999))
PRINT LEN(_INFLATE$("not zlib data"))
SYSTEM
