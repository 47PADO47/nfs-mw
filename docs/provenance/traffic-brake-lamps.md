# Traffic brake-lamp evidence

- **Specification:** [traffic brake lamps](../specs/traffic-brake-lamps.md),
  written before implementation.
- Read-only PC vehicle FX task `0x6703B0` reads regular brake input, tests whether
  its owner is a player, and keeps a non-player uninterrupted brake-duration
  accumulator. It enables group 56 only after that accumulator exceeds 0.5;
  a released input resets it immediately. Player input skips the delay.
- Analysis image SHA-256 is
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  Offline section mapping confirms the 255-byte task and its threshold constant
  are identical in the selected executable with SHA-256
  `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
  No live process was launched or patched.
- The CC0 [dbalatoni13/nfsmw reference](https://github.com/dbalatoni13/nfsmw)
  corroborates `IInput::GetControls`, the regular-brake field, `ISimable::IsPlayer`
  and the VehicleFX brake-group mapping. Existing NOTICE credit applies.
- The remake previously drove NPC brake textures and flares directly from every
  regular-brake pulse, omitting this FX admission state. Regressions retain the
  actual brake control while checking short pulses, sustained braking, immediate
  release, player input and fixed-tick equivalence across presentation rates.
- This identifies a concrete cause consistent with reported fast blinking.
  It does not establish that every reported flash has this cause, or prove
  complete original traffic trajectories or live visual equivalence.
