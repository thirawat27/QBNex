#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/.." && pwd)"
QB="$REPO_ROOT/qb"
SRC_TINY_REL="tests/fixtures/label_recompile_success.bas"
SRC_WARN_REL="tests/fixtures/unused_variable_warning.bas"
SRC_MEDIUM_REL="tests/fixtures/benchmark_large.bas"
SRC_LARGE_REL="tests/fixtures/benchmark_stress.bas"

if [ ! -x "$QB" ]; then
  echo "[FAIL] qb not found at \"$QB\""
  echo "Build QBNex first, then run this benchmark again."
  exit 2
fi

if [ ! -f "$REPO_ROOT/$SRC_TINY_REL" ] || [ ! -f "$REPO_ROOT/$SRC_WARN_REL" ] || [ ! -f "$REPO_ROOT/$SRC_MEDIUM_REL" ] || [ ! -f "$REPO_ROOT/$SRC_LARGE_REL" ]; then
  echo "[FAIL] One or more benchmark fixtures are missing."
  exit 2
fi

mkdir -p "$REPO_ROOT/temp"
cd "$REPO_ROOT"

TMPDIR="$(mktemp -d "./temp/qbnex_benchmark_XXXXXX")"
BIN_TINY="$TMPDIR/tiny_output"
BIN_WARN="$TMPDIR/warn_output"
BIN_MEDIUM="$TMPDIR/medium_output"
BIN_LARGE="$TMPDIR/large_output"

measure_case() {
  case_name=$1
  iterations=$2
  source_path=$3
  output_path=$4
  extra_flag=${5:-}
  total_ms=0
  current=1

  while [ "$current" -le "$iterations" ]; do
    rm -f "$output_path"
    if [ -n "$extra_flag" ]; then
      result="$(perl -MTime::HiRes=time -e '
        my $start = time;
        {
          local *STDOUT;
          local *STDERR;
          open STDOUT, ">", "/dev/null" or die $!;
          open STDERR, ">", "/dev/null" or die $!;
          system @ARGV;
        }
        my $status = $? >> 8;
        my $elapsed = int((time - $start) * 1000 + 0.5);
        print "$status $elapsed";
      ' "$QB" "$source_path" "$extra_flag" -o "$output_path")"
    else
      result="$(perl -MTime::HiRes=time -e '
        my $start = time;
        {
          local *STDOUT;
          local *STDERR;
          open STDOUT, ">", "/dev/null" or die $!;
          open STDERR, ">", "/dev/null" or die $!;
          system @ARGV;
        }
        my $status = $? >> 8;
        my $elapsed = int((time - $start) * 1000 + 0.5);
        print "$status $elapsed";
      ' "$QB" "$source_path" -o "$output_path")"
    fi

    status=${result%% *}
    elapsed_ms=${result##* }

    if [ "$status" -ne 0 ]; then
      echo "[FAIL] benchmark case failed: $case_name exit=$status"
      rm -rf "$TMPDIR"
      exit 1
    fi
    if [ ! -f "$output_path" ]; then
      echo "[FAIL] benchmark output missing: $case_name"
      rm -rf "$TMPDIR"
      exit 1
    fi

    total_ms=$((total_ms + elapsed_ms))
    current=$((current + 1))
  done

  avg_ms=$((total_ms / iterations))
  printf '%-14s %10s %7s\n' "$case_name" "$iterations" "$avg_ms"
}

echo "BENCHMARK_SMOKE_OK"
printf '%-14s %10s %7s\n' "Case" "Iterations" "AvgMs"
measure_case "tiny-success" 5 "$SRC_TINY_REL" "$BIN_TINY"
measure_case "warning-path" 5 "$SRC_WARN_REL" "$BIN_WARN" "-w"
measure_case "medium-parse" 3 "$SRC_MEDIUM_REL" "$BIN_MEDIUM"
measure_case "large-parse" 2 "$SRC_LARGE_REL" "$BIN_LARGE"

rm -rf "$TMPDIR"
