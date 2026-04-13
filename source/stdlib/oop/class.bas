' ============================================================================
' QBNex Standard Library - OOP Foundation: Class Registry
' ============================================================================
' The first field of any "class" UDT should be a QBNex_ObjectHeader.
' This keeps the model explicit and compatible with classic BASIC TYPEs.
' Native CLASS ... END CLASS syntax in the compiler lowers into these runtime
' primitives and a generated TYPE-compatible object layout.
' ============================================================================

TYPE QBNex_ObjectHeader
    ClassID AS LONG
    Flags AS LONG
END TYPE

TYPE QBNex_ClassInfo
    ClassName AS STRING * 64
    BaseClassID AS LONG
    MethodCount AS LONG
END TYPE

TYPE QBNex_MethodSlot
    MethodName AS STRING * 64
    MethodSlot AS LONG
END TYPE

DIM SHARED QBNEX_ClassCount AS LONG
DIM SHARED QBNEX_ClassRegistry(1 TO 256) AS QBNex_ClassInfo
DIM SHARED QBNEX_MethodRegistry(1 TO 256, 1 TO 64) AS QBNex_MethodSlot

FUNCTION QBNEX_FindClass& (className AS STRING)
    DIM i AS LONG
    DIM lookupName AS STRING

    lookupName = UCASE$(RTRIM$(className))
    FOR i = 1 TO QBNEX_ClassCount
        IF UCASE$(RTRIM$(QBNEX_ClassRegistry(i).ClassName)) = lookupName THEN
            QBNEX_FindClass = i
            EXIT FUNCTION
        END IF
    NEXT
END FUNCTION

FUNCTION QBNEX_RegisterClass& (className AS STRING, baseClassID AS LONG)
    DIM classID AS LONG

    classID = QBNEX_FindClass(className)
    IF classID <> 0 THEN
        QBNEX_RegisterClass = classID
        EXIT FUNCTION
    END IF

    QBNEX_ClassCount = QBNEX_ClassCount + 1
    IF QBNEX_ClassCount > 256 THEN
        PRINT "ERROR: Maximum class limit exceeded"
        SYSTEM 1
    END IF

    QBNEX_ClassRegistry(QBNEX_ClassCount).ClassName = RTRIM$(className)
    QBNEX_ClassRegistry(QBNEX_ClassCount).BaseClassID = baseClassID
    QBNEX_ClassRegistry(QBNEX_ClassCount).MethodCount = 0
    QBNEX_RegisterClass = QBNEX_ClassCount
END FUNCTION

SUB QBNEX_RegisterMethod (classID AS LONG, methodName AS STRING, methodSlot AS LONG)
    DIM index AS LONG
    DIM count AS LONG
    DIM lookupName AS STRING

    IF classID < 1 OR classID > QBNEX_ClassCount THEN EXIT SUB

    lookupName = UCASE$(RTRIM$(methodName))
    count = QBNEX_ClassRegistry(classID).MethodCount

    FOR index = 1 TO count
        IF UCASE$(RTRIM$(QBNEX_MethodRegistry(classID, index).MethodName)) = lookupName THEN
            QBNEX_MethodRegistry(classID, index).MethodSlot = methodSlot
            EXIT SUB
        END IF
    NEXT

    count = count + 1
    IF count > 64 THEN
        PRINT "ERROR: Maximum method limit exceeded for class "; RTRIM$(QBNEX_ClassRegistry(classID).ClassName)
        SYSTEM 1
    END IF

    QBNEX_MethodRegistry(classID, count).MethodName = RTRIM$(methodName)
    QBNEX_MethodRegistry(classID, count).MethodSlot = methodSlot
    QBNEX_ClassRegistry(classID).MethodCount = count
END SUB

FUNCTION QBNEX_FindMethodSlot& (classID AS LONG, methodName AS STRING)
    DIM currentID AS LONG
    DIM index AS LONG
    DIM lookupName AS STRING

    currentID = classID
    lookupName = UCASE$(RTRIM$(methodName))

    DO WHILE currentID > 0 AND currentID <= QBNEX_ClassCount
        FOR index = 1 TO QBNEX_ClassRegistry(currentID).MethodCount
            IF UCASE$(RTRIM$(QBNEX_MethodRegistry(currentID, index).MethodName)) = lookupName THEN
                QBNEX_FindMethodSlot = QBNEX_MethodRegistry(currentID, index).MethodSlot
                EXIT FUNCTION
            END IF
        NEXT
        currentID = QBNEX_ClassRegistry(currentID).BaseClassID
    LOOP
END FUNCTION

FUNCTION QBNEX_ClassName$ (classID AS LONG)
    IF classID > 0 AND classID <= QBNEX_ClassCount THEN
        QBNEX_ClassName = RTRIM$(QBNEX_ClassRegistry(classID).ClassName)
    END IF
END FUNCTION

FUNCTION QBNEX_IsInstance& (classID AS LONG, className AS STRING)
    DIM lookupID AS LONG
    DIM currentID AS LONG

    lookupID = QBNEX_FindClass(className)
    IF lookupID = 0 THEN EXIT FUNCTION

    currentID = classID
    DO WHILE currentID > 0 AND currentID <= QBNEX_ClassCount
        IF currentID = lookupID THEN
            QBNEX_IsInstance = -1
            EXIT FUNCTION
        END IF
        currentID = QBNEX_ClassRegistry(currentID).BaseClassID
    LOOP
END FUNCTION

SUB QBNEX_ObjectInit (header AS QBNex_ObjectHeader, classID AS LONG)
    header.ClassID = classID
    header.Flags = 0
END SUB

FUNCTION QBNEX_ObjectClassName$ (header AS QBNex_ObjectHeader)
    QBNEX_ObjectClassName = QBNEX_ClassName$(header.ClassID)
END FUNCTION

FUNCTION QBNEX_ObjectIs& (header AS QBNex_ObjectHeader, className AS STRING)
    QBNEX_ObjectIs = QBNEX_IsInstance(header.ClassID, className)
END FUNCTION
