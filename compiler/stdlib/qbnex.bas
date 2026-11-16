'===============================================================================
' QBNex Standard Library ('$IMPORT:'qbnex')
'===============================================================================
' Every module in one import. Each one can also be imported on its own, before
' or after 'qbnex', and is loaded once:
'
'   collections  List, Dict, Set          strings   Str_* text helpers
'   json         parse, read, build       url       Url TYPE, encode, resolve
'   path         file paths               csv       CSV rows
'   math         Math_*                   sys       platform, env, arguments
'   result       Result TYPE              core      LastError$
'   regex, hash, zip, http, utf8, datetime   (native: not in the web build)
'   oop.class, oop.interface                  (used by CLASS)
'===============================================================================

'$IMPORT:'core'
'$IMPORT:'collections'
'$IMPORT:'strings'
'$IMPORT:'json'
'$IMPORT:'url'
'$IMPORT:'path'
'$IMPORT:'csv'
'$IMPORT:'math'
'$IMPORT:'sys'
'$IMPORT:'result'
'$IMPORT:'regex'
'$IMPORT:'hash'
'$IMPORT:'zip'
'$IMPORT:'http'
'$IMPORT:'utf8'
'$IMPORT:'datetime'
'$IMPORT:'oop.class'
'$IMPORT:'oop.interface'
