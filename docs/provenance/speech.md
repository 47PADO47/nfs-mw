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
- **Known differences from the original:** sentences are built by the rules of [speech-sentences.md](speech-sentences.md)
  (provenance [speech-sentences.md](speech-sentences.md)); the four-list pipeline is one queue; the sort order of equal priorities is arrival order (the
  original's comparators are not in the readable decomp); a bank's takes are chosen by the bank's own repeat memory; no radio click, low-pass, panning or per-speaker volume; no track-streamer gating; the
  dispatcher's clock stops while the pause menu is up.

## Chase flows (2026-10-20)

- **Spec:** [specs/ai-pursuit-speech.md](../specs/ai-pursuit-speech.md), [specs/ai-pursuit-speech-events.md](../specs/ai-pursuit-speech-events.md)
- **Sources read:** dbalatoni13/nfsmw (decompilation, CC0-1.0), under `src/Speed/Indep/Src/Speech/`: `SoundAI.{h,cpp}`, `PursuitFlow.{h,cpp}`, `StrategyFlow.{h,cpp}`, `RoadblockFlow.{h,cpp}`,
  `Observer.{h,cpp}`, `EAXCharacter.{h,cpp}` (the actor methods that fill each sentence's parameters), `EAXCop.cpp`, `EAXDispatch.cpp`, `EAXAirSupport.cpp`, `MiscSpeech.cpp` (`GetLocation`, the quadrant lines, the 911 helpers),
  `MWRoadNames.h` (the road names, as facts for the table road id to region and location); `Generated/AttribSys/Classes/{speechtune,pursuitlevels}.h` (field names). Read for understanding; no code copied.
  Two functions were read in `speed.exe` with Ghidra, for behaviour only: the one that builds a new speech actor (`0x0071fa90`: the voice takeover by a closer car) and the roadblock flow's outcome (`0x0071f1b0`:
  an engaged roadblock keeps its outcome timer set). The install's `GLOBAL/ATTRIBUTES.BIN` (`speechtune`, `pursuitlevels` fields, `pvehicle.VerbalType`, the car parts' `SPEECHCOLOUR`), read as bytes.
- **Implemented:** 2026-10-20, from the specs only: `libs/blackbox-pursuit/src/speech/` (the machine), `crates/nfsmw-data/src/speech_chase.rs` (the tuning and the car description data),
  `crates/nfsmw/src/audio/chase/radio/` (the game's side: snapshot, tables, parameters), `crates/nfsmw/src/scenes/world/ai/police/speech_feed.rs` and `scenes/world/speech_look.rs` (the world's feed).
- **Checked against the game by:** installed tests: every one of the machine's lines is an event of the install; `speechtune` equals the table of the events spec; every stock paint of the car tables maps to a
  speech colour and every player car's `VerbalType` is a single `car_type` bit; a scripted chase timeline (64 requests, 23 lines) assembles to audio with the real recordings. Unit tests check each flow of the spec
  on scripted chases. No comparison with the running original; nobody has listened to the output.
- **Known differences from the original:** the queue's "busy" includes waiting requests; "first cop of the sort" is the cop that joined first (the original sorts pointers); a cop's sight is derived from the world's sight test;
  "time expired" fires for an evaded pursuit at once (the decompiled branch order would delay it); the assault infraction stands for a deliberate hit on the nearest cop; the seeded generator's sequence differs.
