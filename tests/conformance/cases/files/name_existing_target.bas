$CONSOLE:ONLY
' NAME does not replace a file that exists: error 58 (the target was overwritten)
ON ERROR GOTO h
OPEN "conf_name_a.tmp" FOR OUTPUT AS #1: PRINT #1, "first": CLOSE #1
OPEN "conf_name_b.tmp" FOR OUTPUT AS #1: PRINT #1, "second": CLOSE #1
NAME "conf_name_a.tmp" AS "conf_name_b.tmp"
OPEN "conf_name_b.tmp" FOR INPUT AS #1: LINE INPUT #1, l$: CLOSE #1
PRINT "target holds: "; l$
NAME "conf_name_missing.tmp" AS "conf_name_c.tmp"
KILL "conf_name_b.tmp"
NAME "conf_name_a.tmp" AS "conf_name_b.tmp"
PRINT _FILEEXISTS("conf_name_a.tmp"); _FILEEXISTS("conf_name_b.tmp")
KILL "conf_name_b.tmp"
SYSTEM
h:
PRINT "error"; ERR
RESUME NEXT
