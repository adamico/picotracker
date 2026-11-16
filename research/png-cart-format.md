# PNG cart format: reading and writing

Research for issue #3 (child of map #1). Question: how is a `.p8.png` Cart
encoded, and what does it take to read and write one? Can Export rewrite only
the SFX (0x3200) and Pattern (0x3100) bytes and leave the compressed code
exactly as it was?

## Answer in brief

- A `.p8.png` is a **160x205 RGBA PNG**. Each pixel carries **one cart byte** in
  the two low bits of each channel (A = bits 7-6, R = 5-4, G = 3-2, B = 1-0),
  in row-major order: 160 x 205 = 32800 = 0x8020 bytes.
- Bytes 0x0000-0x42ff are the ROM data (gfx, map, flags, song, sfx), copied
  verbatim; 0x4300-0x7fff hold the Lua code (raw, old `:c:`, or new `\0pxa`);
  0x8000-0x801f is a trailer with version bytes and a **SHA-1 of bytes
  0x0000-0x7fff**.
- The label is a 128x128 image drawn at pixel (16, 24) of the cartridge
  picture; its high bits are the visible picture, its low bits are cart data.
- **Yes, Export can leave the code untouched.** Copy 0x4300-0x7fff byte for
  byte, rewrite 0x3100-0x42ff, and **recompute the SHA-1 at 0x8006-0x8019**
  (or the cart is rejected as corrupted). The cleanest way is to patch the
  low bits of the original PNG's pixels in place, which also keeps the label
  and cartridge art exactly. shrinko8 already does this byte copy behind its
  `keep_compression` option. The tracker never has to decompress or compress
  Lua for a round trip.

## 1. How the data is hidden in the pixels

The wiki says "Each PICO-8 byte is stored as the two least significant bits of
each of the four color channels, ordered ARGB", and that the image is "160
pixels wide and 205 pixels high, for a possible storage of 32,800 (0x8020)
bytes" [wiki].

Decode, per pixel `(r, g, b, a)` in row-major order (byte index `x + y*160`):

```
byte = ((a & 3) << 6) | ((r & 3) << 4) | ((g & 3) << 2) | (b & 3)
```

This is the same in all three implementations: shrinko8 `read_cart_from_image`
[shrinko8 pico_cart.py L220-235], zepto8 `cart::load_png` [zepto8 cart.cpp
L64-103] and fake-08 `Cart::loadCartFromPng` [fake-08 cart.cpp L153-176].

Encode is the reverse: keep the top 6 bits of each channel and replace the low
2 bits (`b = (b & ~3) | (byte & 3)`, `g ... (byte >> 2) & 3`, `r ... (byte >> 4) & 3`,
`a ... (byte >> 6) & 3`) [shrinko8 pico_cart.py L288-297; zepto8 cart.cpp
`save_png` L599-640].

All three readers reject any other size: shrinko8 throws "Png has wrong size"
unless it is exactly 160x205 [pico_cart.py L194-195]; fake-08 checks
`width != 160 || height != 205` [cart.cpp L295, L320]; zepto8 checks
`width * height != 160 * 205` [cart.cpp L76].

Implementation notes for Rust:

- Decode to 8-bit RGBA with **no** gamma, colour-management or
  premultiplied-alpha conversion, since the alpha low bits are data. (Inference:
  any transform that alters low bits would corrupt the cart.)
- The PNG compression (zlib) can change on re-save; only the decoded pixels
  matter.

### Byte layout of the 0x8020 bytes

| Range | Contents | Source |
|---|---|---|
| 0x0000-0x0fff | GFX | manual "Base RAM Memory Layout" |
| 0x1000-0x1fff | GFX2 / MAP2 (shared) | manual |
| 0x2000-0x2fff | MAP | manual |
| 0x3000-0x30ff | GFX flags | manual |
| 0x3100-0x31ff | SONG (our Pattern bytes; 64 x 4) | manual; fake-08 cart.cpp L190-194 |
| 0x3200-0x42ff | SFX (64 x 68) | manual; fake-08 cart.cpp L196-201 |
| 0x4300-0x7fff | Lua code, 0x3d00 bytes | wiki; shrinko8 pico_defs.py `k_rom_size = 0x4300`, `k_code_size` |
| 0x8000 | version ID | wiki "Version ID" |
| 0x8001-0x8003 | real PICO-8 version, e.g. 0,2,4 | wiki |
| 0x8004 | platform ID ('w', 'l', 'x', 'E') | wiki |
| 0x8005 | version letter suffix | wiki |
| 0x8006-0x8019 | SHA-1 of bytes 0x0000-0x7fff, or all zero | wiki; shrinko8 pico_cart.py L121-124, L147 |
| 0x801a-0x801f | zero | wiki |

