' ============================================================================
' QBNex Standard Library
' ============================================================================
' Preferred entrypoint:
'   '$IMPORT:'qbnex'
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

TYPE QBNex_InterfaceInfo
    InterfaceName AS STRING * 64
END TYPE

TYPE QBNex_List
    Handle AS LONG
    Count AS LONG
END TYPE

TYPE QBNex_Stack
    Items AS QBNex_List
END TYPE

TYPE QBNex_Queue
    Items AS QBNex_List
END TYPE

TYPE QBNex_HashSet
    Items AS QBNex_List
END TYPE

TYPE QBNex_Dictionary
    Keys AS QBNex_List
    Values AS QBNex_List
END TYPE

TYPE QBNex_StringBuilder
    Buffer AS STRING
    PartCount AS LONG
END TYPE

TYPE QBNex_Result
    Ok AS LONG
    Code AS LONG
    Value AS STRING
    Message AS STRING
    Context AS STRING
    Source AS STRING
    Cause AS STRING
END TYPE

TYPE QBNex_Date
    Year AS LONG
    Month AS LONG
    Day AS LONG
    Hour AS LONG
    Minute AS LONG
    Second AS LONG
    Millisecond AS LONG
END TYPE

CONST QBNEX_RESULT_CODE_OK = 0
CONST QBNEX_RESULT_CODE_ERROR = 1

DIM SHARED QBNEX_ClassCount AS LONG
DIM SHARED QBNEX_ClassRegistry(1 TO 256) AS QBNex_ClassInfo
DIM SHARED QBNEX_MethodRegistry(1 TO 256, 1 TO 64) AS QBNex_MethodSlot

DIM SHARED QBNEX_InterfaceCount AS LONG
DIM SHARED QBNEX_InterfaceRegistry(1 TO 256) AS QBNex_InterfaceInfo
DIM SHARED QBNEX_ClassInterfaceCount(1 TO 256) AS LONG
DIM SHARED QBNEX_ClassInterfaces(1 TO 256, 1 TO 32) AS LONG

DIM SHARED QBNEX_ListPool(1 TO 256) AS STRING
DIM SHARED QBNEX_ListPoolUsed(1 TO 256) AS LONG
DIM SHARED QBNEX_ListPoolCount(1 TO 256) AS LONG

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
    QBNEX_FindClass = 0
END FUNCTION

FUNCTION QBNEX_EnsureClass& (className AS STRING, baseClassID AS LONG)
    DIM classID AS LONG

    classID = QBNEX_FindClass(className)
    IF classID = 0 THEN classID = QBNEX_RegisterClass(className, baseClassID)
    QBNEX_EnsureClass = classID
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
        QBNEX_RegisterClass = 0
        EXIT FUNCTION
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
        EXIT SUB
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
    QBNEX_FindMethodSlot = 0
END FUNCTION

FUNCTION QBNEX_ClassName$ (classID AS LONG)
    IF classID > 0 AND classID <= QBNEX_ClassCount THEN
        QBNEX_ClassName = RTRIM$(QBNEX_ClassRegistry(classID).ClassName)
    ELSE
        QBNEX_ClassName = ""
    END IF
END FUNCTION

FUNCTION QBNEX_IsInstance& (classID AS LONG, className AS STRING)
    DIM lookupID AS LONG
    DIM currentID AS LONG

    lookupID = QBNEX_FindClass(className)
    IF lookupID = 0 THEN
        QBNEX_IsInstance = 0
        EXIT FUNCTION
    END IF

    currentID = classID
    DO WHILE currentID > 0 AND currentID <= QBNEX_ClassCount
        IF currentID = lookupID THEN
            QBNEX_IsInstance = -1
            EXIT FUNCTION
        END IF
        currentID = QBNEX_ClassRegistry(currentID).BaseClassID
    LOOP
    QBNEX_IsInstance = 0
END FUNCTION

SUB QBNEX_ObjectInit (header AS QBNex_ObjectHeader, classID AS LONG)
    header.ClassID = classID
    header.Flags = 0
END SUB

FUNCTION QBNEX_ObjectClassName$ (header AS QBNex_ObjectHeader)
    QBNEX_ObjectClassName = QBNEX_ClassName$(header.ClassID)
END FUNCTION

FUNCTION QBNEX_ObjectIs& (header AS QBNex_ObjectHeader, className AS STRING)
    QBNEX_ObjectIs = QBNEX_IsInstance&(header.ClassID, className)
END FUNCTION

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
    QBNEX_FindInterface = 0
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
        QBNEX_RegisterInterfaceName = 0
        EXIT FUNCTION
    END IF

    QBNEX_InterfaceRegistry(QBNEX_InterfaceCount).InterfaceName = RTRIM$(interfaceName)
    QBNEX_RegisterInterfaceName = QBNEX_InterfaceCount
END FUNCTION

FUNCTION QBNEX_EnsureInterfaceName& (interfaceName AS STRING)
    DIM interfaceID AS LONG

    interfaceID = QBNEX_FindInterface(interfaceName)
    IF interfaceID = 0 THEN interfaceID = QBNEX_RegisterInterfaceName(interfaceName)
    QBNEX_EnsureInterfaceName = interfaceID
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
        PRINT "ERROR: Maximum interface limit exceeded"
        EXIT SUB
    END IF

    QBNEX_ClassInterfaces(classID, count) = interfaceID
    QBNEX_ClassInterfaceCount(classID) = count
END SUB

FUNCTION QBNEX_Implements& (classID AS LONG, interfaceName AS STRING)
    DIM interfaceID AS LONG
    DIM currentID AS LONG
    DIM index AS LONG

    interfaceID = QBNEX_FindInterface(interfaceName)
    IF interfaceID = 0 THEN
        QBNEX_Implements = 0
        EXIT FUNCTION
    END IF

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
    QBNEX_Implements = 0
END FUNCTION

SUB List_Init (listRef AS QBNex_List)
    DIM i AS LONG

    FOR i = 1 TO 256
        IF QBNEX_ListPoolUsed(i) = 0 THEN
            listRef.Handle = i
            listRef.Count = 0
            QBNEX_ListPoolUsed(i) = -1
            QBNEX_ListPool(i) = ""
            QBNEX_ListPoolCount(i) = 0
            EXIT SUB
        END IF
    NEXT

    PRINT "ERROR: List pool exhausted"
    listRef.Handle = 0
    listRef.Count = 0
