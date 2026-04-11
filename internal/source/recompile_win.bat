@echo off
cd %0\..\
echo Recompiling...
cd ../c
c_compiler\bin\g++ -mconsole -s -Wfatal-errors -w -Wall qbx.cpp  libqb\os\win\libqb_2_1.1_0000000001000.o  ..\..\.\internal\temp\icon.o -D DEPENDENCY_NO_SOCKETS -D DEPENDENCY_NO_PRINTER -D DEPENDENCY_ICON -D DEPENDENCY_NO_SCREENIMAGE   parts\core\os\win\src.a -lopengl32 -lglu32   -mwindows -static-libgcc -static-libstdc++ -D GLEW_STATIC -D FREEGLUT_STATIC     -lwinmm -lgdi32 -o "..\..\qb64.exe"
pause
