'===============================================================================
' QBNex Standard Library - result ('$IMPORT:'result')
'===============================================================================
' A value or an error, returned from your own SUBs instead of raising ERROR.
' Read the fields directly; the SUBs set them.
'
'   SUB LoadConfig (r AS Result)
'       IF _FILEEXISTS("app.ini") = 0 THEN r.Fail "app.ini is missing", 53: EXIT SUB
'       r.Ok "loaded"
'   END SUB
'
'   DIM r AS Result
'   LoadConfig r
'   IF r.Failed THEN r.Context "starting": PRINT r.Message ELSE PRINT r.Value
'===============================================================================

TYPE Result
    Failed AS LONG
    Value AS STRING
    Message AS STRING
    Code AS LONG
END TYPE

' Success with value (clears an earlier error).
SUB Result_Ok (r AS Result, OPTIONAL value AS STRING = "")
    r.Failed = 0
    r.Value = value
    r.Message = ""
    r.Code = 0
END SUB

' An error with message and code (default 1).
SUB Result_Fail (r AS Result, message AS STRING, OPTIONAL code AS LONG = 1)
    r.Failed = -1
    r.Value = ""
    r.Message = message
    r.Code = code
END SUB

' Puts "where: " in front of the message of an error, to say where it happened.
SUB Result_Context (r AS Result, where AS STRING)
    IF r.Failed THEN r.Message = where + ": " + r.Message
END SUB