END SUB

FUNCTION List_Count& (listRef AS QBNex_List)
    List_Count = listRef.Count
END FUNCTION

SUB List_Add (listRef AS QBNex_List, item AS STRING)
    DIM handle AS LONG

    handle = listRef.Handle
    IF handle < 1 OR handle > 256 THEN EXIT SUB

    QBNEX_ListPool(handle) = QBNEX_ListPool(handle) + MKL$(LEN(item)) + item
    QBNEX_ListPoolCount(handle) = QBNEX_ListPoolCount(handle) + 1
    listRef.Count = QBNEX_ListPoolCount(handle)
END SUB

FUNCTION List_Get$ (listRef AS QBNex_List, index AS LONG)
    DIM handle AS LONG
    DIM position AS LONG
    DIM currentIndex AS LONG
    DIM itemLength AS LONG

    handle = listRef.Handle
    IF handle < 1 OR handle > 256 THEN EXIT FUNCTION
    IF index < 0 OR index >= QBNEX_ListPoolCount(handle) THEN EXIT FUNCTION

    position = 1
    FOR currentIndex = 0 TO QBNEX_ListPoolCount(handle) - 1
        itemLength = CVL(MID$(QBNEX_ListPool(handle), position, 4))
        position = position + 4
        IF currentIndex = index THEN
            List_Get = MID$(QBNEX_ListPool(handle), position, itemLength)
            EXIT FUNCTION
        END IF
        position = position + itemLength
    NEXT
END FUNCTION

SUB List_Set (listRef AS QBNex_List, index AS LONG, item AS STRING)
    DIM rebuilt AS STRING
    DIM i AS LONG
    DIM currentValue AS STRING

    IF index < 0 OR index >= listRef.Count THEN EXIT SUB

    FOR i = 0 TO listRef.Count - 1
        IF i = index THEN
            rebuilt = rebuilt + MKL$(LEN(item)) + item
        ELSE
            currentValue = List_Get$(listRef, i)
            rebuilt = rebuilt + MKL$(LEN(currentValue)) + currentValue
        END IF
    NEXT

    QBNEX_ListPool(listRef.Handle) = rebuilt
END SUB

FUNCTION List_IndexOf& (listRef AS QBNex_List, item AS STRING)
    DIM i AS LONG

    FOR i = 0 TO listRef.Count - 1
        IF List_Get$(listRef, i) = item THEN
            List_IndexOf = i
            EXIT FUNCTION
        END IF
    NEXT

    List_IndexOf = -1
END FUNCTION

FUNCTION List_Contains& (listRef AS QBNex_List, item AS STRING)
    IF List_IndexOf&(listRef, item) >= 0 THEN List_Contains = -1
END FUNCTION

SUB List_RemoveAt (listRef AS QBNex_List, index AS LONG)
    DIM rebuilt AS STRING
    DIM i AS LONG
    DIM currentValue AS STRING

    IF index < 0 OR index >= listRef.Count THEN EXIT SUB

    FOR i = 0 TO listRef.Count - 1
        IF i <> index THEN
            currentValue = List_Get$(listRef, i)
            rebuilt = rebuilt + MKL$(LEN(currentValue)) + currentValue
        END IF
    NEXT

    QBNEX_ListPool(listRef.Handle) = rebuilt
    QBNEX_ListPoolCount(listRef.Handle) = QBNEX_ListPoolCount(listRef.Handle) - 1
    listRef.Count = QBNEX_ListPoolCount(listRef.Handle)
END SUB

SUB List_Clear (listRef AS QBNex_List)
    IF listRef.Handle < 1 OR listRef.Handle > 256 THEN EXIT SUB

    QBNEX_ListPool(listRef.Handle) = ""
    QBNEX_ListPoolCount(listRef.Handle) = 0
    listRef.Count = 0
END SUB

FUNCTION List_Join$ (listRef AS QBNex_List, separator AS STRING)
    DIM i AS LONG
    DIM joined AS STRING

    FOR i = 0 TO listRef.Count - 1
        IF i > 0 THEN joined = joined + separator
        joined = joined + List_Get$(listRef, i)
    NEXT

    List_Join = joined
END FUNCTION

SUB List_Free (listRef AS QBNex_List)
    IF listRef.Handle < 1 OR listRef.Handle > 256 THEN EXIT SUB

    QBNEX_ListPool(listRef.Handle) = ""
    QBNEX_ListPoolCount(listRef.Handle) = 0
    QBNEX_ListPoolUsed(listRef.Handle) = 0
    listRef.Handle = 0
    listRef.Count = 0
END SUB

SUB Stack_Init (stackRef AS QBNex_Stack)
    List_Init stackRef.Items
END SUB

SUB Stack_Push (stackRef AS QBNex_Stack, item AS STRING)
    List_Add stackRef.Items, item
END SUB

FUNCTION Stack_Peek$ (stackRef AS QBNex_Stack)
    IF stackRef.Items.Count = 0 THEN
        Stack_Peek = ""
        EXIT FUNCTION
    END IF
    Stack_Peek = List_Get$(stackRef.Items, stackRef.Items.Count - 1)
END FUNCTION

FUNCTION Stack_Pop$ (stackRef AS QBNex_Stack)
    DIM valueText AS STRING

    IF stackRef.Items.Count = 0 THEN
        Stack_Pop = ""
        EXIT FUNCTION
    END IF
    valueText = List_Get$(stackRef.Items, stackRef.Items.Count - 1)
    List_RemoveAt stackRef.Items, stackRef.Items.Count - 1
    Stack_Pop = valueText
END FUNCTION

FUNCTION Stack_Count& (stackRef AS QBNex_Stack)
    Stack_Count = stackRef.Items.Count
END FUNCTION

SUB Stack_Clear (stackRef AS QBNex_Stack)
    List_Clear stackRef.Items
END SUB

SUB Stack_Free (stackRef AS QBNex_Stack)
    List_Free stackRef.Items
END SUB

SUB Queue_Init (queueRef AS QBNex_Queue)
    List_Init queueRef.Items
END SUB

