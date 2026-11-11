PRINT "window"
INPUT "Name"; n$
INPUT "Age"; a
OPEN "answer.txt" FOR OUTPUT AS #1
PRINT #1, n$; a
CLOSE #1
SLEEP
OPEN "slept.txt" FOR OUTPUT AS #1
PRINT #1, "ok"
CLOSE #1
END
