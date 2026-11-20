' Shared compiler state declarations that must remain near the top-level include
' area because QB64 requires DIM/TYPE/CONST statements before later SUB/FUNCTION
' implementations.

' Startup/platform state
DIM SHARED OS_BITS AS LONG, WindowTitle AS STRING
DIM SHARED os AS STRING
DIM SHARED MacOSX AS LONG
DIM SHARED pathsep AS STRING * 1

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

' Token index (utilities/tokens.bas): a line of elements separated by sp is kept as the
' positions of its elements, for the four lines used last (round robin), so that
' getelement$, numelements, getelements$ and elementpos find an element without
' scanning the line. Slot s (0-3) holds the text tkSrc<s+1>, its element count
' tkCnt(s) and the first character of element k at tkPos(s * tkStride + k)
' (tkPos(s * tkStride + count + 1) = LEN + 2, one past the end).
DIM SHARED tkSrc1 AS STRING, tkSrc2 AS STRING, tkSrc3 AS STRING, tkSrc4 AS STRING
DIM SHARED tkCnt(0 TO 3) AS LONG, tkNext AS LONG, tkStride AS LONG
tkStride = 256
REDIM SHARED tkPos(0 TO 4 * tkStride - 1) AS LONG
