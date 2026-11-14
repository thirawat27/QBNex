$CONSOLE:ONLY
' DEF FN in an included file, single-line and DEF ... END DEF.
'$INCLUDE:'def_fn_in_include.bi'
DEF FNdbl (x) = x * 2
PRINT FNinc(1); FNbig(3, 9); FNdbl(4); fromInclude
SYSTEM
