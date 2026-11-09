$CONSOLE:ONLY
' ENVIRON / ENVIRON$: '=' or space separator, removal, lookup by index
ENVIRON "QB_CONF_A=alpha"
ENVIRON "QB_CONF_B beta gamma"
ENVIRON "QB_CONF_C=x=y"
PRINT "["; ENVIRON$("QB_CONF_A"); "]["; ENVIRON$("QB_CONF_B"); "]["; ENVIRON$("QB_CONF_C"); "]"
ENVIRON "QB_CONF_A="
PRINT "["; ENVIRON$("QB_CONF_A"); "]["; ENVIRON$("QB_CONF_NONE"); "]"
found = 0
FOR i = 1 TO 1000
    e$ = ENVIRON$(i)
    IF e$ = "" THEN EXIT FOR
    IF LEFT$(e$, 10) = "QB_CONF_B=" THEN found = found + 1: PRINT e$
NEXT
PRINT found; i > 2
PRINT "["; ENVIRON$(100000); "]"
SYSTEM
