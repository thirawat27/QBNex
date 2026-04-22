FUNCTION GetCachedNmOutputPath$ (filePath AS STRING, dynamicSymbols AS LONG)
    CONST NM_CACHE_SLOTS = 32
    STATIC cacheKey(1 TO NM_CACHE_SLOTS) AS STRING
    STATIC cachePath(1 TO NM_CACHE_SLOTS) AS STRING
    STATIC nextSlot AS LONG

    cacheId$ = os$ + "|" + str2$(MacOSX) + "|" + str2$(dynamicSymbols) + "|" + filePath
    FOR i = 1 TO NM_CACHE_SLOTS
        IF cacheKey(i) = cacheId$ THEN
            GetCachedNmOutputPath$ = cachePath(i)
            EXIT FUNCTION
        END IF
    NEXT

    nextSlot = nextSlot + 1
    IF nextSlot > NM_CACHE_SLOTS THEN nextSlot = 1

    cacheFile$ = tmpdir$ + "nm_cache_" + str2$(nextSlot) + ".txt"
    cacheKey(nextSlot) = cacheId$
    cachePath(nextSlot) = cacheFile$

    IF os$ = "WIN" THEN
        nmCommand$ = "cmd.exe /c internal\c\c_compiler\bin\nm.exe " + CHR$(34) + filePath + CHR$(34)
        IF dynamicSymbols THEN nmCommand$ = nmCommand$ + " -D"
        nmCommand$ = nmCommand$ + " --demangle -g >" + QuotedFilename$(cacheFile$)
        SHELL _HIDE nmCommand$
    ELSEIF os$ = "LNX" THEN
        nmCommand$ = "nm " + CHR$(34) + filePath + CHR$(34)
        IF dynamicSymbols AND MacOSX = 0 THEN nmCommand$ = nmCommand$ + " -D"
        IF MacOSX = 0 THEN nmCommand$ = nmCommand$ + " --demangle -g"
        nmCommand$ = nmCommand$ + " >" + QuotedFilename$(cacheFile$) + " 2>" + QuotedFilename$(tmpdir$ + "nm_error.txt")
        SHELL _HIDE nmCommand$
    END IF

    GetCachedNmOutputPath$ = cacheFile$
END FUNCTION

