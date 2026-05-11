FUNCTION RequiresGuiCore% (symbolName AS STRING)
    DIM upperName AS STRING

    upperName = UCASE$(RTRIM$(symbolName))

    SELECT CASE upperName
    CASE "SCREEN", "LINE", "DRAW", "PSET", "PRESET", "CIRCLE", "PAINT", "VIEW", "WINDOW", "PCOPY", "POINT", "PMAP", "PALETTE", "CLS", "LOCATE"
        RequiresGuiCore% = -1
        EXIT FUNCTION
    CASE "_GL", "_GLRENDER", "_DISPLAYORDER", "_MAPTRIANGLE", "_DEPTHBUFFER"
        RequiresGuiCore% = -1
        EXIT FUNCTION
    END SELECT

    IF LEFT$(upperName, 7) = "_SCREEN" THEN
        RequiresGuiCore% = -1
        EXIT FUNCTION
    END IF

    IF LEFT$(upperName, 6) = "_MOUSE" THEN
        RequiresGuiCore% = -1
        EXIT FUNCTION
    END IF

    IF LEFT$(upperName, 11) = "_FULLSCREEN" THEN
        RequiresGuiCore% = -1
        EXIT FUNCTION
    END IF
END FUNCTION

FUNCTION DependencyFileFingerprint$ (filePath AS STRING)
    DIM fh AS LONG
    DIM fileLength AS _INTEGER64
    DIM remaining AS _INTEGER64
    DIM chunkSize AS LONG
    DIM chunkData AS STRING
    DIM h1 AS _UNSIGNED LONG
    DIM h2 AS _UNSIGNED LONG

    IF _FILEEXISTS(filePath) = 0 THEN EXIT FUNCTION

    h1 = &H811C9DC5
    h2 = &H9E3779B9

    fh = FREEFILE
    OPEN filePath FOR BINARY AS #fh
    fileLength = LOF(fh)
    remaining = fileLength

    DO WHILE remaining > 0
        chunkSize = 4096
        IF remaining < chunkSize THEN chunkSize = remaining
        chunkData = SPACE$(chunkSize)
        GET #fh, , chunkData
        FOR i = 1 TO chunkSize
            b = ASC(chunkData, i)
            h1 = (h1 XOR b) * &H1000193
            h2 = ((h2 XOR b) + &H9E3779B9) * 33
        NEXT
        remaining = remaining - chunkSize
    LOOP

    CLOSE #fh
    DependencyFileFingerprint$ = HEX$(h1) + "_" + HEX$(h2) + "_" + str2$(fileLength)
END FUNCTION

FUNCTION LibqbBuildSignature$ (depstr AS STRING, defines AS STRING, sourcePath AS STRING, headerPath AS STRING, commonPath AS STRING)
    DIM signature AS STRING

    signature = "libqb-cache-v2|" + depstr + "|" + defines
    signature = signature + "|" + DependencyFileFingerprint$(sourcePath)
    signature = signature + "|" + DependencyFileFingerprint$("internal\c\libqb.mm")
    signature = signature + "|" + DependencyFileFingerprint$(headerPath)
    signature = signature + "|" + DependencyFileFingerprint$(commonPath)

    LibqbBuildSignature$ = signature
END FUNCTION

FUNCTION ReadWholeTextFile$ (filePath AS STRING)
    DIM fh AS LONG
    DIM textValue AS STRING

    IF _FILEEXISTS(filePath) = 0 THEN EXIT FUNCTION

    fh = FREEFILE
    OPEN filePath FOR BINARY AS #fh
    textValue = SPACE$(LOF(fh))
    IF LEN(textValue) THEN GET #fh, , textValue
    CLOSE #fh

    ReadWholeTextFile$ = textValue
END FUNCTION

FUNCTION StripTrailingLineBreaks$ (textValue AS STRING)
    DO WHILE LEN(textValue) > 0
        IF RIGHT$(textValue, 1) <> CHR$(10) AND RIGHT$(textValue, 1) <> CHR$(13) THEN EXIT DO
        textValue = LEFT$(textValue, LEN(textValue) - 1)
    LOOP

    StripTrailingLineBreaks$ = textValue
END FUNCTION