SUB Queue_Enqueue (queueRef AS QBNex_Queue, item AS STRING)
    List_Add queueRef.Items, item
END SUB

FUNCTION Queue_Peek$ (queueRef AS QBNex_Queue)
    IF queueRef.Items.Count = 0 THEN
        Queue_Peek = ""
        EXIT FUNCTION
    END IF
    Queue_Peek = List_Get$(queueRef.Items, 0)
END FUNCTION

FUNCTION Queue_Dequeue$ (queueRef AS QBNex_Queue)
    DIM valueText AS STRING

    IF queueRef.Items.Count = 0 THEN
        Queue_Dequeue = ""
        EXIT FUNCTION
    END IF
    valueText = List_Get$(queueRef.Items, 0)
    List_RemoveAt queueRef.Items, 0
    Queue_Dequeue = valueText
END FUNCTION

FUNCTION Queue_Count& (queueRef AS QBNex_Queue)
    Queue_Count = queueRef.Items.Count
END FUNCTION

SUB Queue_Clear (queueRef AS QBNex_Queue)
    List_Clear queueRef.Items
END SUB

SUB Queue_Free (queueRef AS QBNex_Queue)
    List_Free queueRef.Items
END SUB

SUB HashSet_Init (setRef AS QBNex_HashSet)
    List_Init setRef.Items
END SUB

SUB HashSet_Add (setRef AS QBNex_HashSet, item AS STRING)
    IF List_Contains&(setRef.Items, item) THEN EXIT SUB
    List_Add setRef.Items, item
END SUB

FUNCTION HashSet_Contains& (setRef AS QBNex_HashSet, item AS STRING)
    HashSet_Contains = List_Contains&(setRef.Items, item)
END FUNCTION

SUB HashSet_Remove (setRef AS QBNex_HashSet, item AS STRING)
    DIM index AS LONG

    index = List_IndexOf&(setRef.Items, item)
    IF index >= 0 THEN List_RemoveAt setRef.Items, index
END SUB

FUNCTION HashSet_Count& (setRef AS QBNex_HashSet)
    HashSet_Count = setRef.Items.Count
END FUNCTION

FUNCTION HashSet_ToString$ (setRef AS QBNex_HashSet, separator AS STRING)
    HashSet_ToString = List_Join$(setRef.Items, separator)
END FUNCTION

SUB HashSet_Clear (setRef AS QBNex_HashSet)
    List_Clear setRef.Items
END SUB

SUB HashSet_Free (setRef AS QBNex_HashSet)
    List_Free setRef.Items
END SUB

SUB Dict_Init (dictRef AS QBNex_Dictionary)
    List_Init dictRef.Keys
    List_Init dictRef.Values
END SUB

SUB Dict_Set (dictRef AS QBNex_Dictionary, keyText AS STRING, valueText AS STRING)
    DIM index AS LONG

    index = List_IndexOf&(dictRef.Keys, keyText)
    IF index >= 0 THEN
        List_Set dictRef.Values, index, valueText
    ELSE
        List_Add dictRef.Keys, keyText
        List_Add dictRef.Values, valueText
    END IF
END SUB

FUNCTION Dict_Get$ (dictRef AS QBNex_Dictionary, keyText AS STRING, defaultValue AS STRING)
    DIM index AS LONG

    index = List_IndexOf&(dictRef.Keys, keyText)
    IF index >= 0 THEN
        Dict_Get = List_Get$(dictRef.Values, index)
    ELSE
        Dict_Get = defaultValue
    END IF
END FUNCTION

FUNCTION Dict_Has& (dictRef AS QBNex_Dictionary, keyText AS STRING)
    IF List_IndexOf&(dictRef.Keys, keyText) >= 0 THEN Dict_Has = -1
END FUNCTION

SUB Dict_Remove (dictRef AS QBNex_Dictionary, keyText AS STRING)
    DIM index AS LONG

    index = List_IndexOf&(dictRef.Keys, keyText)
    IF index < 0 THEN EXIT SUB
    List_RemoveAt dictRef.Keys, index
    List_RemoveAt dictRef.Values, index
END SUB

FUNCTION Dict_Count& (dictRef AS QBNex_Dictionary)
    Dict_Count = dictRef.Keys.Count
END FUNCTION

FUNCTION Dict_KeyAt$ (dictRef AS QBNex_Dictionary, itemIndex AS LONG)
    Dict_KeyAt = List_Get$(dictRef.Keys, itemIndex)
END FUNCTION

FUNCTION Dict_ValueAt$ (dictRef AS QBNex_Dictionary, itemIndex AS LONG)
    Dict_ValueAt = List_Get$(dictRef.Values, itemIndex)
END FUNCTION

SUB Dict_Clear (dictRef AS QBNex_Dictionary)
    List_Clear dictRef.Keys
    List_Clear dictRef.Values
END SUB

SUB Dict_Free (dictRef AS QBNex_Dictionary)
    List_Free dictRef.Keys
    List_Free dictRef.Values
END SUB

SUB SB_Init (builder AS QBNex_StringBuilder)
    builder.Buffer = ""
    builder.PartCount = 0
END SUB

SUB SB_Append (builder AS QBNex_StringBuilder, text AS STRING)
    builder.Buffer = builder.Buffer + MKL$(LEN(text)) + text
    builder.PartCount = builder.PartCount + 1
END SUB

SUB SB_AppendLine (builder AS QBNex_StringBuilder, text AS STRING)
    SB_Append builder, text + CHR$(13) + CHR$(10)
END SUB

SUB SB_Clear (builder AS QBNex_StringBuilder)
    builder.Buffer = ""
    builder.PartCount = 0
END SUB

FUNCTION SB_Length& (builder AS QBNex_StringBuilder)
    DIM position AS LONG
    DIM itemLength AS LONG
    DIM index AS LONG
    DIM totalLength AS LONG

    position = 1
    FOR index = 1 TO builder.PartCount
        itemLength = CVL(MID$(builder.Buffer, position, 4))
        position = position + 4
        totalLength = totalLength + itemLength
        position = position + itemLength
    NEXT

    SB_Length = totalLength
END FUNCTION

