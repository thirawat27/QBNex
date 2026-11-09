$CONSOLE:ONLY
' SHELL output goes to the same (captured) stdout; exit codes from SHELL()
' and _SHELLHIDE(); _DONTWAIT finishes later
SHELL "echo from shell"
SHELL "cmd /c echo second& echo third"
r = SHELL("exit 7")
PRINT "rc"; r
PRINT "hide"; _SHELLHIDE("cmd /c exit 4")
PRINT "hide2"; _SHELLHIDE("exit 2")
SHELL _HIDE "cmd /c echo hidden> conf_shell_tmp.txt"
OPEN "conf_shell_tmp.txt" FOR INPUT AS #1: LINE INPUT #1, l$: CLOSE #1
PRINT "["; l$; "]"
KILL "conf_shell_tmp.txt"
SHELL _DONTWAIT _HIDE "cmd /c echo later> conf_shell_tmp2.txt"
t# = TIMER
DO WHILE _FILEEXISTS("conf_shell_tmp2.txt") = 0 AND TIMER - t# < 10: _DELAY .05: LOOP
_DELAY .3
OPEN "conf_shell_tmp2.txt" FOR INPUT AS #1: LINE INPUT #1, l$: CLOSE #1
PRINT "["; l$; "]"
KILL "conf_shell_tmp2.txt"
SYSTEM
