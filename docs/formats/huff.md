# HUFF compression — Need for Speed: Most Wanted (2005)

`HUFF` is MW's wrapper for Huffman-compressed data, used by most compressed textures. It is a
16-byte header like the JDLZ and RAWW wrappers (see [bchunk.md](bchunk.md) §2–§3), followed by a
stream in the format of EA's in-house compression library: stream type **`0x30FB`, "Huffman with
runlength"**, by Frank Barchard at EA Canada. Related types from the same library are RefPack
(`0x10FB`) and BTree (`0x46FB`).

This spec describes what [`crates/nfsmw-compress/src/huff.rs`](../../crates/nfsmw-compress/src/huff.rs)
implements (`nfsmw_compress::huff_decompress`). Every HUFF blob in the PC v1.3 install decodes with
it (§10).

- **Platform:** PC. The wrapper header is little-endian. The stream is read as an MSB-first bit
  stream, so its multi-byte header fields are **big-endian**.
- **Provenance:** `huff.rs` was written from this document. No third-party code was copied; see
  [References](#references) for what was read and under which license.

---

## 1. Where HUFF is used

| Location | HUFF blobs | Notes |
|---|---|---|
| Car texture packs, `CARS/**` (TPK `0x33310003` entries) | **10,738** | vs 2,127 JDLZ in the same packs **[verified]** |
| `GLOBAL/` texture packs: `RIVALS.BIN` 41, `HUDS_Custom_*.bin` 20, `DYNTEX.BIN` 14 | 75 | vs 285 JDLZ (incl. `CardTextures.bin`, `HUDTEXTURESPHOTOFINISH.BIN`) **[verified]** |
| `GLOBAL/gameplay.bin` (an AttribSys `VPAK`, not bChunk) | 253 | small blobs: 165–2,427 bytes decompressed, 350 on average **[verified]** |
| `0x00030210 FEngCompressedPackage` chunks (`GLOBALB.BUN`, `InGameB.bun`, `INGAMEC.BUN`, `Ingameb.lzc`) | 1 per file | **[verified]** |

No file in the install is HUFF-wrapped as a whole. `TRACKS/`, `LANGUAGES/`, `NIS/` and the live
`FRONTEND/` files contain no HUFF blobs. Their texture packs with entries use JDLZ
(`TrackMaps.bin` 297, `LanguageTextures.bin` 4) **[verified]**.

**Which codec a texture gets:** the game's packer (`LZCompress`) compresses with both HUFF and
JDLZ and keeps JDLZ if it is smaller. Otherwise it keeps HUFF, or falls back to RAWW if HUFF would
be bigger than the raw data plus its header **[decomp]**.

### Texture pack entries (`0x33310003`)

Texture blobs are found through the 24-byte `eStreamingEntry` records described in
[textures.md](textures.md). Facts that matter for HUFF:

| Field | Fact | Evidence |
|---|---|---|
| `ChunkByteOffset` | Each TexturePack (`0xB3300000`) with entries starts at file offset 0, so this is both a file offset and an offset from the TexturePack. Which one the engine means is **[unconfirmed]** | **[verified]** |
| `ChunkByteSize` | = HUFF `compressed_size` **+ 16** (§2) | **[verified]**, 10,813 / 10,813 |
| `UncompressedSize` | = the wrapper's `decompressed_size`. The output ends with a 0x9C-byte trailer (see textures.md) whose u32 at +0x24 is `NameHash` | **[verified]**, all 10,813 HUFF and 3,026 JDLZ textures |
| `UserFlags` / `Flags` / `RefCount` / `ChunkData` | 0 / 1 / 0 / 0 in all 13,839 records in the install (backups, `TrackMaps.bin` and `LanguageTextures.bin` included). Every blob is `HUFF` or `JDLZ`; none is `RAWW` | **[verified]** |

---

## 2. Wrapper header (16 bytes)

```
offset  size  field              value / notes
0x00    u8[4] magic              "HUFF"
0x04    u8    version            0x01
0x05    u8    header_size        0x10 (nfsmw-compress's `Header` calls this byte `flags`)
0x06    u16   flags              0
0x08    u32   decompressed_size  size of the output
0x0C    u32   compressed_size    size of the EA stream, NOT including this 16-byte header
0x10    u8[]  stream             EA "Huffman with runlength" stream (§3–§8)
```

| Claim | Evidence |
|---|---|
| Bytes 4–7 are `01 10 00 00` | **[verified]**, all 10,813 blobs |
| `compressed_size` excludes the header. JDLZ's includes it (bchunk.md §3.1) | **[verified]**: TPK entry size = field + 16 for all 10,813. **[decomp]**: `HUFFCompress` stores the encoder's output length |
| Byte 5 is the header size and bytes 6–7 a u16 flags field | **[decomp]** `LZHeader` in `Misc/LZCompress.hpp` |
| The game checks only magic and version | **[decomp]** `HUFFDecompress` |

The decoder also requires the stream's own size field (§3) to equal `decompressed_size`. This is
true for every blob **[verified]**.

Example (BMW M3 GTR texture `E6A9684F`):

```
48 55 46 46  01 10 00 00  9C 40 00 00  BA 2E 00 00 | 30 FB 00 40 9C 12 96 5C 82 ...
"HUFF"       ver/hdrsize  0x409C out   0x2EBA in    | type  size     clue  table ...
```

The TPK entry gives a compressed size of 0x2ECA (11,978) = 0x2EBA + 16.

---

## 3. Stream header

The stream is one MSB-first bit stream: the first byte's top bit comes first. The header fields
happen to be byte-aligned.

| Field | Bits | Notes |
|---|---|---|
| `type` | 16 | `0x30FB` in every MW blob **[verified]** |
| `total_size` | 24 or 32 | **present only if `type & 0x0100`**; skipped by decoders |
| `size` | 24 or 32 | decompressed size of this stream; 32 bits if `type & 0x8000` |
| `clue` | 8 | the escape symbol (§6) |

The type id is a set of flags on top of the library's `0xFB` marker byte. The MW decomp's decoder
handles all of them **[decomp]**, the same way as EA's released library source:

| Bit | Meaning |
|---|---|
| `0x8000` | 32-bit size fields instead of 24-bit |
| `0x0100` | an extra leading size field, which decoders skip. EA's library source calls it the total unpacked size of a "composite" multi-part object; `size` is then this part's. It is **not** a compressed size |
| `0x0200` | delta filter (§8) |
| `0x0400` | delta-of-delta filter (§8) |

So the valid ids are `30FB`–`35FB` and `B0FB`–`B5FB`. Only `30FB` occurs in MW **[verified]**.
`huff.rs` also implements the other eleven, but they are tested only with synthetic streams.

---

## 4. Bit reader

- Bits are consumed MSB-first, byte by byte. EA's decoder fetches big-endian 16-bit words and
  shifts them in MSB-first, which gives the same order.
- EA's decoder reads ahead in 16-bit units, so it can touch a few bytes past the last bit it uses.
  `huff.rs` reads zeros past the end of the input, but fails ("stream ended early") if it ever
  *consumes* a bit beyond `compressed_size`.

---

## 5. Variable-length numbers

Code-length counts, symbol skips and run lengths all use one variable-length integer code,
similar to Elias gamma:

1. Count the `0` bits before the first `1`. Call the count `z`, and consume the `1` too.
2. Read `z + 2` more bits as an unsigned integer `v`.
3. The number is `v + 2^(z+2) − 4`.

| Bits on the wire | Value bits | Range |
|---|---|---|
| `1 vv` | 2 | 0 – 3 |
| `01 vvv` | 3 | 4 – 11 |
| `001 vvvv` | 4 | 12 – 27 |
| `0001 vvvvv` | 5 | 28 – 59 |
| *z* zeros, `1`, *z+2* bits | *z+2* | 2^(z+2)−4 – 2^(z+3)−5 |

So `100` = 0, `101` = 1, `01000` = 4, `0010000` = 12.

**Large numbers:** the largest prefix in the install is `z = 12` (a run of 30,000)
**[verified]**. EA's encoder caps runs at 30,000 bytes **[decomp]**. For `z ≥ 16` (values ≥
262,140), EA's reference decoder reads one more value bit than its encoder writes, so such
numbers have no single meaning. `huff.rs` rejects any number with more than 15 leading zeros as
corrupt.

