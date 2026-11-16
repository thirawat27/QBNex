'===============================================================================
' QBNex Standard Library - zip ('$IMPORT:'zip')
'===============================================================================
' Zip archives, like Python's zipfile. Member names use "/" between folders.
' Errors leave LastError set.
'
'   DIM names AS List
'   Zip_List "a.zip", names              the member names
'   s$ = Zip_Read("a.zip", "dir/f.txt") one member
'   Zip_Extract "a.zip", "outdir"        every member (refuses "../x" names)
'   z& = Zip_Create("new.zip")
'   Zip_Write z&, "hello.txt", "hello"   a member from a string
'   Zip_AddFile z&, "docs/a.md", "a.md"  a member from a file
'   Zip_Close z&
'===============================================================================

'$IMPORT:'ext'
'$IMPORT:'collections'

DECLARE LIBRARY
    FUNCTION qb_ext_zip_list& (BYVAL path AS _OFFSET, BYVAL pl AS _INTEGER64)
    FUNCTION qb_ext_zip_read&& (BYVAL path AS _OFFSET, BYVAL pl AS _INTEGER64, BYVAL entryName AS _OFFSET, BYVAL nl AS _INTEGER64)
    FUNCTION qb_ext_zip_extract& (BYVAL path AS _OFFSET, BYVAL pl AS _INTEGER64, BYVAL dest AS _OFFSET, BYVAL dl AS _INTEGER64)
    FUNCTION qb_ext_zip_create& (BYVAL path AS _OFFSET, BYVAL pl AS _INTEGER64)
    FUNCTION qb_ext_zip_add& (BYVAL h AS LONG, BYVAL entryName AS _OFFSET, BYVAL nl AS _INTEGER64, BYVAL d AS _OFFSET, BYVAL dl AS _INTEGER64, BYVAL kind AS LONG)
    FUNCTION qb_ext_zip_close& (BYVAL h AS LONG)
END DECLARE

' Replaces the items of names with the member names of the archive.
SUB Zip_List (archive AS STRING, names AS List)
    DIM n AS LONG, i AS LONG
    n = qb_ext_zip_list&(_OFFSET(archive), LEN(archive))
    Ext_Sync
    List_Clear names
    FOR i = 0 TO n - 1
        List_Add names, Ext_Part$(i)
    NEXT
END SUB

' The bytes of one member.
FUNCTION Zip_Read$ (archive AS STRING, member AS STRING)
    DIM r AS _INTEGER64
    r = qb_ext_zip_read&&(_OFFSET(archive), LEN(archive), _OFFSET(member), LEN(member))
    Ext_Sync
    IF r >= 0 THEN Zip_Read$ = Ext_Result$
END FUNCTION

' Writes every member under folder.
SUB Zip_Extract (archive AS STRING, folder AS STRING)
    DIM n AS LONG
    n = qb_ext_zip_extract&(_OFFSET(archive), LEN(archive), _OFFSET(folder), LEN(folder))
    Ext_Sync
END SUB

' Starts a new archive; returns its handle (0 on error).
FUNCTION Zip_Create& (archive AS STRING)
    DIM h AS LONG
    h = qb_ext_zip_create&(_OFFSET(archive), LEN(archive))
    Ext_Sync
    IF h > 0 THEN Zip_Create& = h
END FUNCTION

' Adds a member holding bytes.
SUB Zip_Write (z AS LONG, member AS STRING, bytes AS STRING)
    DIM r AS LONG
    r = qb_ext_zip_add&(z, _OFFSET(member), LEN(member), _OFFSET(bytes), LEN(bytes), 0)
    Ext_Sync
END SUB

' Adds a member with the contents of the file source.
SUB Zip_AddFile (z AS LONG, member AS STRING, source AS STRING)
    DIM r AS LONG
    r = qb_ext_zip_add&(z, _OFFSET(member), LEN(member), _OFFSET(source), LEN(source), 1)
    Ext_Sync
END SUB

' Adds an empty folder member.
SUB Zip_AddFolder (z AS LONG, member AS STRING)
    DIM r AS LONG
    r = qb_ext_zip_add&(z, _OFFSET(member), LEN(member), 0, 0, 2)
    Ext_Sync
END SUB

' Finishes the archive and writes it.
SUB Zip_Close (z AS LONG)
    DIM r AS LONG
    r = qb_ext_zip_close&(z)
    Ext_Sync
END SUB
