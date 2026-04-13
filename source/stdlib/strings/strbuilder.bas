' ============================================================================
' QBNex Standard Library - Strings: StringBuilder
' ============================================================================

TYPE QBNex_StringBuilder
    Buffer AS STRING
    PartCount AS LONG
END TYPE

SUB SB_Init (builder AS QBNex_StringBuilder)
    builder.Buffer = ""
    builder.PartCount = 0
END SUB

SUB SB_Append (builder AS QBNex_StringBuilder, text AS STRING)
    builder.Buffer = builder.Buffer + MKL$(LEN(text)) + text
    builder.PartCount = builder.PartCount + 1
END SUB

SUB SB_AppendLine (builder AS QBNex_StringBuilder, text AS STRING)
    SB_Append builder, text + CHR$(13) + CHR$(10)
END SUB

SUB SB_Clear (builder AS QBNex_StringBuilder)
    builder.Buffer = ""
    builder.PartCount = 0
END SUB

FUNCTION SB_Length& (builder AS QBNex_StringBuilder)
    DIM position AS LONG
    DIM itemLength AS LONG
    DIM index AS LONG
    DIM totalLength AS LONG

    position = 1
    FOR index = 1 TO builder.PartCount
        itemLength = CVL(MID$(builder.Buffer, position, 4))
        position = position + 4
        totalLength = totalLength + itemLength
        position = position + itemLength
    NEXT

    SB_Length = totalLength
END FUNCTION

FUNCTION SB_ToString$ (builder AS QBNex_StringBuilder)
    DIM position AS LONG
    DIM itemLength AS LONG
    DIM index AS LONG
    DIM assembled AS STRING

    position = 1
    FOR index = 1 TO builder.PartCount
        itemLength = CVL(MID$(builder.Buffer, position, 4))
        position = position + 4
        assembled = assembled + MID$(builder.Buffer, position, itemLength)
        position = position + itemLength
    NEXT

    SB_ToString = assembled
END FUNCTION

SUB SB_Free (builder AS QBNex_StringBuilder)
    SB_Clear builder
END SUB