FUNCTION ResolveBuildLinkSymbols%
    IF os$ = "WIN" THEN
        FOR x = 1 TO ResolveStaticFunctions
            IF LEN(ResolveStaticFunction_File$(x)) THEN
                n = 0
                nmOutputPath$ = GetCachedNmOutputPath$(ResolveStaticFunction_File$(x), 0)
                fh = FREEFILE
                s$ = " " + ResolveStaticFunction_Name$(x) + "("
                OPEN nmOutputPath$ FOR BINARY AS #fh
                DO UNTIL EOF(fh)
                    LINE INPUT #fh, a$
                    IF LEN(a$) THEN
                        x1 = INSTR(a$, s$)
                        IF x1 THEN
                            IF ResolveStaticFunction_Method(x) = 1 THEN
                                x1 = x1 + 1
                                x2 = INSTR(x1, a$, ")")
                                fh2 = FREEFILE
                                OPEN tmpdir$ + "global.txt" FOR APPEND AS #fh2
                                PRINT #fh2, "extern void " + MID$(a$, x1, x2 - x1 + 1) + ";"
                                CLOSE #fh2
                            END IF
                            n = n + 1
                        END IF
                    END IF
                LOOP
                CLOSE #fh
                IF n > 1 THEN
                    a$ = "Unable to resolve multiple instances of sub/function '" + ResolveStaticFunction_Name$(x) + "' in '" + ResolveStaticFunction_File$(x) + "'"
                    ResolveBuildLinkSymbols% = -1
                    EXIT FUNCTION
                END IF

                IF n = 0 THEN
                    fh = FREEFILE
                    s$ = " " + ResolveStaticFunction_Name$(x)
                    OPEN nmOutputPath$ FOR BINARY AS #fh
                    DO UNTIL EOF(fh)
                        LINE INPUT #fh, a$
                        IF LEN(a$) THEN
                            x1 = INSTR(a$, s$)
                            IF RIGHT$(a$, LEN(s$)) = s$ THEN
                                fh2 = FREEFILE
                                IF ResolveStaticFunction_Method(x) = 1 THEN
                                    OPEN tmpdir$ + "global.txt" FOR APPEND AS #fh2
                                    PRINT #fh2, "extern " + CHR$(34) + "C" + CHR$(34) + "{"
                                    PRINT #fh2, "extern void " + s$ + "(void);"
                                    PRINT #fh2, "}"
                                ELSE
                                    OPEN tmpdir$ + "externtype" + str2(x) + ".txt" FOR OUTPUT AS #fh2
                                    PRINT #fh2, "extern " + CHR$(34) + "C" + CHR$(34) + " "
                                END IF
                                CLOSE #fh2
                                n = n + 1
                                EXIT DO
                            END IF
                        END IF
                    LOOP
                    CLOSE #fh
                END IF

                IF n = 0 THEN
                    nmOutputDynamicPath$ = GetCachedNmOutputPath$(ResolveStaticFunction_File$(x), -1)
                    fh = FREEFILE
                    s$ = " " + ResolveStaticFunction_Name$(x) + "("
                    OPEN nmOutputDynamicPath$ FOR BINARY AS #fh
                    DO UNTIL EOF(fh)
                        LINE INPUT #fh, a$
                        IF LEN(a$) THEN
                            x1 = INSTR(a$, s$)
                            IF x1 THEN
                                IF ResolveStaticFunction_Method(x) = 1 THEN
                                    x1 = x1 + 1
                                    x2 = INSTR(x1, a$, ")")
                                    fh2 = FREEFILE
                                    OPEN tmpdir$ + "global.txt" FOR APPEND AS #fh2
                                    PRINT #fh2, "extern void " + MID$(a$, x1, x2 - x1 + 1) + ";"
                                    CLOSE #fh2
                                END IF
                                n = n + 1
                            END IF
                        END IF
                    LOOP
                    CLOSE #fh
                    IF n > 1 THEN
                        a$ = "Unable to resolve multiple instances of sub/function '" + ResolveStaticFunction_Name$(x) + "' in '" + ResolveStaticFunction_File$(x) + "'"
                        ResolveBuildLinkSymbols% = -1
                        EXIT FUNCTION
                    END IF
                END IF

                IF n = 0 THEN
                    fh = FREEFILE
                    s$ = " " + ResolveStaticFunction_Name$(x)
                    OPEN nmOutputDynamicPath$ FOR BINARY AS #fh
                    DO UNTIL EOF(fh)
                        LINE INPUT #fh, a$
                        IF LEN(a$) THEN
                            x1 = INSTR(a$, s$)
                            IF RIGHT$(a$, LEN(s$)) = s$ THEN
                                fh2 = FREEFILE
                                IF ResolveStaticFunction_Method(x) = 1 THEN
                                    OPEN tmpdir$ + "global.txt" FOR APPEND AS #fh2
                                    PRINT #fh2, "extern " + CHR$(34) + "C" + CHR$(34) + "{"
                                    PRINT #fh2, "extern void " + s$ + "(void);"
                                    PRINT #fh2, "}"
                                ELSE
                                    OPEN tmpdir$ + "externtype" + str2(x) + ".txt" FOR OUTPUT AS #fh2
                                    PRINT #fh2, "extern " + CHR$(34) + "C" + CHR$(34) + " "
                                END IF
                                CLOSE #fh2
                                n = n + 1
                                EXIT DO
                            END IF
                        END IF
                    LOOP
                    CLOSE #fh
                    IF n = 0 THEN
                        a$ = "Could not find sub/function '" + ResolveStaticFunction_Name$(x) + "' in '" + ResolveStaticFunction_File$(x) + "'"
                        ResolveBuildLinkSymbols% = -1
                        EXIT FUNCTION
                    END IF
                END IF
            END IF
        NEXT
        EXIT FUNCTION
    END IF

    IF os$ <> "LNX" THEN EXIT FUNCTION

    FOR x = 1 TO ResolveStaticFunctions
        IF LEN(ResolveStaticFunction_File$(x)) THEN
            n = 0
            nmOutputPath$ = GetCachedNmOutputPath$(ResolveStaticFunction_File$(x), 0)

            IF MacOSX = 0 THEN
                fh = FREEFILE
                s$ = " " + ResolveStaticFunction_Name$(x) + "("
                OPEN nmOutputPath$ FOR BINARY AS #fh
                DO UNTIL EOF(fh)
                    LINE INPUT #fh, a$
                    IF LEN(a$) THEN
                        x1 = INSTR(a$, s$)
                        IF x1 THEN
                            IF ResolveStaticFunction_Method(x) = 1 THEN
                                x1 = x1 + 1
                                x2 = INSTR(x1, a$, ")")
                                fh2 = FREEFILE
                                OPEN tmpdir$ + "global.txt" FOR APPEND AS #fh2
                                PRINT #fh2, "extern void " + MID$(a$, x1, x2 - x1 + 1) + ";"
                                CLOSE #fh2
                            END IF
                            n = n + 1
                        END IF
                    END IF
                LOOP
                CLOSE #fh
                IF n > 1 THEN
                    a$ = "Unable to resolve multiple instances of sub/function '" + ResolveStaticFunction_Name$(x) + "' in '" + ResolveStaticFunction_File$(x) + "'"
                    ResolveBuildLinkSymbols% = -1
                    EXIT FUNCTION
                END IF
            END IF

            IF n = 0 THEN
                fh = FREEFILE
                s$ = " " + ResolveStaticFunction_Name$(x)
                s2$ = s$
                IF MacOSX THEN s$ = " _" + ResolveStaticFunction_Name$(x)
                OPEN nmOutputPath$ FOR BINARY AS #fh
                DO UNTIL EOF(fh)
                    LINE INPUT #fh, a$
                    IF LEN(a$) THEN
                        x1 = INSTR(a$, s$)
                        IF RIGHT$(a$, LEN(s$)) = s$ THEN
                            fh2 = FREEFILE
                            IF ResolveStaticFunction_Method(x) = 1 THEN
                                OPEN tmpdir$ + "global.txt" FOR APPEND AS #fh2
                                PRINT #fh2, "extern " + CHR$(34) + "C" + CHR$(34) + "{"
                                PRINT #fh2, "extern void " + s2$ + "(void);"
                                PRINT #fh2, "}"
                            ELSE
                                OPEN tmpdir$ + "externtype" + str2(x) + ".txt" FOR OUTPUT AS #fh2
                                PRINT #fh2, "extern " + CHR$(34) + "C" + CHR$(34) + " "
                            END IF
                            CLOSE #fh2
                            n = n + 1
                            EXIT DO
                        END IF
                    END IF
                LOOP
                CLOSE #fh
            END IF

            IF n = 0 THEN
                IF MacOSX = 0 THEN
                    nmOutputDynamicPath$ = GetCachedNmOutputPath$(ResolveStaticFunction_File$(x), -1)
                    fh = FREEFILE
                    s$ = " " + ResolveStaticFunction_Name$(x) + "("
                    OPEN nmOutputDynamicPath$ FOR BINARY AS #fh
                    DO UNTIL EOF(fh)
                        LINE INPUT #fh, a$
                        IF LEN(a$) THEN
                            x1 = INSTR(a$, s$)
                            IF x1 THEN
                                IF ResolveStaticFunction_Method(x) = 1 THEN
                                    x1 = x1 + 1
                                    x2 = INSTR(x1, a$, ")")
                                    fh2 = FREEFILE
                                    OPEN tmpdir$ + "global.txt" FOR APPEND AS #fh2
                                    PRINT #fh2, "extern void " + MID$(a$, x1, x2 - x1 + 1) + ";"
                                    CLOSE #fh2
                                END IF
                                n = n + 1
                            END IF
                        END IF
                    LOOP
                    CLOSE #fh
                    IF n > 1 THEN
                        a$ = "Unable to resolve multiple instances of sub/function '" + ResolveStaticFunction_Name$(x) + "' in '" + ResolveStaticFunction_File$(x) + "'"
                        ResolveBuildLinkSymbols% = -1
                        EXIT FUNCTION
                    END IF
                END IF
            END IF

            IF n = 0 AND MacOSX = 0 THEN
                fh = FREEFILE
                s$ = " " + ResolveStaticFunction_Name$(x)
                OPEN nmOutputDynamicPath$ FOR BINARY AS #fh
                DO UNTIL EOF(fh)
                    LINE INPUT #fh, a$
                    IF LEN(a$) THEN
                        x1 = INSTR(a$, s$)
                        IF RIGHT$(a$, LEN(s$)) = s$ THEN
                            fh2 = FREEFILE
                            IF ResolveStaticFunction_Method(x) = 1 THEN
                                OPEN tmpdir$ + "global.txt" FOR APPEND AS #fh2
                                PRINT #fh2, "extern " + CHR$(34) + "C" + CHR$(34) + "{"
                                PRINT #fh2, "extern void " + s$ + "(void);"
                                PRINT #fh2, "}"
                            ELSE
                                OPEN tmpdir$ + "externtype" + str2(x) + ".txt" FOR OUTPUT AS #fh2
                                PRINT #fh2, "extern " + CHR$(34) + "C" + CHR$(34) + " "
                            END IF
                            CLOSE #fh2
                            n = n + 1
                            EXIT DO
                        END IF
                    END IF
                LOOP
                CLOSE #fh
            END IF

            IF n = 0 THEN
                a$ = "Could not find sub/function '" + ResolveStaticFunction_Name$(x) + "' in '" + ResolveStaticFunction_File$(x) + "'"
                ResolveBuildLinkSymbols% = -1
                EXIT FUNCTION
            END IF
        END IF
    NEXT
