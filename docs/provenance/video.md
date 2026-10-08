# Movies (`blackbox-movie`)

- **Spec:** [formats/video.md](../formats/video.md)
- **Sources read for the spec:**
  - [ruffle-rs/nihav-vp6](https://github.com/ruffle-rs/nihav-vp6) (MIT, `COPYING`: the VP5/VP6 subset of
    NihAV, relicensed by Kostya Shishkov for Ruffle). Read for the VP6 frame header bit layout, and used
    **as a dependency** (not copied) for the VP6 decoder.
  - [FFmpeg](https://github.com/FFmpeg/FFmpeg) `libavformat/electronicarts.c` (LGPL-2.1+). Read as a
    reference for **facts only**: that the `MVhd` frame count comes before the two rate fields, and that
    the audio of this header type defaults to 48,000 Hz. No code was copied or translated.
  - [vgmstream](https://github.com/vgmstream/vgmstream) `ea_schl.c` (ISC-style): `SCHl` tag meanings.
- **Implemented:** 2026-10-08. The demuxer (block walk, `MVhd`, audio and video packets, timing helper)
  is written from the layout in `formats/video.md`, which was measured on the install. No decompiled
  code is involved.
- **Checked against the game by:** a probe over all 32 `MOVIES/*.vp6` of PC v1.3 (Python, not
  committed): block walk to the end of every file; all block sizes multiples of 4; `MVhd` frame count
  equals `MV0K + MV0F`; `SCCl` equals the `SCDl` count; the sum of the `SCDl` sample counts equals the
  `SCHl` sample-count tag; key frames start with bit 0 clear and inter frames with bit 0 set; every key
  frame is VP6 version field 7, simple profile. `#[ignore]` tests in the crate repeat these on the
  install (`NFSMW_GAME_DIR`), then decode every frame.
- **Known differences from the original:** the original player (EA's `rcmp`/`avplayer`) is not read.
  Presentation time is `frame index / fps` with fps from `MVhd`; whether the game resyncs to the audio
  clock is unknown **[unconfirmed]**.
