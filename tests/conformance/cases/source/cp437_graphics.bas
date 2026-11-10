$CONSOLE:ONLY
' CP437 box drawing: E2 95 90 (‚ïê) is also valid UTF-8, as is ‚ïë
' but the file also has bytes that are not UTF-8 (…Õª∞±≤)
READ a$
PRINT LEN(a$); ASC(a$, 1); ASC(a$, 4)
SYSTEM
DATA "‚ïê…Õª‚ïë"
