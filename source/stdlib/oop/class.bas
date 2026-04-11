' =============================================================================
' QBNex Object-Oriented Programming Foundation — class.bas
' =============================================================================
'
' Implements CLASS / END CLASS syntax on top of QBNex's existing TYPE system.
'
' Design goals:
'   • Zero runtime overhead for simple value objects (stored as TYPEs)
'   • Virtual method dispatch via integer vtable indices
'   • Single-level inheritance via EXTENDS keyword
'   • Compatible with all QBNex data types and arrays
'
' -------------------------------------------------------------------
' Syntax reference implemented by the compiler pre-processor:
'
'   CLASS <Name> [EXTENDS <Base>]
'       [PRIVATE | PUBLIC | PROTECTED]
'       DIM <field> AS <Type>
'       ...
'       SUB <Method>([args])  ... END SUB
'       FUNCTION <Method>([args]) AS <Type>  ... END FUNCTION
'   END CLASS
'
'   DIM obj AS NEW <ClassName>     ' heap-allocated reference
'   DIM obj AS <ClassName>         ' stack/value object
'
'   obj.MethodName(args)
'   DESTROY obj                    ' explicit destructor call + dealloc
' -------------------------------------------------------------------
'
' HOW IT WORKS  (translator-level, no true vtable in BASIC):
'
'   The compiler pre-processor rewrites CLASS blocks into:
'     1. A TYPE definition containing all fields
'     2. A LONG field "__vtable_id" as the first element
'     3. Global SUB/FUNCTION wrappers named  <ClassName>_<Method>
'     4. A factory SUB  New_<ClassName>(obj AS <ClassName>)
'     5. A destructor  Destroy_<ClassName>(obj AS <ClassName>)
'
'   Method calls  obj.Foo(x)  are rewritten to
'                 <ClassName>_Foo(obj, x)
'
' =============================================================================

' ---------------------------------------------------------------------------
' OOP Runtime Support — VTable Registry
' ---------------------------------------------------------------------------

CONST QBNEX_MAX_CLASSES   = 256  ' max registered class types
CONST QBNEX_MAX_METHODS   = 64   ' max virtual methods per class
CONST QBNEX_CLASS_NONE    = 0    ' sentinel: no class

' Class descriptor record
TYPE QBNex_ClassInfo
    ClassName   AS STRING * 64   ' class name (upper-cased)
    BaseClassID AS LONG          ' ID of parent class (0 = no parent)
    MethodCount AS LONG          ' number of registered methods
END TYPE

' Method slot inside a class
TYPE QBNex_MethodSlot
    MethodName  AS STRING * 64   ' method name (upper-cased)
    MethodIndex AS LONG          ' positional index (for dispatch)
END TYPE

' Global class registry
DIM SHARED QBNEX_ClassRegistry(1 TO QBNEX_MAX_CLASSES)      AS QBNex_ClassInfo
DIM SHARED QBNEX_MethodRegistry(1 TO QBNEX_MAX_CLASSES, _
                                 1 TO QBNEX_MAX_METHODS)     AS QBNex_MethodSlot
DIM SHARED QBNEX_ClassCount AS LONG
QBNEX_ClassCount = 0

' ---------------------------------------------------------------------------
' FUNCTION  QBNEX_RegisterClass(name$, baseName$) AS LONG
'
'   Registers a class and returns its numeric class ID.
'   baseName$ = "" means no parent.
'
'   Called automatically by generated factory subs.
' ---------------------------------------------------------------------------
FUNCTION QBNEX_RegisterClass& (name$, baseName$)
    DIM i AS LONG, baseID AS LONG

    ' check for duplicates
    FOR i = 1 TO QBNEX_ClassCount
        IF UCASE$(RTRIM$(QBNEX_ClassRegistry(i).ClassName)) = UCASE$(name$) THEN
            QBNEX_RegisterClass& = i
            EXIT FUNCTION
        END IF
    NEXT i

    ' resolve base class
    baseID = QBNEX_CLASS_NONE
    IF LEN(TRIM$(baseName$)) > 0 THEN
        FOR i = 1 TO QBNEX_ClassCount
            IF UCASE$(RTRIM$(QBNEX_ClassRegistry(i).ClassName)) = UCASE$(baseName$) THEN
                baseID = i
                EXIT FOR
            END IF
        NEXT i
    END IF

    ' register
    QBNEX_ClassCount = QBNEX_ClassCount + 1
    IF QBNEX_ClassCount > QBNEX_MAX_CLASSES THEN
        PRINT "QBNex OOP Error: class registry overflow (max " + STR$(QBNEX_MAX_CLASSES) + ")"
        END 1
    END IF

    QBNEX_ClassRegistry(QBNEX_ClassCount).ClassName   = name$
    QBNEX_ClassRegistry(QBNEX_ClassCount).BaseClassID = baseID
    QBNEX_ClassRegistry(QBNEX_ClassCount).MethodCount = 0

    QBNEX_RegisterClass& = QBNEX_ClassCount