FUNCTION SB_ToString$ (builder AS QBNex_StringBuilder)
    DIM position AS LONG
    DIM itemLength AS LONG
    DIM index AS LONG
    DIM assembled AS STRING

    position = 1
    FOR index = 1 TO builder.PartCount
        itemLength = CVL(MID$(builder.Buffer, position, 4))
        position = position + 4
        assembled = assembled + MID$(builder.Buffer, position, itemLength)
        position = position + itemLength
    NEXT

    SB_ToString = assembled
END FUNCTION

SUB SB_Free (builder AS QBNex_StringBuilder)
    SB_Clear builder
END SUB

FUNCTION Text_Repeat$ (valueText AS STRING, repeatCount AS LONG)
    DIM index AS LONG
    DIM resultText AS STRING

    IF repeatCount <= 0 THEN
        Text_Repeat = ""
        EXIT FUNCTION
    END IF
    FOR index = 1 TO repeatCount
        resultText = resultText + valueText
    NEXT
    Text_Repeat = resultText
END FUNCTION

FUNCTION Text_StartsWith& (valueText AS STRING, prefixText AS STRING)
    IF LEN(prefixText) = 0 THEN
        Text_StartsWith = -1
        EXIT FUNCTION
    END IF
    IF LEN(valueText) < LEN(prefixText) THEN
        Text_StartsWith = 0
        EXIT FUNCTION
    END IF
    IF LEFT$(valueText, LEN(prefixText)) = prefixText THEN
        Text_StartsWith = -1
    ELSE
        Text_StartsWith = 0
    END IF
END FUNCTION

FUNCTION Text_EndsWith& (valueText AS STRING, suffixText AS STRING)
    IF LEN(suffixText) = 0 THEN
        Text_EndsWith = -1
        EXIT FUNCTION
    END IF
    IF LEN(valueText) < LEN(suffixText) THEN
        Text_EndsWith = 0
        EXIT FUNCTION
    END IF
    IF RIGHT$(valueText, LEN(suffixText)) = suffixText THEN
        Text_EndsWith = -1
    ELSE
        Text_EndsWith = 0
    END IF
END FUNCTION

FUNCTION Text_Contains& (valueText AS STRING, searchText AS STRING)
    IF INSTR(valueText, searchText) <> 0 THEN
        Text_Contains = -1
    ELSE
        Text_Contains = 0
    END IF
END FUNCTION

FUNCTION Text_PadLeft$ (valueText AS STRING, totalWidth AS LONG, padText AS STRING)
    DIM resultText AS STRING
    DIM deficit AS LONG

    resultText = valueText
    IF LEN(padText) = 0 THEN padText = " "
    deficit = totalWidth - LEN(resultText)
    IF deficit <= 0 THEN
        Text_PadLeft = resultText
        EXIT FUNCTION
    END IF
    Text_PadLeft = Text_Repeat$(padText, deficit) + resultText
END FUNCTION

FUNCTION Text_PadRight$ (valueText AS STRING, totalWidth AS LONG, padText AS STRING)
    DIM resultText AS STRING
    DIM deficit AS LONG

    resultText = valueText
    IF LEN(padText) = 0 THEN padText = " "
    deficit = totalWidth - LEN(resultText)
    IF deficit <= 0 THEN
        Text_PadRight = resultText
        EXIT FUNCTION
    END IF
    Text_PadRight = resultText + Text_Repeat$(padText, deficit)
END FUNCTION

FUNCTION Json_Escape$ (valueText AS STRING)
    DIM index AS LONG
    DIM currentChar AS STRING
    DIM resultText AS STRING

    FOR index = 1 TO LEN(valueText)
        currentChar = MID$(valueText, index, 1)
        SELECT CASE ASC(currentChar)
        CASE 34
            resultText = resultText + CHR$(92) + CHR$(34)
        CASE 92
            resultText = resultText + CHR$(92) + CHR$(92)
        CASE 9
            resultText = resultText + CHR$(92) + "t"
        CASE 10
            resultText = resultText + CHR$(92) + "n"
        CASE 13
            resultText = resultText + CHR$(92) + "r"
        CASE ELSE
            resultText = resultText + currentChar
        END SELECT
    NEXT

    Json_Escape = resultText
END FUNCTION

FUNCTION Json_String$ (valueText AS STRING)
    Json_String = CHR$(34) + Json_Escape$(valueText) + CHR$(34)
END FUNCTION

FUNCTION Json_Pair$ (keyText AS STRING, valueJson AS STRING)
    Json_Pair = Json_String$(keyText) + ":" + valueJson
END FUNCTION

FUNCTION Json_Object2$ (keyA AS STRING, valueAJson AS STRING, keyB AS STRING, valueBJson AS STRING)
    Json_Object2 = "{" + Json_Pair$(keyA, valueAJson) + "," + Json_Pair$(keyB, valueBJson) + "}"
END FUNCTION

FUNCTION Json_Object3$ (keyA AS STRING, valueAJson AS STRING, keyB AS STRING, valueBJson AS STRING, keyC AS STRING, valueCJson AS STRING)
    Json_Object3 = "{" + Json_Pair$(keyA, valueAJson) + "," + Json_Pair$(keyB, valueBJson) + "," + Json_Pair$(keyC, valueCJson) + "}"
END FUNCTION

FUNCTION Json_Array2$ (valueAJson AS STRING, valueBJson AS STRING)
    Json_Array2 = "[" + valueAJson + "," + valueBJson + "]"
END FUNCTION

FUNCTION Json_Array3$ (valueAJson AS STRING, valueBJson AS STRING, valueCJson AS STRING)
    Json_Array3 = "[" + valueAJson + "," + valueBJson + "," + valueCJson + "]"
END FUNCTION

FUNCTION Env_Get$ (varName AS STRING, defaultValue AS STRING)
    DIM value AS STRING

    value = ENVIRON$(varName)
    IF LEN(value) = 0 THEN
        Env_Get = defaultValue
    ELSE
        Env_Get = value
    END IF
END FUNCTION

FUNCTION Env_Has& (varName AS STRING)
    Env_Has = LEN(ENVIRON$(varName)) <> 0
END FUNCTION