The manual confirms the cart ROM is "same layout as base ram until 0x4300"
[manual, "Memory" section]. The wiki says a non-zero hash "is checked and the
cartridge is treated as corrupted if wrong" [wiki]; shrinko8 enforces the same:
`if hash != bytes(20) and hash != hashlib.sha1(buffer[:k_cart_size]).digest():
throw("corrupted cart (wrong hash)")` [pico_cart.py L123-124]. All big-endian
multi-byte fields here and in the code headers are MSB first.

## 2. Code formats at 0x4300

Detection by the first four bytes, identical in the official Lexaloffle C
(`is_compressed_format_header`), zepto8 `code::decompress`, shrinko8
`uncompress_code` and fake-08:

| Bytes 0x4300-0x4303 | Format |
|---|---|
| `\0pxa` | new PXA compression (PICO-8 0.2.0+) |
| `:c:\0` | old compression (all versions) |
| anything else | plain text up to the first NUL (max 0x3d00 bytes) |

[lexaloffle pxa_compress_snippets.c L808-822; zepto8 code.cpp L153-164;
shrinko8 pico_compress.py L40-41, L48-148; lexaloffle README].

### Old `:c:` format

Header: `:c:\0`, then decompressed length (u16, MSB first), then two zero
bytes; data starts at 0x4308 [wiki "Old Compressed Format"]. Byte stream:

- `0x00`: next byte is a literal.
- `0x01-0x3b`: a character from the table
  `\n 0123456789abcdefghijklmnopqrstuvwxyz!#%(){}[]<>+=/*:;.,~_`.
- `0x3c-0xff`: back-reference with the next byte:
  `offset = (b0 - 0x3c) * 16 + (b1 & 0xf)`, `length = (b1 >> 4) + 2`; length
  can not exceed offset.

[wiki; shrinko8 pico_compress.py L30-36, L109-142; fake-08 cart.cpp L233-270].
Official C: `p8_compress.c` in Lexaloffle's repo [lexaloffle README].

### New PXA format

Header: `\0pxa`, decompressed length (u16 MSB), then **compressed length
including the 8-byte header** (u16 MSB); bit stream starts at 0x4308
[wiki "New Compressed Format"; lexaloffle pxa_compress_snippets.c L473-477,
L684-685, which write `bytes_written` (header included) to bytes 6-7].

Bits are read **LSB to MSB within each byte** [wiki; zepto8 code.cpp L210-215].
State is a 256-entry move-to-front table, initially identity. Per group:

- Header bit `1`: literal. Read a unary count `u` (count of 1 bits before a 0),
  then `index = read_bits(4 + u) + (((1 << u) - 1) << 4)`; output `mtf[index]`
  and move it to the front.
- Header bit `0`: back-reference. `offset_bits = bit ? (bit ? 5 : 10) : 15`,
  `offset = read_bits(offset_bits) + 1`; length = 3 plus 3-bit parts, adding
  until a part is not 7. Copy may overlap.
- Special case: `offset_bits == 10 && offset == 1` starts a raw block of 8-bit
  characters (not byte-aligned) up to a NUL.

[wiki; shrinko8 pico_compress.py L54-107; zepto8 code.cpp `pxa_decompress`].
The official compressor falls back to storing the code raw when PXA output
would be larger than the input [lexaloffle pxa_compress_snippets.c L688-697].

### Size limit discrepancy

The code region is 0x8000 - 0x4300 = 0x3d00 = 15616 bytes [shrinko8
pico_defs.py], but the manual says "the compressed size of the code must be
less than 15360 bytes so that the total data is <= 32k" [manual, "Code size
restrictions"]. Not resolved here; irrelevant if code bytes are copied as-is.

## 3. Image size and the label

- Image: 160x205 [wiki; all three readers].
- Label: 128x128 at offset (16, 24); title text (first two `--` comment lines
  of the code) is drawn at (18, 167) in a 124x16 box with PICO-8's font
  [shrinko8 pico_cart.py L168-173, L269-284]. The manual: CTRL-7 "grab[s]
  whatever is on the screen", and "The first two lines of the program starting
  with '--' are also drawn to the cart's label" [manual, "Saving .p8.png carts
  with a text label and preview image"].
