$CONSOLE:ONLY
' A runtime error in module code after the SUBs of an $INCLUDE file (moved in front of
' the first SUB) and in a SUB of that file reports the line inside the included file
ON ERROR GOTO trap
'$INCLUDE:'include_error_line_helper.bm'
PRINT "end"
END

trap:
PRINT "error"; ERR; "at"; _ERRORLINE; _INCLERRORLINE; RIGHT$(_INCLERRORFILE$, 28)
RESUME NEXT
