'Dependency flags of the program (DEPENDENCY_*): the QIR emitter reads them (console only, GUI core).
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
    CASE "_KEYHIT", "_KEYDOWN", "_TITLE", "_ICON", "_EXIT", "_DISPLAY", "_FONT", "_PRINTSTRING", "_PUTIMAGE"
        'keys held down, the title bar, the close button and what is drawn exist only for a window
        RequiresGuiCore% = -1
        EXIT FUNCTION
    CASE "_RESIZE", "_RESIZEWIDTH", "_RESIZEHEIGHT", "_SCALEDWIDTH", "_SCALEDHEIGHT", "_ALLOWFULLSCREEN", "_WINDOWHASFOCUS", "_WINDOWHANDLE"
        RequiresGuiCore% = -1
        EXIT FUNCTION
    CASE "_ACCEPTFILEDROP", "_TOTALDROPPEDFILES", "_DROPPEDFILE", "_FINISHDROP"
        RequiresGuiCore% = -1
        EXIT FUNCTION
    CASE "_DEVICES", "_DEVICE", "_DEVICEINPUT", "_BUTTON", "_BUTTONCHANGE", "_AXIS", "_WHEEL", "_LASTBUTTON", "_LASTAXIS", "_LASTWHEEL"
        'the keyboard and the mouse are devices of the program's window
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

SUB SetDependency (requirement)
    IF requirement THEN
        DEPENDENCY(requirement) = 1
    END IF
END SUB
