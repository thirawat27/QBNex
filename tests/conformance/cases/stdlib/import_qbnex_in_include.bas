$CONSOLE:ONLY
' 'qbnex' imported in an $INCLUDE file, with one of its modules imported before
' it and another one after it: each module is in the program once.
'$INCLUDE:'import_qbnex_in_include.bi'
'$IMPORT:'collections.list'
DIM st AS QBNex_Stack
Stack_Init st: Stack_Push st, "p"
PRINT Stack_Pop$(st); Text_Repeat$("ab", 2); QBNEX_EnsureClass&("X", 0); QBNex_StdLib_Version$ <> ""
SYSTEM
