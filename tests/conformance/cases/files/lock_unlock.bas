$CONSOLE:ONLY
' LOCK / UNLOCK as on Windows: a range that overlaps one the file number holds,
' and UNLOCK of a range that was not locked as it names, are "Permission
' denied" (70); a LOCK and the UNLOCK that matches it work.
ON ERROR GOTO oops
OPEN "lk_t.dat" FOR BINARY AS #1
d$ = "0123456789"
PUT #1, 1, d$
LOCK #1, 1 TO 4
PRINT "locked"
LOCK #1, 3 TO 6
UNLOCK #1, 2 TO 4
UNLOCK #1, 1 TO 4
PRINT "unlocked"
LOCK #1
UNLOCK #1
CLOSE #1
KILL "lk_t.dat"
PRINT "done"
SYSTEM
oops:
PRINT "error"; ERR
RESUME NEXT
