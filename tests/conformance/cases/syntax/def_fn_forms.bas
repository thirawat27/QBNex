$CONSOLE:ONLY
' DEF FN after a label, followed by another statement, with a DIMmed variable, a
' TYPE field, and under a DEFtype that changes later in the file.
TYPE pt
    x AS SINGLE
END TYPE
DIM p AS pt
DIM total AS DOUBLE
DIM cnt AS INTEGER, nm AS STRING
DEFINT A-Z
n = 7
p.x = 4
total = 2.5: cnt = 3: nm = "ab"
lbl: DEF FNb (x) = x + 1
DEF FNa (x) = x * 2: PRINT "same line"; FNa(2)
DEF FNhalf (x) = x * 3 + n
DEF FNpx! = p.x * 2
DEF FNt# = total * cnt
DEF FNn$ = nm + "!"
PRINT FNb(4); FNa(5); FNhalf(3); FNpx!; FNt#; FNn$
DEFSNG A-Z
PRINT "done"
SYSTEM
