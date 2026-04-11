' ============================================================================
' QBNex Standard Library - Main Include File
' ============================================================================
' Include this file to load the entire QBNex Standard Library
' 
' Usage:
'   '$INCLUDE:'stdlib/qbnex_stdlib.bas'
'
' Or include individual modules:
'   '$INCLUDE:'stdlib/collections/list.bas'
'   '$INCLUDE:'stdlib/strings/encoding.bas'
'
' ============================================================================
' QBNex Standard Library v1.0.0
' Complete Standard Library Ecosystem for QBNex BASIC
' 
' Provides:
'   - OOP Foundation (Classes, Interfaces, Generics)
'   - Collections (List, Dictionary, Stack, Queue, Set)
'   - String Utilities (StringBuilder, Encoding, Regex)
'   - Math (Vector, Matrix, Statistics)
'   - I/O (Path, CSV, JSON)
'   - DateTime (Arithmetic, Formatting)
'   - Error Handling (Structured errors, assertions)
'   - System Integration (Environment, Process, Arguments)
' ============================================================================

' ============================================================================
' OOP Foundation
' ============================================================================
'$INCLUDE:'oop/class.bas'
'$INCLUDE:'oop/interface.bas'
'$INCLUDE:'oop/generics.bas'

' ============================================================================
' Collections
' ============================================================================
'$INCLUDE:'collections/list.bas'
'$INCLUDE:'collections/dictionary.bas'
'$INCLUDE:'collections/stack.bas'
'$INCLUDE:'collections/queue.bas'
'$INCLUDE:'collections/set.bas'

' ============================================================================
' String Utilities
' ============================================================================
'$INCLUDE:'strings/strbuilder.bas'
'$INCLUDE:'strings/encoding.bas'
'$INCLUDE:'strings/regex.bas'

' ============================================================================
' Math
' ============================================================================
'$INCLUDE:'math/vector.bas'
'$INCLUDE:'math/matrix.bas'
'$INCLUDE:'math/stats.bas'

' ============================================================================
' I/O
' ============================================================================
'$INCLUDE:'io/path.bas'
'$INCLUDE:'io/csv.bas'
'$INCLUDE:'io/json.bas'

' ============================================================================
' DateTime
' ============================================================================
'$INCLUDE:'datetime/datetime.bas'

' ============================================================================
' Error Handling
' ============================================================================
'$INCLUDE:'error/error.bas'

' ============================================================================
' System Integration
' ============================================================================
'$INCLUDE:'sys/env.bas'
'$INCLUDE:'sys/process.bas'
'$INCLUDE:'sys/args.bas'

' ============================================================================
' Library Information
' ============================================================================

CONST QBNEX_STDLIB_VERSION = "1.0.0"
CONST QBNEX_STDLIB_BUILD = "2026.04.12"

' ============================================================================
' FUNCTION: QBNex_StdLib_Version
' Get library version string
' ============================================================================
FUNCTION QBNex_StdLib_Version$ ()
    QBNex_StdLib_Version = QBNEX_STDLIB_VERSION
END FUNCTION

' ============================================================================
' FUNCTION: QBNex_StdLib_Info
' Get library information
' ============================================================================
FUNCTION QBNex_StdLib_Info$ ()
    DIM info AS STRING
    info = "QBNex Standard Library v" + QBNEX_STDLIB_VERSION + CHR$(13) + CHR$(10)
    info = info + "Build: " + QBNEX_STDLIB_BUILD + CHR$(13) + CHR$(10)
    info = info + "Complete Standard Library Ecosystem for QBNex BASIC" + CHR$(13) + CHR$(10)
    info = info + CHR$(13) + CHR$(10)
    info = info + "Modules:" + CHR$(13) + CHR$(10)
    info = info + "  - OOP Foundation (Classes, Interfaces, Generics)" + CHR$(13) + CHR$(10)
    info = info + "  - Collections (List, Dictionary, Stack, Queue, Set)" + CHR$(13) + CHR$(10)
    info = info + "  - String Utilities (StringBuilder, Encoding, Regex)" + CHR$(13) + CHR$(10)
    info = info + "  - Math (Vector, Matrix, Statistics)" + CHR$(13) + CHR$(10)
    info = info + "  - I/O (Path, CSV, JSON)" + CHR$(13) + CHR$(10)
    info = info + "  - DateTime (Arithmetic, Formatting)" + CHR$(13) + CHR$(10)
    info = info + "  - Error Handling (Structured errors, assertions)" + CHR$(13) + CHR$(10)
    info = info + "  - System Integration (Environment, Process, Arguments)" + CHR$(13) + CHR$(10)
    
    QBNex_StdLib_Info = info
END FUNCTION

' ============================================================================
' SUB: QBNex_StdLib_PrintInfo
' Print library information to console
' ============================================================================
SUB QBNex_StdLib_PrintInfo ()
    PRINT QBNex_StdLib_Info
END SUB
