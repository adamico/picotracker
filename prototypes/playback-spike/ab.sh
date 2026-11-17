#!/usr/bin/env bash
# PROTOTYPE — renders every coverage item with the spike, then compares it with
# PICO-8's recording (ref_<name>.wav) if one is present.
#   ./ab.sh [ref_dir]      ref_dir defaults to ~/.lexaloffle/pico-8/carts
set -euo pipefail
cd "$(dirname "$0")"
REF=${1:-$HOME/.lexaloffle/pico-8/carts}
OUT=out
mkdir -p "$OUT"
cargo build -q
python3 carts/make_coverage.py >/dev/null
while read -r name kind n desc; do
  secs=4; [ "$kind" = music ] && secs=20
  ./target/debug/playback-spike carts/coverage.p8 "$kind" "$n" --wav "$OUT/spike_$name.wav" --secs "$secs" >"$OUT/spike_$name.log"
done < carts/items.txt
python3 compare.py carts/items.txt "$OUT" "$REF"
