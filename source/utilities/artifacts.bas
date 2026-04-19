FUNCTION QuotedFilename$ (f$)

    IF os$ = "WIN" THEN
        QuotedFilename$ = CHR$(34) + f$ + CHR$(34)
        EXIT FUNCTION
    END IF

    IF os$ = "LNX" THEN
        QuotedFilename$ = "'" + f$ + "'"
        EXIT FUNCTION
    END IF

END FUNCTION

SUB PreparePendingOutputBinary (sourceBaseName AS STRING)
    DIM outputName AS STRING
    DIM pathOut AS STRING
    DIM currentdir AS STRING

    pendingOutputBinary$ = path.exe$ + sourceBaseName + extension$

    IF LEN(outputfile_cmd$) = 0 THEN EXIT SUB

    pathOut = getfilepath$(outputfile_cmd$)
    outputName = MID$(outputfile_cmd$, LEN(pathOut) + 1)
    IF outputName = "" THEN outputName = outputfile_cmd$
    outputName = RemoveFileExtension$(outputName)

    IF LEN(pathOut) THEN
        IF _DIREXISTS(pathOut) THEN
            currentdir = _CWD$
            CHDIR pathOut
            pathOut = _CWD$
            CHDIR currentdir
            IF RIGHT$(pathOut, 1) <> pathsep$ THEN pathOut = pathOut + pathsep$
        END IF
    END IF

    pendingOutputBinary$ = pathOut + outputName + extension$
END SUB

SUB ResolveBuildOutputTarget (fileName AS STRING, exePath AS STRING)
    DIM pathOut AS STRING
    DIM currentdir AS STRING

    IF LEN(outputfile_cmd$) = 0 THEN
        pendingOutputBinary$ = exePath + fileName + extension$
        EXIT SUB
    END IF

    pathOut = getfilepath$(outputfile_cmd$)
    fileName = MID$(outputfile_cmd$, LEN(pathOut) + 1)
    IF fileName = "" THEN fileName = outputfile_cmd$
    fileName = RemoveFileExtension$(fileName)

    IF LEN(pathOut) THEN
        IF _DIREXISTS(pathOut) = 0 THEN
            PRINT
            PRINT "Can't create output executable - path not found: " + pathOut
            IF ConsoleMode THEN SYSTEM 1
            END 1
        END IF

        currentdir = _CWD$
        CHDIR pathOut
        pathOut = _CWD$
        CHDIR currentdir
        IF RIGHT$(pathOut, 1) <> pathsep$ THEN pathOut = pathOut + pathsep$
        exePath = pathOut
        SaveExeWithSource = -1
    END IF

    pendingOutputBinary$ = exePath + fileName + extension$
END SUB

SUB WarnIfStaleOutputBinary
    DIM outputPath AS STRING
    DIM sourcePath AS STRING

    outputPath = RTRIM$(pendingOutputBinary$)
    sourcePath = RTRIM$(sourcefile$)

    IF outputPath = "" THEN EXIT SUB
    IF sourcePath = "" THEN EXIT SUB
    IF _FILEEXISTS(outputPath) = 0 THEN EXIT SUB
    IF _FILEEXISTS(sourcePath) = 0 THEN EXIT SUB

    PRINT
    PRINT "Warning: Existing output was not updated because compilation failed."
    PRINT "Existing executable may be stale:"
    PRINT "  Source: "; sourcePath
    PRINT "  Output: "; outputPath
END SUB

SUB InitializeCompilationLog
    compilelog$ = tmpdir$ + "compilelog.txt"
    OPEN compilelog$ FOR OUTPUT AS #1
    CLOSE #1
END SUB

FUNCTION PrepareExecutableOutputTarget% (outputBaseName AS STRING)
    DIM originalExePath AS STRING

    IF NOT QuietMode THEN
        PRINT "Compiling program..."
        PRINT
    END IF

    originalExePath = path.exe$
    ResolveBuildOutputTarget outputBaseName, path.exe$

    IF path.exe$ = "../../" OR path.exe$ = "..\..\" THEN path.exe$ = ""
    IF _FILEEXISTS(path.exe$ + outputBaseName + extension$) THEN
        E = 0
        ON ERROR GOTO prepare_output_target_error
        KILL path.exe$ + outputBaseName + extension$
        IF E = 1 THEN
            a$ = "CANNOT CREATE " + CHR$(34) + outputBaseName + extension$ + CHR$(34) + " BECAUSE THE FILE IS ALREADY IN USE!"
            path.exe$ = originalExePath
            PrepareExecutableOutputTarget% = -1
            EXIT FUNCTION
        END IF
    END IF

    path.exe$ = originalExePath
    pendingOutputBinary$ = path.exe$ + outputBaseName + extension$
    PrepareExecutableOutputTarget% = 0
    EXIT FUNCTION

prepare_output_target_error:
    E = 1
    RESUME NEXT
END FUNCTION

