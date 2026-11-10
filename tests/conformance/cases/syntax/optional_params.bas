$CONSOLE:ONLY
DECLARE SUB Greet (who$, OPTIONAL greeting$ = "Hello", OPTIONAL times% = 1)
Greet "Ada"
Greet "Bob", "Hi"
CALL Greet("Cy", "Yo", 2)
PRINT Scale(3); Scale(3, 10)
x = Scale(2) + 1: PRINT x
SYSTEM

SUB Greet (who$, OPTIONAL greeting$ = "Hello", OPTIONAL times% = 1)
    FOR i% = 1 TO times%
        PRINT greeting$; ", "; who$
    NEXT
END SUB

FUNCTION Scale& (v AS LONG, OPTIONAL factor AS LONG = 2)
    Scale& = v * factor
END FUNCTION
