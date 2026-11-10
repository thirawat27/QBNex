'===============================================================================
' QBNex Standard Library - Extension core ('$IMPORT:'ext.core')
'===============================================================================
' Shared plumbing for the ext.* modules (regex, hash, zip, http). The work is
' done by the Rust extension library (crates/qb-ext) that is linked into every
' program; results come back through a buffer that Ext_Result$ copies out.
'
'   Ext_LastError$   message of the last failed ext.* call ("" after success)
'   Ext_PartCount&   number of list items left by the last list-producing call
'   Ext_Part$(i&)    item i (0-based) of that list
'===============================================================================

DECLARE LIBRARY
    FUNCTION qb_ext_result_len&& ()
    FUNCTION qb_ext_result_copy&& (BYVAL dst AS _OFFSET, BYVAL cap AS _INTEGER64)
    FUNCTION qb_ext_error_len&& ()
    FUNCTION qb_ext_error_copy&& (BYVAL dst AS _OFFSET, BYVAL cap AS _INTEGER64)
    FUNCTION qb_ext_part_count& ()
    FUNCTION qb_ext_part&& (BYVAL index AS LONG)
END DECLARE

FUNCTION Ext_Result$ ()
    DIM n AS _INTEGER64, copied AS _INTEGER64, buffer AS STRING
    n = qb_ext_result_len&&
    IF n <= 0 THEN EXIT FUNCTION
    buffer = SPACE$(n)
    copied = qb_ext_result_copy&&(_OFFSET(buffer), n)
    Ext_Result$ = LEFT$(buffer, copied)
END FUNCTION

FUNCTION Ext_LastError$ ()
    DIM n AS _INTEGER64, copied AS _INTEGER64, buffer AS STRING
    n = qb_ext_error_len&&
    IF n <= 0 THEN EXIT FUNCTION
    buffer = SPACE$(n)
    copied = qb_ext_error_copy&&(_OFFSET(buffer), n)
    Ext_LastError$ = LEFT$(buffer, copied)
END FUNCTION

FUNCTION Ext_PartCount& ()
    Ext_PartCount& = qb_ext_part_count&
END FUNCTION

FUNCTION Ext_Part$ (index AS LONG)
    IF qb_ext_part&&(index) < 0 THEN EXIT FUNCTION
    Ext_Part$ = Ext_Result$
END FUNCTION
