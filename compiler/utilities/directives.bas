FUNCTION HandleSimpleDirective% (upperLine AS STRING)
    DIM tempPos AS LONG
    DIM l$
    DIM r$

    HandleSimpleDirective% = 0

    IF LEFT$(upperLine, 5) = "$LET " THEN
        temp$ = LTRIM$(MID$(upperLine, 5))
        tempPos = INSTR(temp$, "=")
        IF tempPos = 0 THEN
            a$ = "Invalid Syntax.  $LET <flag> = <value>"
            HandleSimpleDirective% = 2
            EXIT FUNCTION
        END IF

        l$ = RTRIM$(LEFT$(temp$, tempPos - 1))
        r$ = LTRIM$(MID$(temp$, tempPos + 1))
        layout$ = SCase$("$Let ") + l$ + " = " + r$

        FOR i = 7 TO UserDefineCount
            IF UserDefineName$(i) = l$ THEN
                UserDefineValue$(i) = r$
                HandleSimpleDirective% = 1
                EXIT FUNCTION
            END IF
        NEXT

        UserDefineCount = UserDefineCount + 1
        UserDefineName$(UserDefineCount) = l$
        UserDefineValue$(UserDefineCount) = r$
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$COLOR:0" THEN
        layout$ = SCase$("$Color:0")
        addmetainclude$ = ResolveColorSupportInclude$(0)
        layoutdone = 1
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$COLOR:32" THEN
        layout$ = SCase$("$Color:32")
        addmetainclude$ = ResolveColorSupportInclude$(32)
        layoutdone = 1
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$NOPREFIX" THEN
        layout$ = SCase$("$NoPrefix")
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$VIRTUALKEYBOARD:ON" THEN
        layout$ = SCase$("$VirtualKeyboard:On")
        addWarning linenumber, inclevel, inclinenumber(inclevel), incname$(inclevel), "Deprecated feature", "$VirtualKeyboard"
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$VIRTUALKEYBOARD:OFF" THEN
        layout$ = SCase$("$VirtualKeyboard:Off")
        addWarning linenumber, inclevel, inclinenumber(inclevel), incname$(inclevel), "Deprecated feature", "$VirtualKeyboard"
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$DEBUG" THEN
        layout$ = SCase$("$Debug")
        addWarning linenumber, inclevel, inclinenumber(inclevel), incname$(inclevel), "$Debug", "$Debug is not supported in the CLI compiler"
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$CHECKING:OFF" THEN
        layout$ = SCase$("$Checking:Off")
        NoChecks = 1
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$CHECKING:ON" THEN
        layout$ = SCase$("$Checking:On")
        NoChecks = 0
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$CONSOLE" THEN
        layout$ = SCase$("$Console")
        Console = 1
        AutoConsoleOnlyEligible = 0
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$CONSOLE:ONLY" THEN
        layout$ = SCase$("$Console:Only")
        DEPENDENCY(DEPENDENCY_CONSOLE_ONLY) = DEPENDENCY(DEPENDENCY_CONSOLE_ONLY) OR 1
        Console = 1
        AutoConsoleOnlyEligible = 0
        IF prepass = 0 THEN
            IF NoChecks = 0 THEN PRINT #12, "do{"
            PRINT #12, "sub__dest(func__console());"
            PRINT #12, "sub__source(func__console());"
            HandleSimpleDirective% = 3
        ELSE
            HandleSimpleDirective% = 1
        END IF
        EXIT FUNCTION
    END IF

    IF upperLine = "$ASSERTS" THEN
        layout$ = SCase$("$Asserts")
        Asserts = 1
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$ASSERTS:CONSOLE" THEN
        layout$ = SCase$("$Asserts:Console")
        Asserts = 1
        Console = 1
        AutoConsoleOnlyEligible = 0
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$SCREENHIDE" THEN
        layout$ = SCase$("$ScreenHide")
        ScreenHide = 1
        AutoConsoleOnlyEligible = 0
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$SCREENSHOW" THEN
        layout$ = SCase$("$ScreenShow")
        ScreenHide = 0
        AutoConsoleOnlyEligible = 0
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$RESIZE:OFF" THEN
        layout$ = SCase$("$Resize:Off")
        Resize = 0
        Resize_Scale = 0
        AutoConsoleOnlyEligible = 0
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$RESIZE:ON" THEN
        layout$ = SCase$("$Resize:On")
        Resize = 1
        Resize_Scale = 0
        AutoConsoleOnlyEligible = 0
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$RESIZE:STRETCH" THEN
        layout$ = SCase$("$Resize:Stretch")
        Resize = 1
        Resize_Scale = 1
        AutoConsoleOnlyEligible = 0
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$RESIZE:SMOOTH" THEN
        layout$ = SCase$("$Resize:Smooth")
        Resize = 1
        Resize_Scale = 2
        AutoConsoleOnlyEligible = 0
        HandleSimpleDirective% = 1
        EXIT FUNCTION
    END IF