SUB WriteWindowsManifestFiles (outputBaseName AS STRING)
    manifest = FREEFILE
    OPEN tmpdir$ + outputBaseName + extension$ + ".manifest" FOR OUTPUT AS #manifest
    PRINT #manifest, "<?xml version=" + QuotedFilename("1.0") + " encoding=" + QuotedFilename("UTF-8") + " standalone=" + QuotedFilename("yes") + "?>"
    PRINT #manifest, "<assembly xmlns=" + QuotedFilename("urn:schemas-microsoft-com:asm.v1") + " manifestVersion=" + QuotedFilename("1.0") + ">"
    PRINT #manifest, "<assemblyIdentity"
    PRINT #manifest, "    version=" + QuotedFilename("1.0.0.0")
    PRINT #manifest, "    processorArchitecture=" + QuotedFilename("*")
    PRINT #manifest, "    name=" + QuotedFilename(viCompanyName$ + "." + viProductName$ + "." + viProductName$)
    PRINT #manifest, "    type=" + QuotedFilename("win32")
    PRINT #manifest, "/>"
    PRINT #manifest, "<description>" + viFileDescription$ + "</description>"
    PRINT #manifest, "<dependency>"
    PRINT #manifest, "    <dependentAssembly>"
    PRINT #manifest, "        <assemblyIdentity"
    PRINT #manifest, "            type=" + QuotedFilename("win32")
    PRINT #manifest, "            name=" + QuotedFilename("Microsoft.Windows.Common-Controls")
    PRINT #manifest, "            version=" + QuotedFilename("6.0.0.0")
    PRINT #manifest, "            processorArchitecture=" + QuotedFilename("*")
    PRINT #manifest, "            publicKeyToken=" + QuotedFilename("6595b64144ccf1df")
    PRINT #manifest, "            language=" + QuotedFilename("*")
    PRINT #manifest, "        />"
    PRINT #manifest, "    </dependentAssembly>"
    PRINT #manifest, "</dependency>"
    PRINT #manifest, "</assembly>"
    CLOSE #manifest

    manifestembed = FREEFILE
    OPEN tmpdir$ + "manifest.h" FOR OUTPUT AS #manifestembed
    PRINT #manifestembed, "#ifndef RESOURCE_H"
    PRINT #manifestembed, "#define   RESOURCE_H"
    PRINT #manifestembed, "#ifdef    __cplusplus"
    PRINT #manifestembed, "extern " + QuotedFilename("C") + " {"
    PRINT #manifestembed, "#endif"
    PRINT #manifestembed, "#ifdef    __cplusplus"
    PRINT #manifestembed, "}"
    PRINT #manifestembed, "#endif"
    PRINT #manifestembed, "#endif    /* RESOURCE_H */"
    PRINT #manifestembed, "#define CREATEPROCESS_MANIFEST_RESOURCE_ID 1 /*Defined manifest file*/"
    PRINT #manifestembed, "#define RT_MANIFEST                       24"
    CLOSE #manifestembed
END SUB

SUB AppendVersionInfoResource (outputBaseName AS STRING)
    iconfilehandle = FREEFILE
    OPEN tmpdir$ + "icon.rc" FOR APPEND AS #iconfilehandle
    PRINT #iconfilehandle, ""
    PRINT #iconfilehandle, "#include " + QuotedFilename("manifest.h")
    PRINT #iconfilehandle, ""
    PRINT #iconfilehandle, "CREATEPROCESS_MANIFEST_RESOURCE_ID RT_MANIFEST " + QuotedFilename(outputBaseName + extension$ + ".manifest")
    PRINT #iconfilehandle, ""
    PRINT #iconfilehandle, "1 VERSIONINFO"
    IF LEN(viFileVersionNum$) THEN PRINT #iconfilehandle, "FILEVERSION     "; viFileVersionNum$
    IF LEN(viProductVersionNum$) THEN PRINT #iconfilehandle, "PRODUCTVERSION  "; viProductVersionNum$
    PRINT #iconfilehandle, "BEGIN"
    PRINT #iconfilehandle, "    BLOCK " + QuotedFilename$("StringFileInfo")
    PRINT #iconfilehandle, "    BEGIN"
    PRINT #iconfilehandle, "        BLOCK " + QuotedFilename$("040904E4")
    PRINT #iconfilehandle, "        BEGIN"
    PRINT #iconfilehandle, "            VALUE " + QuotedFilename$("CompanyName") + "," + QuotedFilename$(viCompanyName$ + "\0")
    PRINT #iconfilehandle, "            VALUE " + QuotedFilename$("FileDescription") + "," + QuotedFilename$(viFileDescription$ + "\0")
    PRINT #iconfilehandle, "            VALUE " + QuotedFilename$("FileVersion") + "," + QuotedFilename$(viFileVersion$ + "\0")
    PRINT #iconfilehandle, "            VALUE " + QuotedFilename$("InternalName") + "," + QuotedFilename$(viInternalName$ + "\0")
    PRINT #iconfilehandle, "            VALUE " + QuotedFilename$("LegalCopyright") + "," + QuotedFilename$(viLegalCopyright$ + "\0")
    PRINT #iconfilehandle, "            VALUE " + QuotedFilename$("LegalTrademarks") + "," + QuotedFilename$(viLegalTrademarks$ + "\0")
    PRINT #iconfilehandle, "            VALUE " + QuotedFilename$("OriginalFilename") + "," + QuotedFilename$(viOriginalFilename$ + "\0")
    PRINT #iconfilehandle, "            VALUE " + QuotedFilename$("ProductName") + "," + QuotedFilename$(viProductName$ + "\0")
    PRINT #iconfilehandle, "            VALUE " + QuotedFilename$("ProductVersion") + "," + QuotedFilename$(viProductVersion$ + "\0")
    PRINT #iconfilehandle, "            VALUE " + QuotedFilename$("Comments") + "," + QuotedFilename$(viComments$ + "\0")
    PRINT #iconfilehandle, "            VALUE " + QuotedFilename$("Web") + "," + QuotedFilename$(viWeb$ + "\0")
    PRINT #iconfilehandle, "        END"
    PRINT #iconfilehandle, "    END"
    PRINT #iconfilehandle, "    BLOCK " + QuotedFilename$("VarFileInfo")
    PRINT #iconfilehandle, "    BEGIN"
    PRINT #iconfilehandle, "            VALUE " + QuotedFilename$("Translation") + ", 0x409, 0x04E4"
    PRINT #iconfilehandle, "    END"
    PRINT #iconfilehandle, "END"
    CLOSE #iconfilehandle