END FUNCTION

SUB BuildProgramDataObject
    IF inline_DATA THEN EXIT SUB
    IF DataOffset = 0 THEN EXIT SUB

    IF os$ = "WIN" THEN
        IF OS_BITS = 32 THEN
            a$ = ReadCachedFirstLine$(".\internal\c\makedat_win32.txt")
        ELSE
            a$ = ReadCachedFirstLine$(".\internal\c\makedat_win64.txt")
        END IF
        a$ = a$ + " " + tmpdir2$ + "data.bin " + tmpdir2$ + "data.o"
        CHDIR ".\internal\c"
        SHELL _HIDE "cmd /c " + a$ + " 2>> ..\..\" + compilelog$
        CHDIR "..\.."
        EXIT SUB
    END IF

    IF os$ = "LNX" THEN
        SHELL _HIDE "ld --verbose >internal/temp/ld-output.txt"
        OPEN "internal/temp/ld-output.txt" FOR BINARY AS #150
        DO UNTIL EOF(150)
            LINE INPUT #150, a$
            IF LEN(a$) THEN
                s$ = "OUTPUT_FORMAT(" + CHR$(34)
                x1 = INSTR(a$, s$)
                IF x1 THEN
                    x1 = x1 + LEN(s$)
                    x2 = INSTR(x1, a$, CHR$(34))
                    format$ = MID$(a$, x1, x2 - x1)
                ELSE
                    s$ = "OUTPUT_ARCH("
                    x1 = INSTR(a$, s$)
                    IF x1 THEN
                        x1 = x1 + LEN(s$)
                        x2 = INSTR(x1, a$, ")")
                        architecture$ = MID$(a$, x1, x2 - x1)
                    END IF
                END IF
            END IF
        LOOP
        CLOSE #150
        a$ = "objcopy -Ibinary -O" + format$ + " -B" + architecture$ + " " + tmpdir2$ + "data.bin " + tmpdir2$ + "data.o"
        CHDIR ".\internal\c"
        SHELL _HIDE a$ + " 2>> ../../" + compilelog$
        CHDIR "..\.."
    END IF