END FUNCTION

FUNCTION HandlePrepassConditionalDirective% (upperLine AS STRING)
    DIM conditionText AS STRING
    DIM conditionResult AS INTEGER

    HandlePrepassConditionalDirective% = 0

    IF upperLine = "$END IF" OR upperLine = "$ENDIF" THEN
        IF DefineElse(ExecCounter) = 0 THEN
            a$ = "$END IF without $IF"
            HandlePrepassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        DefineElse(ExecCounter) = 0
        ExecCounter = ExecCounter - 1
        HandlePrepassConditionalDirective% = 1
        EXIT FUNCTION
    END IF

    IF LEFT$(upperLine, 4) = "$IF " THEN
        IF RIGHT$(upperLine, 5) <> " THEN" THEN
            a$ = "$IF without THEN"
            HandlePrepassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        conditionText = LTRIM$(MID$(upperLine, 4))
        conditionText = RTRIM$(LEFT$(conditionText, LEN(conditionText) - 4))

        ExecCounter = ExecCounter + 1
        ExecLevel(ExecCounter) = -1
        DefineElse(ExecCounter) = 1

        conditionResult = EvalPreIF(conditionText, a$)
        IF a$ <> "" THEN
            HandlePrepassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        IF conditionResult <> 0 THEN
            ExecLevel(ExecCounter) = ExecLevel(ExecCounter - 1)
            IF ExecLevel(ExecCounter) = 0 THEN DefineElse(ExecCounter) = DefineElse(ExecCounter) OR 4
        END IF

        HandlePrepassConditionalDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$ELSE" THEN
        IF DefineElse(ExecCounter) = 0 THEN
            a$ = "$ELSE without $IF"
            HandlePrepassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        IF DefineElse(ExecCounter) AND 2 THEN
            a$ = "$IF block already has $ELSE statement in it"
            HandlePrepassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        DefineElse(ExecCounter) = DefineElse(ExecCounter) OR 2
        IF DefineElse(ExecCounter) AND 4 THEN
            ExecLevel(ExecCounter) = -1
        ELSE
            ExecLevel(ExecCounter) = ExecLevel(ExecCounter - 1)
        END IF

        HandlePrepassConditionalDirective% = 1
        EXIT FUNCTION
    END IF

    IF LEFT$(upperLine, 5) = "$ELSE" THEN
        conditionText = LTRIM$(MID$(upperLine, 6))
        IF LEFT$(conditionText, 3) <> "IF " THEN EXIT FUNCTION

        IF DefineElse(ExecCounter) = 0 THEN
            a$ = "$ELSE IF without $IF"
            HandlePrepassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        IF DefineElse(ExecCounter) AND 2 THEN
            a$ = "$ELSE IF cannot follow $ELSE"
            HandlePrepassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        IF RIGHT$(conditionText, 5) <> " THEN" THEN
            a$ = "$ELSE IF without THEN"
            HandlePrepassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        conditionText = LTRIM$(MID$(conditionText, 3))
        conditionText = RTRIM$(LEFT$(conditionText, LEN(conditionText) - 4))

        IF DefineElse(ExecCounter) AND 4 THEN
            ExecLevel(ExecCounter) = -1
            HandlePrepassConditionalDirective% = 1
            EXIT FUNCTION
        END IF

        conditionResult = EvalPreIF(conditionText, a$)
        IF a$ <> "" THEN
            HandlePrepassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        IF conditionResult <> 0 THEN
            ExecLevel(ExecCounter) = ExecLevel(ExecCounter - 1)
            IF ExecLevel(ExecCounter) = 0 THEN DefineElse(ExecCounter) = DefineElse(ExecCounter) OR 4
        END IF

        HandlePrepassConditionalDirective% = 1
        EXIT FUNCTION
    END IF
