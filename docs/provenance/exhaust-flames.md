# Exhaust flames

- **Spec:** [exhaust-flames](../specs/exhaust-flames.md).
- **Sources read for the spec:** dbalatoni13/nfsmw (decompiled, CC0-1.0), under `src/Speed/Indep/Src`:
  `World/CarRenderConn.cpp`, `World/VehicleRenderConn.cpp` and `.h`, `World/CarRender.cpp`, `World/Car.hpp`,
  `Ecstasy/EmitterSystem.cpp` and `.h`, `Physics/Behaviors/DrawCar.cpp`, `Physics/Behaviors/EngineRacer.cpp`,
  `Physics/PhysicsUpgrades.hpp`, `Generated/AttribSys/Classes/{ecar,emitterdata,emittergroup,pvehicle}.h`,
  `Generated/AttribSys/Classes/emittergroup_hash.h`. The installed game's own data (`attributes.bin`, the car
  solids, the particle texture pack) was read with the repository's readers to fill in the values.
- **Implemented:** 2026-10-09, from the spec only (decompiled code not open while writing the Rust).
- **Checked against the game by:** unit tests of the trigger, the shift timer, the spawn and update rules; real-install
  tests of the data (groups, markers, textures); a rendered capture of the flame. No side-by-side capture of the
  original game was available, so colours, size and the look of the flame are unverified against it.
- **Known differences from the original:**
  - Who raises the up-shift and down-shift events is not in the decompiled sources; the rewrite raises one per
    change of forward gear. The curve basis `hermite_basis` and the platform particle blend state are not in the
    sources either (inferred, see the spec).
  - The installed engine upgrade level is a console setting (default 0), not career data.
  - Only the player's car; the miss-shift smoke (drag races), distance culling and the global particle caps are
    not reproduced; the emitter library omits start delays and on/off cycles (unused by these effects).

No game asset is stored in the repository: the particle definitions, the pipe markers and the textures are read
from the install at run time.
