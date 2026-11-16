' Standard library calls without the type suffix of the FUNCTION:
' plain calls, calls without arguments and method calls.
'$IMPORT:'qbnex'
DIM s AS STRING
s = Str_Replace("hello", "l", "L")
PRINT s; " "; Str_PadLeft("7", 3, "0"); " "; Path_Base("a/b.txt")
PRINT Utf8_Len("ab"); Math_Clamp(15, 0, 10); LEN(LastError)
PRINT HEX$(Hash_Crc32("123456789"))
o = Json_Object
Json_SetStr o, "a", "b"
PRINT Json_Stringify(o)
s = Regex_Find("\d+", "ab12")
PRINT s; Regex_Pos
DIM u AS Url
u.Parse "http://h/p?q=1"
PRINT u.Query("q"); " "; u.Format
DIM d AS Dict
d.Set "k", "v"
PRINT d.Get("k"); d.Get("zz", "-"); d.Key(0); d.Value(0)
DIM l AS List
l.Add "x": l.Add "y"
PRINT l.Get(-1); l.Join("+"); l.Count; List_Count(l)
PRINT l.Pop; l.Pop(0); l.Count
