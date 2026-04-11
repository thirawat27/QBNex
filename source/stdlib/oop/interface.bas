' ============================================================================
' QBNex Standard Library - OOP Foundation: Interface System
' ============================================================================
' Implements interface contracts and IMPLEMENTS checking
' ============================================================================

' Interface implementation mapping
TYPE QBNex_InterfaceImpl
    ClassID AS LONG
    InterfaceName AS STRING * 64
END TYPE

' Global interface registry (max 512 implementations)
DIM SHARED QBNEX_InterfaceRegistry(1 TO 512) AS QBNex_InterfaceImpl
DIM SHARED QBNEX_InterfaceCount AS LONG

' ============================================================================
' SUB: QBNEX_RegisterInterface
' Register that a class implements an interface
' ============================================================================
SUB QBNEX_RegisterInterface (classID AS LONG, interfaceName AS STRING)
    QBNEX_InterfaceCount = QBNEX_InterfaceCount + 1
    IF QBNEX_InterfaceCount > 512 THEN
        PRINT "ERROR: Maximum interface implementation limit (512) exceeded"
        EXIT SUB
    END IF
    
    QBNEX_InterfaceRegistry(QBNEX_InterfaceCount).ClassID = classID
    QBNEX_InterfaceRegistry(QBNEX_InterfaceCount).InterfaceName = interfaceName
END SUB

' ============================================================================
' FUNCTION: QBNEX_Implements
' Check if a class implements an interface
' ============================================================================
FUNCTION QBNEX_Implements& (classID AS LONG, interfaceName AS STRING)
    DIM i AS LONG
    FOR i = 1 TO QBNEX_InterfaceCount
        IF QBNEX_InterfaceRegistry(i).ClassID = classID THEN
            IF RTRIM$(QBNEX_InterfaceRegistry(i).InterfaceName) = interfaceName THEN
                QBNEX_Implements = -1
                EXIT FUNCTION
            END IF
        END IF
    NEXT i
    QBNEX_Implements = 0
END FUNCTION

' ============================================================================
' FUNCTION: QBNEX_GetInterfaces
' Get all interfaces implemented by a class (comma-separated)
' ============================================================================
FUNCTION QBNEX_GetInterfaces$ (classID AS LONG)
    DIM result AS STRING
    DIM i AS LONG
    DIM first AS LONG
    
    first = -1
    result = ""
    
    FOR i = 1 TO QBNEX_InterfaceCount
        IF QBNEX_InterfaceRegistry(i).ClassID = classID THEN
            IF first THEN
                first = 0
            ELSE
                result = result + ","
            END IF
            result = result + RTRIM$(QBNEX_InterfaceRegistry(i).InterfaceName)
        END IF
    NEXT i
    
    QBNEX_GetInterfaces = result
END FUNCTION