FUNCTION LibqbObjectDefinesProgramEntry% (objectPath AS STRING)
    DIM nmOutputPath AS STRING
    DIM nmCommand AS STRING
    DIM fh AS LONG
    DIM symbolLine AS STRING

    IF _FILEEXISTS(objectPath) = 0 THEN EXIT FUNCTION

    nmOutputPath = tmpdir$ + "libqb_symbols.txt"
    IF os$ = "WIN" THEN
        nmCommand = "cmd.exe /c internal\c\c_compiler\bin\nm.exe " + QuotedFilename$(objectPath) + " >" + QuotedFilename$(nmOutputPath)
    ELSE
        nmCommand = "nm " + QuotedFilename$(objectPath) + " >" + QuotedFilename$(nmOutputPath) + " 2>" + QuotedFilename$(tmpdir$ + "libqb_symbols_error.txt")
    END IF
    SHELL _HIDE nmCommand

    IF _FILEEXISTS(nmOutputPath) = 0 THEN EXIT FUNCTION

    fh = FREEFILE
    OPEN nmOutputPath FOR INPUT AS #fh
    DO UNTIL EOF(fh)
        LINE INPUT #fh, symbolLine
        IF INSTR(symbolLine, " T _Z6QBMAIN") OR INSTR(symbolLine, " T QBMAIN") OR INSTR(symbolLine, " T _QBMAIN") THEN
            LibqbObjectDefinesProgramEntry% = -1
            EXIT DO
        END IF
    LOOP
    CLOSE #fh
END FUNCTION

SUB WriteLibqbBuildSignature (signaturePath AS STRING, signature AS STRING)
    DIM fh AS LONG

    fh = FREEFILE
    OPEN signaturePath FOR OUTPUT AS #fh
    PRINT #fh, signature
    CLOSE #fh
END SUB

SUB EnsureWindowsCommonPCH (depstr$, defines$, pchOptions$)
    DIM pchRoot$
    DIM pchPlatformRoot$
    DIM pchHeaderRel$
    DIM pchHeaderPath$
    DIM pchBinaryRel$
    DIM pchBinaryPath$
    DIM ffh AS LONG

    pchOptions$ = ""
    IF os$ <> "WIN" THEN EXIT SUB

    pchRoot$ = "internal\c\pch"
    pchPlatformRoot$ = pchRoot$ + "\win"
    IF _DIREXISTS(pchRoot$) = 0 THEN MKDIR pchRoot$
    IF _DIREXISTS(pchPlatformRoot$) = 0 THEN MKDIR pchPlatformRoot$

    pchHeaderRel$ = "pch\win\common_" + depstr$ + ".h"
    pchHeaderPath$ = "internal\c\" + pchHeaderRel$
    pchBinaryRel$ = pchHeaderRel$ + ".gch"
    pchBinaryPath$ = "internal\c\" + pchBinaryRel$

    IF _FILEEXISTS(pchHeaderPath$) = 0 THEN
        ffh = FREEFILE
        OPEN pchHeaderPath$ FOR OUTPUT AS #ffh
        PRINT #ffh, "#include " + CHR$(34) + "..\..\common.h" + CHR$(34)
        CLOSE #ffh
    END IF

    IF _FILEEXISTS(pchBinaryPath$) = 0 THEN
        CHDIR "internal\c"
        SHELL _HIDE GDB_Fix("cmd /c c_compiler\bin\g++ -x c++-header -w -Wall " + defines$ + " " + pchHeaderRel$ + " -o " + pchBinaryRel$) + " 2>> ..\..\" + compilelog$
        CHDIR "..\.."
    END IF

    pchOptions$ = " -Winvalid-pch -include " + pchHeaderRel$ + " "
END SUB

