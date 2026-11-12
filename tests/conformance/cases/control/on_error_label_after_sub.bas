$CONSOLE:ONLY
ON ERROR GOTO handler
x = 5
ERROR 11
PRINT "after"; x
CALL s
GOSUB extra
PRINT "end"
END

SUB s
    PRINT "in s"
END SUB

handler:
PRINT "handled"; ERR; x
RESUME NEXT

extra:
PRINT "extra"; x
RETURN
