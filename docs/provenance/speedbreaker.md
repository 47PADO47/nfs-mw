# Speedbreaker meter and clock

- **Spec:** [speedbreaker](../specs/speedbreaker.md).
- **Sources:** original PC 1.3 executable (proprietary behavior reference),
  locally inspected on 2026-10-13; [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw),
  CC0 `LocalPlayer.cpp`, `PInput.cpp`, `QuickGame.h`, `SuspensionRacer.cpp`.
- **Identity:** executable SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- **Implementation:** independently from the written spec, replacing custom
  five-second drain, slow-driving recharge and hold-to-activate behavior.
- **Validation:** original static instruction inspection for meter, toggle,
  minimum speed, recharge globals, real/simulation clocks and quarter-speed
  ramp. Ten synthetic vectors were replayed through original meter/time-manager
  machine code in isolated x86 memory with synthetic entity/player interfaces.
  Synthetic state-transition tests check the replacement meter.
  The shared-world clock integrates entry/exit ramps over real frames and
  advances the existing fixed-step accumulator, including deterministic capture
  batches. Synthetic tests compare 5/30/60/120/144/240 FPS tick counts,
  transition integrals, real-time depletion on zero-tick frames and clock pause
  preservation. This wiring was implemented from the existing spec only.
- **Vehicle effects (E11):** the mass, downforce, drag, front-grip, tire-drag and steering effects are applied
  through `ManeuverAssist` only while active. The CC0 `SuspensionRacer.cpp` GameBreaker path matches their full-weight
  values. Ghidra on `speed.exe` found the PC constants (`0.25`, `0.75`, `25.0`, `60.0`, `2.0`) in the data table and
  one code reference to `60.0`, but not the begin/end event, so the PC mapping is not verified in the binary. The
  reference's 1/3 s weight ramp-out after exit is not implemented.
- **Configuration (E11):** `NFSMWSpeedFixerSettings.ini` with the mod's keys and units; clamps and vanilla fallback
  are unit-tested. The keys and the omission rules follow the NFSMWSpeedFixer mod. Spec: [speedbreaker](../specs/speedbreaker.md).
- **Limits:** no live original-game driving measurement; cinematic/multiplayer
  gates, audio/camera treatment, suspension/grip/collision-mass and drift
  recharge need separate integration/verification. Full parity is not claimed.
