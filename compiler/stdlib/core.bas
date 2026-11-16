'===============================================================================
' QBNex Standard Library - core ('$IMPORT:'core')
'===============================================================================
' LastError is the message of the last library call that failed, for every
' module ("" after a call that worked). Every other module imports this one.
'===============================================================================

DIM SHARED Core_ErrorText AS STRING

' Why the last library call failed; "" when it worked.
FUNCTION LastError$ ()
    LastError$ = Core_ErrorText
END FUNCTION

' Sets LastError ("" clears it). For modules that report errors.
SUB Core_SetError (message AS STRING)
    Core_ErrorText = message
END SUB
