#!/usr/bin/env sh
set -eu
# Runtime errors go to stderr (exit code 1) instead of a modal dialog.
export QBNEX_NO_ERROR_DIALOG=1

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/../.." && pwd)"
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

SMOKE_TMP="$(mktemp -d "./temp/qbnex_audio_smoke_XXXXXX")"
OUT_ZMODE="$SMOKE_TMP/audio_zmode.txt"
OUT_LINK="$SMOKE_TMP/audio_link.txt"
AUDIO_EXE="$SMOKE_TMP/audio_synth"
SIMPLE_SRC="$SMOKE_TMP/simple_beep.bas"
SIMPLE_EXE="$SMOKE_TMP/simple_beep"
OUT_SIMPLE="$SMOKE_TMP/simple_beep.txt"

{
  echo 'PRINT "simple"'
  echo 'BEEP'
} >"$SIMPLE_SRC"

set +e
"$QB" "$SRC" -z >"$OUT_ZMODE" 2>&1
EC_ZMODE=$?
"$QB" "$SRC" -o "$AUDIO_EXE" >"$OUT_LINK" 2>&1
EC_LINK=$?
"$QB" "$SIMPLE_SRC" -o "$SIMPLE_EXE" >"$OUT_SIMPLE" 2>&1
EC_SIMPLE=$?
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

audio_size="$(wc -c <"$AUDIO_EXE" | tr -d ' ')"
simple_size="$(wc -c <"$SIMPLE_EXE" | tr -d ' ')"
if [ "$simple_size" -ge "$audio_size" ]; then
  echo "[FAIL] Simple PRINT+BEEP executable should stay smaller than audio synth executable."
  echo "AUDIO_SMOKE_FAIL"
  echo "simple=$simple_size audio=$audio_size"
  exit 1
fi

echo "AUDIO_SMOKE_OK"
rm -rf "$SMOKE_TMP"
exit 0
