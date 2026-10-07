# `tools/chunkdump.py`: bChunk tree dumper

Prints the chunk tree of any MW `.BUN` / `.BIN` / `.LZC` file. It needs only Python 3.10+ with the
standard library; nothing to install.

- Removes whole-file `JDLZ` / `RAWW` wrappers automatically.
- Names chunk IDs using [`tools/bchunk_names.py`](../../tools/bchunk_names.py) (216 known IDs from the
  decomp).
- Recognizes bare JDLZ blobs (compressed add-on car geometry) and can decompress them (`--inflate`).
- Recognizes `VPAK` attribute databases and says so instead of misparsing them.
- Uses `mmap`, so the 533 MB `STREAML2RA.BUN` summary takes about 3 seconds.

Format reference: [formats/bchunk.md](../formats/bchunk.md).

## Usage

```bash
python tools/chunkdump.py FILE [FILE ...] [options]
```

| Option | Effect |
|---|---|
| `-d N`, `--depth N` | Print at most N levels (1 = top level only) |
| `-n N`, `--limit N` | Print at most N children per container, then summarize the rest by ID (default 40, 0 = all) |
| `-s`, `--strings` | Show the longest ASCII string near the start of each leaf (finds names fast) |
| `-x N`, `--hex N` | Show the first N payload bytes of each leaf |
| `--padding` | Also show `00000000` padding chunks (hidden by default) |
| `--inflate` | Decompress bare JDLZ blocks and show the chunks inside |
| `--summary` | Per-ID table: count, total payload bytes, nesting depths |
| `--find ID` | List every chunk with this ID, with its parent path |
| `--extract OFFSET -o FILE` | Write the chunk starting at OFFSET (header included) to FILE |
| `--save-unwrapped PATH` | Write the decompressed contents of a JDLZ/RAWW file |

Numbers accept `0x` prefixes. The exit code is non-zero if any file failed to parse completely.

## Reading the output

```
@00000D00  80134010 SolidPack  (16,444 B, 5 children)
  @00000D08  00134011 SolidInfo (SolidListObjHead)  (176 B)  "3CBMWM3GTR_BASE_A"
```

- `@00000D00`: file offset of the chunk **header**. The payload starts 8 bytes later. Inside an inflated
  JDLZ block, offsets are relative to the decompressed block.
- `80134010 SolidPack`: ID and name (`?` if unknown).
- `(16,444 B, 5 children)`: payload size (header not included) and number of children for containers.
- `"…"`: the string hint (`-s`). It's a heuristic and may include a stray byte or two before the real
  name.
- `!! …`: a parse problem (e.g. a container whose payload isn't chunks, or a truncated file).
- `... 92 more: 92x 80134010 SolidPack (1,566,056 B)`: children beyond `--limit`, grouped by ID.

## Recipes

```bash
G="D:/Need For Speed Most Wanted Black Edition"

# What's in a car?
python tools/chunkdump.py "$G/CARS/BMWM3GTR/GEOMETRY.BIN" -s -d 2

# Which chunk types make up the open world, and how big are they?
python tools/chunkdump.py "$G/TRACKS/STREAML2RA.BUN" --summary

# Where are all the animation banks in a cutscene?
python tools/chunkdump.py "$G/NIS/Scene_ArrestF02_BundleB.bun" --find 0x00E34010

# Pull one chunk out for a hex editor or another tool
python tools/chunkdump.py "$G/NIS/Scene_ArrestF02_BundleB.bun" --extract 0x0014C320 -o anim.chunk

# Look inside compressed add-on car geometry
python tools/chunkdump.py "$G/CARS/LEVIN/GEOMETRY.BIN" --inflate -s -d 4 -n 3

# Decompress an original .lzc
python tools/chunkdump.py "$G/GLOBAL/GLOBALB.LZC.bacc" -d 1 --save-unwrapped globalb.bin
```

## Tests

```bash
python -m unittest discover tests
```

The tests in `tests/test_chunkdump.py` use small synthetic data, so no game files are needed. They
cover nesting, padding, malformed containers, overruns, the three JDLZ code paths, RAWW, bare JDLZ blobs
and the CLI modes. The decompressor was also checked against real data: `gameplay.lzc.bak`
decompresses byte-for-byte to `gameplay.bak`.

## Validation against the install

Run over every data file in `D:\Need For Speed Most Wanted Black Edition`: 688 bChunk files and 514,808
chunks parsed with **0 failures**. The only files it rejects are genuinely not chunk files (sound
reverb data, `GlobalMemoryFile.bin`) and one corrupt add-on file (`CARS/FXXEVO/VINYLS.BIN`). See
[install-layout.md](../install-layout.md).

## Limitations

- **Structure only.** It shows structure, not meaning; struct layouts are in the format docs.
- **No `HUFF`/`COMP`.** These whole-file compression types aren't implemented; neither appears in this
  install.
- **Chunk-level JDLZ.** JDLZ payloads inside a chunk (e.g. `CompTPKBlock`) are flagged
  `[JDLZ-compressed payload]` but not decompressed.
- **`--extract` scope.** It only addresses chunks in the file itself, not inside inflated blocks.
- **`--save-unwrapped` with several files.** Every input writes to the same output path; pass one file
  at a time.