END FUNCTION

' ---------------------------------------------------------------------------
' FUNCTION  QBNEX_FindClass&(name$) AS LONG
'   Returns class ID or 0 if not found.
' ---------------------------------------------------------------------------
FUNCTION QBNEX_FindClass& (name$)
    DIM i AS LONG
    FOR i = 1 TO QBNEX_ClassCount
        IF UCASE$(RTRIM$(QBNEX_ClassRegistry(i).ClassName)) = UCASE$(name$) THEN
            QBNEX_FindClass& = i
            EXIT FUNCTION
        END IF
    NEXT i
    QBNEX_FindClass& = QBNEX_CLASS_NONE
END FUNCTION

' ---------------------------------------------------------------------------
' SUB  QBNEX_RegisterMethod(classID, methodName$)
'   Registers a method slot for a class.
' ---------------------------------------------------------------------------
SUB QBNEX_RegisterMethod (classID AS LONG, methodName$)
    DIM mc AS LONG
    IF classID < 1 OR classID > QBNEX_ClassCount THEN EXIT SUB
    mc = QBNEX_ClassRegistry(classID).MethodCount + 1
    IF mc > QBNEX_MAX_METHODS THEN
        PRINT "QBNex OOP Error: method registry overflow for class " + _
              TRIM$(QBNEX_ClassRegistry(classID).ClassName)
        END 1
    END IF
    QBNEX_MethodRegistry(classID, mc).MethodName  = methodName$
    QBNEX_MethodRegistry(classID, mc).MethodIndex = mc
    QBNEX_ClassRegistry(classID).MethodCount      = mc
END SUB

' ---------------------------------------------------------------------------
' FUNCTION  QBNEX_IsInstance&(vtableID, className$) AS LONG
'
'   Runtime type check — returns -1 (TRUE) if the object whose vtable ID
'   is vtableID is an instance of className$ (or a subclass thereof).
'
'   Usage (generated by compiler):
'
'     IF QBNEX_IsInstance(obj.__vtable_id, "Animal") THEN ...
' ---------------------------------------------------------------------------
FUNCTION QBNEX_IsInstance& (vtableID AS LONG, className$)
    DIM classID AS LONG, cur AS LONG
    classID = QBNEX_FindClass&(className$)
    IF classID = QBNEX_CLASS_NONE THEN QBNEX_IsInstance& = 0: EXIT FUNCTION
    cur = vtableID
    DO WHILE cur <> QBNEX_CLASS_NONE
        IF cur = classID THEN QBNEX_IsInstance& = -1: EXIT FUNCTION
        cur = QBNEX_ClassRegistry(cur).BaseClassID
    LOOP
    QBNEX_IsInstance& = 0
END FUNCTION

' ---------------------------------------------------------------------------
' SUB  QBNEX_DumpClasses
'   Debug helper — prints all registered classes and their methods.
' ---------------------------------------------------------------------------
SUB QBNEX_DumpClasses
    DIM i AS LONG, j AS LONG
    PRINT "--- QBNex Class Registry (" + STR$(QBNEX_ClassCount) + " classes) ---"
    FOR i = 1 TO QBNEX_ClassCount
        PRINT "  [" + STR$(i) + "] " + TRIM$(QBNEX_ClassRegistry(i).ClassName);
        IF QBNEX_ClassRegistry(i).BaseClassID > 0 THEN
            PRINT " EXTENDS " + TRIM$(QBNEX_ClassRegistry(QBNEX_ClassRegistry(i).BaseClassID).ClassName)
        ELSE
            PRINT
        END IF
        FOR j = 1 TO QBNEX_ClassRegistry(i).MethodCount
            PRINT "        ." + TRIM$(QBNEX_MethodRegistry(i, j).MethodName)
        NEXT j
    NEXT i
    PRINT "---------------------------------------------"
END SUB