END SUB

FUNCTION PrepareWindowsBuildCommand$ (file$, libqb$, libs$, defines$, pchOptions$)
    DIM targetOutputFile AS STRING

    a$ = GDB_Fix(ReadCachedFirstLine$(".\internal\c\makeline_win.txt"))

    IF RIGHT$(a$, 7) = " ..\..\" THEN a$ = LEFT$(a$, LEN(a$) - 6)
    x = INSTR(a$, "qbx.cpp")
    IF x <> 0 AND tempfolderindex <> 1 THEN a$ = LEFT$(a$, x - 1) + "qbx" + str2$(tempfolderindex) + ".cpp" + RIGHT$(a$, LEN(a$) - (x + 6))

    IF Console THEN
        x = INSTR(a$, " -s")
        a$ = LEFT$(a$, x - 1) + " -mconsole" + RIGHT$(a$, LEN(a$) - x + 1)
    END IF

    IF DEPENDENCY(DEPENDENCY_CONSOLE_ONLY) THEN
        a$ = StrRemove(a$, "-mwindows")
        a$ = StrRemove(a$, "-lopengl32")
        a$ = StrRemove(a$, "-lglu32")
        a$ = StrRemove(a$, "parts\core\os\win\src.a")
        a$ = StrRemove(a$, "-D FREEGLUT_STATIC")
        a$ = StrRemove(a$, "-D GLEW_STATIC")
    END IF

    a$ = StrRemove(a$, "-lws2_32")
    IF DEPENDENCY(DEPENDENCY_SOCKETS) THEN
        x = INSTR(a$, " -o")
        a$ = LEFT$(a$, x - 1) + " -lws2_32" + RIGHT$(a$, LEN(a$) - x + 1)
    END IF

    a$ = StrRemove(a$, "-lwinspool")
    IF DEPENDENCY(DEPENDENCY_PRINTER) THEN
        x = INSTR(a$, " -o")
        a$ = LEFT$(a$, x - 1) + " -lwinspool" + RIGHT$(a$, LEN(a$) - x + 1)
    END IF

    a$ = StrRemove(a$, "-lwinmm")
    IF DEPENDENCY(DEPENDENCY_AUDIO_OUT) <> 0 OR DEPENDENCY(DEPENDENCY_CONSOLE_ONLY) = 0 THEN
        x = INSTR(a$, " -o")
        a$ = LEFT$(a$, x - 1) + " -lwinmm" + RIGHT$(a$, LEN(a$) - x + 1)
    END IF

    a$ = StrRemove(a$, "-lksguid")
    IF DEPENDENCY(DEPENDENCY_AUDIO_OUT) THEN
        x = INSTR(a$, " -o")
        a$ = LEFT$(a$, x - 1) + " -lksguid" + RIGHT$(a$, LEN(a$) - x + 1)
    END IF

    a$ = StrRemove(a$, "-ldxguid")
    IF DEPENDENCY(DEPENDENCY_AUDIO_OUT) THEN
        x = INSTR(a$, " -o")
        a$ = LEFT$(a$, x - 1) + " -ldxguid" + RIGHT$(a$, LEN(a$) - x + 1)
    END IF

    a$ = StrRemove(a$, "-lole32")
    IF DEPENDENCY(DEPENDENCY_AUDIO_OUT) THEN
        x = INSTR(a$, " -o")
        a$ = LEFT$(a$, x - 1) + " -lole32" + RIGHT$(a$, LEN(a$) - x + 1)
    END IF

    a$ = StrRemove(a$, "-lgdi32")
    IF DEPENDENCY(DEPENDENCY_ICON) <> 0 OR DEPENDENCY(DEPENDENCY_SCREENIMAGE) <> 0 OR DEPENDENCY(DEPENDENCY_PRINTER) <> 0 THEN
        x = INSTR(a$, " -o")
        a$ = LEFT$(a$, x - 1) + " -lgdi32" + RIGHT$(a$, LEN(a$) - x + 1)
    END IF

    IF inline_DATA = 0 THEN
        IF DataOffset THEN
            x = INSTR(a$, ".cpp ")
            IF x THEN
                x = x + 3
                a$ = LEFT$(a$, x) + " " + tmpdir2$ + "data.o" + " " + RIGHT$(a$, LEN(a$) - x)
            END IF
        END IF
    END IF

    IF LEN(mylib$) THEN
        x = INSTR(a$, ".cpp ")
        IF x THEN
            x = x + 3
            a$ = LEFT$(a$, x) + " " + mylib$ + " " + RIGHT$(a$, LEN(a$) - x)
        END IF
    END IF

    IF LEN(libs$) THEN
        x = INSTR(a$, ".cpp ")
        IF x THEN
            x = x + 5
            a$ = LEFT$(a$, x - 1) + libs$ + RIGHT$(a$, LEN(a$) - x + 1)
        END IF
    END IF

    IF LEN(defines$) THEN
        x = INSTR(a$, ".cpp ")
        IF x THEN
            x = x + 5
            a$ = LEFT$(a$, x - 1) + defines$ + RIGHT$(a$, LEN(a$) - x + 1)
        END IF
    END IF

    IF LEN(pchOptions$) THEN
        x = INSTR(a$, ".cpp ")
        IF x THEN
            x = x + 5
            a$ = LEFT$(a$, x - 1) + pchOptions$ + RIGHT$(a$, LEN(a$) - x + 1)
        END IF
    END IF

    x = INSTR(a$, ".cpp ")
    IF x THEN
        x = x + 5
        a$ = LEFT$(a$, x - 1) + libqb$ + RIGHT$(a$, LEN(a$) - x + 1)
    END IF

    IF ExeIconSet OR VersionInfoSet THEN
        IF x THEN a$ = LEFT$(a$, x + LEN(libqb$)) + "..\..\" + tmpdir$ + "icon.o " + MID$(a$, x + LEN(libqb$) + 1)
    END IF

    targetOutputFile = pendingOutputBinary$
    IF LEN(targetOutputFile) = 0 THEN targetOutputFile = path.exe$ + file$ + extension$

    PrepareWindowsBuildCommand$ = a$ + QuotedFilename$(targetOutputFile)
