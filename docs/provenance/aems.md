# AEMS module banks

Module: [`libs/blackbox-aems`](../../libs/blackbox-aems) (the bank reader and the module interpreter) and the
`audio::aems` module of [`crates/nfsmw`](../../crates/nfsmw) (the engine's sample layer, the sputters and the
sweeteners).

- **Spec:** [docs/specs/aems.md](../specs/aems.md) (the interpreter),
  [docs/specs/engine-sound-aems.md](../specs/engine-sound-aems.md) (the car sound's use) and the layout in
  [docs/formats/aems.md](../formats/aems.md).
- **Sources read for the spec:** dbalatoni13/nfsmw (https://github.com/dbalatoni13/nfsmw, CC0-1.0, decompiled;
  GameCube build): `src/Speed/Indep/Libs/snd/9/extern/aemsdef.h`, `source/library/cmn/{saems.c,saemsi.h,
  saemstimupdt.c,sndcmn.h}`; `src/Speed/Indep/Src/EAXSound/CARSFX/CARSFX_{Engine,SparkChatter,Shifting}.cpp`,
  `SND_GEN/ENGINES_AEMS2.h`, `UG/NFSUG_CarsSFXLoadData.cpp`. Read for understanding only; no code copied. The PC
  `speed.exe` was disassembled (a local, untracked copy of the user's own install) for the function table of the
  node numbers: functions 0 to 3, 7, 11, 15, 28 and 31 were read to confirm the numbering and the node layouts;
  nothing from it is in the repository beyond the facts in the specs.
- **Install probes (no game data copied):** all 301 `.abk` banks were walked: headers, modules, the code of every
  module decoded with a throwaway disassembler (capstone) to find the (small) instruction set, and every module
  run on its authored state with a Python port of the interpreter; the engine, sweetener and sputter modules were
  then driven with plausible parameters to see what they do (the tables in the specs). Only layout, counts and
  the behaviour numbers of the specs are quoted.
- **Implemented:** 2026-10-08, from the specs (the decompiled code was not open while writing the Rust).
- **Checked against the game by:** unit tests on synthetic modules built from the layout (every node kind, the
  code decoder, the player protocol), tests that all shipped modules decode and run, and the behaviour checks
  above on the M3's banks. Not compared with the running game's voices; the spec's "How to check it" says how.
- **Known differences from the original:**
  - The random generator and the sine table of the sound system are not in the sources and are replaced.
  - The update period is the fixed 60 Hz tick instead of the sound system's variable timer.
  - Nodes the install never uses (global variables, function nodes, set-global) are not implemented.
  - The GameCube build was read; the PC build was confirmed for the numbered functions only.
