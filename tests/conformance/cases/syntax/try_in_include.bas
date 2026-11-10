$CONSOLE:ONLY
' TRY inside a FUNCTION that comes from an $INCLUDE file
PRINT SafeDiv$(53)
PRINT SafeDiv$(0)
SYSTEM
'$INCLUDE:'try_helper.bi'
