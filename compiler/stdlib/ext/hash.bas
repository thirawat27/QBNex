'===============================================================================
' QBNex Standard Library - Hashing and encodings ('$IMPORT:'ext.hash')
'===============================================================================
' Digests are lower-case hex strings of the bytes of the argument:
'   Hash_CRC32$  Hash_MD5$  Hash_SHA1$  Hash_SHA256$  Hash_SHA512$
'   Hash_CRC32Value~&(s$)        CRC32 as a number (zip/PNG/zlib CRC)
' Encodings (decoders return "" and set Ext_LastError$ on invalid input):
'   Base64_Encode$ / Base64_Decode$ (also accepts URL-safe text, ignores
'   line breaks), Base64Url_Encode$ (no padding), Hex_Encode$ / Hex_Decode$
'===============================================================================

'$IMPORT:'ext.core'

DECLARE LIBRARY
    FUNCTION qb_ext_hash&& (BYVAL kind AS LONG, BYVAL d AS _OFFSET, BYVAL dl AS _INTEGER64)
    FUNCTION qb_ext_crc32~& (BYVAL d AS _OFFSET, BYVAL dl AS _INTEGER64)
    FUNCTION qb_ext_codec&& (BYVAL op AS LONG, BYVAL d AS _OFFSET, BYVAL dl AS _INTEGER64)
END DECLARE

FUNCTION HashExt_Digest$ (kind AS LONG, dataText AS STRING)
    IF qb_ext_hash&&(kind, _OFFSET(dataText), LEN(dataText)) < 0 THEN EXIT FUNCTION
    HashExt_Digest$ = Ext_Result$
END FUNCTION

FUNCTION HashExt_Codec$ (op AS LONG, dataText AS STRING)
    IF qb_ext_codec&&(op, _OFFSET(dataText), LEN(dataText)) < 0 THEN EXIT FUNCTION
    HashExt_Codec$ = Ext_Result$
END FUNCTION

FUNCTION Hash_CRC32$ (dataText AS STRING)
    Hash_CRC32$ = HashExt_Digest$(1, dataText)
END FUNCTION

FUNCTION Hash_MD5$ (dataText AS STRING)
    Hash_MD5$ = HashExt_Digest$(2, dataText)
END FUNCTION

FUNCTION Hash_SHA1$ (dataText AS STRING)
    Hash_SHA1$ = HashExt_Digest$(3, dataText)
END FUNCTION

FUNCTION Hash_SHA256$ (dataText AS STRING)
    Hash_SHA256$ = HashExt_Digest$(4, dataText)
END FUNCTION

FUNCTION Hash_SHA512$ (dataText AS STRING)
    Hash_SHA512$ = HashExt_Digest$(5, dataText)
END FUNCTION

FUNCTION Hash_CRC32Value~& (dataText AS STRING)
    Hash_CRC32Value~& = qb_ext_crc32~&(_OFFSET(dataText), LEN(dataText))
END FUNCTION

FUNCTION Base64_Encode$ (dataText AS STRING)
    Base64_Encode$ = HashExt_Codec$(1, dataText)
END FUNCTION

FUNCTION Base64_Decode$ (encodedText AS STRING)
    Base64_Decode$ = HashExt_Codec$(2, encodedText)
END FUNCTION

FUNCTION Base64Url_Encode$ (dataText AS STRING)
    Base64Url_Encode$ = HashExt_Codec$(3, dataText)
END FUNCTION

FUNCTION Hex_Encode$ (dataText AS STRING)
    Hex_Encode$ = HashExt_Codec$(4, dataText)
END FUNCTION

FUNCTION Hex_Decode$ (encodedText AS STRING)
    Hex_Decode$ = HashExt_Codec$(5, encodedText)
END FUNCTION
