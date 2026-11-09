$CONSOLE:ONLY
PRINT HexBytes$(MKI$(-2)); HexBytes$(MKL$(305419896))
PRINT HexBytes$(MKS$(1.5)); HexBytes$(MKD$(-0.1#))
PRINT CVI(MKI$(12345)); CVL(MKL$(-7)); CVS(MKS$(2.5)); CVD(MKD$(1D+300))
PRINT HexBytes$(_MK$(_INTEGER64, 1)); _CV(_INTEGER64, _MK$(_INTEGER64, -5))
SYSTEM

FUNCTION HexBytes$ (s AS STRING)
    FOR i = 1 TO LEN(s): h$ = h$ + RIGHT$("0" + HEX$(ASC(s, i)), 2) + " ": NEXT
    HexBytes$ = h$
END FUNCTION
