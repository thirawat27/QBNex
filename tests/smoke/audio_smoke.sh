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
"$QB" "$SRC" -z -o "$SMOKE_TMP/audio_zmode" >"$OUT_ZMODE" 2>&1
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

# Sound handles on the silent device (it plays against the clock): a streamed
# file, _SNDLIMIT counted from the sound's start (it was engine time, so a
# sound played half a second into the program stopped at once).
PLAYBACK_SRC="$SMOKE_TMP/playback.bas"
PLAYBACK_EXE="$SMOKE_TMP/playback"
cat >"$PLAYBACK_SRC" <<'BAS'
$CONSOLE:ONLY
f$ = "qbnex_smoke_tone.wav"
n& = 22050
OPEN f$ FOR BINARY AS #1
h$ = "RIFF" + MKL$(36 + n& * 2) + "WAVEfmt " + MKL$(16) + MKI$(1) + MKI$(1) + MKL$(22050) + MKL$(44100) + MKI$(2) + MKI$(16) + "data" + MKL$(n& * 2)
PUT #1, , h$
s$ = SPACE$(n& * 2)
FOR i& = 0 TO n& - 1
    MID$(s$, i& * 2 + 1, 2) = MKI$(INT(SIN(i& / 10) * 8000))
NEXT
PUT #1, , s$
CLOSE #1
h& = _SNDOPEN(f$, "STREAM")
PRINT "stream"; h& > 0; "len"; INT(_SNDLEN(h&) * 10 + .5) / 10
_DELAY .5
_SNDPLAY h&
_SNDLIMIT h&, .4
_DELAY .15
PRINT "limit playing"; _SNDPLAYING(h&)
_DELAY .6
PRINT "limit stopped"; _SNDPLAYING(h&)
_SNDCLOSE h&
d& = _SNDOPEN(f$)
PRINT "decoded len"; INT(_SNDLEN(d&) * 10 + .5) / 10
_SNDLOOP d&
_SNDCLOSE d&
KILL f$
SYSTEM
BAS
OUT_PLAYBACK="$SMOKE_TMP/playback.txt"
if ! "$QB" "$PLAYBACK_SRC" -o "$PLAYBACK_EXE" >"$OUT_PLAYBACK" 2>&1; then
  echo "[FAIL] Sound handle program should compile."
  echo "AUDIO_SMOKE_FAIL"
  cat "$OUT_PLAYBACK"
  exit 1
fi
(cd "$SMOKE_TMP" && QB_AUDIO_DEVICE=null QBNEX_HEADLESS=1 timeout 60 ./playback </dev/null) >"$OUT_PLAYBACK" 2>&1 || true
EXPECTED_PLAYBACK="stream-1 len 1
limit playing-1
limit stopped 0
decoded len 1"
if [ "$(tr -d '\r' <"$OUT_PLAYBACK" | sed 's/ *$//')" != "$EXPECTED_PLAYBACK" ]; then
  echo "[FAIL] Sound handles on the silent device:"
  cat "$OUT_PLAYBACK"
  echo "AUDIO_SMOKE_FAIL"
  exit 1
fi

echo "AUDIO_SMOKE_OK"
rm -rf "$SMOKE_TMP"
exit 0