---

## 6. Code table

Right after the header comes a **canonical Huffman code** over byte values.

### 6.1 The clue byte

`clue` (8 bits, §3) names the symbol that acts as an **escape**: in the data (§7), decoding the
clue's code starts a run, an escaped byte or the end of the stream. The encoder picks a byte
value that never occurs in the data when one exists **[decomp]**. When all 256 values occur, the clue's own
occurrences are written as escaped bytes. 4,341 of the 10,813 MW streams have 256 symbols
**[verified]**. The clue always has a code.

### 6.2 Code-length counts

For code lengths `L = 1, 2, 3, …` read one number (§5): `count[L]`, the number of codes that are
`L` bits long. Codes are assigned canonically:

```
first[1] = 0
first[L] = (first[L-1] + count[L-1]) << 1      # smallest L-bit code
```

**Stop after the first `L` where `first[L] + count[L] == 2^L`**, i.e. when the codes fill the
code space exactly. `count[L]` may be 0 for some lengths. The last `L` read is the longest code
length.

- The longest code in MW is 15 bits **[verified]** (distribution: 7–15). EA's encoder clips its
  tree to 15 bits **[decomp]**. `huff.rs` accepts up to 16.
- Corrupt-stream checks in `huff.rs`: `first[L] + count[L] > 2^L` (over-subscribed), more than 256
  symbols, or no stop by `L = 16`.

