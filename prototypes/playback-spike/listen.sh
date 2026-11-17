#!/usr/bin/env bash
# PROTOTYPE — A/B one item: PICO-8 first, then the spike.   ./listen.sh sfx25 [sfx16 ...]
cd "$(dirname "$0")/out"
for n in "$@"; do
  echo "== $n: PICO-8"; aplay -q "ref_$n.wav"; sleep 0.4
  echo "== $n: spike";  aplay -q "spike_$n.wav"; sleep 0.8
done
