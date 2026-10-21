# Native collision and destruction parity

This work keeps the native contact solver and installed game attributes. It does
not introduce an alternative solver or a replacement mass database. It corrects
the inputs, dispatch order and consumers of vehicle and prop contacts.

## Implemented

- Read authored prop immobility, eligibility and world/object collision flags;
  preserve installed mass, inertia and breakaway settings. Separate object-pair
  integration from end-frame drag, resident world barriers, ground support and sleep.
- Match scalar contact transforms, world inertia and moment projection order,
  including distinct physical and collision-notification responses for immobile bodies.
- Configure the supported NPC simple-chassis centre of gravity, steering range
  and first-landing yaw damping without changing the player chassis.
- Feed police health, shock and destruction from owner-specific collision
  callbacks and installed damage tuning. Select authored replacement body parts,
  damaged glass and health/death emitters independently for actors sharing a rig.
- Select owner/material-specific hit, scrape, detachment, ground and bottom-out
  sound and particle actions. Keep source collision effects independent of the
  damage sequencer and audio playback, with bounded caches and particle budgets.
- Execute sequenced prop particle actions with authored clocks, activation ranges,
  wrapper intensity, inherited velocity, textures and atlas frames. Use the
  petrol-station moment recording for explosion audio and source camera shake.
- Show fractional pursuit heat and provide session-only F8 pursuit-level controls.
- Cache police models by requested vehicle identity when they inherit geometry.
  Discard duplicate/orphaned results before GPU upload, preventing repeated Cross
  loading during high-heat pursuits.
- Execute authored named lamp-damage actions, including installed cop light-bar
  detachment triggers, and send damaged emergency lamps to the siren service.
  Keep these flags independent of cosmetic panels and police health.
- Retain the initial impact-sound azimuth while ongoing scrapes follow the view;
  apply the original weak smokable-wall gate only when creating a scrape voice.
- Restore successful-reset object-collision grace and its contact-sensitive
  countdown, preserving ground support and world/barrier responses.

Contracts and provenance are indexed in [specs](specs/README.md) and
[provenance](provenance/README.md). Start with [native impacts](specs/native-impact-parity.md),
[world barriers](specs/prop-world-barriers.md),
[NPC chassis state](specs/npc-chassis-contact-state.md),
[police damage](specs/cop-visible-damage.md) and
[source contact effects](specs/source-contact-effects.md).

## Evidence and limits

Synthetic regressions cover arithmetic, notification mass selection, contact order,
eligibility, particle controls, damage state and resource ownership. Ignored tests
selected by `NFSMW_GAME_DIR` exercise installed attributes, police parts/effects,
collision sound selection, prop barriers, pursuit HUD and the real model loader.
Headless native Vulkan/DX12 cases verify effects and damaged-part renderer delivery.

Separate private comparisons execute original contact/geometry routines and replay
natural recorded contact inputs against the Rust reference driver. These isolated
comparisons do not prove identical city-wide trajectories or audiovisual output.
The public reference examples accept scalar inputs and read the user's installation;
original machine code, recordings, assets and analysis tools are not distributed.

Known remaining gaps include full instance-triangle collisions, custom world OBB
coverage, jointed scenery constraints, and the remainder of the original simple
suspension/traction/scheduling behavior. Tracked object scrapes retain the callback
world frame rather than rebuilding the original owner-relative tracking transform;
between callbacks, scrape velocity and visual culling retain their previous sample.
See each contract's limitations before claiming original-game parity.
The model-cache fix preserves alias identity; it does not independently validate
every alias-specific physics or AI profile selected through geometry inheritance.

The high-heat loading regression covers 1000 repeated Cross requests plus stale
completions without further mesh or texture uploads. A user playtest confirmed a
substantial improvement after the fix; CPU limiting around 100 FPS remains reported.
Further long-pursuit profiling and matched original/remake playtests remain necessary.
