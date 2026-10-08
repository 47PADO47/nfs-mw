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
- **Checked against the game by:**
  - EA-XA: output compared sample for sample with FFmpeg (`adpcm_ea_r3` on the first music stream, `adpcm_ea_r2`
    on a mono and a stereo cut-scene stream); zero differences. FFmpeg was only run as a black box on scratch
    copies outside the repository.
  - `#[ignore]` tests in `libs/ea-audio/tests/real_install.rs` read the install through `NFSMW_GAME_DIR`: 301 banks
    list and decode 2,577 sounds (the sample tables reference exactly these), 3,257 music streams (203.9 min, stored
    durations within 2 ms of the decoded lengths), 13,562 MicroTalk streams in `copspeech.big`, 142 streams in
    `NISAudio.big`, and 160 `.gin` files. Every stream decodes to its header's sample count; none is noise-like;
    clipping is under 0.1 % except in the cop speech (0.1 %, runs of at most 8 samples, speech mastered at full
    scale). No decoded audio is committed or written by the tests.
  - Bank layout cross-checks: each stereo sound's second channel starts exactly 4 zero bytes after the first
    channel's last frame (40 of 40); EA-XA coefficient indexes stay below 4 in all bank, cut-scene and the first 400
    music streams.
  - MicroTalk has no second implementation available, so it rests on the port plus the checks above.
- **Known differences from the original:**
  - EA-XAS and MicroTalk use float arithmetic in the original decoders; the Rust code uses `f32`, so rounding of
    a sample can differ by one LSB from other implementations.
  - Not implemented: EALayer3/MPEG, PS-ADPCM, DSP, IMA, N64 and PCM codecs (not present in MW), MicroTalk
    loop-reset handling (no MicroTalk stream in MW loops), the `SCLl` loop block, EA-XAS version 1, the CBX
    variant of MicroTalk, `.ast` streamed bank entries (none in MW).
  - The `.idx`, `.evt` and `.csi` companions and the PathFinder node graph are not parsed.
