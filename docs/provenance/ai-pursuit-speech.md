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
- **Air-support pass (E9 speech):** `disptimeexpired` (180) is wired on the lost spell's evade timeout, gated by the `SearchPattern`
  switch. The RevisedAirSupport keys are read from `NFSMWAirSupportSettings.ini` (`audio/chase/air_support.rs`), clean-room from
  `docs/mods/nfsmwrevisedairsupport.md` (behaviour and key names only). Speech collection names were checked in `/speed.exe`
  (`AnytimeEvents_WeatherReport`, `AnytimeEvents_IntentToRam`, `AnytimeEvents_DispTimeExpired`, `AnytimeEvents_LostSuspect`,
  `Setup_VehicleReport`); numbers are from the attribute data. The mod's own enum values (113, 155, 219) agree with the data.
- **Implemented:** partly. The opening line (`setup_initpursuit`), the lost and regained sight (`lostvisual`, `regainvisual`) and
  the heat jump (`heatjump`) are requested from the chase in `crates/nfsmw/src/audio/chase.rs` and `audio/chase/cues.rs`. Also wired
  from the feed's air-support state, each gated by its RevisedAirSupport switch: rain (`weatherreport`), roadblock call
  (`callforrb`), been hit (`collisionworld`), helicopter reacquire and spotted (`helilostvisual`, `spotted`, with `heli_present`), and
  cop down (`unitdisabled`). Ram, vehicle report, escaped car, 911, fast air, rain flag and strategy lines are not (see the TODO in
  `chase.rs` and events spec section 5). Sight is read from the feed's `intensity` (1 while seen), not a seen flag.
- **Air-support edges (E9 speech, second pass):** the rain threshold (0.5), the been-hit collection (`collisionworld`), the kill
  line (`unitdisabled` for `KillShot`) and the switch-to-line mapping are **guesses** from the mod's descriptions and the spec table,
  to check in play. Tests: `audio/chase/line_tests.rs`, `audio/chase/air_support_tests.rs`.
- **Event names confirmed in `/speed.exe`:** `Setup_InitPursuit`, `AnytimeEvents_LostVisual`, `AnytimeEvents_RegainVisual`,
  `AnytimeEvents_HeatJump` (Ghidra strings, base `0x400000`). Second pass: `HeliSpecific_HeliLostVisual`, `AnytimeEvents_Spotted`,
  `AnytimeEvents_UnitDisabled`, `AnytimeEvents_CollisionWorld`, `StaticRoadblock_CallForRB`, plus the `CollWorld_*` variants. No
  rain-flag, been-hit or kill-shot string under those names.
- **Checked against the game by:** pending (the spec lists a play-through checklist: opening line, stop branch, assault restart, lost-contact sequence, arrest
  bullhorn, 911 call).
- **Known differences from the original:** none yet.
