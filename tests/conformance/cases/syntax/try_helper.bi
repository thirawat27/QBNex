' used by try_in_include.bas
FUNCTION SafeDiv$ (code AS INTEGER)
    TRY
        IF code THEN ERROR code
        SafeDiv$ = "ok"
    CATCH e
        SafeDiv$ = "error" + STR$(e)
    END TRY
END FUNCTION