END FUNCTION

FUNCTION HandlePrepassMetaDirective% (upperLine AS STRING)
    DIM equalPos AS LONG
    DIM defineName AS STRING
    DIM defineValue AS STRING
    DIM normalizedValue AS STRING
    DIM charCode AS INTEGER

    HandlePrepassMetaDirective% = 0

    IF LEFT$(upperLine, 7) = "$ERROR " THEN
        a$ = "Compilation check failed: " + LTRIM$(MID$(upperLine, 7))
        HandlePrepassMetaDirective% = 2
        EXIT FUNCTION
    END IF

    IF LEFT$(upperLine, 5) <> "$LET " THEN EXIT FUNCTION

    defineValue = LTRIM$(MID$(upperLine, 5))
    equalPos = INSTR(defineValue, "=")
    IF equalPos = 0 THEN
        a$ = "Invalid Syntax.  $LET <flag> = <value>"
        HandlePrepassMetaDirective% = 2
        EXIT FUNCTION
    END IF

    defineName = RTRIM$(LEFT$(defineValue, equalPos - 1))
    defineValue = LTRIM$(MID$(defineValue, equalPos + 1))
    IF validname(defineName) = 0 THEN
        a$ = "Invalid flag name"
        HandlePrepassMetaDirective% = 2
        EXIT FUNCTION
    END IF

    IF LEFT$(defineValue, 1) = CHR$(34) THEN defineValue = LTRIM$(MID$(defineValue, 2))
    IF RIGHT$(defineValue, 1) = CHR$(34) THEN defineValue = RTRIM$(LEFT$(defineValue, LEN(defineValue) - 1))
    IF LEFT$(defineValue, 1) = "-" THEN
        normalizedValue = "-"
        defineValue = LTRIM$(MID$(defineValue, 2))
    ELSE
        normalizedValue = ""
    END IF

    FOR i = 1 TO LEN(defineValue)
        charCode = ASC(defineValue, i)
        SELECT CASE charCode
        CASE 32
        CASE 46
            normalizedValue = normalizedValue + "."
        CASE IS < 48, IS > 90
            a$ = "Invalid value"
            HandlePrepassMetaDirective% = 2
            EXIT FUNCTION
        CASE ELSE
            normalizedValue = normalizedValue + CHR$(charCode)
        END SELECT
    NEXT
    defineValue = normalizedValue

    FOR i = 8 TO UserDefineCount
            IF UserDefineName$(i) = defineName THEN
                UserDefineValue$(i) = defineValue
            HandlePrepassMetaDirective% = 1
            EXIT FUNCTION
        END IF
    NEXT

    UserDefineCount = UserDefineCount + 1
    UserDefineName$(UserDefineCount) = defineName
    UserDefineValue$(UserDefineCount) = defineValue

    HandlePrepassMetaDirective% = 1
END FUNCTION

