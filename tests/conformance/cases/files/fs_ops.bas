$CONSOLE:ONLY
' Error paths (75/76/53) are not covered here (the old C++ runtime hung in its
' MessageBox on them).
' MKDIR / CHDIR / _CWD$ / NAME / KILL with wildcards / RMDIR / _FILEEXISTS / _DIREXISTS
d$ = "conf_fsops_dir"
MKDIR d$
PRINT _DIREXISTS(d$); _FILEEXISTS(d$)
CHDIR d$
PRINT RIGHT$(_CWD$, LEN(d$))
FOR i = 1 TO 3
    OPEN "f" + LTRIM$(STR$(i)) + ".txt" FOR OUTPUT AS #1: PRINT #1, i: CLOSE #1
NEXT
OPEN "keep.dat" FOR OUTPUT AS #1: CLOSE #1
NAME "f1.txt" AS "g1.txt"
PRINT _FILEEXISTS("f1.txt"); _FILEEXISTS("g1.txt"); _FILEEXISTS("x.txt")
KILL "*.txt"
PRINT _FILEEXISTS("g1.txt"); _FILEEXISTS("f3.txt"); _FILEEXISTS("keep.dat")
CHDIR ".."
PRINT _DIREXISTS(d$)
KILL d$ + "/keep.dat"
RMDIR d$
PRINT _DIREXISTS(d$)
SYSTEM
