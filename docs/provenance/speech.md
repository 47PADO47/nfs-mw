# Speech

- **Spec:** [specs/speech.md](../specs/speech.md), [formats/audio.md § Speech](../formats/audio.md#speech-and-nis-streams-big--idx--evt--csi)
- **Sources read for the spec:** dbalatoni13/nfsmw (decompilation, CC0-1.0): `src/Speed/Indep/Src/EAXSound/Stream/SpeechManager.cpp`,
  `SpeechManager.hpp`, `SpeechModule.hpp`, `GameSpeech.cpp`; `EAXSound/SND_GEN/COPSPEECH.cpp` and `.hpp` (event names and
  parameter types); `Generated/AttribSys/Classes/speech.h` (field names); `Speech/SoundAI.cpp` (units of the heat and
  speed inputs only); the header-only `Libs/spch/dev/include/spch/spch.h` and `Libs/csis/dev/include/csis/csis.h`.
  The install's own files, read as bytes: `SOUND/SPEECH/copspeech.idx`, `.evt`, `.csi`, `.big` and `GLOBAL/ATTRIBUTES.BIN`.
- **Implemented:** 2026-10-09, from the spec only (the decompiled code was not open while writing the Rust):
  `libs/ea-audio/src/speech/` (the `.idx` reader, from the byte layout measured on the install),
  `crates/nfsmw-data/src/speech.rs` (the `speech` and `speechtune` classes) and `crates/nfsmw/src/audio/speech/`
  (the queue, the checks, the recordings, the output).
- **Checked against the game by:** `ea-audio`'s real-install test: all 579 bank headers parse, the take counts add up
  to the 13,562 streams of `copspeech.big` and every take start is a stream start (the table was derived from the
  bytes, with no reference for the SPCH bank header). `nfsmw-data`'s real-install test: 133 events with the values
  read from the database. `nfsmw`'s real-install tests: the 28 events with banks of their own decode for several
  speakers, and the dispatcher says one of them and holds back a sentence. The unit tests check each rule of the spec
  on synthetic events. No comparison with the running original, and nobody has listened to the output.
- **Known differences from the original:** sentences (the `.evt` rules) are not resolved, so only the 28 events
  with banks of their own can be said; the four-list pipeline is one queue; the sort order of equal priorities is arrival order (the
  original's comparators are not in the readable decomp); take choice avoids the last half of the bank instead of
  SPCH's repeat rule; no radio click, low-pass, panning or per-speaker volume; no track-streamer gating; the
  dispatcher's clock stops while the pause menu is up.
