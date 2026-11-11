' c%() and c!() share the name c; REDIM c%() must resize c%(), not c!()
DIM c%(0), c!(3)
REDIM c%(1 TO 5)
c%(5) = 9: c!(3) = 1.5
PRINT UBOUND(c%), UBOUND(c!), c%(5), c!(3)

