# Garage inspection and gauge preview provenance

Specification: [garage-inspection](../specs/garage-inspection.md), extending
[garage-game-ui](../specs/garage-game-ui.md) and the existing frontend camera,
car-lighting and HUD specifications. Date: 2026-10-24.

Sources read:

- The installed PC game assets: frontend attributes, original customization
  packages and `HUDS_Custom_00..10` texture packs. Proprietary; read at runtime,
  never copied into the repository.
- The matched retail PC executable, SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  Ghidra program identity checked before bounded inspection of the SelectCar
  update and three speed setters. Static analysis, not a live execution trace.
- [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0, read-only
  reference: generated frontend schema and GarageMain input/idle behavior.
  No decompiled source was copied into the implementation.
- Existing project HUD texture/scale functions and frontend camera mover,
  reused under this repository's MIT OR Apache-2.0 license.

The installed schema probe confirmed the Bool type, field width and offset of
`cam_user_rotate`. A private inventory records hashes and per-screen values.
The private native evidence retains the exact executable identity, inspected
procedures and binary float constants. Public files contain behavioral facts
and identifiers only.

The gauge object inventory came from the existing FEng package reader's
`dump-screen` command. Unit tests use synthetic FEng objects; installed tests
select and decode the original texture identities for every stock skin.

Synthetic inspection-entry tests check its native duration, lens and target,
continued progress after input release, and handoff to a new screen without
applying stale motion. No assertion here certifies full original-renderer visual
parity: native renderer captures and controller input remain separate checks
alongside the state-machine and installed-data tests.
