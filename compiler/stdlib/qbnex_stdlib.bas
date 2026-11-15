' ============================================================================
' QBNex Standard Library
' ============================================================================
' Preferred entrypoint: '$IMPORT:'qbnex' (stdlib.bas, which includes this file)
'
' 'qbnex' is the modules below, each imported by its own name: a program may
' also import any of them on its own, before or after 'qbnex', and gets each
' once. The modules are the only copy of their code.
' ============================================================================

'$IMPORT:'oop.class'
'$IMPORT:'oop.interface'
'$IMPORT:'collections.list'
'$IMPORT:'collections.stack'
'$IMPORT:'collections.queue'
'$IMPORT:'collections.set'
'$IMPORT:'collections.dictionary'
'$IMPORT:'strings.strbuilder'
'$IMPORT:'strings.text'
'$IMPORT:'io.json'
'$IMPORT:'sys.env'
'$IMPORT:'sys.args'
'$IMPORT:'sys.datetime'
'$IMPORT:'io.path'
'$IMPORT:'io.csv'
'$IMPORT:'math.numeric'
'$IMPORT:'error.result'

FUNCTION QBNex_StdLib_Version$ ()
    QBNex_StdLib_Version = "2.0.0-preview.3"
END FUNCTION

FUNCTION QBNex_StdLib_Info$ ()
    DIM text AS STRING

    text = "QBNex Standard Library v" + QBNex_StdLib_Version$ + CHR$(13) + CHR$(10)
    text = text + "Build: 2026.04.13" + CHR$(13) + CHR$(10)
    text = text + "Modules: oop, collections, strings, sys, io, math, data, result, datetime, class syntax"

    QBNex_StdLib_Info = text
END FUNCTION

SUB QBNex_StdLib_PrintInfo ()
    PRINT QBNex_StdLib_Info$
END SUB
