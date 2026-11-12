$CONSOLE:ONLY
' run by chain_sub_main.bas (CHAIN from inside a SUB): its COMMON values and arrays arrive
' here in the same order; the exit code reaches whoever started chain_sub_main
DIM w(1 TO 3) AS LONG
COMMON a%, b#, c$, d&&, e!, f~%%, g%%
COMMON w() AS LONG, t$(), z#()
PRINT a%; b#; c$; d&&; e!; f~%%; g%%
PRINT w(1); w(2); w(3)
PRINT LBOUND(t$); UBOUND(t$); " "; t$(-1); "|"; t$(0); "|"; t$(1)
PRINT UBOUND(z#, 1); LBOUND(z#, 2); UBOUND(z#, 2); z#(2, 2); z#(1, 1)
SYSTEM 3
