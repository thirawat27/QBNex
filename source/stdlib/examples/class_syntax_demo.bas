' ============================================================================
' QBNex Native Class Syntax Demo
' ============================================================================

'$IMPORT:'qbnex'

CLASS Animal
    Name AS STRING * 32

    CONSTRUCTOR (petName AS STRING)
        ME.Name = petName
    END CONSTRUCTOR

    METHOD Describe$ ()
        Describe$ = "Animal:" + RTRIM$(ME.Name)
    END METHOD
END CLASS

CLASS Dog EXTENDS Animal IMPLEMENTS IPet, IWalker
    Breed AS STRING * 32

    CONSTRUCTOR (petName AS STRING, breedName AS STRING)
        THIS.Name = petName
        THIS.Breed = breedName
    END CONSTRUCTOR

    FUNCTION Describe$ ()
        Describe$ = "Dog:" + RTRIM$(ME.Name) + ":" + RTRIM$(THIS.Breed)
    END FUNCTION

    METHOD Rename (nextName AS STRING)
        ME.Name = nextName
    END METHOD
END CLASS

SUB ClassSyntaxDemo ()
    DIM pet AS Dog

    __QBNEX_Dog_CTOR pet, "Buddy", "Collie"
    PRINT pet.Describe$()
    CALL pet.Rename("Scout")

    PRINT pet.Describe$()
    PRINT QBNEX_ObjectClassName$(pet.Header)
    PRINT QBNEX_ObjectIs&(pet.Header, "Animal")
    PRINT QBNEX_Implements&(pet.Header.ClassID, "IPet")
END SUB
