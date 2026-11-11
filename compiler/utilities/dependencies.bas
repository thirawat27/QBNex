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
