' Runtime include pack.
' Stdlib helpers are loaded before compiler-specific wrappers so the compiler can
' reuse the same generic path/text behavior instead of copying logic again.

'$INCLUDE:'..\stdlib\strings\text.bas'
'$INCLUDE:'..\stdlib\io\path.bas'
'$INCLUDE:'..\utilities\strings.bas'
'$INCLUDE:'..\utilities\config.bas'
'$INCLUDE:'..\utilities\file.bas'
'$INCLUDE:'..\subs_functions\extensions\opengl\opengl_methods.bas'
'$INCLUDE:'..\utilities\ini-manager\ini.bm'
'$INCLUDE:'..\utilities\error_handler.bas'
