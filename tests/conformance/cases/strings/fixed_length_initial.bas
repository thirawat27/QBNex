$CONSOLE:ONLY
' A fixed-length string starts as CHR$(0) bytes: as a variable, in an array and in a TYPE.
TYPE T
  nm AS STRING * 4
END TYPE
DIM f AS STRING * 4, r AS T, fa(2) AS STRING * 2
PRINT ASC(f); ASC(r.nm); ASC(fa(1))
PRINT f = STRING$(4, 0)
SYSTEM
