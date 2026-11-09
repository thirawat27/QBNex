rem R8: freeglut/GLEW were removed (the window is the Rust runtime, crates/qb-runtime).
rem The frozen stage0 compiler (internal/source, see L-00) still runs this script when src.a is missing.
type NUL > src.a
