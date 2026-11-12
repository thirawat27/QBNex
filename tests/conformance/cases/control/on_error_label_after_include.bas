$CONSOLE:ONLY
' Module code with labels after an $INCLUDE that brings the program's SUBs
ON ERROR GOTO handler
x = 7
ERROR 5
greet
GOSUB more
PRINT "end"
END
'$INCLUDE:'label_after_include_helper.bm'
handler:
PRINT "handled"; ERR; x
RESUME NEXT
more:
PRINT "more"; x
RETURN
