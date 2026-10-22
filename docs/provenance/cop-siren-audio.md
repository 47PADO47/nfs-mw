# Cop siren audio (state machine, voice follower, SIREN module playback)

## Damaged light service override, 21 October 2026

The CC0 `SoundCar.cpp` SoundCop service independently overrides the pursuit
state with SIREN_DIE when its damage interface reports any LIGHT_COPS bit.
The matched PC function at 0x694660 is not defined in the saved Ghidra database;
bounded static disassembly of its 108 bytes confirms the 0x7000 mask query and
the subsequent packet state 3 assignment. These bytes are identical in the
privately identified D and selected E executables. This is static consumer
evidence, not an original-game acoustic comparison. Implement the override in
the world sound feed after the ordinary pursuit state machine, without changing
police health or interpreting cosmetic panels as damaged lamps.

- **Spec:** [docs/specs/cop-siren-audio.md](../specs/cop-siren-audio.md); the short form is section 10 of
  [ai-pursuit-speech.md](../specs/ai-pursuit-speech.md).
- **Sources read for the spec:** dbalatoni13/nfsmw (CC0, decompiled), `src/Speed/Indep/Src/`: `AI/Common/AIVehiclePursuit.cpp`
  (`UpdateSiren`), `AI/AIVehiclePursuit.h`, `EAXSound/CARSFX/CARSFX_Siren.{hpp,cpp}`,
  `EAXSound/SND_GEN/COP_SIREN_AEMS.{h,cpp}` (class and word names only), `EAXSound/States/Managers/STATEMGR_{CopCar,CarState}.cpp`,
  `EAXSound/EAXSOund.hpp`, `EAXSound/EAXSoundTypes.h`, `Physics/Behaviors/SoundCar.cpp`. Read for understanding only; no code
  copied. State names and numbers are interface facts.
- **Checked in `speed.exe` (Ghidra, base 0x400000):** the strings `SIREN_MB.abk` (0x0089793C, referenced from the start-up
  bank loader at 0x004CF010), `SIREN`, `SIREN_BED` and `CARSFX_Siren` exist. A search of the executable for the
  `pursuitlevels` field hashes as immediates found none (the fields are looked up by name at run time), so `UpdateSiren`
  itself was **not** located or compared; its logic is **[decomp]** only.
- **Data read from the install on 2026-10-20:** `GLOBAL/ATTRIBUTES.BIN` (`pursuitlevels` siren fields in all ten heat rows,
  `speechtune` intensity marks), `SOUND/IG_GLOBAL/SIREN_MB.abk` (modules, interfaces; module behaviour by running it with
  the project's AEMS interpreter), `SOUND/MIXMAPS/MAPOUTPUT.mxb` state 4 (controls, sub-mixes and masters of object 17).
- **Implemented:** the pure rules (`libs/blackbox-pursuit/src/siren`), the data reader (`nfsmw_data::pursuit_siren`), the
  world feed (`scenes/world/ai/sirens.rs`) and the playback (`crates/nfsmw/src/audio/siren`).
- **Checked against the game by:** pending. There is no listening or live comparison.
- **Known differences from the original:** the light bar gate is applied last; no speech focus (the first 5 s and 5 s of lost
  contact stand in); the pressing branch's speech-line condition is never true; one machine per car keyed by the car's id.
