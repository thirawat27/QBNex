$CONSOLE:ONLY
' LPRINT fills a printer page; the runner keeps the page in a file (QBNEX_LPRINT)
LPRINT "Hello";
LPRINT " printer"
LPRINT 1; 2, 3
PRINT LPOS(1)
LPRINT "x";
PRINT LPOS(1)
LPRINT
WIDTH LPRINT 10
LPRINT "abcdefghijklmn"
LPRINT USING "##.#"; 3.14159
LPRINT "page"; CHR$(12); "next page"
PRINT "done"
SYSTEM
