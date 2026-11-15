'String SPacer/delimiter constants
'sp is used as the primary string spacer
'sp2 & sp3 are used when further delimiation is required
'for instance, sp2 is used for embedding spacing info for auto-layout tools
DIM SHARED sp AS STRING * 1, sp2 AS STRING * 1, sp3 AS STRING * 1
sp = CHR$(13): sp2 = CHR$(10): sp3 = CHR$(26)
DIM SHARED sp_asc AS LONG, sp2_asc AS LONG, sp3_asc AS LONG
sp_asc = ASC(sp): sp2_asc = ASC(sp2): sp3_asc = ASC(sp3)
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
