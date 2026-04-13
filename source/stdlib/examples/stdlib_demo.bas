' ============================================================================
' QBNex Standard Library Demo
' ============================================================================

'$IMPORT:'qbnex'

TYPE Animal
    Header AS QBNex_ObjectHeader
    Name AS STRING * 32
    Age AS INTEGER
END TYPE

TYPE Dog
    Header AS QBNex_ObjectHeader
    Name AS STRING * 32
    Age AS INTEGER
    Breed AS STRING * 32
END TYPE

SUB RegisterDomain ()
    DIM animalClassID AS LONG
    DIM dogClassID AS LONG

    animalClassID = QBNEX_FindClass("Animal")
    IF animalClassID <> 0 THEN EXIT SUB

    animalClassID = QBNEX_RegisterClass("Animal", 0)
    dogClassID = QBNEX_RegisterClass("Dog", animalClassID)

    QBNEX_RegisterMethod animalClassID, "Describe", 1
    QBNEX_RegisterMethod dogClassID, "Describe", 1
    QBNEX_RegisterMethod dogClassID, "Bark", 2
    QBNEX_RegisterInterface dogClassID, "IPet"
END SUB

SUB New_Dog (dog AS Dog, petName AS STRING, age AS INTEGER, breed AS STRING)
    DIM dogClassID AS LONG

    dogClassID = QBNEX_FindClass("Dog")
    IF dogClassID = 0 THEN
        RegisterDomain
        dogClassID = QBNEX_FindClass("Dog")
    END IF

    QBNEX_ObjectInit dog.Header, dogClassID
    dog.Name = petName
    dog.Age = age
    dog.Breed = breed
END SUB

SUB Demo_Collections ()
    DIM modules AS QBNex_List
    DIM loadOrder AS QBNex_Queue
    DIM features AS QBNex_HashSet
    DIM history AS QBNex_Stack
    DIM report AS QBNex_StringBuilder

    List_Init modules
    Queue_Init loadOrder
    HashSet_Init features
    Stack_Init history

    List_Add modules, "oop.class"
    List_Add modules, "oop.interface"
    List_Add modules, "collections.list"
    List_Add modules, "collections.stack"
    List_Add modules, "collections.queue"
    List_Add modules, "collections.set"
    List_Add modules, "strings.strbuilder"
    List_Add modules, "strings.text"
    List_Add modules, "sys.env"
    List_Add modules, "sys.args"
    List_Add modules, "io.path"
    List_Add modules, "io.csv"
    List_Add modules, "math.numeric"

    Queue_Enqueue loadOrder, "core"
    Queue_Enqueue loadOrder, "collections"
    Queue_Enqueue loadOrder, "text"

    HashSet_Add features, "OOP"
    HashSet_Add features, "Collections"
    HashSet_Add features, "CSV"
    HashSet_Add features, "Math"

    Stack_Push history, "init"
    Stack_Push history, "registry"
    Stack_Push history, "ready"

    SB_Init report
    SB_AppendLine report, "Loaded modules:"
    SB_AppendLine report, "  " + List_Join$(modules, ", ")
    SB_AppendLine report, "Queue head: " + Queue_Peek$(loadOrder)
    SB_AppendLine report, "Set members: " + HashSet_ToString$(features, " | ")
    SB_AppendLine report, "Latest stack item: " + Stack_Peek$(history)

    PRINT SB_ToString$(report)

    SB_Free report
    Stack_Free history
    HashSet_Free features
    Queue_Free loadOrder
    List_Free modules
END SUB

SUB Demo_OOP ()
    DIM pet AS Dog

    New_Dog pet, "Buddy", 3, "Collie"

    PRINT "Pet class: "; QBNEX_ObjectClassName$(pet.Header)
    PRINT "Is Animal: "; QBNEX_ObjectIs&(pet.Header, "Animal")
    PRINT "Is Dog: "; QBNEX_ObjectIs&(pet.Header, "Dog")
    PRINT "Implements IPet: "; QBNEX_Implements&(pet.Header.ClassID, "IPet")
    PRINT "Describe slot: "; QBNEX_FindMethodSlot&(pet.Header.ClassID, "Describe")
    PRINT "Bark slot: "; QBNEX_FindMethodSlot&(pet.Header.ClassID, "Bark")
END SUB

SUB Demo_System ()
    DIM nowValue AS QBNex_Date

    Date_SetNow nowValue
    PRINT "Platform: "; Env_Platform$
    PRINT "64-bit: "; Env_Is64Bit&
    PRINT "Home: "; Env_GetHome$
    PRINT "Joined path: "; Path_Join$(Env_GetHome$, "qbnex/demo/output.txt")
    PRINT "File name: "; Path_FileName$("src/stdlib/demo.bas")
    PRINT "Arg count: "; Args_Count&
    PRINT "CSV sample: "; CSV_Row3$("module", "status", "ok")
    PRINT "Pad right: "; Text_PadRight$("QBNex", 10, ".")
    PRINT "Clamp: "; Math_Clamp#(12#, 0#, 10#)
    PRINT "Date ISO: "; Date_ToISOString$(nowValue)
    PRINT "Date.now: "; Date_NowMs#
END SUB

SUB Demo_Data ()
    DIM metadata AS QBNex_Dictionary
    DIM outcome AS QBNex_Result

    Dict_Init metadata
    Dict_Set metadata, "name", "QBNex"
    Dict_Set metadata, "layer", "stdlib"

    PRINT "Dict count: "; Dict_Count&(metadata)
    PRINT "Dict name: "; Dict_Get$(metadata, "name", "")
    PRINT "JSON sample: "; Json_Object3$("name", Json_String$(Dict_Get$(metadata, "name", "")), "layer", Json_String$(Dict_Get$(metadata, "layer", "")), "status", Json_String$("ok"))

    Result_Ok outcome, "stable"
    PRINT "Result ok: "; Result_IsOk&(outcome)
    PRINT "Result value: "; Result_Value$(outcome, "")

    Dict_Free metadata
END SUB

SUB StdLibDemo ()
    CLS
    QBNex_StdLib_PrintInfo
    PRINT

    RegisterDomain
    Demo_Collections
    Demo_OOP
    PRINT
    Demo_System
    PRINT
    Demo_Data

    PRINT
    PRINT "Standard library demo complete."
END SUB