SUB PrepareDependencyBuildInputs (defines$, libs$, libqb$, pchOptions$, o$, win, lnx, mac)
    DIM defines_header$
    DIM ver$
    DIM depstr$
    DIM localpath$
    DIM libname$
    DIM libpath$
    DIM libfile$
    DIM d$
    DIM d1$
    DIM d2$
    DIM d3$
    DIM libqbObjectPath AS STRING
    DIM libqbSourcePath AS STRING
    DIM libqbHeaderPath AS STRING
    DIM commonHeaderPath AS STRING
    DIM libqbSignaturePath AS STRING
    DIM libqbSignature AS STRING
    DIM storedLibqbSignature AS STRING
    DIM rebuildLibqb AS _BYTE

    o$ = LCASE$(os$)
    win = 0: IF os$ = "WIN" THEN win = 1
    lnx = 0: IF os$ = "LNX" THEN lnx = 1
    mac = 0: IF MacOSX THEN mac = 1: o$ = "osx"
    defines$ = ""
    pchOptions$ = ""
    defines_header$ = " -D "
    ver$ = Version$
    x = INSTR(ver$, "."): IF x THEN ASC(ver$, x) = 95
    libs$ = ""

    IF DEPENDENCY(DEPENDENCY_GL) THEN
        defines$ = defines$ + defines_header$ + "DEPENDENCY_GL"
    END IF

    IF DEPENDENCY(DEPENDENCY_SCREENIMAGE) THEN
        DEPENDENCY(DEPENDENCY_IMAGE_CODEC) = 1
    END IF

    IF DEPENDENCY(DEPENDENCY_IMAGE_CODEC) THEN
        defines$ = defines$ + defines_header$ + "DEPENDENCY_IMAGE_CODEC"
    END IF

    IF DEPENDENCY(DEPENDENCY_CONSOLE_ONLY) THEN
        defines$ = defines$ + defines_header$ + "DEPENDENCY_CONSOLE_ONLY"
    END IF

    IF DEPENDENCY(DEPENDENCY_SOCKETS) THEN
        defines$ = defines$ + defines_header$ + "DEPENDENCY_SOCKETS"
    ELSE
        defines$ = defines$ + defines_header$ + "DEPENDENCY_NO_SOCKETS"
    END IF

    IF DEPENDENCY(DEPENDENCY_PRINTER) THEN
        defines$ = defines$ + defines_header$ + "DEPENDENCY_PRINTER"
    ELSE
        defines$ = defines$ + defines_header$ + "DEPENDENCY_NO_PRINTER"
    END IF

    IF DEPENDENCY(DEPENDENCY_ICON) THEN
        defines$ = defines$ + defines_header$ + "DEPENDENCY_ICON"
    ELSE
        defines$ = defines$ + defines_header$ + "DEPENDENCY_NO_ICON"
    END IF

    IF DEPENDENCY(DEPENDENCY_SCREENIMAGE) THEN
        defines$ = defines$ + defines_header$ + "DEPENDENCY_SCREENIMAGE"
    ELSE
        defines$ = defines$ + defines_header$ + "DEPENDENCY_NO_SCREENIMAGE"
    END IF

    IF DEPENDENCY(DEPENDENCY_LOADFONT) THEN
        d$ = "internal\c\parts\video\font\ttf\"
        IF _FILEEXISTS(d$ + "os\" + o$ + "\src.o") = 0 THEN Build d$ + "os\" + o$
        defines$ = defines$ + defines_header$ + "DEPENDENCY_LOADFONT"
        libs$ = libs$ + " " + "parts\video\font\ttf\os\" + o$ + "\src.o"
    END IF

    localpath$ = "internal\c\"

    IF DEPENDENCY(DEPENDENCY_DEVICEINPUT) THEN
        defines$ = defines$ + defines_header$ + "DEPENDENCY_DEVICEINPUT"
        libname$ = "input\game_controller"
        libpath$ = "parts\" + libname$ + "\os\" + o$
        libfile$ = libpath$ + "\src.a"
        IF _FILEEXISTS(localpath$ + libfile$) = 0 THEN Build localpath$ + libpath$
        libs$ = libs$ + " " + libfile$
    END IF

    IF DEPENDENCY(DEPENDENCY_AUDIO_DECODE) THEN DEPENDENCY(DEPENDENCY_AUDIO_CONVERSION) = 1
    IF DEPENDENCY(DEPENDENCY_AUDIO_CONVERSION) THEN DEPENDENCY(DEPENDENCY_AUDIO_OUT) = 1
    IF DEPENDENCY(DEPENDENCY_AUDIO_DECODE) THEN DEPENDENCY(DEPENDENCY_AUDIO_OUT) = 1

    IF DEPENDENCY(DEPENDENCY_AUDIO_CONVERSION) THEN
        defines$ = defines$ + defines_header$ + "DEPENDENCY_AUDIO_CONVERSION"
        d1$ = "parts\audio\conversion"
        d2$ = d1$ + "\os\" + o$
        d3$ = "internal\c\" + d2$
        IF _FILEEXISTS(d3$ + "\src.a") = 0 THEN Build d3$
        libs$ = libs$ + " " + d2$ + "\src.a"
    END IF

    IF DEPENDENCY(DEPENDENCY_AUDIO_DECODE) THEN
        defines$ = defines$ + defines_header$ + "DEPENDENCY_AUDIO_DECODE"

        d1$ = "parts\audio\decode\mp3_mini"
        d2$ = d1$ + "\os\" + o$
        d3$ = "internal\c\" + d2$
        IF _FILEEXISTS(d3$ + "\src.a") = 0 THEN Build d3$
        libs$ = libs$ + " " + d2$ + "\src.a"

        d1$ = "parts\audio\decode\ogg"
        d2$ = d1$ + "\os\" + o$
        d3$ = "internal\c\" + d2$
        IF _FILEEXISTS(d3$ + "\src.o") = 0 THEN Build d3$
        libs$ = libs$ + " " + d2$ + "\src.o"
    END IF

    IF DEPENDENCY(DEPENDENCY_AUDIO_OUT) THEN
        defines$ = defines$ + defines_header$ + "DEPENDENCY_AUDIO_OUT"
        d1$ = "parts\audio\out"
        d2$ = d1$ + "\os\" + o$
        d3$ = "internal\c\" + d2$
        IF _FILEEXISTS(d3$ + "\src.a") = 0 THEN Build d3$
        libs$ = libs$ + " " + d2$ + "\src.a"
    END IF

    IF DEPENDENCY(DEPENDENCY_ZLIB) THEN
        defines$ = defines$ + defines_header$ + "DEPENDENCY_ZLIB"
        IF MacOSX THEN
            libs$ = libs$ + " -lz"
        ELSE
            libs$ = libs$ + " -l:libz.a"
        END IF
    END IF

    IF LEN(libs$) THEN libs$ = libs$ + " "
    PATH_SLASH_CORRECT libs$
    IF LEN(defines$) THEN defines$ = defines$ + " "

    IF mac = 0 THEN
        d1$ = "parts\core"
        d2$ = d1$ + "\os\" + o$
        d3$ = "internal\c\" + d2$
        IF _FILEEXISTS(d3$ + "\src.a") = 0 THEN Build d3$
    END IF

    depstr$ = ver$ + "_"
    FOR i = 1 TO DEPENDENCY_LAST
        IF DEPENDENCY(i) THEN depstr$ = depstr$ + "1" ELSE depstr$ = depstr$ + "0"
    NEXT
    libqb$ = " libqb\os\" + o$ + "\libqb_" + depstr$ + ".o "
    PATH_SLASH_CORRECT libqb$

    libqbObjectPath = "internal\c\" + LTRIM$(RTRIM$(libqb$))
    libqbSourcePath = "internal\c\libqb.cpp"
    libqbHeaderPath = "internal\c\libqb.h"
    commonHeaderPath = "internal\c\common.h"
    libqbSignaturePath = libqbObjectPath + ".sig"
    libqbSignature = LibqbBuildSignature$(depstr$, defines$, libqbSourcePath, libqbHeaderPath, commonHeaderPath)

    rebuildLibqb = 0
    IF _FILEEXISTS(libqbObjectPath) = 0 THEN
        rebuildLibqb = -1
    ELSEIF _FILEEXISTS(libqbSignaturePath) = 0 THEN
        rebuildLibqb = -1
    ELSE
        storedLibqbSignature = StripTrailingLineBreaks$(ReadWholeTextFile$(libqbSignaturePath))
        IF storedLibqbSignature <> libqbSignature THEN
            rebuildLibqb = -1
        ELSEIF LibqbObjectDefinesProgramEntry%(libqbObjectPath) THEN
            rebuildLibqb = -1
        END IF
    END IF

    IF rebuildLibqb THEN
        IF _FILEEXISTS(libqbObjectPath) THEN KILL libqbObjectPath
        IF _FILEEXISTS(libqbSignaturePath) THEN KILL libqbSignaturePath
        CHDIR "internal\c"
        IF os$ = "WIN" THEN
            SHELL _HIDE GDB_Fix("cmd /c c_compiler\bin\g++ -c -s -w -Wall libqb.cpp -D FREEGLUT_STATIC " + defines$ + " -o libqb\os\" + o$ + "\libqb_" + depstr$ + ".o") + " 2>> ..\..\" + compilelog$
        ELSE
            IF mac THEN
                SHELL _HIDE GDB_Fix("g++ -c -s -w -Wall libqb.mm " + defines$ + " -o libqb/os/" + o$ + "/libqb_" + depstr$ + ".o") + " 2>> ../../" + compilelog$
            ELSE
                SHELL _HIDE GDB_Fix("g++ -c -s -w -Wall libqb.cpp -D FREEGLUT_STATIC " + defines$ + " -o libqb/os/" + o$ + "/libqb_" + depstr$ + ".o") + " 2>> ../../" + compilelog$
            END IF
        END IF
        CHDIR "..\.."
        IF _FILEEXISTS(libqbObjectPath) THEN
            IF LibqbObjectDefinesProgramEntry%(libqbObjectPath) = 0 THEN
                WriteLibqbBuildSignature libqbSignaturePath, libqbSignature
            END IF
        END IF
    END IF

    IF DEPENDENCY(DEPENDENCY_AUDIO_OUT) THEN
        IF mac THEN defines$ = defines$ + " -framework AudioUnit -framework AudioToolbox "
    END IF

    EnsureWindowsCommonPCH depstr$, defines$, pchOptions$
END SUB
