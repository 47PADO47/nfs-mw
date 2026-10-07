# blackbox-chunk

Zero-copy parser for the **bChunk** container used by EA Black Box games for almost every data file
(`u32 id, u32 size, payload`; bit 31 = container). Handles alignment padding the way the engine does and
bare JDLZ blobs between chunks. Chunk ids used by the other libraries are in `ids`, grouped by domain.

Spec: `docs/formats/bchunk.md`. Little-endian (PC) data only so far.

License: MIT OR Apache-2.0.
