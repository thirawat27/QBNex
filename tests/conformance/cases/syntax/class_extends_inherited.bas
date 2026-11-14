$CONSOLE:ONLY
'$IMPORT:'qbnex'
' A method of the base class is called on an object of the derived class, also from
' a method of the derived class and as a statement without CALL.
CLASS Animal
    Legs AS INTEGER
    SUB AddLegs (k AS INTEGER)
        ME.Legs = ME.Legs + k
    END SUB
    FUNCTION Total% (m AS INTEGER)
        Total% = ME.Legs * m
    END FUNCTION
END CLASS
CLASS Dog EXTENDS Animal
    Tricks AS INTEGER
    SUB Learn ()
        ME.Tricks = ME.Tricks + 1
        ME.AddLegs(1)
    END SUB
END CLASS
DIM d AS Dog
CALL d.AddLegs(4)
d.Learn()
PRINT d.Legs; d.Tricks; d.Total%(2)
SYSTEM
