'===============================================================================
' QBNex Hash Table Compatibility Module
'===============================================================================
' Stage0-compatible bootstrap shim.
' The main compiler still uses the legacy hash-table implementation embedded in
' source/qbnex.bas, so this module only needs lightweight initialization hooks.
'===============================================================================

DIM SHARED HashTableInitialized AS _BYTE

SUB InitMainHashTable
    HashTableInitialized = -1
END SUB

SUB CleanupMainHashTable
    HashTableInitialized = 0
END SUB
