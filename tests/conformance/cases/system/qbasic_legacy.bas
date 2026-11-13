$CONSOLE:ONLY
' QuickBASIC statements about memory, device drivers and events that old programs
' still contain: they compile and answer the way a machine with plenty of memory
' and no light pen or serial event would.
ON ERROR GOTO h
PRINT FRE(-1) > 100000; FRE(-2) > 1000; FRE(0) > 1000; FRE("") = FRE(0); FRE(a$) = FRE(0)
PRINT SETMEM(0) > 100000
TRON
TROFF
OPEN "qblegacy.tmp" FOR OUTPUT AS #1
PRINT "output"; FILEATTR(1, 1)
PRINT #1, "x"
CLOSE #1
OPEN "qblegacy.tmp" FOR INPUT AS #1: PRINT "input"; FILEATTR(1, 1): CLOSE #1
OPEN "qblegacy.tmp" FOR APPEND AS #1: PRINT "append"; FILEATTR(1, 1): CLOSE #1
OPEN "qblegacy.tmp" FOR RANDOM AS #1 LEN = 4: PRINT "random"; FILEATTR(1, 1): CLOSE
OPEN "qblegacy.tmp" FOR BINARY AS #2
PRINT "binary"; FILEATTR(2, 1); FILEATTR(2, 2) > 0
PRINT FILEATTR(2, 3)
PRINT FILEATTR(9, 1)
IOCTL #2, "x"
a$ = IOCTL$(2)
a$ = IOCTL$(9)
PRINT "lock"
LOCK #2, 1 TO 4
UNLOCK #2, 1 TO 4
LOCK #2
UNLOCK #2
LOCK #9
CLOSE
KILL "qblegacy.tmp"
PRINT "pen"
PEN ON: PEN STOP: PEN OFF
PRINT PEN(0); PEN(9)
PRINT PEN(10)
PRINT "com"
COM(1) ON: COM(2) STOP: COM(1) OFF
COM(3) ON
UEVENT ON: UEVENT STOP: UEVENT OFF
ON PEN GOSUB ev
ON UEVENT GOSUB ev
ON COM(1) GOSUB ev
PRINT "unicode"; _MAPUNICODE(65); _MAPUNICODE(1)
_MAPUNICODE 8364 TO 128
PRINT _MAPUNICODE(128)
_MAPUNICODE 70000 TO 1
PRINT "end"
SYSTEM
ev: RETURN
h: PRINT "error"; ERR: RESUME NEXT