### 6.3 Symbol list

`total = Σ count[L]` symbols follow, in **code order**: all 1-bit codes first, then 2-bit codes,
and so on. Within a length, codes are numbered upward from `first[L]`. Each symbol is stored as
one number `k` (§5), a skip over byte values not used yet:

- Keep a cursor, initially `0xFF`, and a set of byte values already listed.
- Move the cursor upward one value at a time, wrapping from `0xFF` to `0x00`, and count only values
  not yet listed. The symbol is the `(k+1)`-th such value. Add it to the set; the cursor stays
  there for the next symbol.

The encoder lists each length's symbols in increasing byte order, so most skips are small. EA's
source calls this "leapfrog" coding. A skip larger than the number of unlisted values wraps
around, so `huff.rs` reduces it modulo that number first. The result is the same.

### 6.4 Worked example

The texture from §2 continues `12 96 5C 82 …`. `clue = 0x12`, then the counts:

```
0x96 0x5C 0x82 = 100 101 100 101 110 01000 ...
                 L1=0 L2=1 L3=0 L4=1 L5=2 L6=4  ...  (then 19, 55, 60, 63, 34, 15, 2 → L13 fills the space)
```

That is 256 symbols with lengths 2–13. The first symbols are `0x00` (code `00`), `0x55` (`0100`),
`0x01` and `0xFF` (`01010`, `01011`), and `0x08, 0x20, 0x21, 0xAA` (`011000`…`011011`).

---

## 7. Data

Decoding repeats these steps until the end marker:

1. Decode one symbol `s` with the code from §6.
2. If `s != clue`, output `s`.
3. If `s == clue`, read a number `n` (§5):
   - **`n > 0` — run:** output the **previous output byte** `n` more times. The encoder writes
     the first byte of a run as a normal symbol, then `clue` + `n` for the repeats. A run before
     any output is corrupt.
   - **`n == 0`:** read 1 bit.
     - `1` — **end of stream.**
     - `0` — **escaped byte:** read 8 bits and output them as a byte. MW uses this only to write
       the clue's own value (22,786 escapes, all of the clue byte) **[verified]**.

