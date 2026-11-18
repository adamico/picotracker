# PICO-8 behaviour checks (throwaway, never merged)

The test rig behind [Check open PICO-8 behaviours against real PICO-8](https://github.com/adamico/picotracker/issues/24). The results are recorded on that ticket and on [Measure the last unchecked behaviours on real PICO-8](https://github.com/adamico/picotracker/issues/29), which added `expi` to `expm`. It needs a licensed copy of PICO-8 0.2.7, and Python 3 with no extra packages.

```bash
./run.sh
```

This generates the driver Carts and records every case to `p8home/carts/*.wav`, with no window and no sound. It uses its own PICO-8 home folder, so your config and carts are left alone.

| Cart | Covers | Analysis |
|---|---|---|
| `expa` | Waveform Instrument byte layout | `ana_a.py`, `ana2.py resample` |
| `expb` | Repeated notes and clicks | `ana2.py click`, `ana3.py` |
| `expc` | Pattern length, silent looping Step, LEN | `ana_rest.py c` |
| `expd` | SPD 0 | `ana_rest.py s` |
| `expe` | SFX Instruments and reverb against the zepto8 port | `ana_e.py`, `ana2.py slide` |
| `expf` | Loops past note 31, byte 66 bit 7, reverb in Patterns | `ana2.py f`, `ana3.py` |
| `expg` | Patterns holding a byte 66 bit 7 SFX | `ana3.py`, `ana4.py` |
| `exph` | LEN with audible notes after it | `ana4.py` |
| `expi` | NOIZ and BUZZ on the Phrase playing a Waveform Instrument note | `ana6.py` |
| `expj` | Bit 7 SFX in a Pattern longer than its 32 notes | `ana5.py` |
| `expk` | Bit 7 SFX beside channels that all loop | `ana5.py` |
| `expl` | The same, with looping channels at different Speeds | `ana6.py` |
| `expm` | Repeat takes, for the noise floor of `expi` | `ana7.py` |

`ana_e.py` and `ana3.py` also need the playback spike built at `spike/prototypes/playback-spike/`, from the `prototype/playback-spike` branch:

```bash
mkdir -p spike && git archive origin/prototype/playback-spike prototypes/playback-spike | tar -x -C spike
```

```bash
(cd spike/prototypes/playback-spike && cargo build --release)
```

The `d_right_len8` cases in `expc` have no notes after LEN, so they can't show whether LEN stops a channel. Use `exph` for that.

The `expi` section of `ana5.py` compares raw samples, which fails because oscillator phase differs between takes. Use `ana6.py` and `ana7.py` for `expi`.
