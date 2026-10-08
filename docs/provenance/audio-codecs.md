# ea-audio (EA containers and codecs)

- **Spec:** [docs/specs/audio-containers.md](../specs/audio-containers.md), with the measured layouts in
  [docs/formats/audio.md](../formats/audio.md)
- **Sources read for the spec:**
  - vgmstream, <https://github.com/vgmstream/vgmstream>, commit `7dc938f` (2026-09-28), ISC-style licence
    (`COPYING`: permission to use, copy, modify and distribute with the notice kept). Files read:
    `src/meta/ea_schl.c`, `ea_schl_abk.c`, `ea_schl_map_mpf_mus.c`, `gin.c`, `src/layout/blocked_ea_schl.c`,
    `src/coding/ea_xa_decoder.c`, `ea_xas_decoder.c`, `ea_mt_decoder.c`, `src/coding/libs/utkdec.c`.
    The files carry no per-file notice; the project-level `COPYING` applies.
  - `utkdec.c` is derived from Andrew D'Addesio's `utkencode`, <https://github.com/daddesio/utkencode>,
    declared Unlicense / public domain in `utkdec.h`.
  - The install itself (PC v1.3): all 301 `.abk`, `MW_Music.mpf/.mus`, `copspeech.big`, `NISAudio.big`.
  - Not used: dbalatoni13/nfsmw (decompiled, CC0). The file layouts and codecs do not need it. FFmpeg's
    `electronicarts.c` (LGPL) and NFS-ModTools / MWEncyclopedia (no licence) were not opened.
- **Implemented:** 2026-10-08. The permissively licensed parts were ported, not copied verbatim: the Rust code
  restructures them (slices instead of stream files, a block iterator instead of vgmstream's layout/state
  machinery, one function per codec). The constant tables (EA-XA coefficients, XAS filter pairs, MicroTalk
  reflection-coefficient table, codebooks and command table) are carried over as numbers. Files that follow a
  vgmstream or utkdec routine closely keep the ISC notice (and, for MicroTalk, the public-domain credit) at the top:
  `libs/ea-audio/src/codec/{xa,xas}.rs` and `libs/ea-audio/src/codec/microtalk/*.rs`. Container parsers
  (`schl`, `abk`, `mus`, `big`) are written from the layouts documented here.
- **Checked against the game by:** `#[ignore]` tests in `libs/ea-audio/tests/` that read the install through
  `NFSMW_GAME_DIR`: every sound of the 301 banks decodes to its declared sample count (2,577), every stream of
  `MW_Music.mus` (3,257), `copspeech.big` (13,562) and `NISAudio.big` (142) decodes to the header's sample count,
  no sample clips in more than a tiny fraction of samples, and all 160 `.gin` files decode to
  their declared sample count. No decoded audio is committed. Exact figures are in the crate README.
- **Known differences from the original:**
  - EA-XAS and MicroTalk use float arithmetic in the original decoders; the Rust code uses `f32`, so rounding of
    a sample can differ by one LSB from other implementations.
  - Not implemented: EALayer3/MPEG, PS-ADPCM, DSP, IMA, N64 and PCM codecs (not present in MW), MicroTalk
    loop-reset handling (no MicroTalk stream in MW loops), the `SCLl` loop block, EA-XAS version 1, the CBX
    variant of MicroTalk, `.ast` streamed bank entries (none in MW).
  - The `.idx`, `.evt` and `.csi` companions and the PathFinder node graph are not parsed.
