'===============================================================================
' QBNex Standard Library - native plumbing ('$IMPORT:'ext')
'===============================================================================
' Used by regex, hash, zip, http, utf8 and datetime. The work is done by the
' Rust library crates/runtime/ext, linked into every native program (the web
' build does not have it). A call leaves its text result in a buffer that
' Ext_Text copies out, and its error message, which becomes LastError.
'===============================================================================

'$IMPORT:'core'

DECLARE LIBRARY
    FUNCTION qb_ext_result_len&& ()
    FUNCTION qb_ext_result_copy&& (BYVAL dst AS _OFFSET, BYVAL cap AS _INTEGER64)
    FUNCTION qb_ext_error_len&& ()
    FUNCTION qb_ext_error_copy&& (BYVAL dst AS _OFFSET, BYVAL cap AS _INTEGER64)
    FUNCTION qb_ext_part_count& ()
    FUNCTION qb_ext_part&& (BYVAL index AS LONG)
END DECLARE

' The text result of the last native call.
FUNCTION Ext_Result$ ()
    DIM n AS _INTEGER64, copied AS _INTEGER64, buffer AS STRING
    n = qb_ext_result_len&&
    IF n <= 0 THEN EXIT FUNCTION
    buffer = SPACE$(n)
    copied = qb_ext_result_copy&&(_OFFSET(buffer), n)
    Ext_Result$ = LEFT$(buffer, copied)
END FUNCTION

' Copies the error of the last native call into LastError.
SUB Ext_Sync ()
    DIM n AS _INTEGER64, copied AS _INTEGER64, buffer AS STRING
    n = qb_ext_error_len&&
    IF n > 0 THEN
        buffer = SPACE$(n)
        copied = qb_ext_error_copy&&(_OFFSET(buffer), n)
        Core_SetError LEFT$(buffer, copied)
    ELSE
        Core_SetError ""
    END IF
END SUB

' The text of a native call that returned r (r > 0: a result is there).
FUNCTION Ext_Text$ (r AS _INTEGER64)
    Ext_Sync
    IF r > 0 THEN Ext_Text$ = Ext_Result$
END FUNCTION

' Number of list items the last native call left (regex groups, split parts...).
FUNCTION Ext_PartCount& ()
    Ext_PartCount& = qb_ext_part_count&
END FUNCTION

' List item i (0-based) of the last native call.
FUNCTION Ext_Part$ (index AS LONG)
    IF qb_ext_part&&(index) < 0 THEN EXIT FUNCTION
    Ext_Part$ = Ext_Result$
END FUNCTION