END SUB

FUNCTION PrepareWindowsResourceArtifacts% (outputBaseName AS STRING)
    IF os$ <> "WIN" THEN EXIT FUNCTION

    IF ExeIconSet OR VersionInfoSet THEN
        IF _FILEEXISTS(tmpdir$ + "icon.o") THEN
            E = 0
            ON ERROR GOTO prepare_windows_resources_error
            KILL tmpdir$ + "icon.o"
            IF E = 1 OR _FILEEXISTS(tmpdir$ + "icon.o") = -1 THEN
                a$ = "Error creating resource file"
                PrepareWindowsResourceArtifacts% = -1
                EXIT FUNCTION
            END IF
        END IF
    END IF

    IF ExeIconSet THEN
        linenumber = ExeIconSet
        wholeline = " $EXEICON:'" + ExeIconFile$ + "'"
    END IF

    IF VersionInfoSet THEN
        WriteWindowsManifestFiles outputBaseName
        AppendVersionInfoResource outputBaseName
    END IF

    IF ExeIconSet OR VersionInfoSet THEN
        ffh = FREEFILE
        OPEN tmpdir$ + "call_windres.bat" FOR OUTPUT AS #ffh
        PRINT #ffh, "internal\c\c_compiler\bin\windres.exe -i " + StrReplace$(tmpdir$, "\", "/") + "icon.rc -o " + StrReplace$(tmpdir$, "\", "/") + "icon.o"
        CLOSE #ffh
        SHELL _HIDE tmpdir$ + "call_windres.bat"
        IF _FILEEXISTS(tmpdir$ + "icon.o") = 0 THEN
            a$ = "Bad icon file"
            IF VersionInfoSet THEN a$ = a$ + " or invalid $VERSIONINFO values"
            PrepareWindowsResourceArtifacts% = -1
            EXIT FUNCTION
        END IF
    END IF

    PrepareWindowsResourceArtifacts% = 0
    EXIT FUNCTION

prepare_windows_resources_error:
    E = 1
    RESUME NEXT
END FUNCTION

SUB EmitMacOSLauncherScript (outputBaseName AS STRING)
    IF INSTR(_OS$, "[MACOSX]") = 0 THEN EXIT SUB

    ff = FREEFILE
    IF path.exe$ = "./" OR path.exe$ = "../../" OR path.exe$ = "..\..\" THEN path.exe$ = ""
    OPEN path.exe$ + outputBaseName + extension$ + "_start.command" FOR OUTPUT AS #ff
    PRINT #ff, "cd " + CHR$(34) + "$(dirname " + CHR$(34) + "$0" + CHR$(34) + ")" + CHR$(34);
    PRINT #ff, CHR$(10);
    PRINT #ff, "./" + outputBaseName + extension$ + " &";
    PRINT #ff, CHR$(10);
    PRINT #ff, "osascript -e 'tell application " + CHR$(34) + "Terminal" + CHR$(34) + " to close (every window whose name contains " + CHR$(34) + outputBaseName + extension$ + "_start.command" + CHR$(34) + ")' &";
    PRINT #ff, CHR$(10);
    PRINT #ff, "osascript -e 'if (count the windows of application " + CHR$(34) + "Terminal" + CHR$(34) + ") is 0 then tell application " + CHR$(34) + "Terminal" + CHR$(34) + " to quit' &";
    PRINT #ff, CHR$(10);
    PRINT #ff, "exit";
    PRINT #ff, CHR$(10);
    CLOSE #ff
    SHELL _HIDE "chmod +x " + path.exe$ + outputBaseName + extension$ + "_start.command"
END SUB