FUNCTION Env_Platform$ ()
    IF INSTR(_OS$, "WIN") THEN
        Env_Platform = "WINDOWS"
    ELSEIF INSTR(_OS$, "LINUX") THEN
        Env_Platform = "LINUX"
    ELSEIF INSTR(_OS$, "MAC") THEN
        Env_Platform = "MACOS"
    ELSE
        Env_Platform = "UNKNOWN"
    END IF
END FUNCTION

FUNCTION Env_Is64Bit& ()
    Env_Is64Bit = INSTR(_OS$, "64BIT") <> 0
END FUNCTION

FUNCTION Env_GetHome$ ()
    IF INSTR(_OS$, "WIN") THEN
        Env_GetHome = Env_Get$("USERPROFILE", "C:\")
    ELSE
        Env_GetHome = Env_Get$("HOME", "/")
    END IF
END FUNCTION

FUNCTION Args_Count& ()
    Args_Count = _COMMANDCOUNT
END FUNCTION

FUNCTION Args_Get$ (argIndex AS LONG, defaultValue AS STRING)
    IF argIndex < 1 OR argIndex > _COMMANDCOUNT THEN
        Args_Get = defaultValue
    ELSE
        Args_Get = COMMAND$(argIndex)
    END IF
END FUNCTION

FUNCTION Args_Program$ ()
    Args_Program = COMMAND$(0)
END FUNCTION

FUNCTION Args_All$ ()
    DIM argIndex AS LONG
    DIM resultText AS STRING

    FOR argIndex = 1 TO _COMMANDCOUNT
        IF argIndex > 1 THEN resultText = resultText + " "
        resultText = resultText + COMMAND$(argIndex)
    NEXT
    Args_All = resultText
END FUNCTION

FUNCTION Date_Pad2$ (valueNumber AS LONG)
    DIM text AS STRING

    text = LTRIM$(RTRIM$(STR$(valueNumber)))
    IF LEN(text) < 2 THEN text = "0" + text
    Date_Pad2 = text
END FUNCTION

FUNCTION Date_Pad3$ (valueNumber AS LONG)
    DIM text AS STRING

    text = LTRIM$(RTRIM$(STR$(valueNumber)))
    DO WHILE LEN(text) < 3
        text = "0" + text
    LOOP
    Date_Pad3 = text
END FUNCTION

FUNCTION Date_PartValue& (sourceText AS STRING, startPos AS LONG, endPos AS LONG)
    IF endPos < startPos THEN
        Date_PartValue = 0
        EXIT FUNCTION
    END IF
    Date_PartValue = VAL(MID$(sourceText, startPos, endPos - startPos + 1))
END FUNCTION

FUNCTION Date_IsLeapYear& (yearValue AS LONG)
    IF (yearValue MOD 400) = 0 THEN
        Date_IsLeapYear = -1
        EXIT FUNCTION
    END IF
    IF (yearValue MOD 100) = 0 THEN
        Date_IsLeapYear = 0
        EXIT FUNCTION
    END IF
    IF (yearValue MOD 4) = 0 THEN
        Date_IsLeapYear = -1
        EXIT FUNCTION
    END IF
    Date_IsLeapYear = 0
END FUNCTION

FUNCTION Date_DaysInMonth& (yearValue AS LONG, monthValue AS LONG)
    SELECT CASE monthValue
    CASE 1, 3, 5, 7, 8, 10, 12
        Date_DaysInMonth = 31
    CASE 4, 6, 9, 11
        Date_DaysInMonth = 30
    CASE 2
        Date_DaysInMonth = 28
        IF Date_IsLeapYear&(yearValue) THEN Date_DaysInMonth = 29
    END SELECT
END FUNCTION

SUB Date_FromParts (dateRef AS QBNex_Date, yearValue AS LONG, monthIndex AS LONG, dayValue AS LONG, hourValue AS LONG, minuteValue AS LONG, secondValue AS LONG, millisecondValue AS LONG)
    dateRef.Year = yearValue
    dateRef.Month = monthIndex + 1
    dateRef.Day = dayValue
    dateRef.Hour = hourValue
    dateRef.Minute = minuteValue
    dateRef.Second = secondValue
    dateRef.Millisecond = millisecondValue
END SUB

SUB Date_SetNow (dateRef AS QBNex_Date)
    DIM rawDate AS STRING
    DIM rawTime AS STRING
    DIM firstSep AS LONG
    DIM secondSep AS LONG
    DIM timerValue AS DOUBLE

    rawDate = DATE$
    rawTime = TIME$

    firstSep = INSTR(rawDate, "/")
    IF firstSep = 0 THEN firstSep = INSTR(rawDate, "-")
    secondSep = INSTR(firstSep + 1, rawDate, "/")
    IF secondSep = 0 THEN secondSep = INSTR(firstSep + 1, rawDate, "-")

    dateRef.Month = Date_PartValue&(rawDate, 1, firstSep - 1)
    dateRef.Day = Date_PartValue&(rawDate, firstSep + 1, secondSep - 1)
    dateRef.Year = VAL(MID$(rawDate, secondSep + 1))

    dateRef.Hour = VAL(LEFT$(rawTime, 2))
    dateRef.Minute = VAL(MID$(rawTime, 4, 2))
    dateRef.Second = VAL(MID$(rawTime, 7, 2))

    timerValue = TIMER
    dateRef.Millisecond = INT((timerValue - INT(timerValue)) * 1000#)
END SUB

FUNCTION Date_ValueOf# (dateRef AS QBNex_Date)
    DIM yearValue AS LONG
    DIM monthValue AS LONG
    DIM totalDays AS DOUBLE

    FOR yearValue = 1970 TO dateRef.Year - 1
        totalDays = totalDays + 365
        IF Date_IsLeapYear&(yearValue) THEN totalDays = totalDays + 1
    NEXT

    FOR monthValue = 1 TO dateRef.Month - 1
        totalDays = totalDays + Date_DaysInMonth&(dateRef.Year, monthValue)
    NEXT

    totalDays = totalDays + (dateRef.Day - 1)
    Date_ValueOf = (((((totalDays * 24#) + dateRef.Hour) * 60# + dateRef.Minute) * 60# + dateRef.Second) * 1000#) + dateRef.Millisecond
END FUNCTION

FUNCTION Date_NowMs# ()
    DIM nowValue AS QBNex_Date

    Date_SetNow nowValue
    Date_NowMs = Date_ValueOf#(nowValue)
END FUNCTION

SUB Date_FromUnixMs (dateRef AS QBNex_Date, epochMs AS DOUBLE)
    DIM totalSeconds AS DOUBLE
    DIM wholeDays AS LONG
    DIM secondsOfDay AS LONG
    DIM yearValue AS LONG
    DIM monthValue AS LONG
    DIM yearDays AS LONG

    totalSeconds = INT(epochMs / 1000#)
    dateRef.Millisecond = epochMs - totalSeconds * 1000#

    wholeDays = INT(totalSeconds / 86400#)
    secondsOfDay = totalSeconds - wholeDays * 86400

    yearValue = 1970
    DO
        yearDays = 365
        IF Date_IsLeapYear&(yearValue) THEN yearDays = 366
        IF wholeDays < yearDays THEN EXIT DO
        wholeDays = wholeDays - yearDays
        yearValue = yearValue + 1
    LOOP

    monthValue = 1
    DO
        yearDays = Date_DaysInMonth&(yearValue, monthValue)
        IF wholeDays < yearDays THEN EXIT DO
        wholeDays = wholeDays - yearDays
        monthValue = monthValue + 1
    LOOP

    dateRef.Year = yearValue
    dateRef.Month = monthValue
    dateRef.Day = wholeDays + 1
    dateRef.Hour = INT(secondsOfDay / 3600)
    secondsOfDay = secondsOfDay - dateRef.Hour * 3600
    dateRef.Minute = INT(secondsOfDay / 60)
    dateRef.Second = secondsOfDay - dateRef.Minute * 60
END SUB

FUNCTION Date_GetFullYear& (dateRef AS QBNex_Date)
    Date_GetFullYear = dateRef.Year
END FUNCTION

FUNCTION Date_GetMonth& (dateRef AS QBNex_Date)
    Date_GetMonth = dateRef.Month - 1
END FUNCTION

FUNCTION Date_GetDate& (dateRef AS QBNex_Date)
    Date_GetDate = dateRef.Day
END FUNCTION

FUNCTION Date_GetHours& (dateRef AS QBNex_Date)
    Date_GetHours = dateRef.Hour
END FUNCTION

FUNCTION Date_GetMinutes& (dateRef AS QBNex_Date)
    Date_GetMinutes = dateRef.Minute
END FUNCTION

FUNCTION Date_GetSeconds& (dateRef AS QBNex_Date)
    Date_GetSeconds = dateRef.Second
END FUNCTION

FUNCTION Date_GetMilliseconds& (dateRef AS QBNex_Date)
    Date_GetMilliseconds = dateRef.Millisecond
END FUNCTION

FUNCTION Date_GetDay& (dateRef AS QBNex_Date)
    DIM daysSinceEpoch AS DOUBLE

    daysSinceEpoch = INT(Date_ValueOf#(dateRef) / 86400000#)
    Date_GetDay = (daysSinceEpoch + 4) MOD 7
END FUNCTION

FUNCTION Date_ToISOString$ (dateRef AS QBNex_Date)
    Date_ToISOString = LTRIM$(RTRIM$(STR$(dateRef.Year))) + "-" + Date_Pad2$(dateRef.Month) + "-" + Date_Pad2$(dateRef.Day) + "T" + Date_Pad2$(dateRef.Hour) + ":" + Date_Pad2$(dateRef.Minute) + ":" + Date_Pad2$(dateRef.Second) + "." + Date_Pad3$(dateRef.Millisecond)
END FUNCTION

FUNCTION Date_ToJSON$ (dateRef AS QBNex_Date)
    DIM isoText AS STRING

    isoText = Date_ToISOString$(dateRef)
    Date_ToJSON = CHR$(34) + isoText + CHR$(34)
END FUNCTION

FUNCTION Path_Separator$ ()
    IF INSTR(_OS$, "WIN") THEN
        Path_Separator = "\"
    ELSE
        Path_Separator = "/"
    END IF
END FUNCTION

FUNCTION Path_Normalize$ (rawPath AS STRING)
    DIM i AS LONG
    DIM separator AS STRING
    DIM pathChar AS STRING
    DIM previousWasSeparator AS LONG
    DIM normalizedPath AS STRING

    separator = Path_Separator$
    FOR i = 1 TO LEN(rawPath)
        pathChar = MID$(rawPath, i, 1)
        IF pathChar = "\" OR pathChar = "/" THEN
            IF previousWasSeparator = 0 THEN
                normalizedPath = normalizedPath + separator
                previousWasSeparator = -1
            END IF
        ELSE
            normalizedPath = normalizedPath + pathChar
            previousWasSeparator = 0
        END IF
    NEXT

    Path_Normalize = normalizedPath
END FUNCTION

FUNCTION Path_Join$ (basePath AS STRING, leafPath AS STRING)
    DIM separator AS STRING

    separator = Path_Separator$
    basePath = Path_Normalize$(basePath)
    leafPath = Path_Normalize$(leafPath)

    IF LEN(basePath) = 0 THEN
        Path_Join = leafPath
        EXIT FUNCTION
    END IF
    IF LEN(leafPath) = 0 THEN
        Path_Join = basePath
        EXIT FUNCTION
    END IF

    IF RIGHT$(basePath, 1) = separator THEN
        Path_Join = basePath + leafPath
    ELSE
        Path_Join = basePath + separator + leafPath
    END IF
END FUNCTION

FUNCTION Path_FileName$ (rawPath AS STRING)
    DIM i AS LONG
    DIM normalized AS STRING
    DIM separator AS STRING
    DIM position AS LONG

    normalized = Path_Normalize$(rawPath)
    separator = Path_Separator$

    FOR i = 1 TO LEN(normalized)
        IF MID$(normalized, i, 1) = separator THEN position = i
    NEXT

    IF position = 0 THEN
        Path_FileName = normalized
    ELSE
        Path_FileName = MID$(normalized, position + 1)
    END IF
END FUNCTION

FUNCTION Path_DirName$ (rawPath AS STRING)
    DIM i AS LONG
    DIM normalized AS STRING
    DIM separator AS STRING
    DIM position AS LONG

    normalized = Path_Normalize$(rawPath)
    separator = Path_Separator$

    FOR i = 1 TO LEN(normalized)
        IF MID$(normalized, i, 1) = separator THEN position = i
    NEXT

    IF position = 0 THEN
        Path_DirName = ""
    ELSE
        Path_DirName = LEFT$(normalized, position - 1)
    END IF
END FUNCTION

FUNCTION Path_Extension$ (rawPath AS STRING)
    DIM i AS LONG
    DIM filename AS STRING
    DIM position AS LONG

    filename = Path_FileName$(rawPath)
    FOR i = 1 TO LEN(filename)
        IF MID$(filename, i, 1) = "." THEN position = i
    NEXT

    IF position = 0 THEN
        Path_Extension = ""
    ELSE
        Path_Extension = MID$(filename, position)
    END IF
END FUNCTION

FUNCTION CSV_Escape$ (valueText AS STRING)
    DIM index AS LONG
    DIM currentChar AS STRING
    DIM escapedValue AS STRING
    DIM mustQuote AS LONG

    FOR index = 1 TO LEN(valueText)
        currentChar = MID$(valueText, index, 1)
        IF currentChar = CHR$(34) THEN
            escapedValue = escapedValue + CHR$(34) + CHR$(34)
            mustQuote = -1
        ELSE
            escapedValue = escapedValue + currentChar
            IF currentChar = "," OR currentChar = CHR$(13) OR currentChar = CHR$(10) THEN mustQuote = -1
        END IF
    NEXT

    IF INSTR(valueText, CHR$(34)) THEN mustQuote = -1
    IF mustQuote THEN
        CSV_Escape = CHR$(34) + escapedValue + CHR$(34)
    ELSE
        CSV_Escape = escapedValue
    END IF
END FUNCTION

FUNCTION CSV_Row2$ (valueA AS STRING, valueB AS STRING)
    CSV_Row2 = CSV_Escape$(valueA) + "," + CSV_Escape$(valueB)
END FUNCTION

FUNCTION CSV_Row3$ (valueA AS STRING, valueB AS STRING, valueC AS STRING)
    CSV_Row3 = CSV_Escape$(valueA) + "," + CSV_Escape$(valueB) + "," + CSV_Escape$(valueC)
END FUNCTION

FUNCTION CSV_Row4$ (valueA AS STRING, valueB AS STRING, valueC AS STRING, valueD AS STRING)
    CSV_Row4 = CSV_Escape$(valueA) + "," + CSV_Escape$(valueB) + "," + CSV_Escape$(valueC) + "," + CSV_Escape$(valueD)
END FUNCTION

FUNCTION Math_Min# (valueA AS DOUBLE, valueB AS DOUBLE)
    IF valueA <= valueB THEN Math_Min = valueA ELSE Math_Min = valueB
END FUNCTION

FUNCTION Math_Max# (valueA AS DOUBLE, valueB AS DOUBLE)
    IF valueA >= valueB THEN Math_Max = valueA ELSE Math_Max = valueB
END FUNCTION

FUNCTION Math_Clamp# (valueX AS DOUBLE, minValue AS DOUBLE, maxValue AS DOUBLE)
    IF valueX < minValue THEN
        Math_Clamp = minValue
    ELSEIF valueX > maxValue THEN
        Math_Clamp = maxValue
    ELSE
        Math_Clamp = valueX
    END IF
END FUNCTION

FUNCTION Math_Lerp# (valueA AS DOUBLE, valueB AS DOUBLE, factor AS DOUBLE)
    Math_Lerp = valueA + (valueB - valueA) * factor
END FUNCTION

FUNCTION Math_Deg2Rad# (degreesValue AS DOUBLE)
    Math_Deg2Rad = degreesValue * 3.141592653589793# / 180#
END FUNCTION

FUNCTION Math_Rad2Deg# (radiansValue AS DOUBLE)
    Math_Rad2Deg = radiansValue * 180# / 3.141592653589793#
END FUNCTION

FUNCTION Result_TrimText$ (text AS STRING)
    Result_TrimText$ = LTRIM$(RTRIM$(text))
END FUNCTION

FUNCTION Result_PrependDetail$ (existingText AS STRING, newText AS STRING)
    DIM normalizedExisting AS STRING
    DIM normalizedNew AS STRING

    normalizedExisting = Result_TrimText$(existingText)
    normalizedNew = Result_TrimText$(newText)

    IF normalizedNew = "" THEN
        Result_PrependDetail$ = normalizedExisting
    ELSEIF normalizedExisting = "" THEN
        Result_PrependDetail$ = normalizedNew
    ELSEIF UCASE$(normalizedExisting) = UCASE$(normalizedNew) THEN
        Result_PrependDetail$ = normalizedExisting
    ELSEIF INSTR(UCASE$(normalizedExisting), UCASE$(normalizedNew)) > 0 THEN
        Result_PrependDetail$ = normalizedExisting
    ELSE
        Result_PrependDetail$ = normalizedNew + " -> " + normalizedExisting
    END IF
END FUNCTION

SUB Result_Clear (resultRef AS QBNex_Result)
    resultRef.Ok = 0
    resultRef.Code = QBNEX_RESULT_CODE_OK
    resultRef.Value = ""
    resultRef.Message = ""
    resultRef.Context = ""
    resultRef.Source = ""
    resultRef.Cause = ""
END SUB

SUB Result_Copy (resultRef AS QBNex_Result, sourceResult AS QBNex_Result)
    resultRef.Ok = sourceResult.Ok
    resultRef.Code = sourceResult.Code
    resultRef.Value = sourceResult.Value
    resultRef.Message = sourceResult.Message
    resultRef.Context = sourceResult.Context
    resultRef.Source = sourceResult.Source
    resultRef.Cause = sourceResult.Cause
END SUB

SUB Result_Ok (resultRef AS QBNex_Result, valueText AS STRING)
    Result_Clear resultRef
    resultRef.Ok = -1
    resultRef.Value = valueText
END SUB

SUB Result_Fail (resultRef AS QBNex_Result, messageText AS STRING)
    Result_FailCode resultRef, QBNEX_RESULT_CODE_ERROR, messageText
END SUB

SUB Result_FailCode (resultRef AS QBNex_Result, errorCode AS LONG, messageText AS STRING)
    resultRef.Ok = 0
    resultRef.Code = errorCode
    resultRef.Value = ""
    resultRef.Message = Result_TrimText$(messageText)
    resultRef.Context = ""
    resultRef.Source = ""
    resultRef.Cause = ""
END SUB

SUB Result_FailWithContext (resultRef AS QBNex_Result, errorCode AS LONG, messageText AS STRING, contextText AS STRING, sourceText AS STRING)
    Result_FailCode resultRef, errorCode, messageText
    resultRef.Context = Result_TrimText$(contextText)
    resultRef.Source = Result_TrimText$(sourceText)
END SUB

SUB Result_AddContext (resultRef AS QBNex_Result, contextText AS STRING)
    IF resultRef.Ok THEN EXIT SUB
    resultRef.Context = Result_PrependDetail$(resultRef.Context, contextText)
END SUB

SUB Result_SetSource (resultRef AS QBNex_Result, sourceText AS STRING)
    IF resultRef.Ok THEN EXIT SUB
    resultRef.Source = Result_TrimText$(sourceText)
END SUB

SUB Result_SetCause (resultRef AS QBNex_Result, causeText AS STRING)
    IF resultRef.Ok THEN EXIT SUB
    resultRef.Cause = Result_TrimText$(causeText)
END SUB

SUB Result_Propagate (resultRef AS QBNex_Result, sourceResult AS QBNex_Result, contextText AS STRING, sourceText AS STRING)
    Result_Copy resultRef, sourceResult

    IF sourceResult.Ok THEN EXIT SUB

    Result_AddContext resultRef, contextText

    IF Result_TrimText$(sourceText) <> "" THEN resultRef.Source = Result_TrimText$(sourceText)
    IF Result_TrimText$(resultRef.Cause) = "" THEN resultRef.Cause = Result_ErrorChain$(sourceResult)
END SUB

FUNCTION Result_IsOk& (resultRef AS QBNex_Result)
    Result_IsOk = resultRef.Ok
END FUNCTION

FUNCTION Result_IsError& (resultRef AS QBNex_Result)
    IF resultRef.Ok THEN
        Result_IsError = 0
    ELSE
        Result_IsError = -1
    END IF
END FUNCTION

FUNCTION Result_Code& (resultRef AS QBNex_Result)
    Result_Code = resultRef.Code
END FUNCTION

FUNCTION Result_Value$ (resultRef AS QBNex_Result, defaultValue AS STRING)
    IF resultRef.Ok THEN
        Result_Value = resultRef.Value
    ELSE
        Result_Value = defaultValue
    END IF
END FUNCTION

FUNCTION Result_Message$ (resultRef AS QBNex_Result)
    Result_Message = resultRef.Message
END FUNCTION

FUNCTION Result_Context$ (resultRef AS QBNex_Result)
    Result_Context = resultRef.Context
END FUNCTION

FUNCTION Result_Source$ (resultRef AS QBNex_Result)
    Result_Source = resultRef.Source
END FUNCTION

FUNCTION Result_Cause$ (resultRef AS QBNex_Result)
    Result_Cause = resultRef.Cause
END FUNCTION

FUNCTION Result_UnwrapOr$ (resultRef AS QBNex_Result, defaultValue AS STRING)
    Result_UnwrapOr$ = Result_Value$(resultRef, defaultValue)
END FUNCTION

FUNCTION Result_ErrorChain$ (resultRef AS QBNex_Result)
    DIM description AS STRING
    DIM codeText AS STRING

    description = Result_TrimText$(resultRef.Message)
    codeText = ""

    IF resultRef.Code <> QBNEX_RESULT_CODE_OK THEN codeText = "E" + LTRIM$(STR$(resultRef.Code))

    IF codeText <> "" THEN
        IF description = "" THEN
            description = "[" + codeText + "]"
        ELSE
            description = "[" + codeText + "] " + description
        END IF
    END IF

    IF Result_TrimText$(resultRef.Context) <> "" THEN
        IF description = "" THEN
            description = Result_TrimText$(resultRef.Context)
        ELSE
            description = Result_TrimText$(resultRef.Context) + ": " + description
        END IF
    END IF

    IF Result_TrimText$(resultRef.Source) <> "" THEN
        IF description = "" THEN
            description = "source=" + Result_TrimText$(resultRef.Source)
        ELSE
            description = description + " [source=" + Result_TrimText$(resultRef.Source) + "]"
        END IF
    END IF

    IF Result_TrimText$(resultRef.Cause) <> "" THEN
        IF description = "" THEN
            description = "cause: " + Result_TrimText$(resultRef.Cause)
        ELSE
            description = description + " | cause: " + Result_TrimText$(resultRef.Cause)
        END IF
    END IF

    Result_ErrorChain$ = description
END FUNCTION

FUNCTION Result_Describe$ (resultRef AS QBNex_Result)
    IF resultRef.Ok THEN
        Result_Describe$ = "ok: " + resultRef.Value
    ELSE
        Result_Describe$ = Result_ErrorChain$(resultRef)
    END IF
END FUNCTION

FUNCTION Result_Expect$ (resultRef AS QBNex_Result, expectationText AS STRING)
    IF resultRef.Ok THEN
        Result_Expect$ = resultRef.Value
        EXIT FUNCTION
    END IF

    expectationText = Result_TrimText$(expectationText)
    IF expectationText = "" THEN expectationText = "called Result_Expect on an error"

    PRINT "panic: "; expectationText
    PRINT "error: "; Result_ErrorChain$(resultRef)
    SYSTEM 1
END FUNCTION

FUNCTION QBNex_StdLib_Version$ ()
    QBNex_StdLib_Version = "1.0.0"
END FUNCTION

FUNCTION QBNex_StdLib_Info$ ()
    DIM text AS STRING

    text = "QBNex Standard Library v1.0.0" + CHR$(13) + CHR$(10)
    text = text + "Build: 2026.04.13" + CHR$(13) + CHR$(10)
    text = text + "Modules: oop, collections, strings, sys, io, math, data, result, datetime, class syntax"

    QBNex_StdLib_Info = text
END FUNCTION

SUB QBNex_StdLib_PrintInfo ()
    PRINT QBNex_StdLib_Info$
END SUB
