# NPC chassis contact state evidence

- **Specification:** [NPC chassis state affecting contacts](../specs/npc-chassis-contact-state.md).
- **Sources:** read-only inspection of the user's hashed PC executable on
  2026-10-20 and the CC0 [dbalatoni13/nfsmw reference](https://github.com/dbalatoni13/nfsmw),
  `Physics/Behaviors/SuspensionSimple.cpp` and `Chassis.cpp`. No original source,
  machine instructions, captures or installation paths are included here.
- **Original evidence:** bounded PC task instructions select the three ride
  offsets from the previous wheel count before calling the common centre-of-gravity
  routine. Fresh PC steering decompilation confirms the brake, handbrake and
  inverse-throttle blend and its interpolation constants. The simple wheel-force
  routine's landing branch halves local yaw alone, whereas the racer branch
  also damps local roll. The readable reference corroborates these operations.
- **Implementation:** a caller-configured optional chassis state supplies
  support-dependent centre-of-gravity ride offsets, instantaneous steering range
  and landing angular retention. Police and patrol retain this adapter;
  ordinary traffic and trailers now select their distinct installed mechanics
  documented in [traffic suspension runtime](traffic-suspension-runtime.md).
  AI-racer behavior selection remains unverified. Stored body mass and inertia
  remain independently sourced.
- **Validation scope:** synthetic production-path checks cover all support-count
  branches, throttle/brake steering changes, first-landing angular retention and
  independent player configuration. Installed contact captures include police
  collision profiles and health changes, but do not capture complete wheel forces
  or prove the remaining simple-suspension pipeline.
