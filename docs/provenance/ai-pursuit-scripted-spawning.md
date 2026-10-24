# Scripted cop spawns and placement

- Date: 2026-10-23.
- Specification: [ai-pursuit-scripted-spawning.md](../specs/ai-pursuit-scripted-spawning.md).
- Reference: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
  decompiled; `AICopManager.cpp` `TrySpawnCop`, `UpdateSpawnRequests`,
  `SpawnPursuitIVehicle`, and `AISpawnManager.cpp` `CheckSpawnPosition`.
  Read for facts; no code copied.
- Opening script: existing [story-flow.md](../specs/story-flow.md) section 2.4,
  including authored `cop_spawn_1` / `cop_spawn_2` placement evidence.

The runtime keeps queued chase requests until an available car can be placed,
then asks the scene to start their chase. Authored roadblock requests use a
stationary role. Pending chase requests are cancelled when their chase ends.
Player/guest bodies join the overlap check. Traffic lane changes reserve both
lanes until the lateral blend finishes. Pursuit fallback spawns face target
travel; seek-walk spawns reverse the cursor's direction.

Synthetic scenarios were added for request/lockout/start/cancellation,
roadblock placement, player overlap, destination lane occupancy and spawn
facing. These exercise remake state transitions, without claiming matched
original gameplay acceptance. The original off-world simple-physics spawn
fallback remains unimplemented.

[Optional police limits](../specs/pursuit-limit-options.md) are independently
authored enhancements, disabled by default. Their saved options use the normal
settings pipeline. Synthetic coverage includes enable/disable clamping,
reference-cap restoration and F8 serialization with unknown-key preservation.

The support request audit additionally read reconstructed `AIPursuit.cpp`
`GroundSupportRequest::Update` and its call from the pursuit update, and the
manager's activation paths. [The support spec](../specs/ai-pursuit-support.md)
records pending and active duration countdown, strict below-zero expiry and
full-duration restart at activation. The new world fixtures cover delayed model
arrival, pending expiry during search, active heavy cancellation on tracked
contact loss or actual collapse, leader expiry and complete support-car loss.