END FUNCTION

FUNCTION PrepareUnixBuildCommand$ (file$, libqb$, libs$, defines$)
    DIM targetOutputFile AS STRING

    IF INSTR(_OS$, "[MACOSX]") THEN
        a$ = ReadCachedFirstLine$("./internal/c/makeline_osx.txt")
    ELSEIF DEPENDENCY(DEPENDENCY_CONSOLE_ONLY) THEN
        a$ = ReadCachedFirstLine$("./internal/c/makeline_lnx_nogui.txt")
    ELSE
        a$ = ReadCachedFirstLine$("./internal/c/makeline_lnx.txt")
    END IF
    a$ = GDB_Fix(a$)

    x = INSTR(a$, "qbx.cpp")
    IF x <> 0 AND tempfolderindex <> 1 THEN a$ = LEFT$(a$, x - 1) + "qbx" + str2$(tempfolderindex) + ".cpp" + RIGHT$(a$, LEN(a$) - (x + 6))

    IF inline_DATA = 0 THEN
        IF DataOffset THEN
            x = INSTR(a$, "-lrt")
            IF x THEN a$ = LEFT$(a$, x - 1) + " " + tmpdir2$ + "data.o " + RIGHT$(a$, LEN(a$) - x + 1)
        END IF
    END IF

    IF LEN(mylib$) THEN
        x = INSTR(a$, ".cpp ")
        IF x THEN
            x = x + 5
            a$ = LEFT$(a$, x - 1) + " " + mylibopt$ + " " + mylib$ + " " + RIGHT$(a$, LEN(a$) - x + 1)
        END IF
    END IF

    IF LEN(libs$) THEN
        x = INSTR(a$, ".cpp ")
        IF x THEN
            x = x + 5
            a$ = LEFT$(a$, x - 1) + libs$ + RIGHT$(a$, LEN(a$) - x + 1)
        END IF
    END IF

    IF LEN(defines$) THEN
        x = INSTR(a$, ".cpp ")
        IF x THEN
            x = x + 5
            a$ = LEFT$(a$, x - 1) + defines$ + RIGHT$(a$, LEN(a$) - x + 1)
        END IF
    END IF

    x = INSTR(a$, ".cpp ")
    IF x THEN
        x = x + 5
        a$ = LEFT$(a$, x - 1) + libqb$ + RIGHT$(a$, LEN(a$) - x + 1)
    END IF

    targetOutputFile = pendingOutputBinary$
    IF LEN(targetOutputFile) = 0 THEN targetOutputFile = path.exe$ + file$ + extension$

    PrepareUnixBuildCommand$ = a$ + QuotedFilename$(targetOutputFile)
