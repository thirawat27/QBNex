' Main-module ON ERROR handler labels before the first SUB, CLEAR with a stack size and
' END SUB followed by a comment: the layout of SORTDEMO.BAS (issue #2). The handler stays
' in the main module; it used to make END SUB "Unknown statement 'END'".
DEFINT A-Z
DECLARE SUB Show (n)
DIM SHARED total
ON ERROR GOTO Handler
total = 5
ERROR 5
Show total
CLEAR , , 5120
PRINT "after clear"; total
END

Handler:
    PRINT "error"; ERR
    RESUME NEXT

SUB Show (n) STATIC
    PRINT "show"; n
END SUB 'done
