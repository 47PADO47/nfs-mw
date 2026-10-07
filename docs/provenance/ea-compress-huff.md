# ea-compress::huff

- **Spec:** [docs/formats/huff.md](../formats/huff.md)
- **Sources read for the spec:** EA's Command & Conquer Generals source, EAC library
  (`huffdecode.cpp`, `huffencode.cpp`, `huffabout.cpp`), GPL-3.0; dbalatoni13/nfsmw `Misc/LZCompress.cpp/.hpp`
  (decompiled, CC0). Both read for understanding only. Full list with URLs in huff.md, References.
- **Implemented:** 2026-10-07, from the spec only. No third-party code copied or paraphrased.
- **Checked against the game by:** decoding every HUFF blob in the install (10,813) to the expected size,
  with the texture trailer's name hash matching its entry; 12 synthetic unit tests and an `#[ignore]` real-install test.
- **Known differences from the original:** numbers with 16 or more leading zero bits are rejected (EA's
  decoder and encoder disagree on them; none occur in MW).