So the end marker is `clue code, 100, 1`. The encoder then writes one more `0`, plus 7 zero bits
to flush, and keeps only whole bytes. A stream therefore ends with **0 or 1 byte** after the byte
holding the final `1` bit: 9,430 vs 1,383 streams, padding bits always zero **[verified]**.

`huff.rs` also fails if the output would exceed `size`, or if the end marker arrives before
`size` bytes. EA's decoder checks neither.

---

## 8. Post-filters (types `x2FB` / `x4FB`)

After decoding, with `d` the decoded bytes and all sums mod 256:

| Type bits | Output byte `i` |
|---|---|
| neither (`30FB`) | `d[i]` |
| `0x0200` (`32FB`) | `d[0] + d[1] + … + d[i]` — undo one delta step |
| `0x0400` (`34FB`) | running sum of the running sum — undo two delta steps |

Not used in MW **[verified]**. From the library source and the decomp **[decomp]**.

---

## 9. Decoder outline

```
read type (16), [total (24|32) if type & 0x0100], size (24|32), clue (8)
L = 0, next = 0, total = 0
repeat:                                   # §6.2
    L += 1; next <<= 1
    first[L] = next; count[L] = num(); index[L] = total
    next += count[L]; total += count[L]
until next == 1 << L
cursor = 0xFF
for i in 0 .. total:                      # §6.3
    k = num() mod (256 - i)
    cursor = (k+1)-th unused value after cursor (wrapping); mark used
    symbols.append(cursor)
loop:                                     # §7
    s = decode one canonical code (first/count/index/symbols)
    if s != clue:      out.push(s)
    else:
        n = num()
        if n > 0:      out.extend(n × out.last)
        elif bit()==1: break
        else:          out.push(bits(8))
apply the §8 filter
```

To decode a symbol canonically, read bits one at a time into `c` (MSB first). After `L` bits, if
`c − first[L] < count[L]` the symbol is `symbols[index[L] + c − first[L]]`. `huff.rs` speeds this
up with a 10-bit lookup table for short codes, and checks lengths 11–16 against a 16-bit peek for
the rest.

---

## 10. Validation

Results of a scratch harness (not in the repo). It walked `CARS/**/*.BIN`, `GLOBAL/*`,
`TRACKS/STREAML2RA.BUN`, `FRONTEND/**` and `NIS/**`, found every `0x33310003` record (including
inside JDLZ-wrapped files and payloads), and decompressed each blob with
`nfsmw_compress::{huff_decompress, jdlz_decompress}`. Success means `out.len() ==
uncompressed_size` **and** the trailer hash equals `name_hash`. **[verified]**

| Set | HUFF ok / fail | JDLZ ok / fail |
|---|---|---|
| `CARS/` | **10,738 / 0** | 2,127 / 0 |
| `GLOBAL/` | 75 / 0 | 285 / 0 |
| Whole install, all folders except `SOUND`/`MOVIES`, backups included | 10,813 / 0 | 3,026 / 0 |

- **Raw scan:** every `HUFF 01 10 00 00` occurrence in every file (and inside JDLZ/RAWW-unwrapped
  files) was tried: **12,085 occurrences decode**. The 2 failures are false matches inside JDLZ-compressed
  bytes (`gameplay.lzc.bak`, `FRONTB.LZC.bacc`), not HUFF blobs.
- **Stream statistics** (10,813 TPK blobs): type `30FB` only. Longest code 7–15 bits. Clue code
  1–15 bits. Largest number prefix `z = 12`. 15.7 M runs, the longest 30,000. 22,786 escapes.
  Composite, 32-bit and delta types: none.
