OPTION _EXPLICIT

TYPE BenchRecord
    seedValue AS LONG
    delta AS LONG
    weight AS LONG
END TYPE

DECLARE SUB SeedRecords (records() AS BenchRecord)
DECLARE FUNCTION EvalRecord& (record AS BenchRecord, index AS LONG)
DECLARE FUNCTION Mix01& (value AS LONG)
DECLARE FUNCTION Mix02& (value AS LONG)
DECLARE FUNCTION Mix03& (value AS LONG)
DECLARE FUNCTION Mix04& (value AS LONG)
DECLARE FUNCTION Mix05& (value AS LONG)
DECLARE FUNCTION Mix06& (value AS LONG)
DECLARE FUNCTION Mix07& (value AS LONG)
DECLARE FUNCTION Mix08& (value AS LONG)
DECLARE FUNCTION Mix09& (value AS LONG)
DECLARE FUNCTION Mix10& (value AS LONG)
DECLARE FUNCTION Mix11& (value AS LONG)
DECLARE FUNCTION Mix12& (value AS LONG)

DIM records(1 TO 192) AS BenchRecord
DIM i AS LONG
DIM total AS LONG
DIM checksum AS LONG

SeedRecords records()

FOR i = LBOUND(records) TO UBOUND(records)
    total = total + EvalRecord(records(i), i)
    checksum = checksum + Mix11(records(i).seedValue + i) - Mix12(records(i).delta + records(i).weight)
NEXT i

PRINT "stress-total"; total
PRINT "stress-check"; checksum

END

SUB SeedRecords (records() AS BenchRecord)
    DIM i AS LONG

    FOR i = LBOUND(records) TO UBOUND(records)
        records(i).seedValue = Mix01(i) + Mix02(i + 3)
        records(i).delta = Mix03(i + 5) - Mix04(i MOD 17)
        records(i).weight = (Mix05(i) MOD 11) + 1
    NEXT i
END SUB

FUNCTION EvalRecord& (record AS BenchRecord, index AS LONG)
    DIM total AS LONG

    total = record.seedValue
    total = total + Mix06(record.delta + index)
    total = total + Mix07(record.weight + index)
    total = total + Mix08(record.seedValue MOD 23)
    total = total + Mix09(record.delta MOD 19)
    total = total + Mix10(record.weight * 3)

    EvalRecord = total
END FUNCTION

FUNCTION Mix01& (value AS LONG)
    DIM i AS LONG
    DIM total AS LONG

    total = value
    FOR i = 1 TO 6
        total = total + (i * 3) + ((value + i) MOD 5)
    NEXT i
    Mix01 = total
END FUNCTION

FUNCTION Mix02& (value AS LONG)
    DIM i AS LONG
    DIM total AS LONG

    total = value * 2
    FOR i = 1 TO 5
        total = total + (i * i) + ((value + i) MOD 7)
    NEXT i
    Mix02 = total
END FUNCTION

FUNCTION Mix03& (value AS LONG)
    DIM i AS LONG
    DIM total AS LONG

    total = value + 9
    FOR i = 1 TO 7
        total = total + ((value * i) MOD 13)
    NEXT i
    Mix03 = total
END FUNCTION

FUNCTION Mix04& (value AS LONG)
    DIM i AS LONG
    DIM total AS LONG

    total = value
    FOR i = 1 TO 4
        total = total + i + ((value + i) MOD 3)
    NEXT i
    Mix04 = total
END FUNCTION

FUNCTION Mix05& (value AS LONG)
    DIM i AS LONG
    DIM total AS LONG

    total = value * 3
    FOR i = 1 TO 6
        total = total + ((value + i) MOD 9) + i
    NEXT i
    Mix05 = total
END FUNCTION

FUNCTION Mix06& (value AS LONG)
    DIM i AS LONG
    DIM total AS LONG

    total = value
    FOR i = 1 TO 8
        total = total + (i * 2) + ((value + i) MOD 11)
    NEXT i
    Mix06 = total
END FUNCTION

FUNCTION Mix07& (value AS LONG)
    DIM i AS LONG
    DIM total AS LONG

    total = value + 5
    FOR i = 1 TO 6
        total = total + ((value * i) MOD 17)
    NEXT i
    Mix07 = total
END FUNCTION

FUNCTION Mix08& (value AS LONG)
    DIM i AS LONG
    DIM total AS LONG

    total = value
    FOR i = 1 TO 5
        total = total + ((value + i) MOD 19) + (i * 4)
    NEXT i
    Mix08 = total
END FUNCTION

FUNCTION Mix09& (value AS LONG)
    DIM i AS LONG
    DIM total AS LONG

    total = value * 2
    FOR i = 1 TO 7
        total = total + ((value + i) MOD 23) + i
    NEXT i
    Mix09 = total
END FUNCTION

FUNCTION Mix10& (value AS LONG)
    DIM i AS LONG
    DIM total AS LONG

    total = value + 1
    FOR i = 1 TO 8
        total = total + ((value * i) MOD 29)
    NEXT i
    Mix10 = total
END FUNCTION

FUNCTION Mix11& (value AS LONG)
    DIM i AS LONG
    DIM total AS LONG

    total = value
    FOR i = 1 TO 6
        total = total + (i * 5) + ((value + i) MOD 31)
    NEXT i
    Mix11 = total
END FUNCTION

FUNCTION Mix12& (value AS LONG)
    DIM i AS LONG
    DIM total AS LONG

    total = value + 7
    FOR i = 1 TO 6
        total = total + ((value + i) MOD 37) + (i * 2)
    NEXT i
    Mix12 = total
END FUNCTION
