$CONSOLE:ONLY
' QBasic: a string variable may have the name of a built-in string function
' (LCASE next to LCASE$(x$)); name$( is the call, the bare name the variable
DIM upper AS STRING
DIM LCASE AS STRING
DIM UCASE AS STRING
upper = "AbC"
LCASE = LCASE$(upper)
UCASE = UCASE$(upper)
PRINT LCASE; " "; UCASE; " "; LCASE$(UCASE)
SYSTEM
