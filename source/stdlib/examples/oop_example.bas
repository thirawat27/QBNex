' ============================================================================
' QBNex Standard Library - OOP Example
' ============================================================================
' Demonstrates object-oriented programming patterns using the stdlib
' ============================================================================

'$INCLUDE:'../qbnex_stdlib.bas'

' ============================================================================
' Define a simple "Animal" class hierarchy
' ============================================================================

' Animal base class
TYPE Animal
    __vtable_id AS LONG
    NAME AS STRING * 32
    Age AS INTEGER
END TYPE

' Dog class (inherits from Animal)
TYPE Dog
    __vtable_id AS LONG
    NAME AS STRING * 32
    Age AS INTEGER
    Breed AS STRING * 32
END TYPE

' Cat class (inherits from Animal)
TYPE Cat
    __vtable_id AS LONG
    NAME AS STRING * 32
    Age AS INTEGER
    COLOR AS STRING * 32
END TYPE

' ============================================================================
' Class IDs
' ============================================================================
DIM SHARED AnimalClassID AS LONG
DIM SHARED DogClassID AS LONG
DIM SHARED CatClassID AS LONG

' ============================================================================
' Initialize class registry
' ============================================================================
SUB InitializeClasses ()
    AnimalClassID = QBNEX_RegisterClass("Animal", 0)
    DogClassID = QBNEX_RegisterClass("Dog", AnimalClassID)
    CatClassID = QBNEX_RegisterClass("Cat", AnimalClassID)
    
    ' Register methods
    QBNEX_RegisterMethod AnimalClassID, "Speak", 1
    QBNEX_RegisterMethod DogClassID, "Speak", 1
    QBNEX_RegisterMethod DogClassID, "Fetch", 2
    QBNEX_RegisterMethod CatClassID, "Speak", 1
    QBNEX_RegisterMethod CatClassID, "Purr", 2
    
    ' Register interfaces
    QBNEX_RegisterInterface DogClassID, "IPet"
    QBNEX_RegisterInterface CatClassID, "IPet"
    
    PRINT "Classes registered:"
    PRINT "  Animal (ID: "; AnimalClassID; ")"
    PRINT "  Dog (ID: "; DogClassID; ", inherits from Animal)"
    PRINT "  Cat (ID: "; CatClassID; ", inherits from Animal)"
    PRINT
END SUB

' ============================================================================
' Factory functions
' ============================================================================
SUB New_Dog (dog AS Dog, NAME AS STRING, age AS INTEGER, breed AS STRING)
    dog.__vtable_id = DogClassID
    dog.NAME = NAME
    dog.Age = age
    dog.Breed = breed
END SUB

SUB New_Cat (cat AS Cat, NAME AS STRING, age AS INTEGER, COLOR AS STRING)
    cat.__vtable_id = CatClassID
    cat.NAME = NAME
    cat.Age = age
    cat.COLOR = COLOR
END SUB

' ============================================================================
' Method implementations
' ============================================================================
SUB Dog_Speak (dog AS Dog)
    PRINT RTRIM$(dog.NAME); " says: Woof! Woof!"
END SUB

SUB Dog_Fetch (dog AS Dog)
    PRINT RTRIM$(dog.NAME); " fetches the ball!"
END SUB

SUB Cat_Speak (cat AS Cat)
    PRINT RTRIM$(cat.NAME); " says: Meow!"
END SUB

SUB Cat_Purr (cat AS Cat)
    PRINT RTRIM$(cat.NAME); " purrs contentedly..."
END SUB

' ============================================================================
' Polymorphic function
' ============================================================================
SUB DescribeAnimal (vtableID AS LONG, NAME AS STRING, age AS INTEGER)
    PRINT "Animal: "; RTRIM$(NAME)
    PRINT "  Age: "; age
    PRINT "  Type: "; QBNEX_GetClassName(vtableID)
    PRINT "  Is Animal: "; QBNEX_IsInstance(vtableID, "Animal")
    PRINT "  Is Dog: "; QBNEX_IsInstance(vtableID, "Dog")
    PRINT "  Is Cat: "; QBNEX_IsInstance(vtableID, "Cat")
    PRINT "  Implements IPet: "; QBNEX_Implements(vtableID, "IPet")
    PRINT