END FUNCTION

FUNCTION RunNativeBuild% (file$, libqb$, libs$, defines$, pchOptions$)
    IF ResolveBuildLinkSymbols% THEN
        RunNativeBuild% = -1
        EXIT FUNCTION
    END IF

    BuildProgramDataObject

    IF os$ = "WIN" THEN
        a$ = PrepareWindowsBuildCommand$(file$, libqb$, libs$, defines$, pchOptions$)
        EmitBuildSupportScripts a$, file$
        ExecuteBuildCommand a$
        EXIT FUNCTION
    END IF

    IF os$ = "LNX" THEN
        a$ = PrepareUnixBuildCommand$(file$, libqb$, libs$, defines$)
        EmitBuildSupportScripts a$, file$
        ExecuteBuildCommand a$

        IF INSTR(_OS$, "[MACOSX]") THEN
            EmitMacOSLauncherScript file$
        END IF
    END IF
END FUNCTION

SUB EmitBuildSupportScripts (buildCommand$, file$)
    DIM debugTargetPath AS STRING

    debugTargetPath = ResolveOutputBinaryPath$(pendingOutputBinary$)
    IF LEN(debugTargetPath) = 0 THEN debugTargetPath = path.exe$ + file$ + extension$

    IF os$ = "WIN" THEN
        ffh = FREEFILE
        OPEN tmpdir$ + "recompile_win.bat" FOR OUTPUT AS #ffh
        PRINT #ffh, "@echo off"
        PRINT #ffh, "cd %0\..\"
        PRINT #ffh, "echo Recompiling..."
        PRINT #ffh, "cd ../c"
        PRINT #ffh, buildCommand$
        PRINT #ffh, "pause"
        CLOSE ffh

        ffh = FREEFILE
        OPEN tmpdir$ + "debug_win.bat" FOR OUTPUT AS #ffh
        PRINT #ffh, "@echo off"
        PRINT #ffh, "cd %0\..\"
        PRINT #ffh, "cd ../.."
        PRINT #ffh, "echo C++ Debugging: " + file$ + extension$ + " using gdb.exe"
        PRINT #ffh, "echo Debugger commands:"
        PRINT #ffh, "echo After the debugger launches type 'run' to start your program"
        PRINT #ffh, "echo After your program crashes type 'list' to find where the problem is and fix/report it"
        PRINT #ffh, "echo Type 'quit' to exit"
        PRINT #ffh, "echo (the GDB debugger has many other useful commands, this advice is for beginners)"
        PRINT #ffh, "pause"
        PRINT #ffh, "internal\c\c_compiler\bin\gdb.exe " + CHR$(34) + debugTargetPath + CHR$(34)
        PRINT #ffh, "pause"
        CLOSE ffh
        EXIT SUB
    END IF

    IF os$ <> "LNX" THEN EXIT SUB

    IF INSTR(_OS$, "[MACOSX]") THEN
        ffh = FREEFILE
        OPEN tmpdir$ + "recompile_osx.command" FOR OUTPUT AS #ffh
        PRINT #ffh, "cd " + CHR_QUOTE + "$(dirname " + CHR_QUOTE + "$0" + CHR_QUOTE + ")" + CHR_QUOTE + CHR$(10);
        PRINT #ffh, "echo " + CHR_QUOTE + "Recompiling..." + CHR_QUOTE + CHR$(10);
        PRINT #ffh, "cd ../c" + CHR$(10);
        PRINT #ffh, buildCommand$ + CHR$(10);
        PRINT #ffh, "read -p " + CHR_QUOTE + "Press ENTER to exit..." + CHR_QUOTE + CHR$(10);
        CLOSE ffh
        SHELL _HIDE "chmod +x " + QuotedFilename$(tmpdir$ + "recompile_osx.command")

        ffh = FREEFILE
        OPEN tmpdir$ + "debug_osx.command" FOR OUTPUT AS #ffh
        PRINT #ffh, "cd " + CHR_QUOTE + "$(dirname " + CHR_QUOTE + "$0" + CHR_QUOTE + ")" + CHR_QUOTE + CHR$(10);
        PRINT #ffh, "Pause()" + CHR$(10);
        PRINT #ffh, "{" + CHR$(10);
        PRINT #ffh, "OLDCONFIG=`stty -g`" + CHR$(10);
        PRINT #ffh, "stty -icanon -echo min 1 time 0" + CHR$(10);
        PRINT #ffh, "dd count=1 2>/dev/null" + CHR$(10);
        PRINT #ffh, "stty $OLDCONFIG" + CHR$(10);
        PRINT #ffh, "}" + CHR$(10);
        PRINT #ffh, "echo " + CHR_QUOTE + "C++ Debugging: " + file$ + extension$ + " using GDB" + CHR_QUOTE + CHR$(10);
        PRINT #ffh, "echo " + CHR_QUOTE + "Debugger commands:" + CHR_QUOTE + CHR$(10);
        PRINT #ffh, "echo " + CHR_QUOTE + "After the debugger launches type 'run' to start your program" + CHR_QUOTE + CHR$(10);
        PRINT #ffh, "echo " + CHR_QUOTE + "After your program crashes type 'list' to find where the problem is and fix/report it" + CHR_QUOTE + CHR$(10);
        PRINT #ffh, "echo " + CHR_QUOTE + "(the GDB debugger has many other useful commands, this advice is for beginners)" + CHR_QUOTE + CHR$(10);
        PRINT #ffh, "gdb " + CHR$(34) + debugTargetPath + CHR$(34) + CHR$(10);
        PRINT #ffh, "Pause" + CHR$(10);
        CLOSE ffh
        SHELL _HIDE "chmod +x " + QuotedFilename$(tmpdir$ + "debug_osx.command")
        EXIT SUB
    END IF

    ffh = FREEFILE
    OPEN tmpdir$ + "recompile_lnx.sh" FOR OUTPUT AS #ffh
    PRINT #ffh, "#!/bin/sh" + CHR$(10);
    PRINT #ffh, "Pause()" + CHR$(10);
    PRINT #ffh, "{" + CHR$(10);
    PRINT #ffh, "OLDCONFIG=`stty -g`" + CHR$(10);
    PRINT #ffh, "stty -icanon -echo min 1 time 0" + CHR$(10);
    PRINT #ffh, "dd count=1 2>/dev/null" + CHR$(10);
    PRINT #ffh, "stty $OLDCONFIG" + CHR$(10);
    PRINT #ffh, "}" + CHR$(10);
    PRINT #ffh, "echo " + CHR_QUOTE + "Recompiling..." + CHR_QUOTE + CHR$(10);
    PRINT #ffh, "cd ../c" + CHR$(10);
    PRINT #ffh, buildCommand$ + CHR$(10);
    PRINT #ffh, "echo " + CHR_QUOTE + "Press ENTER to exit..." + CHR_QUOTE + CHR$(10);
    PRINT #ffh, "Pause" + CHR$(10);
    CLOSE ffh
    SHELL _HIDE "chmod +x " + QuotedFilename$(tmpdir$ + "recompile_lnx.sh")

    ffh = FREEFILE
    OPEN tmpdir$ + "debug_lnx.sh" FOR OUTPUT AS #ffh
    PRINT #ffh, "#!/bin/sh" + CHR$(10);
    PRINT #ffh, "Pause()" + CHR$(10);
    PRINT #ffh, "{" + CHR$(10);
    PRINT #ffh, "OLDCONFIG=`stty -g`" + CHR$(10);
    PRINT #ffh, "stty -icanon -echo min 1 time 0" + CHR$(10);
    PRINT #ffh, "dd count=1 2>/dev/null" + CHR$(10);
    PRINT #ffh, "stty $OLDCONFIG" + CHR$(10);
    PRINT #ffh, "}" + CHR$(10);
    PRINT #ffh, "echo " + CHR_QUOTE + "C++ Debugging: " + file$ + extension$ + " using GDB" + CHR_QUOTE + CHR$(10);
    PRINT #ffh, "echo " + CHR_QUOTE + "Debugger commands:" + CHR_QUOTE + CHR$(10);
    PRINT #ffh, "echo " + CHR_QUOTE + "After the debugger launches type 'run' to start your program" + CHR_QUOTE + CHR$(10);
    PRINT #ffh, "echo " + CHR_QUOTE + "After your program crashes type 'list' to find where the problem is and fix/report it" + CHR_QUOTE + CHR$(10);
    PRINT #ffh, "echo " + CHR_QUOTE + "(the GDB debugger has many other useful commands, this advice is for beginners)" + CHR_QUOTE + CHR$(10);
    PRINT #ffh, "gdb " + CHR$(34) + debugTargetPath + CHR$(34) + CHR$(10);
    PRINT #ffh, "Pause" + CHR$(10);
    CLOSE ffh
    SHELL _HIDE "chmod +x " + QuotedFilename$(tmpdir$ + "debug_lnx.sh")
