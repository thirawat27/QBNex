#!/usr/bin/env sh
set -eu
# A compiled program in a real X11 window (Xvfb, no QBNEX_HEADLESS): the window
# shows the drawing and takes a key. Skipped where Xvfb or ImageMagick is missing.
export QBNEX_NO_ERROR_DIALOG=1
unset QBNEX_HEADLESS || true

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/../.." && pwd)"
QB="$REPO_ROOT/qb"
SRC="tests/fixtures/window_draw.bas"

for tool in xvfb-run xdotool; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "[SKIP] window smoke: $tool is not installed"
    exit 0
  fi
done
if [ ! -x "$QB" ]; then
  echo "[FAIL] qb not found at \"$QB\""
  exit 2
fi

mkdir -p "$REPO_ROOT/temp"
cd "$REPO_ROOT"
SMOKE_TMP="$(mktemp -d "./temp/qbnex_window_smoke_XXXXXX")"
EXE="$SMOKE_TMP/window_draw"
trap 'rm -rf "$SMOKE_TMP"' EXIT

# Window commands given while the window does not exist yet ($SCREENHIDE) are
# kept and applied when _SCREENSHOW makes it (they used to be dropped).
cat > "$SMOKE_TMP/pending.bas" <<'BAS'
$SCREENHIDE
SCREEN _NEWIMAGE(320, 200, 32)
_TITLE "QBNex pending smoke"
_SCREENMOVE 120, 80
_SCREENSHOW
OPEN COMMAND$ + ".ready" FOR OUTPUT AS #1: CLOSE #1
_DELAY 4
SYSTEM
BAS
"$QB" "$SMOKE_TMP/pending.bas" -o "$SMOKE_TMP/pending" > "$SMOKE_TMP/build.txt" 2>&1 || { cat "$SMOKE_TMP/build.txt"; echo "[FAIL] window smoke: build"; exit 1; }
cat > "$SMOKE_TMP/pending.sh" <<'DRIVE'
set -eu
exe="$1"; tag="$2"
"$exe" "$tag" 2> "$tag.stderr" & pid=$!
i=0; while [ ! -f "$tag.ready" ] && [ $i -lt 100 ]; do sleep 0.1; i=$((i + 1)); done
sleep 1
w="$(timeout 10 xdotool search --sync --limit 1 --name "QBNex pending smoke")"
xdotool getwindowgeometry "$w" > "$tag.geometry"
wait $pid
DRIVE
if ! timeout 60 xvfb-run -a -s "-screen 0 800x600x24" sh "$SMOKE_TMP/pending.sh" "$SMOKE_TMP/pending" "$SMOKE_TMP/pend"; then
  cat "$SMOKE_TMP/pend.stderr" 2>/dev/null || true
  echo "[FAIL] window smoke: the hidden window did not show"
  exit 1
fi
if ! grep -q "Position: 120,80 " "$SMOKE_TMP/pend.geometry"; then
  cat "$SMOKE_TMP/pend.geometry"
  echo "[FAIL] window smoke: _SCREENMOVE before _SCREENSHOW was lost"
  exit 1
fi
echo "[PASS] window smoke: _SCREENMOVE while hidden applied at _SCREENSHOW"

for tool in import convert; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "[SKIP] window smoke (drawing): $tool is not installed"
    exit 0
  fi
done
"$QB" "$SRC" -o "$EXE" > "$SMOKE_TMP/build.txt" 2>&1 || { cat "$SMOKE_TMP/build.txt"; echo "[FAIL] window smoke: build"; exit 1; }

cat > "$SMOKE_TMP/drive.sh" <<'DRIVE'
set -eu
exe="$1"; tag="$2"; shot="$3"
"$exe" "$tag" 2> "$tag.stderr" & pid=$!
i=0; while [ ! -f "$tag.ready" ] && [ $i -lt 100 ]; do sleep 0.1; i=$((i + 1)); done
sleep 1
import -window root "$shot"
timeout 15 xdotool search --sync --limit 1 --name "QBNex window smoke" windowfocus key Escape || true
wait $pid
DRIVE

TAG="$SMOKE_TMP/run"
SHOT="$SMOKE_TMP/shot.png"
if ! timeout 60 xvfb-run -a -s "-screen 0 800x600x24" sh "$SMOKE_TMP/drive.sh" "$EXE" "$TAG" "$SHOT"; then
  cat "$TAG.stderr" 2>/dev/null || true
  echo "[FAIL] window smoke: the program did not end cleanly"
  exit 1
fi
if [ -s "$TAG.stderr" ]; then
  cat "$TAG.stderr"
  echo "[FAIL] window smoke: the program wrote to stderr (no window?)"
  exit 1
fi
colors="$(convert "$SHOT" -format %c histogram:info:)"
for c in '#FF0000' '#00FF00' '#000080' '#FFFFFF'; do
  case "$colors" in
    *"$c"*) ;;
    *) echo "$colors" | head -20; echo "[FAIL] window smoke: colour $c is not on the screen"; exit 1 ;;
  esac
done
if [ "$(tr -d ' \r' < "$TAG.done")" != "-1" ]; then
  echo "[FAIL] window smoke: the window did not get the Esc key"
  exit 1
fi
echo "[PASS] window smoke: drawing on screen, Esc received"
