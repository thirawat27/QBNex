# This script lives in scripts/; everything below runs from the repository root.
cd "$(dirname "$0")/.."
set -u

# QIR pipeline (no C/C++ toolchain), opt-in until the cutover: see docs/rust-port/cutover-plan.md
if [ "${QBNEX_QIR_CUTOVER:-}" = "1" ]; then
  exec sh scripts/setup_qir.sh --enable
fi
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

mkdir -p ./internal/temp
find ./internal/temp -mindepth 1 -maxdepth 1 -exec rm -rf {} \;

if ! command -v clang++ >/dev/null 2>&1; then
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

echo "Building library 'qb-runtime' (Rust runtime)"
if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo was not found. Install Rust from https://rustup.rs and run this setup script again."
  Pause
  exit 1
fi
pushd crates/qb-runtime/os/macos >/dev/null
rm -f libqb_runtime.a
sh ./setup_build.command
if [ ! -f ./libqb_runtime.a ]; then
  echo "Compilation of ./crates/qb-runtime/os/macos/libqb_runtime.a failed!"
  Pause
  exit 1
fi
popd >/dev/null

echo "Building 'QBNex' (~3 min)"
mkdir -p ./internal/temp
cp ./internal/source/* ./internal/temp/
cp -r ./compiler/* ./internal/temp/
pushd internal/c >/dev/null
clang++ -w qbx.cpp libqb/os/osx/libqb_setup.o ../../crates/qb-runtime/os/macos/libqb_runtime.a -lSystem -lc -lm -framework AppKit -framework CoreGraphics -framework QuartzCore -framework Foundation -framework IOKit -framework CoreFoundation -lobjc -framework Cocoa -o ../../qb-stage0
popd >/dev/null

echo ""
if [ -f ./qb-stage0 ]; then
  rm -f ./qb
  echo "Self-hosting 'QBNex'"
  ./qb-stage0 ./compiler/qbnex.bas -o ./qb
  chmod +x ./qb
fi

if [ -f ./qb ]; then
  if [ -z "${QBNEX_KEEP_STAGE0:-}" ]; then
    rm -f ./qb-stage0
  fi
  # Product-name alias: users can run either ./qb or ./qbnex.
  rm -f ./qbnex
  ln -s qb ./qbnex 2>/dev/null || cp ./qb ./qbnex
  cleanup_setup_artifacts
  echo "QBNex CLI compiler is ready:"
  echo "  ./qb yourfile.bas"
  echo "  ./qbnex yourfile.bas"
  echo ""
  if [ -z "${QBNEX_CI:-}" ]; then
    echo "Press any key to continue..."
    Pause
  fi
else
  cleanup_setup_artifacts
  echo "Compilation of QBNex failed!"
  if [ -z "${QBNEX_CI:-}" ]; then
    Pause
  fi
  exit 1
fi
