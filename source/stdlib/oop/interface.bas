' =============================================================================
' QBNex OOP — Interface & Implements Support — interface.bas
' =============================================================================
'
' An INTERFACE is a pure-abstract contract: no fields, only method signatures.
' A class declares IMPLEMENTS <InterfaceName> to satisfy the contract.
'
' Syntax:
'
'   INTERFACE ISerializable
'       FUNCTION Serialize$()
'       SUB Deserialize(data$)
'   END INTERFACE
'
'   CLASS JsonDocument IMPLEMENTS ISerializable
'       DIM content$ AS STRING
'       FUNCTION Serialize$()
'           Serialize$ = content$
'       END FUNCTION
'       SUB Deserialize(data$)
'           content$ = data$
'       END SUB
'   END CLASS
'
'   ' Compile-time check (enforced by pre-processor):
'   DIM doc AS NEW JsonDocument
'   DIM s AS ISerializable = doc   ' interface reference
'   PRINT s.Serialize$()
'
' =============================================================================

' ---------------------------------------------------------------------------
' Interface Registry
' ---------------------------------------------------------------------------

CONST QBNEX_MAX_INTERFACES     = 128
CONST QBNEX_MAX_IFACE_METHODS  = 32

TYPE QBNex_InterfaceInfo
    IFaceName    AS STRING * 64
    MethodCount  AS LONG
END TYPE

TYPE QBNex_IFaceMethodSlot
    MethodName   AS STRING * 64
    ReturnType   AS STRING * 32  ' "VOID", "LONG", "STRING", etc.
END TYPE

TYPE QBNex_ImplRecord
    ClassID      AS LONG
    InterfaceID  AS LONG
END TYPE

DIM SHARED QBNEX_IFaceRegistry(1 TO QBNEX_MAX_INTERFACES)  AS QBNex_InterfaceInfo
DIM SHARED QBNEX_IFaceMethodReg(1 TO QBNEX_MAX_INTERFACES, _
                                 1 TO QBNEX_MAX_IFACE_METHODS) AS QBNex_IFaceMethodSlot
DIM SHARED QBNEX_IFaceCount     AS LONG
QBNEX_IFaceCount = 0

CONST QBNEX_MAX_IMPL = 512
DIM SHARED QBNEX_ImplTable(1 TO QBNEX_MAX_IMPL) AS QBNex_ImplRecord
DIM SHARED QBNEX_ImplCount AS LONG
QBNEX_ImplCount = 0

' ---------------------------------------------------------------------------
' FUNCTION  QBNEX_RegisterInterface&(name$) AS LONG
' ---------------------------------------------------------------------------
FUNCTION QBNEX_RegisterInterface& (name$)
    DIM i AS LONG
    FOR i = 1 TO QBNEX_IFaceCount
        IF UCASE$(RTRIM$(QBNEX_IFaceRegistry(i).IFaceName)) = UCASE$(name$) THEN
            QBNEX_RegisterInterface& = i: EXIT FUNCTION
        END IF
    NEXT i
    QBNEX_IFaceCount = QBNEX_IFaceCount + 1
    IF QBNEX_IFaceCount > QBNEX_MAX_INTERFACES THEN
        PRINT "QBNex OOP Error: interface registry overflow"
        END 1
    END IF
    QBNEX_IFaceRegistry(QBNEX_IFaceCount).IFaceName   = name$
    QBNEX_IFaceRegistry(QBNEX_IFaceCount).MethodCount = 0
    QBNEX_RegisterInterface& = QBNEX_IFaceCount
END FUNCTION

' ---------------------------------------------------------------------------
' SUB  QBNEX_RegisterIFaceMethod(ifaceID, methodName$, returnType$)
' ---------------------------------------------------------------------------
SUB QBNEX_RegisterIFaceMethod (ifaceID AS LONG, methodName$, returnType$)
    DIM mc AS LONG
    IF ifaceID < 1 OR ifaceID > QBNEX_IFaceCount THEN EXIT SUB
    mc = QBNEX_IFaceRegistry(ifaceID).MethodCount + 1
    IF mc > QBNEX_MAX_IFACE_METHODS THEN
        PRINT "QBNex OOP Error: interface method overflow"
        END 1
    END IF
    QBNEX_IFaceMethodReg(ifaceID, mc).MethodName  = methodName$
    QBNEX_IFaceMethodReg(ifaceID, mc).ReturnType  = returnType$
    QBNEX_IFaceRegistry(ifaceID).MethodCount      = mc
END SUB

' ---------------------------------------------------------------------------
' SUB  QBNEX_RegisterImpl(classID, interfaceID)
'   Records that classID implements interfaceID.
' ---------------------------------------------------------------------------
SUB QBNEX_RegisterImpl (classID AS LONG, interfaceID AS LONG)
    QBNEX_ImplCount = QBNEX_ImplCount + 1
    IF QBNEX_ImplCount > QBNEX_MAX_IMPL THEN
        PRINT "QBNex OOP Error: implementation table overflow"
        END 1
    END IF
    QBNEX_ImplTable(QBNEX_ImplCount).ClassID     = classID
    QBNEX_ImplTable(QBNEX_ImplCount).InterfaceID = interfaceID
END SUB

' ---------------------------------------------------------------------------
' FUNCTION  QBNEX_Implements&(classID, interfaceID) AS LONG
'   Returns -1 if classID (or any ancestor) implements interfaceID.
' ---------------------------------------------------------------------------
FUNCTION QBNEX_Implements& (classID AS LONG, interfaceID AS LONG)
    DIM i AS LONG, cur AS LONG
    cur = classID
    DO WHILE cur > QBNEX_CLASS_NONE
        FOR i = 1 TO QBNEX_ImplCount
            IF QBNEX_ImplTable(i).ClassID     = cur AND _
               QBNEX_ImplTable(i).InterfaceID = interfaceID THEN
               QBNEX_Implements& = -1
               EXIT FUNCTION
            END IF
        NEXT i
        cur = QBNEX_ClassRegistry(cur).BaseClassID
    LOOP
    QBNEX_Implements& = 0
END FUNCTION
