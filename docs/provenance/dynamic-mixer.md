# Dynamic mixer

Module: [`libs/blackbox-mixmap`](../../libs/blackbox-mixmap) (the map reader and the evaluator) and the
`audio::mixer` module of [`crates/nfsmw`](../../crates/nfsmw) (what the car sound publishes and reads).

- **Spec:** [docs/specs/dynamic-mixer.md](../specs/dynamic-mixer.md) (the evaluator),
  [docs/specs/car-sound-mixer.md](../specs/car-sound-mixer.md) (the car sound's side) and the file layout in
  [docs/formats/mixmap.md](../formats/mixmap.md).
- **Sources read for the spec:** dbalatoni13/nfsmw (https://github.com/dbalatoni13/nfsmw, CC0-1.0, decompiled;
  GameCube build), under `src/Speed/Indep/Src/EAXSound/`: `Dynamic_Mixer/{NFSMixMap,NFSMixMapState,NFSMixMaster,
  NFSMixShape}.{hpp,cpp}`, `Dynamic_Mixer/NFSMixerDefines.hpp`; `EAXSound.cpp` (the object callbacks),
  `SndBase.{hpp,cpp}` (`GetDMixOutput`), `States/Managers/STATEMGR_PlayerCar.cpp`, `EAXCar.{hpp,cpp}`;
  `sfxctl/SFXCTL_{Physics,Engine,HybridMotor,Tunnel,MasterVol,3DObjPos}.cpp`; `CARSFX/CARSFX_{Engine,Shifting,
  Turbo,Nitrous,SparkChatter,Skids,Roadnoise,WindNoise,BottomOut}.cpp`, `CARSFX/SFXObj_{Enums.hpp,Collision.cpp}`;
  `EAXSoundEnums.hpp`. Read for understanding only; no code copied. The two big tables of `NFSMixShape.cpp`
  (dB to Q15 and back) and the cosine table are mathematical functions; the spec gives the formulas, the code
  computes them, and the original's tables were not copied (they were compared on their end points and a sample of
  entries to find the formulas).
- **Install probes (no game data copied):** the four `.mxb` files were parsed with a throwaway script to find
  the layout (the sections tile each state and the file exactly; constants such as `0xD8F0` in every master
  channel; counts per state; which dwords differ between the four maps) and evaluated with a Python port of the
  evaluator at car-sound inputs to see whether the outputs are plausible (a gain of 0.33 for the engine,
  attenuations of a few dB for the effects). Only layout, counts and ranges are quoted in the docs.
- **Implemented:** 2026-10-08, from the specs (the decompiled code was not open while writing the Rust).
- **Checked against the game by:** unit tests on synthetic maps built from the layout (every element kind, every
  curve shape, an AR/ASR/ATR run, a 3D rolloff, a channel sum, the preset conversions), tests that the four real
  maps parse and evaluate without non-finite values, and the arithmetic round trips. Not compared with the
  running game's `GetDMixOutput` values; the spec's "How to check it" says how.
- **Known differences from the original:**
  - The master channels' starting level is read from the high half of the data dword (the console loader's
    first-short read); the low half is the same constant in every channel. Without this every output is silent,
    which is how it was decided (docs/formats/mixmap.md).
  - Doppler is not computed; the 32-instance limit and the memory pools are not reproduced; "attached" is a flag.
  - The GameCube build was read; the PC build may differ in details.
  - `dB`/`q15` are computed, not looked up, and differ from the original's tables by up to 2 in `q15` (0.006 %).
