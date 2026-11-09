$CONSOLE:ONLY
' STR$ and PRINT number formatting (qb_runtime::numfmt), incl. the DOUBLE extra-decimal quirk.
DIM d AS DOUBLE, s AS SINGLE, f AS _FLOAT
DIM b AS _BYTE, ub AS _UNSIGNED _BYTE, i AS INTEGER, ui AS _UNSIGNED INTEGER
DIM l AS LONG, ul AS _UNSIGNED LONG, q AS _INTEGER64, uq AS _UNSIGNED _INTEGER64
b = -128: ub = 255: i = -32768: ui = 65535: l = -2147483648: ul = 4294967295
q = -9223372036854775807 - 1: uq = 18446744073709551615
PRINT "["; STR$(b); "]["; STR$(ub); "]["; STR$(i); "]["; STR$(ui); "]"
PRINT "["; STR$(l); "]["; STR$(ul); "]["; STR$(q); "]["; STR$(uq); "]"
FOR k = 0 TO 11
    READ d
    s = d
    PRINT "["; STR$(d); "]["; STR$(s); "]"; d; s
NEXT
DATA 42,7,-0.25,0.1,1E15,1E16,1E-16,1.5E-10,1E-17,123456789012345.6,0.000123,1.25E-5
f = 1.5: PRINT "["; STR$(f); "]"
s = 12345678: PRINT "["; STR$(s); "]"; s
s = 1.5E-8: PRINT "["; STR$(s); "]"
s = -3.25E+20: PRINT "["; STR$(s); "]"
d = 1.797693134862316D+308: PRINT "["; STR$(d); "]"
PRINT "["; STR$(0#); "]["; STR$(0!); "]["; STR$(-0.000001#); "]["; STR$(1 / 3); "]["; STR$(2# / 3#); "]"
SYSTEM
