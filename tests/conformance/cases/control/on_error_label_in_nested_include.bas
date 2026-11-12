$CONSOLE:ONLY
' Module code with labels after the SUBs, in a file included by an included file
ON ERROR GOTO oops
y = 4
ERROR 9
hello
GOSUB tail
PRINT "end"
END
'$INCLUDE:'label_nested_outer_helper.bm'
