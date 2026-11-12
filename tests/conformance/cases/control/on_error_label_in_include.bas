$CONSOLE:ONLY
' Module code with labels after the SUBs, inside an $INCLUDE file
ON ERROR GOTO handler
x = 3
ERROR 7
CALL s
greet
GOSUB more
PRINT "end"
END

SUB s
    PRINT "in s"
END SUB

'$INCLUDE:'label_in_include_helper.bm'
