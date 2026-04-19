SUB PrepareDependencyBuildInputs (defines$, libs$, libqb$, o$, win, lnx, mac)
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
    DIM rebuildLibqb AS _BYTE

    o$ = LCASE$(os$)
    win = 0: IF os$ = "WIN" THEN win = 1
    lnx = 0: IF os$ = "LNX" THEN lnx = 1
    mac = 0: IF MacOSX THEN mac = 1: o$ = "osx"
    defines$ = ""
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

    rebuildLibqb = 0
    IF _FILEEXISTS(libqbObjectPath) = 0 THEN
        rebuildLibqb = -1
    ELSE
        rebuildLibqb = -1
    END IF

    IF rebuildLibqb THEN
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
    END IF

    IF DEPENDENCY(DEPENDENCY_AUDIO_OUT) THEN
        IF mac THEN defines$ = defines$ + " -framework AudioUnit -framework AudioToolbox "
    END IF
END SUB