FUNCTION HandleMainPassConditionalDirective% (upperLine AS STRING)
    DIM conditionText AS STRING
    DIM conditionResult AS INTEGER
    DIM comparePos AS LONG
    DIM compareOp AS STRING
    DIM l$
    DIM r$

    HandleMainPassConditionalDirective% = 0

    IF upperLine = "$END IF" OR upperLine = "$ENDIF" THEN
        IF DefineElse(ExecCounter) = 0 THEN
            a$ = "$END IF without $IF"
            HandleMainPassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        DefineElse(ExecCounter) = 0
        ExecCounter = ExecCounter - 1
        layout$ = SCase$("$End If")
        controltype(controllevel) = 0
        controllevel = controllevel - 1
        HandleMainPassConditionalDirective% = 1
        EXIT FUNCTION
    END IF

    IF LEFT$(upperLine, 4) = "$IF " THEN
        IF SelectCaseCounter > 0 AND SelectCaseHasCaseBlock(SelectCaseCounter) = 0 THEN
            a$ = "Expected CASE expression"
            HandleMainPassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        conditionText = LTRIM$(MID$(upperLine, 4))
        conditionText = RTRIM$(LEFT$(conditionText, LEN(conditionText) - 4))

        comparePos = 0
        IF comparePos = 0 THEN compareOp = "<=": comparePos = INSTR(conditionText, compareOp)
        IF comparePos = 0 THEN compareOp = "=<": comparePos = INSTR(conditionText, compareOp): compareOp = "<="
        IF comparePos = 0 THEN compareOp = ">=": comparePos = INSTR(conditionText, compareOp)
        IF comparePos = 0 THEN compareOp = "=>": comparePos = INSTR(conditionText, compareOp): compareOp = ">="
        IF comparePos = 0 THEN compareOp = "<>": comparePos = INSTR(conditionText, compareOp)
        IF comparePos = 0 THEN compareOp = "><": comparePos = INSTR(conditionText, compareOp): compareOp = "<>"
        IF comparePos = 0 THEN compareOp = "=": comparePos = INSTR(conditionText, compareOp)
        IF comparePos = 0 THEN compareOp = ">": comparePos = INSTR(conditionText, compareOp)
        IF comparePos = 0 THEN compareOp = "<": comparePos = INSTR(conditionText, compareOp)

        ExecCounter = ExecCounter + 1
        ExecLevel(ExecCounter) = -1
        DefineElse(ExecCounter) = 1

        conditionResult = EvalPreIF(conditionText, a$)
        IF a$ <> "" THEN
            HandleMainPassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        IF conditionResult <> 0 THEN
            ExecLevel(ExecCounter) = ExecLevel(ExecCounter - 1)
            IF ExecLevel(ExecCounter) = 0 THEN DefineElse(ExecCounter) = DefineElse(ExecCounter) OR 4
        END IF

        controllevel = controllevel + 1
        controltype(controllevel) = 6

        IF comparePos = 0 THEN
            layout$ = SCase$("$If ") + conditionText + SCase$(" Then")
        ELSE
            l$ = RTRIM$(LEFT$(conditionText, comparePos - 1))
            r$ = LTRIM$(MID$(conditionText, comparePos + LEN(compareOp)))
            layout$ = SCase$("$If ") + l$ + " " + compareOp + " " + r$ + SCase$(" Then")
        END IF

        HandleMainPassConditionalDirective% = 1
        EXIT FUNCTION
    END IF

    IF upperLine = "$ELSE" THEN
        IF DefineElse(ExecCounter) = 0 THEN
            a$ = "$ELSE without $IF"
            HandleMainPassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        IF DefineElse(ExecCounter) AND 2 THEN
            a$ = "$IF block already has $ELSE statement in it"
            HandleMainPassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        DefineElse(ExecCounter) = DefineElse(ExecCounter) OR 2
        IF DefineElse(ExecCounter) AND 4 THEN
            ExecLevel(ExecCounter) = -1
        ELSE
            ExecLevel(ExecCounter) = ExecLevel(ExecCounter - 1)
        END IF

        layout$ = SCase$("$Else")
        lhscontrollevel = lhscontrollevel - 1
        HandleMainPassConditionalDirective% = 1
        EXIT FUNCTION
    END IF

    IF LEFT$(upperLine, 5) = "$ELSE" THEN
        conditionText = LTRIM$(MID$(upperLine, 6))
        IF LEFT$(conditionText, 3) <> "IF " THEN EXIT FUNCTION

        IF DefineElse(ExecCounter) = 0 THEN
            a$ = "$ELSE IF without $IF"
            HandleMainPassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        IF DefineElse(ExecCounter) AND 2 THEN
            a$ = "$ELSE IF cannot follow $ELSE"
            HandleMainPassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        IF RIGHT$(conditionText, 5) <> " THEN" THEN
            a$ = "$ELSE IF without THEN"
            HandleMainPassConditionalDirective% = 2
            EXIT FUNCTION
        END IF

        conditionText = LTRIM$(MID$(conditionText, 3))
        conditionText = RTRIM$(LEFT$(conditionText, LEN(conditionText) - 4))

        IF DefineElse(ExecCounter) AND 4 THEN
            ExecLevel(ExecCounter) = -1
        ELSE
            conditionResult = EvalPreIF(conditionText, a$)
            IF a$ <> "" THEN
                HandleMainPassConditionalDirective% = 2
                EXIT FUNCTION
            END IF

            IF conditionResult <> 0 THEN
                ExecLevel(ExecCounter) = ExecLevel(ExecCounter - 1)
                IF ExecLevel(ExecCounter) = 0 THEN DefineElse(ExecCounter) = DefineElse(ExecCounter) OR 4
            END IF
        END IF

        lhscontrollevel = lhscontrollevel - 1
        comparePos = INSTR(conditionText, "=")
        IF comparePos = 0 THEN
            layout$ = SCase$("$ElseIf ") + conditionText + SCase$(" Then")
        ELSE
            l$ = RTRIM$(LEFT$(conditionText, comparePos - 1))
            r$ = LTRIM$(MID$(conditionText, comparePos + 1))
            layout$ = SCase$("$ElseIf ") + l$ + " = " + r$ + SCase$(" Then")
        END IF

        HandleMainPassConditionalDirective% = 1
        EXIT FUNCTION
    END IF
