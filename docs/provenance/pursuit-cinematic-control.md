# Provenance: pursuit cinematic control and arrest placement

Date: 2026-10-23. Spec:
[pursuit-cinematic-control.md](../specs/pursuit-cinematic-control.md).

The Ghidra program identity was checked before queries: PC 1.3 `speed.exe`,
SHA-256 `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
Native functions read: arrest query `0x0044da10`, road initialization
`0x007862d0`, segment search `0x00785a00`, road-nav setup `0x00777d30`,
and angle conversion `0x005cb5d0`. The angle scale at `0x00890980` is
`1 / 65536`. The decompiler did not contain a defined function for the old
handler address `0x0044dc70`; the body was not recreated or silently assumed
to have been read in this pass.

The CC0-1.0 reference reconstruction
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) supplies structure
names and complementary behavior: `AIVehicle.cpp` (`SetAiControl`, human
update), `AIGoal.cpp` (racer actions), `AnimChooseArrest.cpp` (road boundaries
and scene ownership), and `AICopManager.cpp` / `AITrafficManager.cpp`
(population gates). Source was read for facts, then closed before writing
the specification and implementation. No original code or assets are included.

Implemented behavior:

- Reuse the existing race action, obstacle trail and PID controller on the
  physical player during a breaker moment. Ownership masks pedals, buttons,
  drag steering, manual shifting and Speedbreaker, then releases queued
  one-shot inputs with the temporary state. Damage and the body persist.
- Hold new traffic, patrol, wave, support, roadblock and queued cop placements
  without using the session lockout or clearing requests. Existing physics,
  retirement and support timers continue.
- Query a cop-road navigator at the player's position with zero heading
  weight; use its road-boundary midpoint for the arrest NIS. An unavailable
  road goes through the existing no-scene flow.

Synthetic regressions exercise complete input replacement and arrest priority,
queued placement preservation/resume, temporary racer steering, asymmetric
road midpoint placement and the missing-road outcome. The parent run records
which checks were executed and their results. No new original-game recording
or live physical comparison was performed.

Limits: temporary cruise speed preserves the entry speed as explicit rewrite
policy; the original non-race speed target remains unresolved. The precise
arrest transform-axis convention, per-scene camera framing and physical
autopilot behavior remain unverified. NIS codecs and authored tracks are reused;
this change adds no procedural animation or speculative scene timing.
