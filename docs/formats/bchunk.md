# bChunk format — Need for Speed: Most Wanted (2005)

The container format used by almost every MW data file (`.BUN`, `.BIN`, `.LZC`, and
others). A file is a flat sequence of **chunks**; chunks may nest. This spec describes
what [`tools/chunkdump.py`](../../tools/chunkdump.py) implements and is the reference for
anyone parsing or rebuilding these files.

- **Platform:** PC release — **little-endian** throughout. (The GameCube and Xbox 360 releases are big-endian PowerPC builds; the PS2 release is little-endian MIPS. Only PC is covered here.)
- **Tested with:** Python 3.10+, standard library only.
- **Chunk-ID names:** [`tools/bchunk_names.py`](../../tools/bchunk_names.py), sourced from the `dbalatoni13/nfsmw` decompilation (see [References](#references)).

---

## 1. Chunk structure

Every chunk is an 8-byte header followed by its payload:

```
offset  size  field    notes
0x00    u32   id       little-endian. Bit 31 (0x80000000) set => payload is child chunks.
0x04    u32   size     payload size in BYTES, NOT including this 8-byte header.
0x08    u8[]  payload  `size` bytes.
```

The next chunk begins immediately at `offset + 8 + size`. A file (or a container's
payload) is parsed by repeating this until the end of the range is reached.

- **`end = offset + 8 + size`** — a well-formed chunk never extends past its parent's end.
- A trailing fragment of **fewer than 8 bytes** is a parse error (not a chunk).

### Container vs. leaf

Bit 31 of `id` is the **container bit** (`CONTAINER_BIT = 0x80000000`):

- **Set** → the payload is itself a sequence of child chunks; recurse into `[offset+8, end)`.
- **Clear** → the payload is leaf data (structs, tables, strings, compressed blobs).

The remaining 31 bits are the logical type id. Names such as `0x80134001 MeshContainerInfo`
(container) vs `0x00135200 LightMaterials` (leaf) are catalogued in `bchunk_names.py`; an
unknown id is still a valid chunk, just unnamed. In the stock files every chunk with bit 31 set
really does contain child chunks.

### Padding chunks

- **`id == 0x00000000` ("Padding")** — filler chunks inserted to align the chunk that
  follows. They carry no meaning; `chunkdump.py` hides them unless `--padding` is given.
- **`0x11` payload padding** — when a payload must start on an aligned address, MW left-pads
  it with `0x11` bytes. The engine computes the start with `bChunk::GetAlignedData(alignment)`,
  which rounds `offset + 8` **up to the alignment** (0x10 for most structs, 0x80 for vertex
  buffers). Do the same: compute the aligned offset, don't `lstrip(b"")`. Stripping
  also eats real data that happens to start with `0x11`; this happened on one
  `SceneryInstances` chunk in `STREAML2RA.BUN`. Sections in the stream file are 0x800-aligned,
  so file offsets and load addresses agree on alignment.

---

## 2. Whole-file compression wrappers

Before chunk parsing, a file may be wrapped in a whole-file header identified by a 4-byte
magic. Strip the wrapper, then parse the result as chunks.

| Magic | Meaning | Handling |
|---|---|---|
| `JDLZ` | JDLZ-compressed (see §3) | Decompress to get the chunk stream. |
| `RAWW` | Stored / uncompressed wrapper | Payload is `data[16 : 16+size]`, where `size = u32 @ 0x08`. |
| `HUFF` | Huffman-compressed | **Not implemented.** |
| `COMP` | (generic compressed) | **Not implemented.** |
| *(none)* | Bare chunk stream | Parse directly. |

### Not a bChunk file: `VPAK`

If the (unwrapped) data begins with **`VPAK`**, it is an **AttribSys VPAK** database
(attributes / gameplay data: cars, physics, AI, pursuit tuning), *not* a bChunk file. Edit
it with VltEd / Attribulator. See [attributes.md](attributes.md).

---

## 3. JDLZ compression

### 3.1 Header (16 bytes)

```
offset  size  field              value / notes
0x00    u8[4] magic              "JDLZ"
0x04    u8    version            0x02
0x05    u8    flags              0x10
0x06    u16   reserved           0
0x08    u32   decompressed_size  size of the output
0x0C    u32   compressed_size    total blob size, INCLUDING this 16-byte header
0x10    u8[]  lz_stream          compressed data
```

### 3.2 Stream algorithm

JDLZ is an LZ77 variant driven by **two independent 1-bit flag streams**, `flags1` and
`flags2`, each refilled one byte at a time. The decoder keeps a running output buffer and
emits literals or back-references into it.

Refill convention: a flag register is reloaded when it equals `1` by reading the next input
byte and setting `reg = byte | 0x100` (the high sentinel bit marks "8 bits remaining"); the
register is consumed with `reg & 1` then `reg >>= 1`.

Per step (while input and output remain):

1. If `flags1 == 1`, reload `flags1` from the next byte.
2. If `flags2 == 1`, reload `flags2` from the next byte.
3. If `flags1 & 1` → **back-reference** (read two bytes `b0, b1`):
   - If `flags2 & 1` (short-distance form):
     - `length   = (((b0 & 0xF0) << 4) | b1) + 3`
     - `distance = (b0 & 0x0F) + 1`
   - else (long-distance form):
     - `length   = (b0 & 0x1F) + 3`
     - `distance = (((b0 & 0xE0) << 3) | b1) + 17`
   - Copy `length` bytes from `output[pos - distance]`. When `distance < length` the copy
     **overlaps** and repeats the last `distance` bytes (byte-by-byte copy, not `memmove`).
   - Consume one bit of `flags2` (`flags2 >>= 1`).
   - A back-reference before the start of the output is a corrupt stream.
4. else → **literal**: copy one input byte to the output.
5. Consume one bit of `flags1` (`flags1 >>= 1`).

Decoding ends when `output` reaches `decompressed_size`. Stopping short of that is an error.

Reference implementation: `jdlz_decompress()` in `tools/chunkdump.py`.

---

## 4. Bare JDLZ blobs between chunks

In some files — notably **add-on car `GEOMETRY.BIN`** — a bare JDLZ blob is placed *where a
chunk would be*, and its **16-byte JDLZ header stands in for the chunk header**. Detect this
during chunk parsing:

- At a chunk boundary, if `id == 0x5A4C444A` (`"JDLZ"` read as a little-endian u32) and at
  least 16 bytes remain, treat the region as a JDLZ blob rather than an ordinary chunk.
- The blob's extent is `compressed_size` (the u32 at `blob+0x0C`, header included); it must
  not overrun the parent's end.
- Decompressing it (chunkdump's `--inflate`) yields a buffer that **itself parses as
  chunks**. Offsets inside an inflated blob are **relative to the blob**, not the file.

### Chunk-level JDLZ payloads

Separately from bare blobs, some ordinary chunks carry a JDLZ blob as their **payload**. For
example, `0x0003A100 CompTPKBlock` is used by the minimap tiles in
`TRACKS/L2RA/MINI_MAP_*.BIN`. `chunkdump.py` flags these as `[JDLZ-compressed payload]` but does
not decompress them.

---

## 5. Parsing algorithm (recap)

```
parse_range(buf, start, end):
    pos = start
    while pos < end:
        if end - pos < 8:            -> trailing-bytes error
        id, size = u32, u32 @ pos
        if id == JDLZ and end-pos >= 16:
            blob = parse_jdlz_blob(...)   # extent = compressed_size
            pos = blob.end; continue
        chunk.end = pos + 8 + size
        if chunk.end > end:          -> overrun error
        if id & 0x80000000:          # container
            parse_range(buf, pos+8, chunk.end)   # recurse
        pos = chunk.end
```

A container that fails to parse as child chunks is recorded as a note but does not abort the
whole file — it is simply treated as opaque.

### How the game loads chunks

The engine's `bChunkLoader` (`src/Speed/Indep/bWare/Inc/bChunk.hpp` in the decomp) registers a
loader/unloader function per chunk ID, bucketed by `(id + (id >> 6) + (id >> 12)) & 0x3F`.
`bChunkLoader::CallLoaders()` walks a chunk stream and dispatches each chunk to its loader.
Loading is **in place**: structs are used where they were read, and on-disk pointer fields are
zero until load time (for example `pMemory` in `TrackStreamingSection`, see
[maps.md](maps.md)).

### Validation

Run over the whole PC install, this algorithm parses 688 bChunk files and 514,808 chunks
(including the chunks inside inflated bare blobs) with 0 failures. `jdlz_decompress()`
reproduces `GLOBAL/gameplay.bak` byte-for-byte from the JDLZ original `gameplay.lzc.bak`. See
[../install-layout.md](../install-layout.md).

---

## 6. Common chunk IDs (selected)

Full map in `tools/bchunk_names.py`. A few examples (container ids have bit 31 set):

| id | name |
|---|---|
| `0x00000000` | Padding |
| `0x00034026` | Smokeables |
| `0x00034101` | ScenerySectionHeader |
| `0x00034102` | SceneryInfos |
| `0x00034103` | SceneryInstances |
| `0x00034105` | SceneryTreeNodes |

Where the community lists two names for one id (`"A / B"`), `bchunk_names.py` keeps both.

---

## 7. Tooling

```bash
python tools/chunkdump.py FILE [FILE ...]      # dump the chunk tree
  -d / --depth N        max tree depth (1 = top level only)
  -n / --limit N        max children per container before summarising (0 = all)
  -s / --strings        show first ASCII string in each leaf
  -x / --hex N          show first N payload bytes of each leaf
  --padding             include 0x00000000 padding chunks
  --inflate             decompress bare JDLZ blocks and show their chunks
  --summary             per-id count/size table instead of a tree
  --find ID             list every chunk with this id and its parents (e.g. --find 0x80134000)
  --extract OFFSET      write the chunk at OFFSET (header included) to a file
  -o PATH               output path for --extract
  --save-unwrapped PATH write the decompressed data of a JDLZ/RAWW file
```

Numbers accept `0x` prefixes. See also [`../TOOLS_AND_SKILLS.md`](../TOOLS_AND_SKILLS.md).

---

## 8. Edge cases & gotchas

- **Little-endian only** here; a big-endian file (console) will mis-parse.
- **Align, don't strip:** skip `0x11` padding by rounding the offset up to the alignment (§1), never by stripping `0x11` bytes.
- **Inflated-blob offsets are blob-relative** — don't `--extract` by a global offset inside one.
- **`HUFF`/`COMP` wrappers are unimplemented** — decompress them with another tool first.
- A `VPAK` file is not a bChunk file (see §2).
- **Not bChunk at all:** `SOUND/**` (EA audio), `MOVIES/*.vp6` (VP6 video), and the tiny
  `GLOBAL/*MemoryFile.bin` files (a `MEMO` `0x53219999` chunk or no header).
- `size` excludes the 8-byte header; off-by-8 mistakes are the most common parsing bug.

---

## References

- bChunk IDs & names: `dbalatoni13/nfsmw` decompilation, `symbols/bchunks.txt`
  (<https://github.com/dbalatoni13/nfsmw>, CC0-1.0).
- Reference parser/decompressor: [`tools/chunkdump.py`](../../tools/chunkdump.py).
