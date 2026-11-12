$CONSOLE:ONLY
' TIME$ = and DATE$ = move a program-local clock (the OS clock is not set):
' TIME$, DATE$ and TIMER then read the set time advancing from there.
' A badly formed value is an illegal function call, as in QBasic.
TIME$ = "13:45:00"
PRINT LEFT$(TIME$, 5)
t! = TIMER
PRINT t! >= 49500 AND t! < 49560
TIME$ = "7:05"
PRINT LEFT$(TIME$, 5)
DATE$ = "02-29-2024"
PRINT DATE$
DATE$ = "12/31/99"
PRINT DATE$; " "; LEFT$(TIME$, 5)
ON ERROR GOTO bad
TIME$ = "24:00:00"
DATE$ = "02-30-2023"
TIME$ = "12-00"
DATE$ = "01-01-1979"
PRINT DATE$; " "; LEFT$(TIME$, 5)
SYSTEM
bad:
PRINT "error"; ERR
RESUME NEXT
