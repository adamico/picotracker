#!/usr/bin/env bash
# Re-records the PICO-8 reference set with real PICO-8 and rewrites its fingerprints.
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/record-reference.sh [case ...]

Re-records the PICO-8 reference set on a machine with a licensed copy of PICO-8 0.2.7,
with no window and no sound, then rewrites the committed fingerprints.

  1. Writes the driver Carts to crates/pikutracker-core/tests/reference_set/carts/.
  2. Plays each Cart in PICO-8, which records crates/pikutracker-core/tests/reference_set/
     recordings/<case>.wav. The recordings are gitignored and stay on this machine.
  3. Measures each recording into tests/reference_set/fingerprints/<case>.ron, and writes
     our synth's samples beside it as recordings/<case>.synth.wav for A/B listening.

With case names, only those Carts are recorded again; every fingerprint is still rewritten
from the recordings on disk. Then run `cargo test --test reference_set` and commit the Carts
and fingerprints together.

PICO8=/path/to/pico8 overrides the pico8 on PATH. PICO-8 runs in its own home folder under
recordings/, so your PICO-8 config and Carts are left alone.

Loosening a limit in the comparison with these fingerprints needs an A/B listening check by
the owner, noted in the commit.
EOF
}

case "${1:-}" in
  -h | --help) usage; exit 0 ;;
esac

ROOT=$(cd "$(dirname "$0")/.." && pwd)
SET="$ROOT/crates/pikutracker-core/tests/reference_set"
RECORDINGS="$SET/recordings"
HOME_FOLDER="$RECORDINGS/pico8-home"
PICO8=${PICO8:-$(command -v pico8 || echo "$HOME/pico-8/pico8")}
EXPECTED_VERSION=43
TIMEOUT_SECONDS=60

[ -x "$PICO8" ] || { echo "PICO-8 not found at $PICO8: set PICO8=/path/to/pico8" >&2; exit 1; }

cd "$ROOT"
cargo test --quiet -p pikutracker-core --test reference_set -- --ignored --exact write_driver_carts

mkdir -p "$HOME_FOLDER/carts"
if [ $# -gt 0 ]; then
  CASES=("$@")
else
  CASES=()
  for cart in "$SET"/carts/*.p8; do CASES+=("$(basename "$cart" .p8)"); done
fi

for name in "${CASES[@]}"; do
  cart="$SET/carts/$name.p8"
  [ -f "$cart" ] || { echo "no case named $name" >&2; exit 1; }
  cp "$cart" "$HOME_FOLDER/carts/case.p8"
  log="$RECORDINGS/$name.log"
  rm -f "$HOME_FOLDER/carts/recording.wav"
  SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy \
    timeout "$TIMEOUT_SECONDS" "$PICO8" -home "$HOME_FOLDER" -run "$HOME_FOLDER/carts/case.p8" \
    >"$log" 2>&1 &
  pid=$!
  for _ in $(seq 1 "$TIMEOUT_SECONDS"); do
    grep -q ALLDONE "$log" 2>/dev/null && break
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.5
  done
  sleep 0.5
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  grep -q ALLDONE "$log" || { echo "$name: PICO-8 did not finish, see $log" >&2; exit 1; }
  grep -q "^VERSION $EXPECTED_VERSION\$" "$log" \
    || { echo "$name: not PICO-8 0.2.7 (stat(5) is not $EXPECTED_VERSION), see $log" >&2; exit 1; }
  mv "$HOME_FOLDER/carts/recording.wav" "$RECORDINGS/$name.wav"
  echo "recorded $name"
done

PIKUTRACKER_RECORDINGS="$RECORDINGS" \
  cargo test --quiet -p pikutracker-core --test reference_set -- --ignored --exact \
  write_fingerprints write_synth_recordings
echo "fingerprints written to $SET/fingerprints"
echo "our synth's samples written beside the recordings as <case>.synth.wav"