END FUNCTION

FUNCTION ValidateVersionInfoValue% (versionInfoKey AS STRING, versionInfoValue AS STRING)
    DIM viCommas AS LONG

    ValidateVersionInfoValue% = 0
    IF LEN(versionInfoValue) = 0 THEN
        a$ = "Expected $VERSIONINFO:" + versionInfoKey + "=#,#,#,# (4 comma-separated numeric values: major, minor, revision and build)"
        ValidateVersionInfoValue% = -1
        EXIT FUNCTION
    END IF

    viCommas = 0
    FOR i = 1 TO LEN(versionInfoValue)
        IF ASC(versionInfoValue, i) = 44 THEN viCommas = viCommas + 1
        IF INSTR("0123456789,", MID$(versionInfoValue, i, 1)) = 0 OR (i = LEN(versionInfoValue) AND viCommas <> 3) OR RIGHT$(versionInfoValue, 1) = "," THEN
            a$ = "Expected $VERSIONINFO:" + versionInfoKey + "=#,#,#,# (4 comma-separated numeric values: major, minor, revision and build)"
            ValidateVersionInfoValue% = -1
            EXIT FUNCTION
        END IF
    NEXT
END FUNCTION

FUNCTION HandleVersionInfoDirective% (upperLine AS STRING, rawLine AS STRING)
    DIM firstDelimiter AS LONG
    DIM secondDelimiter AS LONG
    DIM versionInfoKey AS STRING
    DIM versionInfoValue AS STRING

    HandleVersionInfoDirective% = 0
    IF LEFT$(upperLine, 12) <> "$VERSIONINFO" THEN EXIT FUNCTION

    firstDelimiter = INSTR(upperLine, ":")
    secondDelimiter = INSTR(firstDelimiter + 1, upperLine, "=")
    IF firstDelimiter = 0 OR secondDelimiter = 0 OR secondDelimiter = firstDelimiter + 1 THEN
        a$ = "Expected $VERSIONINFO:key=value"
        HandleVersionInfoDirective% = 2
        EXIT FUNCTION
    END IF

    versionInfoKey = LTRIM$(RTRIM$(MID$(upperLine, firstDelimiter + 1, secondDelimiter - firstDelimiter - 1)))
    versionInfoValue = StrReplace$(LTRIM$(RTRIM$(MID$(rawLine, secondDelimiter + 1))), CHR$(34), "'")

    SELECT CASE versionInfoKey
    CASE "FILEVERSION#"
        IF ValidateVersionInfoValue%(versionInfoKey, versionInfoValue) THEN
            HandleVersionInfoDirective% = 2
            EXIT FUNCTION
        END IF
        viFileVersionNum$ = versionInfoValue
        layout$ = SCase$("$VersionInfo:FILEVERSION#=") + versionInfoValue
    CASE "PRODUCTVERSION#"
        IF ValidateVersionInfoValue%(versionInfoKey, versionInfoValue) THEN
            HandleVersionInfoDirective% = 2
            EXIT FUNCTION
        END IF
        viProductVersionNum$ = versionInfoValue
        layout$ = SCase$("$VersionInfo:PRODUCTVERSION#=") + versionInfoValue
    CASE "COMPANYNAME"
        viCompanyName$ = versionInfoValue
        layout$ = SCase$("$VersionInfo:") + "CompanyName=" + versionInfoValue
    CASE "FILEDESCRIPTION"
        viFileDescription$ = versionInfoValue
        layout$ = SCase$("$VersionInfo:") + "FileDescription=" + versionInfoValue
    CASE "FILEVERSION"
        viFileVersion$ = versionInfoValue
        layout$ = SCase$("$VersionInfo:") + "FileVersion=" + versionInfoValue
    CASE "INTERNALNAME"
        viInternalName$ = versionInfoValue
        layout$ = SCase$("$VersionInfo:") + "InternalName=" + versionInfoValue
    CASE "LEGALCOPYRIGHT"
        viLegalCopyright$ = versionInfoValue
        layout$ = SCase$("$VersionInfo:") + "LegalCopyright=" + versionInfoValue
    CASE "LEGALTRADEMARKS"
        viLegalTrademarks$ = versionInfoValue
        layout$ = SCase$("$VersionInfo:") + "LegalTrademarks=" + versionInfoValue
    CASE "ORIGINALFILENAME"
        viOriginalFilename$ = versionInfoValue
        layout$ = SCase$("$VersionInfo:") + "OriginalFilename=" + versionInfoValue
    CASE "PRODUCTNAME"
        viProductName$ = versionInfoValue
        layout$ = SCase$("$VersionInfo:") + "ProductName=" + versionInfoValue
    CASE "PRODUCTVERSION"
        viProductVersion$ = versionInfoValue
        layout$ = SCase$("$VersionInfo:") + "ProductVersion=" + versionInfoValue
    CASE "COMMENTS"
        viComments$ = versionInfoValue
        layout$ = SCase$("$VersionInfo:") + "Comments=" + versionInfoValue
    CASE "WEB"
        viWeb$ = versionInfoValue
        layout$ = SCase$("$VersionInfo:") + "Web=" + versionInfoValue
    CASE ELSE
        a$ = "Invalid key. (Use FILEVERSION#, PRODUCTVERSION#, CompanyName, FileDescription, FileVersion, InternalName, LegalCopyright, LegalTrademarks, OriginalFilename, ProductName, ProductVersion, Comments or Web)"
        HandleVersionInfoDirective% = 2
        EXIT FUNCTION
    END SELECT

    VersionInfoSet = -1
    HandleVersionInfoDirective% = 1
