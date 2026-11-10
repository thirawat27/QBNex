ON ERROR GOTO handler
OPEN "web_test.txt" FOR OUTPUT AS #1
PRINT #1, "x"
CLOSE #1
PRINT "end"
END
handler:
PRINT "error"; ERR
RESUME NEXT
