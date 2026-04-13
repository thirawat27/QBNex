' ============================================================================
' QBNex Standard Library - OOP Foundation: Interfaces
' ============================================================================

TYPE QBNex_InterfaceInfo
    InterfaceName AS STRING * 64
END TYPE

DIM SHARED QBNEX_InterfaceCount AS LONG
DIM SHARED QBNEX_InterfaceRegistry(1 TO 256) AS QBNex_InterfaceInfo
DIM SHARED QBNEX_ClassInterfaceCount(1 TO 256) AS LONG
DIM SHARED QBNEX_ClassInterfaces(1 TO 256, 1 TO 32) AS LONG

FUNCTION QBNEX_FindInterface& (interfaceName AS STRING)
    DIM i AS LONG
    DIM lookupName AS STRING

    lookupName = UCASE$(RTRIM$(interfaceName))
    FOR i = 1 TO QBNEX_InterfaceCount
        IF UCASE$(RTRIM$(QBNEX_InterfaceRegistry(i).InterfaceName)) = lookupName THEN
            QBNEX_FindInterface = i
            EXIT FUNCTION
        END IF
    NEXT
END FUNCTION

FUNCTION QBNEX_RegisterInterfaceName& (interfaceName AS STRING)
    DIM interfaceID AS LONG

    interfaceID = QBNEX_FindInterface(interfaceName)
    IF interfaceID <> 0 THEN
        QBNEX_RegisterInterfaceName = interfaceID
        EXIT FUNCTION
    END IF

    QBNEX_InterfaceCount = QBNEX_InterfaceCount + 1
    IF QBNEX_InterfaceCount > 256 THEN
        PRINT "ERROR: Maximum interface limit exceeded"
        SYSTEM 1
    END IF

    QBNEX_InterfaceRegistry(QBNEX_InterfaceCount).InterfaceName = RTRIM$(interfaceName)
    QBNEX_RegisterInterfaceName = QBNEX_InterfaceCount
END FUNCTION

SUB QBNEX_RegisterInterface (classID AS LONG, interfaceName AS STRING)
    DIM interfaceID AS LONG
    DIM index AS LONG
    DIM count AS LONG

    IF classID < 1 OR classID > QBNEX_ClassCount THEN EXIT SUB

    interfaceID = QBNEX_RegisterInterfaceName(interfaceName)
    count = QBNEX_ClassInterfaceCount(classID)

    FOR index = 1 TO count
        IF QBNEX_ClassInterfaces(classID, index) = interfaceID THEN EXIT SUB
    NEXT

    count = count + 1
    IF count > 32 THEN
        PRINT "ERROR: Maximum interface limit exceeded for class "; RTRIM$(QBNEX_ClassRegistry(classID).ClassName)
        SYSTEM 1
    END IF

    QBNEX_ClassInterfaces(classID, count) = interfaceID
    QBNEX_ClassInterfaceCount(classID) = count
END SUB

FUNCTION QBNEX_Implements& (classID AS LONG, interfaceName AS STRING)
    DIM interfaceID AS LONG
    DIM currentID AS LONG
    DIM index AS LONG

    interfaceID = QBNEX_FindInterface(interfaceName)
    IF interfaceID = 0 THEN EXIT FUNCTION

    currentID = classID
    DO WHILE currentID > 0 AND currentID <= QBNEX_ClassCount
        FOR index = 1 TO QBNEX_ClassInterfaceCount(currentID)
            IF QBNEX_ClassInterfaces(currentID, index) = interfaceID THEN
                QBNEX_Implements = -1
                EXIT FUNCTION
            END IF
        NEXT
        currentID = QBNEX_ClassRegistry(currentID).BaseClassID
    LOOP
END FUNCTION
