'===============================================================================
' QBNex Standard Library - Lists and dictionaries ('$IMPORT:'ext.collections')
'===============================================================================
' Growable collections addressed by handles (LONG, 0 = none), holding strings
' and numbers side by side. Lists are 0-based. Dictionaries keep keys in
' insertion order; keys are compared exactly (case-sensitive bytes).
' Free a collection with XColl_Free when it is no longer needed.
'
'   l& = XList_New&                 d& = XDict_New&
'   XList_Add l&, "text"            XDict_Set d&, "name", "Ann"
'   XList_AddNum l&, 3.5            XDict_SetNum d&, "age", 42
'   XList_Count&(l&)                XDict_Count&(d&)
'   XList_Get$(l&, i&)              XDict_Get$(d&, "name")     (numbers as text)
'   XList_GetNum#(l&, i&)           XDict_GetNum#(d&, "age")   (strings like VAL)
'   XList_Type&(l&, i&)             XDict_Type&(d&, key$)      0 none, 1 string, 2 number
'   XList_Set / XList_SetNum        XDict_Has&(d&, key$)
'   XList_Insert / XList_InsertNum  XDict_Remove d&, key$
'   XList_Remove l&, i&             XDict_Key$(d&, i&)         keys in order
'   XList_IndexOf&(l&, s$)          XColl_Clear h&, XColl_Free h&
'   XList_IndexOfNum&(l&, n#)
'   XList_Join$(l&, ", ")
'   XList_Sort l&, descending&, ignoreCase&
' Bad handles or indexes set Ext_LastError$.
'===============================================================================

'$IMPORT:'ext.core'

DECLARE LIBRARY
    FUNCTION qb_ext_list_new& ()
    FUNCTION qb_ext_dict_new& ()
    FUNCTION qb_ext_coll_free& (BYVAL h AS LONG)
    FUNCTION qb_ext_coll_clear& (BYVAL h AS LONG)
    FUNCTION qb_ext_list_count&& (BYVAL h AS LONG)
    FUNCTION qb_ext_list_add_str&& (BYVAL h AS LONG, BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64)
    FUNCTION qb_ext_list_add_num&& (BYVAL h AS LONG, BYVAL n AS DOUBLE)
    FUNCTION qb_ext_list_put_str& (BYVAL h AS LONG, BYVAL i AS _INTEGER64, BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64, BYVAL insert AS LONG)
    FUNCTION qb_ext_list_put_num& (BYVAL h AS LONG, BYVAL i AS _INTEGER64, BYVAL n AS DOUBLE, BYVAL insert AS LONG)
    FUNCTION qb_ext_list_type& (BYVAL h AS LONG, BYVAL i AS _INTEGER64)
    FUNCTION qb_ext_list_get_str&& (BYVAL h AS LONG, BYVAL i AS _INTEGER64)
    FUNCTION qb_ext_list_get_num# (BYVAL h AS LONG, BYVAL i AS _INTEGER64)
    FUNCTION qb_ext_list_remove& (BYVAL h AS LONG, BYVAL i AS _INTEGER64)
    FUNCTION qb_ext_list_find_str&& (BYVAL h AS LONG, BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64)
    FUNCTION qb_ext_list_find_num&& (BYVAL h AS LONG, BYVAL n AS DOUBLE)
    FUNCTION qb_ext_list_join&& (BYVAL h AS LONG, BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64)
    FUNCTION qb_ext_list_sort& (BYVAL h AS LONG, BYVAL descending AS LONG, BYVAL ignoreCase AS LONG)
    FUNCTION qb_ext_dict_count&& (BYVAL h AS LONG)
    FUNCTION qb_ext_dict_set_str& (BYVAL h AS LONG, BYVAL k AS _OFFSET, BYVAL kl AS _INTEGER64, BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64)
    FUNCTION qb_ext_dict_set_num& (BYVAL h AS LONG, BYVAL k AS _OFFSET, BYVAL kl AS _INTEGER64, BYVAL n AS DOUBLE)
    FUNCTION qb_ext_dict_type& (BYVAL h AS LONG, BYVAL k AS _OFFSET, BYVAL kl AS _INTEGER64)
    FUNCTION qb_ext_dict_get_str&& (BYVAL h AS LONG, BYVAL k AS _OFFSET, BYVAL kl AS _INTEGER64)
    FUNCTION qb_ext_dict_get_num# (BYVAL h AS LONG, BYVAL k AS _OFFSET, BYVAL kl AS _INTEGER64)
    FUNCTION qb_ext_dict_remove& (BYVAL h AS LONG, BYVAL k AS _OFFSET, BYVAL kl AS _INTEGER64)
    FUNCTION qb_ext_dict_key&& (BYVAL h AS LONG, BYVAL i AS _INTEGER64)
END DECLARE

FUNCTION XList_New& ()
    XList_New& = qb_ext_list_new&
END FUNCTION

FUNCTION XDict_New& ()
    XDict_New& = qb_ext_dict_new&
END FUNCTION

SUB XColl_Free (handle AS LONG)
    DIM r AS LONG
    r = qb_ext_coll_free&(handle)
