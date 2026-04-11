' ============================================================================
' QBNex Standard Library - OOP Foundation: Class System
' ============================================================================
' Implements vtable-based class registry for OOP support in QBNex
' Provides runtime type information and inheritance checking
' ============================================================================

' Class information structure
TYPE QBNex_ClassInfo
    ClassName AS STRING * 64
    BaseClassID AS LONG
    MethodCount AS LONG
END TYPE

' Method slot structure
TYPE QBNex_MethodSlot
    MethodName AS STRING * 64
    MethodIndex AS LONG
END TYPE

' Global class registry (max 256 classes)
DIM SHARED QBNEX_ClassRegistry(1 TO 256) AS QBNex_ClassInfo
DIM SHARED QBNEX_ClassCount AS LONG

' Method registry (256 classes × 64 methods each)
DIM SHARED QBNEX_MethodRegistry(1 TO 256, 1 TO 64) AS QBNex_MethodSlot

' ============================================================================
' FUNCTION: QBNEX_RegisterClass
' Register a new class in the vtable registry
' ============================================================================
FUNCTION QBNEX_RegisterClass& (className AS STRING, baseClassID AS LONG)
    QBNEX_ClassCount = QBNEX_ClassCount + 1
    IF QBNEX_ClassCount > 256 THEN
        PRINT "ERROR: Maximum class limit (256) exceeded"
        SYSTEM
    END IF
    
    QBNEX_ClassRegistry(QBNEX_ClassCount).ClassName = className
    QBNEX_ClassRegistry(QBNEX_ClassCount).BaseClassID = baseClassID
    QBNEX_ClassRegistry(QBNEX_ClassCount).MethodCount = 0
    
    QBNEX_RegisterClass = QBNEX_ClassCount
END FUNCTION

' ============================================================================
' FUNCTION: QBNEX_RegisterMethod
' Register a method for a class
' ============================================================================
SUB QBNEX_RegisterMethod (classID AS LONG, methodName AS STRING, methodIndex AS LONG)
    IF classID < 1 OR classID > QBNEX_ClassCount THEN EXIT SUB
    
    DIM methodCount AS LONG
    methodCount = QBNEX_ClassRegistry(classID).MethodCount + 1
    
    IF methodCount > 64 THEN
        PRINT "ERROR: Maximum method limit (64) exceeded for class "; QBNEX_ClassRegistry(classID).ClassName
        EXIT SUB
    END IF
    
    QBNEX_MethodRegistry(classID, methodCount).MethodName = methodName
    QBNEX_MethodRegistry(classID, methodCount).MethodIndex = methodIndex
    QBNEX_ClassRegistry(classID).MethodCount = methodCount
END SUB

' ============================================================================
' FUNCTION: QBNEX_IsInstance
' Check if an object is an instance of a class (supports inheritance)
' ============================================================================
FUNCTION QBNEX_IsInstance& (vtableID AS LONG, className AS STRING)
    DIM currentID AS LONG
    currentID = vtableID
    
    ' Walk the inheritance chain
    DO WHILE currentID > 0 AND currentID <= QBNEX_ClassCount
        IF RTRIM$(QBNEX_ClassRegistry(currentID).ClassName) = className THEN
            QBNEX_IsInstance = -1
            EXIT FUNCTION
        END IF
        currentID = QBNEX_ClassRegistry(currentID).BaseClassID
    LOOP
    
    QBNEX_IsInstance = 0
END FUNCTION

' ============================================================================
' FUNCTION: QBNEX_GetClassName
' Get the class name for a vtable ID
' ============================================================================
FUNCTION QBNEX_GetClassName$ (vtableID AS LONG)
    IF vtableID > 0 AND vtableID <= QBNEX_ClassCount THEN
        QBNEX_GetClassName = RTRIM$(QBNEX_ClassRegistry(vtableID).ClassName)
    ELSE
        QBNEX_GetClassName = ""
    END IF
END FUNCTION

' ============================================================================
' FUNCTION: QBNEX_FindClass
' Find class ID by name
' ============================================================================
FUNCTION QBNEX_FindClass& (className AS STRING)
    DIM i AS LONG
    FOR i = 1 TO QBNEX_ClassCount
        IF RTRIM$(QBNEX_ClassRegistry(i).ClassName) = className THEN
            QBNEX_FindClass = i
            EXIT FUNCTION
        END IF
    NEXT i
    QBNEX_FindClass = 0
END FUNCTION
