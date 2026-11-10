'===============================================================================
' QBNex Standard Library - Zip archives ('$IMPORT:'ext.zip')
'===============================================================================
'   n& = Zip_List&("a.zip")              members, -1 on error; then
'        Zip_EntryName$(i&), Zip_EntrySize&&(i&) for i& = 0 .. n& - 1
'   s$ = Zip_ReadEntry$("a.zip", "dir/file.txt")
'   n& = Zip_Extract&("a.zip", "outdir")  files written; refuses names that
'                                         would leave outdir ("../x")
'   h& = Zip_Create&("new.zip")           then Zip_AddData& / Zip_AddFile& /
'        Zip_AddFolder& (0 = ok, -1 = error) and Zip_Close&(h&)
' Member names use "/" between folders. Errors set Ext_LastError$.
'===============================================================================

'$IMPORT:'ext.core'

DECLARE LIBRARY
    FUNCTION qb_ext_zip_list& (BYVAL path AS _OFFSET, BYVAL pl AS _INTEGER64)
    FUNCTION qb_ext_zip_entry_size&& (BYVAL index AS LONG)
    FUNCTION qb_ext_zip_read&& (BYVAL path AS _OFFSET, BYVAL pl AS _INTEGER64, BYVAL entryName AS _OFFSET, BYVAL nl AS _INTEGER64)
    FUNCTION qb_ext_zip_extract& (BYVAL path AS _OFFSET, BYVAL pl AS _INTEGER64, BYVAL dest AS _OFFSET, BYVAL dl AS _INTEGER64)
    FUNCTION qb_ext_zip_create& (BYVAL path AS _OFFSET, BYVAL pl AS _INTEGER64)
    FUNCTION qb_ext_zip_add& (BYVAL h AS LONG, BYVAL entryName AS _OFFSET, BYVAL nl AS _INTEGER64, BYVAL d AS _OFFSET, BYVAL dl AS _INTEGER64, BYVAL kind AS LONG)
    FUNCTION qb_ext_zip_close& (BYVAL h AS LONG)
END DECLARE

DIM SHARED ZipNames$(0 TO 65535)
DIM SHARED ZipNameCount&

FUNCTION Zip_List& (zipPath AS STRING)
    DIM n AS LONG, i AS LONG
    n = qb_ext_zip_list&(_OFFSET(zipPath), LEN(zipPath))
    ZipNameCount& = 0
    IF n > 0 THEN
        IF n > 65536 THEN n = 65536
        FOR i = 0 TO n - 1
            ZipNames$(i) = Ext_Part$(i)
        NEXT
        ZipNameCount& = n
    END IF
    Zip_List& = n
END FUNCTION

' Name of member i (0-based) of the last Zip_List&.
FUNCTION Zip_EntryName$ (index AS LONG)
    IF index >= 0 AND index < ZipNameCount& THEN Zip_EntryName$ = ZipNames$(index)
END FUNCTION

' Uncompressed size of member i of the last Zip_List& (-1 if none).
FUNCTION Zip_EntrySize&& (index AS LONG)
    Zip_EntrySize&& = qb_ext_zip_entry_size&&(index)
END FUNCTION

FUNCTION Zip_ReadEntry$ (zipPath AS STRING, entryName AS STRING)
    IF qb_ext_zip_read&&(_OFFSET(zipPath), LEN(zipPath), _OFFSET(entryName), LEN(entryName)) < 0 THEN EXIT FUNCTION
    Zip_ReadEntry$ = Ext_Result$
END FUNCTION

FUNCTION Zip_Extract& (zipPath AS STRING, destFolder AS STRING)
    Zip_Extract& = qb_ext_zip_extract&(_OFFSET(zipPath), LEN(zipPath), _OFFSET(destFolder), LEN(destFolder))
END FUNCTION

FUNCTION Zip_Create& (zipPath AS STRING)
    Zip_Create& = qb_ext_zip_create&(_OFFSET(zipPath), LEN(zipPath))
END FUNCTION

FUNCTION Zip_AddData& (handle AS LONG, entryName AS STRING, dataText AS STRING)
    Zip_AddData& = qb_ext_zip_add&(handle, _OFFSET(entryName), LEN(entryName), _OFFSET(dataText), LEN(dataText), 0)
END FUNCTION

FUNCTION Zip_AddFile& (handle AS LONG, entryName AS STRING, sourcePath AS STRING)
    Zip_AddFile& = qb_ext_zip_add&(handle, _OFFSET(entryName), LEN(entryName), _OFFSET(sourcePath), LEN(sourcePath), 1)
END FUNCTION

FUNCTION Zip_AddFolder& (handle AS LONG, entryName AS STRING)
    Zip_AddFolder& = qb_ext_zip_add&(handle, _OFFSET(entryName), LEN(entryName), 0, 0, 2)
END FUNCTION

FUNCTION Zip_Close& (handle AS LONG)
    Zip_Close& = qb_ext_zip_close&(handle)
END FUNCTION
