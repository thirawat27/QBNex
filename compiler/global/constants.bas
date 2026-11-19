'String SPacer/delimiter constants
'sp is used as the primary string spacer
'sp2 & sp3 are used when further delimiation is required
'for instance, sp2 is used for embedding spacing info for auto-layout tools
'variable-length: every use of a fixed-length string is converted first
DIM SHARED sp AS STRING, sp2 AS STRING, sp3 AS STRING
sp = CHR$(13): sp2 = CHR$(10): sp3 = CHR$(26)
DIM SHARED sp_asc AS LONG, sp2_asc AS LONG, sp3_asc AS LONG
sp_asc = ASC(sp): sp2_asc = ASC(sp2): sp3_asc = ASC(sp3)
'characters of a name (ClassSyntax_IsIdentifierChar%), and those or a type suffix
'(ClassSyntax_IsTypeSuffixChar%), by code; code 0 stands for the empty string
DIM SHARED identChar(0 TO 255) AS INTEGER, identOrSuffixChar(0 TO 255) AS INTEGER
FOR identCode = 0 TO 255
    SELECT CASE identCode
        CASE 48 TO 57, 65 TO 90, 97 TO 122, 95: identChar(identCode) = -1: identOrSuffixChar(identCode) = -1
        CASE 36, 37, 38, 33, 35: identOrSuffixChar(identCode) = -1
    END SELECT
NEXT
'first characters isoperator can find an operator after (it refuses the others at once):
'a check before the call where most elements are names and numbers
DIM SHARED opStart(0 TO 255) AS INTEGER
FOR identCode = 1 TO 255
    SELECT CASE identCode
        CASE 48 TO 57, 34, 40, 41, 44, 46
        CASE 65 TO 90, 97 TO 122
            SELECT CASE identCode AND 223
                CASE 65, 69, 73, 77, 78, 79, 88: opStart(identCode) = -1
            END SELECT
        CASE ELSE: opStart(identCode) = -1
    END SELECT
NEXT
'str2$ of 0 to STR2_CACHED, made once each
CONST STR2_CACHED = 8191
DIM SHARED str2Cache(0 TO STR2_CACHED) AS STRING
IF Debug THEN sp = CHR$(250): sp2 = CHR$(249): sp3 = CHR$(179) 'makes debug output more readable

'ASCII codes
CONST ASC_BACKSLASH = 92
CONST ASC_FORWARDSLASH = 47
CONST ASC_LEFTBRACKET = 40
CONST ASC_RIGHTBRACKET = 41
CONST ASC_FULLSTOP = 46
CONST ASC_COLON = 58
CONST ASC_SEMICOLON = 59
CONST ASC_UNDERSCORE = 95
CONST ASC_QUOTE = 34
CONST ASC_LEFTSQUAREBRACKET = 91
CONST ASC_RIGHTSQUAREBRACKET = 93
CONST ASC_QUESTIONMARK = 63

DIM SHARED CHR_QUOTE AS STRING: CHR_QUOTE = CHR$(34)
DIM SHARED CHR_TAB AS STRING: CHR_TAB = CHR$(9)
DIM SHARED CRLF AS STRING: CRLF = CHR$(13) + CHR$(10) 'carriage return+line feed
