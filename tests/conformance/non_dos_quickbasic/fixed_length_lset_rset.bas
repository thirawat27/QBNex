TYPE RecType
    code AS STRING * 4
END TYPE

DIM rec AS RecType

LSET rec.code = "AB"
PRINT "["; rec.code; "]"

RSET rec.code = "Z"
PRINT "["; rec.code; "]"
