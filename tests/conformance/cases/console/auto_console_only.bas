' No $CONSOLE metacommand and no graphics: the compiler makes it a console-only
' program (stdout, no window, END does not wait for a key).
PRINT 5
x$ = "done"
PRINT x$
END