END SUB

SUB ExecuteBuildCommand (buildCommand$)
    IF No_C_Compile_Mode THEN EXIT SUB

    IF os$ = "WIN" THEN
        CHDIR ".\internal\c"
        SHELL _HIDE "cmd /c " + buildCommand$ + " 2>> ..\..\" + compilelog$
        CHDIR "..\.."
        EXIT SUB
    END IF

    IF os$ = "LNX" THEN
        CHDIR "./internal/c"
        SHELL _HIDE buildCommand$ + " 2>> ../../" + compilelog$
        CHDIR "../.."
    END IF
END SUB

SUB SetDependency (requirement)
    IF requirement THEN
        DEPENDENCY(requirement) = 1
    END IF
END SUB

SUB Build (path$)
    previous_dir$ = _CWD$

    'Count the separators in the path
    depth = 1
    FOR x = 1 TO LEN(path$)
        IF ASC(path$, x) = 92 OR ASC(path$, x) = 47 THEN depth = depth + 1
    NEXT
    CHDIR path$

    return_path$ = ".."
    FOR x = 2 TO depth
        return_path$ = return_path$ + "\.."
    NEXT

    bfh = FREEFILE
    OPEN "build" + BATCHFILE_EXTENSION FOR BINARY AS #bfh
    DO UNTIL EOF(bfh)
        LINE INPUT #bfh, c$
        use = 0
        IF LEN(c$) THEN use = 1
        IF c$ = "pause" THEN use = 0
        IF LEFT$(c$, 1) = "#" THEN use = 0 'eg. #!/bin/sh
        IF LEFT$(c$, 13) = "cd " + CHR$(34) + "$(dirname" THEN use = 0 'eg. cd "$(dirname "$0")"
        IF INSTR(LCASE$(c$), "press any key") THEN EXIT DO
        c$ = GDB_Fix$(c$)
        IF use THEN
            IF os$ = "WIN" THEN
                SHELL _HIDE "cmd /C " + c$ + " 2>> " + QuotedFilename$(return_path$ + "\" + compilelog$)
            ELSE
                SHELL _HIDE c$ + " 2>> " + QuotedFilename$(previous_dir$ + "/" + compilelog$)
            END IF
        END IF
    LOOP
    CLOSE #bfh

    IF os$ = "WIN" THEN
        CHDIR return_path$
    ELSE
        CHDIR previous_dir$
    END IF
END SUB

FUNCTION GDB_Fix$ (g_command$) 'edit a gcc/g++ command line to include debugging info
    c$ = g_command$
    IF Include_GDB_Debugging_Info THEN
        IF LEFT$(c$, 4) = "gcc " OR LEFT$(c$, 4) = "g++ " THEN
            c$ = LEFT$(c$, 4) + " -g " + RIGHT$(c$, LEN(c$) - 4)
            GOTO added_gdb_flag
        END IF
        FOR o = 1 TO 6
            IF o = 1 THEN o$ = "\g++ "
            IF o = 2 THEN o$ = "/g++ "
            IF o = 3 THEN o$ = "\gcc "
            IF o = 4 THEN o$ = "/gcc "
            IF o = 5 THEN o$ = " gcc "
            IF o = 6 THEN o$ = " g++ "
            x = INSTR(UCASE$(c$), UCASE$(o$))
            'note: -g adds debug symbols
            IF x THEN c$ = LEFT$(c$, x - 1) + o$ + " -g " + RIGHT$(c$, LEN(c$) - x - (LEN(o$) - 1)): EXIT FOR
        NEXT
        added_gdb_flag:
        'note: -s strips all debug symbols which is good for size but not for debugging
        x = INSTR(c$, " -s "): IF x THEN c$ = LEFT$(c$, x - 1) + " " + RIGHT$(c$, LEN(c$) - x - 3)
    END IF
    GDB_Fix$ = c$
END FUNCTION