END SUB

' ============================================================================
' Main Program
' ============================================================================

CLS
PRINT "========================================================================"
PRINT "QBNex Standard Library - OOP Example"
PRINT "========================================================================"
PRINT

' Initialize class system
InitializeClasses

' Create instances
DIM myDog AS Dog
DIM myCat AS Cat

New_Dog myDog, "Buddy", 3, "Golden Retriever"
New_Cat myCat, "Whiskers", 2, "Orange"

PRINT "Created instances:"
PRINT "  Dog: "; RTRIM$(myDog.NAME); " ("; RTRIM$(myDog.Breed); ")"
PRINT "  Cat: "; RTRIM$(myCat.NAME); " ("; RTRIM$(myCat.COLOR); ")"
PRINT
PRINT "Press any key to continue..."
SLEEP
CLS

' Demonstrate polymorphism
PRINT "========================================================================"
PRINT "Polymorphism Demonstration"
PRINT "========================================================================"
PRINT

DescribeAnimal myDog.__vtable_id, myDog.NAME, myDog.Age
DescribeAnimal myCat.__vtable_id, myCat.NAME, myCat.Age

PRINT "Press any key to continue..."
SLEEP
CLS

' Demonstrate method calls
PRINT "========================================================================"
PRINT "Method Calls"
PRINT "========================================================================"
PRINT

PRINT "--- Dog Methods ---"
Dog_Speak myDog
Dog_Fetch myDog
PRINT

PRINT "--- Cat Methods ---"
Cat_Speak myCat
Cat_Purr myCat
PRINT

PRINT "Press any key to continue..."
SLEEP
CLS

' Demonstrate interface checking
PRINT "========================================================================"
PRINT "Interface Implementation"
PRINT "========================================================================"
PRINT

PRINT "Checking IPet interface implementation:"
PRINT

IF QBNEX_Implements(myDog.__vtable_id, "IPet") THEN
    PRINT "✓ Dog implements IPet"
    Dog_Speak myDog
ELSE
    PRINT "✗ Dog does not implement IPet"
END IF
PRINT

IF QBNEX_Implements(myCat.__vtable_id, "IPet") THEN
    PRINT "✓ Cat implements IPet"
    Cat_Speak myCat
ELSE
    PRINT "✗ Cat does not implement IPet"
END IF
PRINT

' Demonstrate Optional pattern
PRINT "========================================================================"
PRINT "Optional<T> Pattern"
PRINT "========================================================================"
PRINT

DIM optionalOwner AS QBNex_Optional
Opt_SetNone optionalOwner

PRINT "Dog owner: ";
IF Opt_IsSome(optionalOwner) THEN
    PRINT Opt_Get(optionalOwner)
ELSE
    PRINT "(No owner set)"
END IF

Opt_SetSome optionalOwner, "John Smith"
PRINT "After setting owner: "; Opt_Get(optionalOwner)
PRINT

' Demonstrate Pair pattern
PRINT "========================================================================"
PRINT "Pair Pattern"
PRINT "========================================================================"
PRINT

DIM petInfo AS QBNex_Pair
Pair_Set petInfo, "Species", "Dog"
PRINT "Pet info: "; Pair_First(petInfo); " = "; Pair_Second(petInfo)
PRINT

PRINT "========================================================================"
PRINT "OOP Example Complete!"
PRINT "========================================================================"
PRINT
PRINT "This example demonstrated:"
PRINT "  • Class registration and inheritance"
PRINT "  • Method registration and dispatch"
PRINT "  • Interface implementation"
PRINT "  • Polymorphic type checking"
PRINT "  • Optional<T> and Pair patterns"
PRINT
PRINT "Press any key to exit..."
SLEEP
