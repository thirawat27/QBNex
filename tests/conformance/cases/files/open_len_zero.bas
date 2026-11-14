$CONSOLE:ONLY
' LEN = 0 written in OPEN is an illegal function call; without LEN = the record
' length is the default.
ON ERROR GOTO h
n& = 0
OPEN "open_len_zero.tmp" FOR RANDOM AS #1 LEN = n&
PRINT "after len 0"
OPEN "open_len_zero.tmp" FOR RANDOM AS #1
PRINT LOF(1)
CLOSE #1
OPEN "open_len_zero.tmp" FOR RANDOM AS #1 LEN = 16
a$ = "x"
PUT #1, 2, a$
PRINT LOF(1)
CLOSE #1
KILL "open_len_zero.tmp"
SYSTEM
h:
PRINT "error"; ERR
RESUME NEXT
