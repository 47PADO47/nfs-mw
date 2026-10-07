# ea-compress

EA compression codecs found in EA Black Box games (Need for Speed Underground 2, Most Wanted, Carbon, …):
**JDLZ** (LZ77 with two flag streams), **HUFF** (canonical Huffman + run-length, EA stream type `0x30FB` and
its delta variants) and the stored **RAWW** wrapper. Bytes in, bytes out; no game knowledge.

- `unwrap(&[u8])` detects the 16-byte wrapper header and decompresses.
- Specs: `docs/formats/bchunk.md` (JDLZ, RAWW) and `docs/formats/huff.md` (HUFF, written spec-first).
- Verified on NFS: Most Wanted PC: every JDLZ and HUFF blob in the install decodes.

License: MIT OR Apache-2.0.
