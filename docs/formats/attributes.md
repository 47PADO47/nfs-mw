# AttribSys attributes (`VPAK`) — Need for Speed: Most Wanted (2005)

> **Status: STUB.** This documents what we currently know; the binary layout has **not**
> been reverse-engineered in this repo yet. Sections marked _TODO_ are placeholders. Do not
> treat the byte offsets as authoritative until they're filled in and verified against real
> files. Contributions welcome.

The **AttribSys** ("attribute system") is MW's database of **gameplay/tuning data** — not
geometry or textures. It drives cars, physics/handling, AI, pursuit/heat behaviour, damage,
and similar tunables. Its files are a different container from [bChunk](bchunk.md): where a
bChunk file is a tree of typed binary chunks, an AttribSys pack is a **key→value attribute
database** keyed by hashed names.

- **Detection:** after any whole-file unwrap, the data begins with the magic **`VPAK`**.
  [`tools/chunkdump.py`](../../tools/chunkdump.py) recognises this and reports *"AttribSys
  VPAK database … not a bChunk file"* rather than trying to parse chunks.
- **Where it lives (verified on the PC install):** `GLOBAL/attributes.bin` (main database),
  `GLOBAL/gameplay.bin`, `GLOBAL/FE_ATTRIB.bin`, and `GLOBAL/gameplay.lzc`, which is the same
  database in a `RAWW` wrapper here (the original `gameplay.lzc.bak` is `JDLZ`). These four
  are the only `VPAK` files in the install. `GlobalB.lzc` is an ordinary bChunk file, and
  `GlobalMemoryFile.bin` is neither.
- **Platform:** PC release, **little-endian** (consistent with the rest of this repo). _TODO: confirm._

---

## 1. Concepts

AttribSys models data as a set of **classes**, each with **collections** (instances), each
holding **attributes** (fields). Names are stored as **32-bit hashes**, not strings, so a
raw file shows hashed keys; tooling maps them back to readable names via a dictionary.

| Term | Meaning |
|---|---|
| **Class** | A schema/type, e.g. a car's physics class. |
| **Collection** | A named instance of a class (e.g. a specific car tuning). |
| **Attribute** | A single field (hashed key → typed value). |
| **Hash** | 32-bit name hash used as the on-disk key. _TODO: document the exact hash function (bStringHash / Jenkins-style?)._ |

---

## 2. File container (`VPAK`) — TODO

_TODO: reverse-engineer and document the on-disk layout._ Expected pieces to pin down:

```
offset  size  field        notes
0x00    u8[4] magic        "VPAK"
0x04    ...   ...          TODO: version / header fields
0x08    u32   ?            chunkdump reads a u32 here for the RAWW case; confirm VPAK's own header
...
```

Open questions to resolve against real files:
- Header: version, table count, total size, endianness marker?
- Section/table directory: where are the class table, collection table, attribute table, and the string/hash dictionary?
- Value encoding: inline vs. referenced; how typed (int/float/bool/string/vector/blob)?
- Alignment/padding conventions (cf. bChunk's `0x11` padding).
- Relationship to any surrounding bChunk or `RAWW`/`JDLZ` wrapper (AttribSys data can be stored compressed).

---

## 3. Attribute types — TODO

_TODO: enumerate the value types and their binary encodings (e.g. int32, float32, bool,
string-hash, vector3/4, RGBA, blob), and how arrays/refs are represented._

---

## 4. Name/hash dictionary — TODO

Readable names are recovered by hashing a known wordlist and matching. _TODO:_
- Document the hash algorithm and seed.
- Point to / vendor a names list (community VltEd/Attribulator dictionaries) and note licensing.

---

## 5. Tooling

Existing community tools edit these files directly — prefer them over hand-editing until the
format is documented here:

- **VltEd** (nfs-tools / nlhans) — GUI editor for MW/Carbon/ProStreet VLT attribute data.
- **Attribulator / Attribute (binary)** — scriptable AttribSys editor.

In this repo:
- `chunkdump.py` only **detects** `VPAK` and bails out (it is a bChunk tool). _TODO: add a
  dedicated `vpakdump.py` once the layout above is confirmed._

---

## 6. Next steps to de-stub this doc

1. Collect a few real `VPAK` files from MW 2005 (`GLOBAL/…`), noting any `JDLZ`/`RAWW` wrapper.
2. Diff a VltEd round-trip (load → save with no change) to find stable vs. volatile regions.
3. Map the header + directory tables (§2), then value encodings (§3).
4. Identify the hash function (§4) and wire up a names dictionary.
5. Add `vpakdump.py` + tests mirroring `tools/chunkdump.py` / `tests/test_chunkdump.py`.

---

## References

- Referenced from [`bchunk.md`](bchunk.md) §2 and [`../TOOLS_AND_SKILLS.md`](../TOOLS_AND_SKILLS.md).
- Community editors: VltEd, Attribulator (nfs-tools ecosystem). _TODO: add stable URLs._
- Cross-check field names against the `dbalatoni13/nfsmw` decompilation where AttribSys is referenced:
  `src/Speed/Indep/Tools/AttribSys/` and `symbols/vlt.txt` (known name hashes).
