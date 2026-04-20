OPTION _EXPLICIT

TYPE BenchPoint
    x AS LONG
    y AS LONG
END TYPE

DECLARE FUNCTION Fib& (n AS LONG)
DECLARE FUNCTION SumSquares& (limit AS LONG)
DECLARE FUNCTION MixValue& (seed AS LONG, factor AS LONG)
DECLARE SUB FillPoints (points() AS BenchPoint)
DECLARE SUB PrintSummary (points() AS BenchPoint)

DIM points(1 TO 48) AS BenchPoint

FillPoints points()
PrintSummary points()

END

SUB FillPoints (points() AS BenchPoint)
    DIM i AS LONG

    FOR i = LBOUND(points) TO UBOUND(points)
        points(i).x = Fib((i MOD 12) + 8)
        points(i).y = SumSquares(i) + MixValue(i, 3)
    NEXT i
END SUB

SUB PrintSummary (points() AS BenchPoint)
    DIM i AS LONG
    DIM total AS LONG
    DIM checksum AS LONG

    FOR i = LBOUND(points) TO UBOUND(points)
        total = total + points(i).x + points(i).y
        checksum = checksum + MixValue(points(i).x, (i MOD 5) + 1)
    NEXT i

    PRINT "bench-total"; total
    PRINT "bench-check"; checksum
END SUB

FUNCTION Fib& (n AS LONG)
    DIM a AS LONG
    DIM b AS LONG
    DIM i AS LONG
    DIM nextValue AS LONG

    a = 0
    b = 1

    FOR i = 1 TO n
        nextValue = a + b
        a = b
        b = nextValue
    NEXT i

    Fib = a
END FUNCTION

FUNCTION SumSquares& (limit AS LONG)
    DIM i AS LONG
    DIM total AS LONG

    FOR i = 1 TO limit
        total = total + (i * i)
    NEXT i

    SumSquares = total
END FUNCTION

FUNCTION MixValue& (seed AS LONG, factor AS LONG)
    DIM i AS LONG
    DIM mixed AS LONG

    mixed = seed
    FOR i = 1 TO 8
        mixed = mixed + (factor * i) + ((seed + i) MOD 7)
    NEXT i

    MixValue = mixed
END FUNCTION
