# Car engine sound

Modules: [`libs/blackbox-ginsu`](../../libs/blackbox-ginsu) (the Gnsu tables and the grain synthesiser) and,
later, the `sound` module of [`crates/nfsmw-data`](../../crates/nfsmw-data) (a car's sound set from AttribSys).

- **Spec:** [docs/specs/engine-sound.md](../specs/engine-sound.md) (files, telemetry, engine mix),
  [docs/specs/engine-sound-ginsu.md](../specs/engine-sound-ginsu.md) (the synthesiser) and
  [docs/specs/engine-sound-effects.md](../specs/engine-sound-effects.md) (shifting, turbo, nitrous, skids,
  collisions). Split in three to stay under 500 lines per file. File layout:
  [docs/formats/audio.md](../formats/audio.md#gnsu-granular-engine-sounds-gin).
- **Sources read for the spec:** dbalatoni13/nfsmw (https://github.com/dbalatoni13/nfsmw, CC0-1.0, decompiled;
  GameCube build), under `src/Speed/Indep/Src/EAXSound/`: `Ginsu/{ginsu.h,ginsudata.cpp,ginsusynth.cpp,
  ginsuhelper.h}`; `CARSFX/CARSFX_{Engine,Shifting,Turbo,Nitrous,Skids,SparkChatter,BottomOut}.{hpp,cpp}`,
  `CARSFX/SFXObj_{Collision,EnumAttributes}.{hpp,cpp}`; `sfxctl/SFXCTL_{HybridMotor,Engine,Physics,Shifting,
  AccelTrans,Wheel}.{hpp,cpp}`; `EAXCar.{hpp,cpp}`, `EAXCarState.hpp`, `EAXSoundTypes.h`, `SoundConn.{h,cpp}`,
  `SoundCollision.{hpp,cpp}`, `EAXSndUtil.{h,cpp}`, `EAXAemsManager.cpp` (path table only), `OldSoundTemplates.hpp`;
  `UG/NFSUG_CarsSFXLoadData.cpp`; `States/Registration.cpp` and `States/Managers/STATEMGR_*.cpp` (which sound
  objects each car context creates); `SND_GEN/{ENGINES_AEMS2,MAIN_AEMS,TURBO,ENVIRO_AEMS,STITCH_AEMS}.h`
  (parameter structs and class names only); `Dynamic_Mixer/NFSMixShape.{cpp,hpp}` (curve shapes only; the
  mix-map evaluator was not read); `Physics/Behaviors/SoundCar.cpp`; `World/VisualTreatment.cpp` and
  `Libs/Support/Utility/UBezierLite.cpp` (the spline helper); `Misc/Table.{hpp,cpp}` (running average);
  `Libs/snd/9/include/snd/sndo.h` (units of volume and pitch); `Generated/AttribSys/Classes/
  {engineaudio,shiftpattern,turbosfx,acceltrans,audioimpact,audioscrape,audiosystem,pvehicle}.h`. Read for
  understanding only; no code copied. The numbers in the specs that come from code (thresholds, time
  constants, the 7000 volume slew, ...) are tuning constants recorded as data; per-car values come from the
  install's AttribSys data.
- **Install probes (no game data copied):** the 160 `.gin` headers and tables (counts, ranges, monotonic
  direction, period against frequency); the 70 `engineaudio`, 25 `shiftpattern`, 18 `turbosfx`, 28 `acceltrans`
  collections and the `audiosystem/mostwanted` bank lists via `blackbox-attrib`; file existence of every named
  `.gin` and `.abk`; `BNKl` sound counts of a few banks. Only counts, ranges and names are quoted in the docs.
- **Implemented:** `blackbox-ginsu` 2026-10-08, from the specs only (the decompiled code was not open while
  writing). The `sound` module of `nfsmw-data` follows the same rule.
- **Checked against the game by:** synthetic sine and cycle tests for the library (pitch, joins, determinism);
  `#[ignore]` tests in `crates/nfsmw-data/tests/real_install/` that resolve every car's sound set against the
  install (all named files exist, at stock and fully upgraded) and decode real `.gin` files (via `ea-audio`),
  play them through the synthesiser and measure the period against the file's tables. Nothing yet compared
  with the running game's audio; "How to check it" in each spec lists what to measure.
- **Known differences from the original:**
  - No priming silence, no hardware packet queue: packets are built on demand to fill any block size.
  - Reads outside the recording give silence (the original left stale buffer contents).
  - The synth's output is the mono signal at the file's sample rate; volume, the sub-minimum pitch ratio and the
    mixer's pitch multiplier are applied by the caller or by an optional output gain (the original used the
    sound system's per-voice volume and `SNDpitchmult`).
  - Mixer curves (`MIXMAPS/*.mxb`) are not read: unity volume and pitch until they are.
  - The AEMS sample layer is not reimplemented (its sample selection is bank data, spec question Q5).
  - Expected: the GameCube build was read, so values and some behaviours of the PC build may differ.

## Open questions

Tracked for the implementer; see "Open questions" at the end of each spec: update rate of the control code
(engine-sound Q1), single or dual Ginsu mode in the PC build (Q2), the `Ginsu_ACL_Neg_L_RPM` slip (Q3), the
mixer maps and `SNDvol` scale (Q4), the AEMS layer (Q5).
