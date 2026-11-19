' Shared compiler state declarations that must remain near the top-level include
' area because QB64 requires DIM/TYPE/CONST statements before later SUB/FUNCTION
' implementations.

' Startup/platform state
DIM SHARED OS_BITS AS LONG, WindowTitle AS STRING
DIM SHARED os AS STRING
DIM SHARED MacOSX AS LONG
DIM SHARED pathsep AS STRING

' CLI/build-output state
DIM SHARED ConsoleMode
DIM SHARED ShowWarnings AS _BYTE, QuietMode AS _BYTE, CMDLineFile AS STRING
DIM SHARED MonochromeLoggingMode AS _BYTE
DIM SHARED outputfile_cmd$
'--run: start the executable after a successful build
DIM SHARED RunAfterBuild AS _BYTE
'the arguments after --, quoted for the shell, for the program --run starts
DIM SHARED RunArgs$
'the same arguments as typed, for the "Running" line
DIM SHARED RunArgsShown$
DIM SHARED compilerBannerShown AS _BYTE
DIM SHARED compilerProgressVisible AS _BYTE
DIM SHARED compilerProgressLastPercentage AS LONG
DIM SHARED compilerStartTime AS DOUBLE
DIM SHARED tuiVT AS _BYTE
DIM SHARED compfailed
DIM SHARED extension AS STRING
DIM SHARED path.exe$, path.source$, pendingOutputBinary$
DIM SHARED AutoConsoleOnlyEligible AS _BYTE, AutoConsoleOnlyActive AS _BYTE

' Folder listings read by ResolvePathCase$ (file.bas), CHR$(0) + folder + CHR$(1) + names
DIM SHARED pathCaseListings AS STRING

' Temporary workspace state
DIM SHARED tmpdir AS STRING
DIM SHARED tempfolderindex
DIM SHARED tempfolderindexstr AS STRING
