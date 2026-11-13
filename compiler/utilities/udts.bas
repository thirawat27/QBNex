SUB increaseUDTArrays
    x = UBOUND(udtxname)
    REDIM _PRESERVE udtxname(x + 1000) AS STRING * 256
    REDIM _PRESERVE udtxcname(x + 1000) AS STRING * 256
    REDIM _PRESERVE udtxsize(x + 1000) AS LONG
    REDIM _PRESERVE udtxbytealign(x + 1000) AS INTEGER 'first element MUST be on a byte alignment & size is a multiple of 8
    REDIM _PRESERVE udtxnext(x + 1000) AS LONG
    REDIM _PRESERVE udtxvariable(x + 1000) AS INTEGER 'true if the udt contains variable length elements
    'elements
    REDIM _PRESERVE udtename(x + 1000) AS STRING * 256
    REDIM _PRESERVE udtecname(x + 1000) AS STRING * 256
    REDIM _PRESERVE udtebytealign(x + 1000) AS INTEGER
    REDIM _PRESERVE udtesize(x + 1000) AS LONG
    REDIM _PRESERVE udtetype(x + 1000) AS LONG
    REDIM _PRESERVE udtetypesize(x + 1000) AS LONG
    REDIM _PRESERVE udtearrayelements(x + 1000) AS LONG
    REDIM _PRESERVE udtenext(x + 1000) AS LONG
END SUB