- The label is **not stored as cart bytes**; it is just the visible high bits
  of those pixels. Readers recover it by nearest-colour matching to PICO-8's
  32-colour palette, ignoring the low 2 bits: shrinko8 matches on
  `(r & ~3, g & ~3, b & ~3)` [pico_cart.py L175-218]; zepto8 uses
  `palette::best(p, 32)` [cart.cpp L89-99].
- Writers start from a blank cartridge template PNG (shrinko8
  `template.png`, zepto8 `data/blank.png`), paint label and title, then write
  bytes into the low bits [shrinko8 pico_cart.py L269-302; zepto8 cart.cpp
  L599-640]. That template art belongs to those projects / Lexaloffle; we
  would rather reuse the user's existing PNG.

## 4. Can Export rewrite only SFX and Pattern bytes?

**Yes.** Nothing in the format ties the code bytes to the data bytes except
the trailer hash:

1. Read the PNG, extract all 0x8020 bytes.
2. Overwrite 0x3100-0x31ff (Pattern / song) and 0x3200-0x42ff (SFX) with the
   tracker's data.
3. Leave 0x0000-0x30ff and **0x4300-0x7fff byte for byte** as read. Whatever
   the code format (raw, `:c:`, PXA), it stays valid because it is
   self-contained in its region.
4. Recompute SHA-1 over 0x0000-0x7fff and write it to 0x8006-0x8019. Keep
   0x8000-0x8005 as they were. (Writing all zeros is also accepted by the
   readers that check [wiki; shrinko8 L123], but recomputing is safer.)
5. Write the bytes back into the low 2 bits of the **original** pixels and
   re-encode the PNG. Label, title and cartridge art are unchanged because only
   low bits change. Inference: only pixels for indices 0x3100-0x42ff and
   0x8006-0x8019 actually change.

Precedent: shrinko8 reads the raw code region into `cart.code_rom` with
`keep_compression` and writes it back unchanged instead of recompressing
[shrinko8 pico_cart.py L93-101, L128-137]. Neither zepto8 nor fake-08 does
this: zepto8 always recompresses via `code::compress` on save [zepto8 cart.cpp
L670-690].

What a fuller writer would need (only if we ever create a cart from a `.p8`
text file, not for round-tripping a `.p8.png`):

- A code encoder. Raw text is valid when it is under 0x3d00 bytes; otherwise a
  PXA compressor (Lexaloffle's official C is open source, shrinko8 and zepto8
  have their own).
- A cartridge template image and the PICO-8 font for title text.

## Surprises worth noting for other tickets

- The **SHA-1 trailer** is the one thing that breaks a naive "patch the sfx
  bytes" Export: forget it and PICO-8 reports the cart as corrupted.
- Reading needs Lua decompression **only** if the tracker wants to show or
  edit code; for SFX/Pattern import it can ignore 0x4300+ entirely.
- Alpha carries data: a PNG library that premultiplies or drops alpha will
  silently corrupt carts.
- The manual's 15360-byte code limit disagrees with the 0x3d00 region size.

## Sources

- PICO-8 manual (local): `docs/sources/pico-8_manual.txt`, sections "Saving
  .p8.png carts with a text label and preview image", "Code size restrictions
  for .p8.png / .p8.rom formats", "Base RAM Memory Layout", "Memory".
- [wiki] PICO-8 wiki, P8PNGFileFormat:
  https://pico-8.fandom.com/wiki/P8PNGFileFormat (read via the MediaWiki API,
  2026-11-16).
- Lexaloffle official compression C, published by Lexaloffle Games LLP:
  https://github.com/dansanderson/lexaloffle (`pxa_compress_snippets.c`,
  `p8_compress.c`, `README.md`).
- shrinko8 @ d26c7d0abd6561b998ed6f530fed0ed3bc2216c9:
  https://github.com/thisismypassport/shrinko8 (`pico_cart.py`,
  `pico_compress.py`, `pico_defs.py`).
- zepto8 @ b2929a4fe31e5765ef66d7794edd11086b9638b4:
  https://github.com/samhocevar/zepto8 (`src/pico8/cart.cpp`,
  `src/pico8/code.cpp`).
- fake-08 @ 814991a2571ad3970e386cef48f3b148aa1c27b9:
  https://github.com/jtothebell/fake-08 (`source/cart.cpp`).
