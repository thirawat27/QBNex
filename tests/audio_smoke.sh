#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/.." && pwd)"
QB="$REPO_ROOT/qb"
SRC="tests/fixtures/audio_synth_success.bas"

if [ ! -x "$QB" ]; then
  echo "[FAIL] qb not found at \"$QB\""
  echo "Build QBNex first, then run this smoke test again."
  exit 2
fi

if [ ! -f "$REPO_ROOT/$SRC" ]; then
  echo "[FAIL] Audio fixture source not found at \"$REPO_ROOT/$SRC\""
  exit 2
fi

mkdir -p "$REPO_ROOT/temp"
cd "$REPO_ROOT"

TMPDIR="$(mktemp -d "./temp/qbnex_audio_smoke_XXXXXX")"
OUT_ZMODE="$TMPDIR/audio_zmode.txt"
OUT_LINK="$TMPDIR/audio_link.txt"
AUDIO_EXE="$TMPDIR/audio_synth"
SIMPLE_SRC="$TMPDIR/simple_beep.bas"
SIMPLE_EXE="$TMPDIR/simple_beep"
OUT_SIMPLE="$TMPDIR/simple_beep.txt"
TEXTUI_SRC="$TMPDIR/text_ui.bas"
TEXTUI_EXE="$TMPDIR/text_ui"
OUT_TEXTUI="$TMPDIR/text_ui.txt"

{
  echo 'PRINT "simple"'
  echo 'BEEP'
} >"$SIMPLE_SRC"

{
  echo 'CLS'
  echo 'COLOR 14, 1'
  echo 'LOCATE 2, 4'
  echo 'PRINT "text ui"'
  echo 'BEEP'
} >"$TEXTUI_SRC"

set +e
"$QB" "$SRC" -z >"$OUT_ZMODE" 2>&1
EC_ZMODE=$?
"$QB" "$SRC" -o "$AUDIO_EXE" >"$OUT_LINK" 2>&1
EC_LINK=$?
"$QB" "$SIMPLE_SRC" -o "$SIMPLE_EXE" >"$OUT_SIMPLE" 2>&1
EC_SIMPLE=$?
"$QB" "$TEXTUI_SRC" -o "$TEXTUI_EXE" >"$OUT_TEXTUI" 2>&1
EC_TEXTUI=$?
set -e

if [ "$EC_ZMODE" -ne 0 ]; then
  echo "[FAIL] Audio synth fixture should compile in -z mode."
  echo "AUDIO_SMOKE_FAIL"
  echo "Inspect output: \"$OUT_ZMODE\""
  exit 1
fi

if [ "$EC_LINK" -ne 0 ]; then
  echo "[FAIL] Audio synth fixture should link as an executable."
  echo "AUDIO_SMOKE_FAIL"
  echo "Inspect output: \"$OUT_LINK\""
  exit 1
fi

if [ "$EC_SIMPLE" -ne 0 ]; then
  echo "[FAIL] Simple PRINT+BEEP program should link without the audio runtime."
  echo "AUDIO_SMOKE_FAIL"
  echo "Inspect output: \"$OUT_SIMPLE\""
  exit 1
fi

if [ "$EC_TEXTUI" -ne 0 ]; then
  echo "[FAIL] Text UI program should link without GUI/audio runtime."
  echo "AUDIO_SMOKE_FAIL"
  echo "Inspect output: \"$OUT_TEXTUI\""
  exit 1
fi

audio_size="$(wc -c <"$AUDIO_EXE" | tr -d ' ')"
simple_size="$(wc -c <"$SIMPLE_EXE" | tr -d ' ')"
textui_size="$(wc -c <"$TEXTUI_EXE" | tr -d ' ')"
if [ "$simple_size" -ge "$audio_size" ]; then
  echo "[FAIL] Simple PRINT+BEEP executable should stay smaller than audio synth executable."
  echo "AUDIO_SMOKE_FAIL"
  echo "simple=$simple_size audio=$audio_size"
  exit 1
fi

if [ "$textui_size" -ge "$audio_size" ]; then
  echo "[FAIL] Text UI executable should stay smaller than audio synth executable."
  echo "AUDIO_SMOKE_FAIL"
  echo "textui=$textui_size audio=$audio_size"
  exit 1
fi

echo "AUDIO_SMOKE_OK"
rm -rf "$TMPDIR"
exit 0