END FUNCTION

FUNCTION HandleExeIconDirective% (upperLine AS STRING, rawLine AS STRING)
    DIM firstDelimiter AS LONG
    DIM secondDelimiter AS LONG
    DIM exeIconFile AS STRING
    DIM iconPath AS STRING
    DIM exeIconFileOnly AS STRING
    DIM currentdir$
    DIM iconfilehandle AS LONG
    HandleExeIconDirective% = 0
    IF LEFT$(upperLine, 8) <> "$EXEICON" THEN EXIT FUNCTION

    IF ExeIconSet THEN
        a$ = "$EXEICON already defined"
        HandleExeIconDirective% = 2
        EXIT FUNCTION
    END IF

    firstDelimiter = INSTR(upperLine, "'")
    IF firstDelimiter = 0 THEN
        a$ = "Expected $EXEICON:'filename'"
        HandleExeIconDirective% = 2
        EXIT FUNCTION
    END IF

    secondDelimiter = INSTR(firstDelimiter + 1, upperLine, "'")
    IF secondDelimiter = 0 THEN
        a$ = "Expected $EXEICON:'filename'"
        HandleExeIconDirective% = 2
        EXIT FUNCTION
    END IF

    exeIconFile = RTRIM$(LTRIM$(MID$(rawLine, firstDelimiter + 1, secondDelimiter - firstDelimiter - 1)))
    IF LEN(exeIconFile) = 0 THEN
        a$ = "Expected $EXEICON:'filename'"
        HandleExeIconDirective% = 2
        EXIT FUNCTION
    END IF

    layout$ = SCase$("$ExeIcon:'") + exeIconFile + "'" + MID$(rawLine, secondDelimiter + 1)

    IF INSTR(_OS$, "WIN") THEN
        iconPath = ""
        IF LEFT$(exeIconFile, 2) = "./" OR LEFT$(exeIconFile, 2) = ".\" THEN
            iconPath = path.source$
            IF LEN(iconPath) > 0 AND RIGHT$(iconPath, 1) <> pathsep$ THEN iconPath = iconPath + pathsep$
            exeIconFile = iconPath + MID$(exeIconFile, 3)
        ELSEIF INSTR(exeIconFile, "/") OR INSTR(exeIconFile, "\") THEN
            FOR i = LEN(exeIconFile) TO 1 STEP -1
                IF MID$(exeIconFile, i, 1) = "/" OR MID$(exeIconFile, i, 1) = "\" THEN
                    iconPath = LEFT$(exeIconFile, i)
                    exeIconFileOnly = MID$(exeIconFile, i + 1)

                    IF _DIREXISTS(iconPath) = 0 THEN
                        a$ = "File '" + exeIconFileOnly + "' not found"
                        HandleExeIconDirective% = 2
                        EXIT FUNCTION
                    END IF

                    currentdir$ = _CWD$
                    CHDIR iconPath
                    iconPath = _CWD$
                    CHDIR currentdir$

                    exeIconFile = iconPath + pathsep$ + exeIconFileOnly
                    EXIT FOR
                END IF
            NEXT
        END IF

        IF _FILEEXISTS(exeIconFile) = 0 THEN
            IF LEN(iconPath) THEN
                a$ = "File '" + MID$(exeIconFile, LEN(iconPath) + 1) + "' not found"
            ELSE
                a$ = "File '" + exeIconFile + "' not found"
            END IF
            HandleExeIconDirective% = 2
            EXIT FUNCTION
        END IF

        iconfilehandle = FREEFILE
        OPEN tmpdir$ + "icon.rc" FOR OUTPUT AS #iconfilehandle
        PRINT #iconfilehandle, "0 ICON " + QuotedFilename$(StrReplace$(exeIconFile, "\", "/"))
        CLOSE #iconfilehandle
        IF _FILEEXISTS(tmpdir$ + "icon.rc") = 0 THEN
            a$ = "Error creating icon resource file"
            HandleExeIconDirective% = 2
            EXIT FUNCTION
        END IF
    END IF

    ExeIconSet = linenumber
    SetDependency DEPENDENCY_ICON
    IF NoChecks = 0 THEN PRINT #12, "do{"
    PRINT #12, "sub__icon(NULL,NULL,0);"
    HandleExeIconDirective% = 3
END FUNCTION
