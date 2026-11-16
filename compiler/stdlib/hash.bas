'===============================================================================
' QBNex Standard Library - hash ('$IMPORT:'hash')
'===============================================================================
' Digests and encodings of the bytes of a string, like Python's hashlib,
' base64 and binascii. Digests are lower-case hex.
'
'   Hash_Sha256(s$)  Hash_Sha512(s$)  Hash_Sha1(s$)  Hash_Md5(s$)
'   Hash_Crc32(s$)                    CRC32 as a number (zip, PNG)
'   Base64_Encode(s$)                  Base64_Encode(s$, -1): URL-safe, no padding
'   Base64_Decode(b$)                  either kind; "" and LastError when invalid
'   Hex_Encode("AZ") = "415a"          Hex_Decode("41 42") = "AB"
'===============================================================================

'$IMPORT:'ext'

DECLARE LIBRARY
    FUNCTION qb_ext_hash&& (BYVAL kind AS LONG, BYVAL d AS _OFFSET, BYVAL dl AS _INTEGER64)
    FUNCTION qb_ext_crc32~& (BYVAL d AS _OFFSET, BYVAL dl AS _INTEGER64)
    FUNCTION qb_ext_codec&& (BYVAL op AS LONG, BYVAL d AS _OFFSET, BYVAL dl AS _INTEGER64)
END DECLARE

FUNCTION Hash_Digest$ (kind AS LONG, bytes AS STRING)
    Hash_Digest$ = Ext_Text$(qb_ext_hash&&(kind, _OFFSET(bytes), LEN(bytes)))
END FUNCTION

FUNCTION Hash_Codec$ (op AS LONG, bytes AS STRING)
    Hash_Codec$ = Ext_Text$(qb_ext_codec&&(op, _OFFSET(bytes), LEN(bytes)))
END FUNCTION

' MD5 digest (hex).
FUNCTION Hash_Md5$ (bytes AS STRING)
    Hash_Md5$ = Hash_Digest$(2, bytes)
END FUNCTION

' SHA-1 digest (hex).
FUNCTION Hash_Sha1$ (bytes AS STRING)
    Hash_Sha1$ = Hash_Digest$(3, bytes)
END FUNCTION

' SHA-256 digest (hex).
FUNCTION Hash_Sha256$ (bytes AS STRING)
    Hash_Sha256$ = Hash_Digest$(4, bytes)
END FUNCTION

' SHA-512 digest (hex).
FUNCTION Hash_Sha512$ (bytes AS STRING)
    Hash_Sha512$ = Hash_Digest$(5, bytes)
END FUNCTION

' CRC32 as a number (HEX$ gives its hex digits).
FUNCTION Hash_Crc32~& (bytes AS STRING)
    Hash_Crc32~& = qb_ext_crc32~&(_OFFSET(bytes), LEN(bytes))
END FUNCTION

' Base64 text of bytes; urlSafe gives the URL-safe alphabet without padding.
FUNCTION Base64_Encode$ (bytes AS STRING, OPTIONAL urlSafe AS LONG = 0)
    IF urlSafe THEN Base64_Encode$ = Hash_Codec$(3, bytes) ELSE Base64_Encode$ = Hash_Codec$(1, bytes)
END FUNCTION

' The bytes of Base64 text (either alphabet; line breaks are ignored).
FUNCTION Base64_Decode$ (text AS STRING)
    Base64_Decode$ = Hash_Codec$(2, text)
END FUNCTION

' Hex digits of bytes (lower case).
FUNCTION Hex_Encode$ (bytes AS STRING)
    Hex_Encode$ = Hash_Codec$(4, bytes)
END FUNCTION

' The bytes of hex digits (spaces are ignored).
FUNCTION Hex_Decode$ (text AS STRING)
    Hex_Decode$ = Hash_Codec$(5, text)
END FUNCTION
