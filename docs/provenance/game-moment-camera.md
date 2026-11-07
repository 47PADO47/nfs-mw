# Game Moment Camera

- **Spec:** [game moment camera](../specs/game-moment-camera.md); the breaker's cut is
  [pursuit breaker moment](pursuit-breaker-moment.md).
- **Sources:** private `speed.exe` 1.3 inspection (Ghidra) with the installed PC symbol map; the CC0 `dbalatoni13/nfsmw`
  reconstruction for names (`GOJumpCams`, `FEDatabase` gameplay settings `JumpCam`, `CameraAI`, `CameraAnchor`,
  `CameraParams`, `IPursuit`, `aud_moment_strm` keys, `eTrackPathZoneType`). No original code is included.
- **Addresses read in `speed.exe`:**
  - The option: the profile byte at `+0x47` of the user profile, toggled by `GOJumpCams::Act` `0x510090` and drawn by
    `GOJumpCams::Draw` `0x51C2A0`. It is read by the getter `0x469E30` (no in split screen and online) and directly by
    the breaker gate `0x640F00`.
  - The director: `SelectAction` `0x479900`, `Update` `0x479DE0`, `SetAction` `0x4797F0`, `TotaledStart` `0x479D80`,
    `MaybeDoTotaledCam` `0x479F20` (not gated).
  - The pursuit shot: `MaybeDoPursuitCam` `0x471BF0` from `AIPursuit` when it takes its target (`0x423DF0`), the clock
    `0x469EC0`, and `TrackCopCameraMover` (`Init` `0x475350`, `FindPursuitVehiclePosition` `0x475170`, `Update` `0x47E190`).
  - The jump shot: `EPlayerAirborne` posted by `PVehicle::OnTaskSimulate` `0x686060` and handled at `0x63A7F0`; the jump
    camera `0x479F80`; the prediction `0x471CD0` with `AerodynamicDownforce` `0x66BAE0`.
  - The crash shot: `CDActionDrive::Update` `0x472740` (the roadblock test, the speedbreaker flag, the wheels-on-ground
    flag).
  - The car camera: `TrackCarCameraMover` (constructor `0x47AA20`, `Init` `0x4745E0`, `Update` `0x47DE30`).
  - The slow motion: `CDActionDrive::SetSpecial` `0x481200` and the camera's simulation time multiplier (`+0xCC`,
    clamped by `0x6EA150`).
- **Implementation:** 2026-11-06, from the spec: `settings/game_moments.rs`, the Gameplay rows, `scenes/world/moment_cams*`.
- **Checked:** unit tests of the rules, the jump prediction, both cameras and the director; an installed test plays the
  pursuit shot beside the gas station's cops. Not compared with a running original.
