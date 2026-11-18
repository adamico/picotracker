#!/usr/bin/env bash
# Generates the driver Carts and records them headless with real PICO-8.
#   ./run.sh                  all Carts (about 6 minutes)
#   ./run.sh expg exph        only these Carts
# PICO8=/path/to/pico8 overrides the default ~/pico-8/pico8.
set -u
S=$(cd "$(dirname "$0")" && pwd)
PICO8=${PICO8:-$HOME/pico-8/pico8}
mkdir -p "$S/p8home/carts"
[ -f "$S/p8home/carts/wavout.p8" ] || printf 'pico-8 cartridge // http://www.pico-8.com\nversion 43\n__lua__\n-- target\n' > "$S/p8home/carts/wavout.p8"
python3 "$S/gen.py" "$S/p8home/carts"
for g in gen2 gen3 gen4; do python3 "$S/$g.py" "$S/p8home/carts"; done
[ $# -gt 0 ] || set -- expa expb expc expd expe expf expg exph
for c in "$@"; do
  SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy "$PICO8" -home "$S/p8home" -run "$S/p8home/carts/$c.p8" > "$S/$c.log" 2>&1 &
  P=$!
  for _ in $(seq 1 300); do grep -q ALLDONE "$S/$c.log" 2>/dev/null && break; kill -0 $P 2>/dev/null || break; sleep 1; done
  sleep 1; kill $P 2>/dev/null; wait $P 2>/dev/null
  echo "$c: lines=$(grep -c '^L ' "$S/$c.log") done=$(grep -c ALLDONE "$S/$c.log")"
done
