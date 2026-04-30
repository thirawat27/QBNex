' Shared compiler state declarations that must remain near the top-level include
' area because QB64 requires DIM/TYPE/CONST statements before later SUB/FUNCTION
' implementations.

' Startup/platform state
DIM SHARED OS_BITS AS LONG, WindowTitle AS STRING
DIM SHARED os AS STRING
DIM SHARED MacOSX AS LONG
DIM SHARED inline_DATA
DIM SHARED BATCHFILE_EXTENSION AS STRING
DIM SHARED pathsep AS STRING * 1

' CLI/build-output state
DIM SHARED ConsoleMode, No_C_Compile_Mode
DIM SHARED ShowWarnings AS _BYTE, QuietMode AS _BYTE, CMDLineFile AS STRING
DIM SHARED MonochromeLoggingMode AS _BYTE
DIM SHARED outputfile_cmd$
DIM SHARED compilelog$
DIM SHARED compilerBannerShown AS _BYTE
DIM SHARED compilerProgressRow AS LONG
DIM SHARED compilerProgressVisible AS _BYTE
DIM SHARED compilerProgressLastLength AS LONG
DIM SHARED compfailed
DIM SHARED extension AS STRING
DIM SHARED path.exe$, path.source$, lastBinaryGenerated$, pendingOutputBinary$
DIM SHARED AutoConsoleOnlyEligible AS _BYTE, AutoConsoleOnlyActive AS _BYTE

' Temporary workspace state
DIM SHARED tmpdir AS STRING, tmpdir2 AS STRING
DIM SHARED tempfolderindex
DIM SHARED tempfolderindexstr AS STRING
DIM SHARED tempfolderindexstr2 AS STRING
