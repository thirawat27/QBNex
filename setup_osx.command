cd "$(dirname "$0")"
set -u
Pause()
{
OLDCONFIG=`stty -g`
stty -icanon -echo min 1 time 0
dd count=1 2>/dev/null
stty $OLDCONFIG
}

cleanup_setup_artifacts()
{
rm -f ./temp.7z ./7zr ./7zr.exe >/dev/null 2>&1 || true
rm -rf ./mingw32 ./mingw64 >/dev/null 2>&1 || true
}

echo "QBNex Setup"
echo ""

cleanup_setup_artifacts
find . -name "*.command" -exec chmod +x {} \;

pushd internal/c/libqb >/dev/null
find . -type f -iname "*.a" -exec rm -f {} \;
find . -type f -iname "*.o" -exec rm -f {} \;
popd >/dev/null

pushd internal/c/parts >/dev/null
find . -type f -iname "*.a" -exec rm -f {} \;
find . -type f -iname "*.o" -exec rm -f {} \;
popd >/dev/null

mkdir -p ./internal/temp
find ./internal/temp -mindepth 1 -maxdepth 1 -exec rm -rf {} \;

if [ -z "$(which clang++)" ]; then
  echo "Apple's C++ compiler not found."
  echo "Attempting to install Apple's Command Line Tools for Xcode..."
  echo "After installation is finished, run this setup script again."
  xcode-select --install
  Pause
  exit 1
fi

echo "Building library 'LibQB'"
pushd internal/c/libqb/os/osx >/dev/null
rm -f libqb_setup.o
./setup_build.command
if [ ! -f ./libqb_setup.o ]; then
  echo "Compilation of ./internal/c/libqb/os/osx/libqb_setup.o failed!"
  Pause
  exit 1
fi
popd >/dev/null

echo "Building library 'FreeType'"
pushd internal/c/parts/video/font/ttf/os/osx >/dev/null
rm -f src.o
./setup_build.command
if [ ! -f ./src.o ]; then
  echo "Compilation of ./internal/c/parts/video/font/ttf/os/osx/src.o failed!"
  Pause
  exit 1
fi
popd >/dev/null

echo "Building 'QBNex' (~3 min)"
mkdir -p ./internal/temp
cp ./internal/source/* ./internal/temp/
cp -r ./source/* ./internal/temp/
pushd internal/c >/dev/null
clang++ -w qbx.cpp libqb/os/osx/libqb_setup.o parts/video/font/ttf/os/osx/src.o -framework GLUT -framework OpenGL -framework Cocoa -o ../../qb-stage0
popd >/dev/null

echo ""
if [ -f ./qb-stage0 ]; then
  if [ "${QBNEX_BOOTSTRAP:-0}" = "1" ]; then
    echo "Bootstrapping compiler from source/qbnex.bas..."
    ./qb-stage0 ./source/qbnex.bas -o qb
  elif [ ! -f ./qb ]; then
    echo "Bootstrapping compiler from source/qbnex.bas..."
    ./qb-stage0 ./source/qbnex.bas -o qb
  fi
fi

if [ -f ./qb ]; then
  if [ -z "$QBNEX_KEEP_STAGE0" ]; then
    rm -f ./qb-stage0
  fi
  cleanup_setup_artifacts
  echo "QBNex CLI compiler is ready:"
  echo "  ./qb yourfile.bas"
  echo ""
  if [ -z "$QBNEX_CI" ]; then
    echo "Press any key to continue..."
    Pause
  fi
else
  cleanup_setup_artifacts
  echo "Compilation of QBNex failed!"
  if [ -z "$QBNEX_CI" ]; then
    Pause
  fi
  exit 1
fi
