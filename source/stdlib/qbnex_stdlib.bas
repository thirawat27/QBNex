' =============================================================================
' QBNex Standard Library — Umbrella Include — qbnex_stdlib.bas
' =============================================================================
'
' Include this single file to pull in the entire standard library.
' Only include what you need for smaller build sizes.
'
' Usage:
'
'   '$INCLUDE:'stdlib/qbnex_stdlib.bas'
'
' =============================================================================

' ---- OOP Foundation -------------------------------------------------------
'$INCLUDE:'stdlib/oop/class.bas'
'$INCLUDE:'stdlib/oop/interface.bas'

' ---- Collections ----------------------------------------------------------
'$INCLUDE:'stdlib/collections/list.bas'
'$INCLUDE:'stdlib/collections/dictionary.bas'
'$INCLUDE:'stdlib/collections/stack.bas'
'$INCLUDE:'stdlib/collections/queue.bas'
'$INCLUDE:'stdlib/collections/set.bas'

' ---- Strings ---------------------------------------------------------------
'$INCLUDE:'stdlib/strings/strbuilder.bas'
'$INCLUDE:'stdlib/strings/encoding.bas'
'$INCLUDE:'stdlib/strings/regex.bas'

' ---- Math ------------------------------------------------------------------
'$INCLUDE:'stdlib/math/vector.bas'
'$INCLUDE:'stdlib/math/matrix.bas'
'$INCLUDE:'stdlib/math/stats.bas'

' ---- I/O -------------------------------------------------------------------
'$INCLUDE:'stdlib/io/path.bas'
'$INCLUDE:'stdlib/io/csv.bas'
'$INCLUDE:'stdlib/io/json.bas'

' ---- DateTime --------------------------------------------------------------
'$INCLUDE:'stdlib/datetime/datetime.bas'

' ---- Error Handling --------------------------------------------------------
'$INCLUDE:'stdlib/error/error.bas'

' ---- System ----------------------------------------------------------------
'$INCLUDE:'stdlib/sys/env.bas'
'$INCLUDE:'stdlib/sys/process.bas'
'$INCLUDE:'stdlib/sys/args.bas'
