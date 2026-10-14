# The 2005 Xbox 360 shader container

The layout of the Xbox 360 shader containers of NFS: Most Wanted (2005 XDK), as the fork's reader and its
validator define them. These are facts about a file format, recorded with credit under
[licensing.md](../licensing.md). The code is not copied.

**Evidence:** **[community]**. Read from `nfsmw_contenedor.h`, `nfsmw_buscar_contenedores.cpp` and
`nfsmw_probar_contenedor.cpp` in [codepdbh/nfsmw-android](https://github.com/codepdbh/nfsmw-android) (GPL-3.0,
`shaders/`). Not checked against an Xbox dump here, and not used by the PC game, whose shaders are
[D3DX effects](../formats/shaders.md). Where the fork's own comment says something is an assumption, this page says so.

All multi-byte fields are **big-endian** (the console is PowerPC).

## Two signatures

The first word is the flags. Its high 24 bits are the signature:

| Signature | Compiler | Read by |
|---|---|---|
| `0x102A1100` | 2008 XDK (Sonic Unleashed) | XenosRecomp, as shipped upstream |
| `0x102A0E00` | 2005 XDK (NFS: MW) | The fork's adapter, which converts it to the 2008 layout |

The low bit of the flags is the shader type. A clear bit is a **pixel** shader and a set bit is a **vertex** shader
(`fork: pixel = !(firma & 1)`). The fork's file names follow this: `p_NNNNNN.bin` and `v_NNNNNN.bin`.

## Header (24 bytes)

| Offset | Field | Meaning |
|---|---|---|
| 0 | flags | Signature (above) in the high bytes, type in the low bit |
| 4 | virtual size `vs` | Offset of the microcode. The bytes before it are the header and tables |
| 8 | physical size `ps` | Length of the microcode. The file size is `vs + ps` |
| 12 | `def` | Offset of the immediate constant definitions, or 0 for none |
| 16 | `ct` | Offset of the constant table (CTAB) |
| 20 | `sh` | Offset of the shader header |

The fork's own comment says that for this game the tag is not used as an offset or a length; the microcode starts
at `vs`, as above.

## Constant table (CTAB)

Found at `ct`. Its own base is `ct + 4`, and every offset inside it is relative to that base.

| Relative to base | Field |
|---|---|
| +12 | Number of constants |
| +16 | Offset of the constant info table |

Each constant info entry is 20 bytes. Its first word is the offset of the constant's name (a NUL-terminated string
inside the virtual part). Its word at +12 points at a 16-byte block, which the reader only checks is in range. The
meaning of the rest of the entry is not stated in the fork.

## Shader header

### Pixel shader (`sh`)

| Offset | Field |
|---|---|
| +16 | Interpolator bits: the count is `(word >> 5) & 31` |
| +28 | Output mask. Bits 0–3 are the colour outputs. Bit 5 is the kill (discard) flag, as the fork's test shows. Any other bit makes the container unsupported |
| +32 | Interpolator list, 4 bytes each |

### Vertex shader (`sh`)

| Offset | Field |
|---|---|
| +16 | Interpolator bits, as above |
| +24 | Count of a preceding table of words, which the fork skips over |
| +28 | Count of elements |
| +40 | Start of the table: skip `count(+24) * 4` bytes, then read elements and interpolators, 4 bytes each |

## Microcode and control flow

The microcode is a stream of **12-byte blocks**. Each block holds two **48-bit control-flow (CF) words**, built as:

- first CF word: the first 32-bit word, and the low 16 bits of the second;
- second CF word: the high 16 bits of the second word, and the third word.

Fields of a CF word, from the fork's validator and probe:

| Bits | Field |
|---|---|
| 44–47 | Opcode |
| 0–11 | Address, in blocks, of the target |
| 12–14 | Count of blocks executed from that address (at most 6) |
| 16–27 | Two bits per slot in the group. The probe skips slots whose low bit is set. Its meaning is not given in the fork |

Opcodes 1–6, 13 and 14 are **execute-type** (they run blocks at an address). Opcodes 2, 4, 6 and 14 also **end**
the program.

### What the validator checks

The fork's validator, `ValidarMicrocodigo`, accepts the microcode only if all of these hold:

1. The length is non-zero, a multiple of 12, and inside the file.
2. Every execute-type word has a non-zero address, a count of at most 6, and a target inside the microcode.
3. Every target is **forward** of the block that jumps to it.
4. The program's end is the smallest execute target (the comment says "as in Xenia"). The microcode is cut there.
5. There is at least one execute-type word and at least one ending word. An empty translation is never accepted.

Rule 3 and rule 4 are the fork's rules, not a general description of the Xenos format.

## Immediate constants

If `def` is non-zero, the reader parses a list of constant blocks:

- The region's byte length is the word at `def + 16`. The list starts at `def + 24`.
- Each entry is a word `register << 16 | count`, followed by `count` 32-bit floats.
- `register` must be at least `0x300` and a multiple of 16. The fork maps it to a float4 index as
  `(register - 0x300) / 16`, and the driver copies it to `device + 0x480 + register`.
- `count` must be a multiple of 4.
- A zero word ends the list. It must be followed by another zero word, or the constant is a **masked definition**,
  which the fork does not implement and rejects.

## What the fork does with a container

1. Checks the signature and the header offsets.
2. Converts a 2005 container to the 2008 layout: the header grows by 12 bytes, and the tables are copied after it
   with their relative offsets unchanged. The microcode is copied as-is.
3. Writes the immediate constants as new tables.
4. Hands the result to XenosRecomp.

The fork's regression test confirms that converting a container leaves its microcode bytes unchanged, and that the
colour-output mask and the kill bit agree with what the microcode exports (see
[translator-pipeline.md](translator-pipeline.md#regression-checks-for-the-container-reader)).

## Where the containers are

The translator's comment says the containers were extracted from a disc file named `ZZDATA0.BIN`. The search tool
also scans `.xex` and `.exe` files for the signature. The same comment says that old console dumps "lack the
correct physical part", and they are kept for regression and rejected. No container is included in the repository.