END SUB

SUB XColl_Clear (handle AS LONG)
    DIM r AS LONG
    r = qb_ext_coll_clear&(handle)
END SUB

FUNCTION XList_Count& (handle AS LONG)
    XList_Count& = qb_ext_list_count&&(handle)
END FUNCTION

SUB XList_Add (handle AS LONG, value AS STRING)
    DIM r AS _INTEGER64
    r = qb_ext_list_add_str&&(handle, _OFFSET(value), LEN(value))
END SUB

SUB XList_AddNum (handle AS LONG, value AS DOUBLE)
    DIM r AS _INTEGER64
    r = qb_ext_list_add_num&&(handle, value)
END SUB

SUB XList_Set (handle AS LONG, index AS LONG, value AS STRING)
    DIM r AS LONG
    r = qb_ext_list_put_str&(handle, index, _OFFSET(value), LEN(value), 0)
END SUB

SUB XList_SetNum (handle AS LONG, index AS LONG, value AS DOUBLE)
    DIM r AS LONG
    r = qb_ext_list_put_num&(handle, index, value, 0)
END SUB

SUB XList_Insert (handle AS LONG, index AS LONG, value AS STRING)
    DIM r AS LONG
    r = qb_ext_list_put_str&(handle, index, _OFFSET(value), LEN(value), 1)
END SUB

SUB XList_InsertNum (handle AS LONG, index AS LONG, value AS DOUBLE)
    DIM r AS LONG
    r = qb_ext_list_put_num&(handle, index, value, 1)
END SUB

FUNCTION XList_Type& (handle AS LONG, index AS LONG)
    XList_Type& = qb_ext_list_type&(handle, index)
END FUNCTION

FUNCTION XList_Get$ (handle AS LONG, index AS LONG)
    IF qb_ext_list_get_str&&(handle, index) > 0 THEN XList_Get$ = Ext_Result$
END FUNCTION

FUNCTION XList_GetNum# (handle AS LONG, index AS LONG)
    XList_GetNum# = qb_ext_list_get_num#(handle, index)
END FUNCTION

SUB XList_Remove (handle AS LONG, index AS LONG)
    DIM r AS LONG
    r = qb_ext_list_remove&(handle, index)
END SUB

FUNCTION XList_IndexOf& (handle AS LONG, value AS STRING)
    XList_IndexOf& = qb_ext_list_find_str&&(handle, _OFFSET(value), LEN(value))
END FUNCTION

FUNCTION XList_IndexOfNum& (handle AS LONG, value AS DOUBLE)
    XList_IndexOfNum& = qb_ext_list_find_num&&(handle, value)
END FUNCTION

FUNCTION XList_Join$ (handle AS LONG, separator AS STRING)
    IF qb_ext_list_join&&(handle, _OFFSET(separator), LEN(separator)) > 0 THEN XList_Join$ = Ext_Result$
END FUNCTION

SUB XList_Sort (handle AS LONG, descending AS LONG, ignoreCase AS LONG)
    DIM r AS LONG
    r = qb_ext_list_sort&(handle, descending, ignoreCase)
END SUB

FUNCTION XDict_Count& (handle AS LONG)
    XDict_Count& = qb_ext_dict_count&&(handle)
END FUNCTION

SUB XDict_Set (handle AS LONG, keyText AS STRING, value AS STRING)
    DIM r AS LONG
    r = qb_ext_dict_set_str&(handle, _OFFSET(keyText), LEN(keyText), _OFFSET(value), LEN(value))
END SUB

SUB XDict_SetNum (handle AS LONG, keyText AS STRING, value AS DOUBLE)
    DIM r AS LONG
    r = qb_ext_dict_set_num&(handle, _OFFSET(keyText), LEN(keyText), value)
END SUB

FUNCTION XDict_Type& (handle AS LONG, keyText AS STRING)
    XDict_Type& = qb_ext_dict_type&(handle, _OFFSET(keyText), LEN(keyText))
END FUNCTION

FUNCTION XDict_Has& (handle AS LONG, keyText AS STRING)
    IF qb_ext_dict_type&(handle, _OFFSET(keyText), LEN(keyText)) > 0 THEN XDict_Has& = -1
END FUNCTION

FUNCTION XDict_Get$ (handle AS LONG, keyText AS STRING)
    IF qb_ext_dict_get_str&&(handle, _OFFSET(keyText), LEN(keyText)) > 0 THEN XDict_Get$ = Ext_Result$
END FUNCTION

FUNCTION XDict_GetNum# (handle AS LONG, keyText AS STRING)
    XDict_GetNum# = qb_ext_dict_get_num#(handle, _OFFSET(keyText), LEN(keyText))
END FUNCTION

SUB XDict_Remove (handle AS LONG, keyText AS STRING)
    DIM r AS LONG
    r = qb_ext_dict_remove&(handle, _OFFSET(keyText), LEN(keyText))
END SUB

FUNCTION XDict_Key$ (handle AS LONG, index AS LONG)
    IF qb_ext_dict_key&&(handle, index) > 0 THEN XDict_Key$ = Ext_Result$
END FUNCTION
