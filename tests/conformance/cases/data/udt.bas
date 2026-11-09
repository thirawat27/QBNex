$CONSOLE:ONLY
TYPE Point3
    x AS INTEGER
    y AS SINGLE
    tag AS STRING * 4
END TYPE
TYPE Box
    lo AS Point3
    hi AS Point3
    id AS _INTEGER64
END TYPE
DIM b AS Box
b.lo.x = 1: b.lo.y = 1.5: b.lo.tag = "lo"
b.hi.x = 9: b.hi.y = 9.5: b.hi.tag = "hightag"
b.id = 1234567890123&&
PRINT b.lo.x; b.lo.y; "["; b.lo.tag; "]"; b.hi.x; b.hi.y; "["; b.hi.tag; "]"; b.id
PRINT LEN(b)
DIM pts(2) AS Point3
pts(1).x = 42
PRINT pts(1).x; pts(0).x
SYSTEM