- **Speed:** 81 MB of HUFF input → 2,822 MB of output in 0.6 s, release build. Most of the output
  comes from runs.
- **In-repo check:** `NFSMW_GAME_DIR=<install> cargo test -p nfsmw-compress -- --ignored` runs
  `real_install_bmw_m3_gtr_textures`, the same check on `CARS/BMWM3GTR/TEXTURES.BIN` (3 HUFF
  textures). Synthetic unit tests in `huff.rs` cover every type variant, run and number widths,
  escapes, codes longer than the lookup table, and the corrupt-stream errors.

---

## 11. Edge cases & gotchas

- **The size field differs from JDLZ:** HUFF's u32 at 0x0C excludes the 16-byte header; JDLZ's
  includes it. Treating it as inclusive cuts off the last 16 bytes of the stream, and the decoder
  stops just short of the end (seen: 56 bytes short of 0x409C).
- **`0x0100` is not a compressed-size flag:** it adds a composite total-size field, which is
  skipped (§3).
- **The clue can be a real byte value:** when all 256 values occur, literal clue bytes appear as
  escapes (`clue, 100, 0, byte`).
- **Runs repeat the previous output byte,** not the clue, and never start the stream.
- **Numbers with ≥ 16 leading zeros** are ambiguous in EA's own codec and rejected (§5). They
  never occur in MW.
- **The end marker is in-band:** the decoder stops at it, not at `size`. `huff.rs` treats any
  mismatch between the two as corruption.

---

## References

No permissively licensed (MIT/BSD/Apache/zlib/CC0) implementation of the `0x30FB` decoder was
found. So `huff.rs` is written from this document alone and carries no third-party notice. Searches covered GitHub code for `HUFF_decode`, `huffdecode`, `0x30FB`, `ea_huff` and
`HUFFDecompress`, and the web for EA Huffman / NFS HUFF tools. Sources consulted:

| Source | License | How it was used |
|---|---|---|
| EA, *Command & Conquer Generals / Zero Hour* source, `Generals/Code/Libraries/Source/Compression/EAC/` (`huffdecode.cpp`, `huffencode.cpp`, `huffabout.cpp`, `huffcodex.h`, `codex.h`, `refabout.cpp`): <https://github.com/electronicarts/CnC_Generals_Zero_Hour> | GPL-3.0 with EA's additional terms | **Read for understanding only; no code copied or paraphrased.** Source of the bit-stream layout (§3–§7), the type-id bits, the encoder's 15-bit and 30,000-byte limits and its number widths. |
| dbalatoni13/nfsmw decompilation: `src/Speed/Indep/Src/Misc/LZCompress.cpp/.hpp` and `config/SPEED_EXE_1_3/symbols.json` (`HUFF_decode` @ `0x64DCA0`, `HUFFDecompress` @ `0x6500F0`): <https://github.com/dbalatoni13/nfsmw> | CC0-1.0 (decompiled code) | **Read for understanding only; no code copied.** Basis of the **[decomp]** claims: `LZHeader` fields, HUFF size semantics, HUFF/JDLZ/RAWW choice, and that MW ships the same decoder. |
| NFSTools/GlobalLib, `GlobalLib/Utils/HUFF.cs`: <https://github.com/NFSTools/GlobalLib> | MIT | Read. It is an unfinished decompiler-style port that stops after the code-length table, so nothing was taken from it. |
| NFSTools/Nikki, `Nikki/Utils/Interop.cs`: <https://github.com/NFSTools/Nikki> | MIT | Read. HUFF goes through a native `LZCompressLib.dll` with no source. Not used. |
| QuickBMS, `src/included/ea_huff.c` | GPL | Seen in search results only: an embedded machine-code dump of an EA decoder. Not read or used. |
| Install files, PC v1.3 "Black Edition" | — | Every **[verified]** number above; no game data is in the repo. |
