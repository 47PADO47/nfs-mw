# AI pursuit speech (cop and dispatch radio triggers, pursuit music)

Planned module: a `pursuit_speech` layer of milestone 7 that emits speech requests to the audio engine (milestone 6 left speech open).

- **Spec:** [docs/specs/ai-pursuit-speech.md](../specs/ai-pursuit-speech.md), [docs/specs/ai-pursuit-speech-events.md](../specs/ai-pursuit-speech-events.md).
- **Sources read for the spec:** dbalatoni13/nfsmw (CC0, decompiled), `src/Speed/Indep/Src/`: `Speech/SoundAI.{h,cpp}`, `Speech/PursuitFlow.{h,cpp}`,
  `Speech/StrategyFlow.{h,cpp}`, `Speech/RoadblockFlow.{h,cpp}`, `Speech/Observer.{h,cpp}`, `Speech/MusicFlow.{h,cpp}`, `Speech/EAXCharacter.h`,
  `Speech/EAXCop.cpp`, `Speech/EAXDispatch.cpp`, `Speech/SpeechCache.h`, `EAXSound/EAXSoundTypes.h`, `EAXSound/Stream/SpeechManager.{hpp,cpp}`,
  `EAXSound/Stream/GameSpeech.{hpp,cpp}`, `EAXSound/SND_GEN/COPSPEECH.hpp` (enum and structure names only), `AI/Common/AIVehiclePursuit.cpp`
  (`UpdateSiren`), `Generated/AttribSys/Classes/{speech,speechtune}.h`. Read for understanding only; no code copied. The names of event
  parameters and enumerations in the specs are interface facts.
- **Data read from the install:** AttribSys `speech` (133 collections, 28 fields) and `speechtune` (1 collection, 39 fields) from `GLOBAL/attributes.bin`
  on 2026-10-09; the event table in the events spec is a compact summary of those collections (numbers only).
- **Gaps in the sources:** the sentence-selection data (`.evt`, `.idx`, `.csi` layouts) and the generated sentence functions that map event parameters to
  samples; the road speech id table; the SPAM map used for off-road moments; most of `EAXAirSupport.cpp` and `MiscSpeech.cpp` was only skimmed.
- **Implemented:** not yet.
- **Checked against the game by:** pending (the spec lists a play-through checklist: opening line, stop branch, assault restart, lost-contact sequence, arrest
  bullhorn, 911 call).
- **Known differences from the original:** none yet.
