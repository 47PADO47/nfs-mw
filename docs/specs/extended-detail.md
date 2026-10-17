# Detail controls and extensions

The original contracts are in [original graphics detail](original-graphics-detail.md).
This document specifies remake enhancements and integration, not additional original behavior.

- World detail has four values: low, medium, high, maximum. Low/medium select
  coarse geometry and discard two/one available top texture mips. High/maximum
  use adaptive geometry and full textures. Preserve authored variant filtering
  and missing-slot behavior at every level, including extensions.
- Geometry retention and small-object visibility have independent 1x/1.5x/2x
  projection multipliers. They affect the main view only; zone lists, shadow
  coverage and reflection budgets remain independent.
- Full scenery keeps slot zero throughout each eligible object's main-view
  lifetime, falling back to its authored coarse slot only when slot zero is
  absent. Bypass projected-size mesh and small-object rejection in this mode;
  retain the caller's frustum culling, section residency and variant filters.
  Do not substitute special-view slots or load the entire map. The saved
  `highest_world` key remains compatible with the former diagnostic checkbox.
- Extended geometry uses a ten-percent hysteresis band around the density
  threshold. The original 1x mode has no added hysteresis. State is per main
  view and stable instance identity; prune it as scenery leaves the view.
- Texture residency changes apply to the next world scene. Existing and newly
  streamed sections in a running scene use the same captured mip policy. UI,
  car and sky textures retain their independent full-quality route. This is a
  bounded world-texture policy, not a sampler bias or exact reproduction of the
  original pack-category exemption. Display the restart requirement explicitly.
- Car geometry exposes Low/High adaptive choices and an always-highest
  enhancement. Missing authored parts may only use their established assembly
  fallback, never an unrelated part. LOD changes preserve animation, physical
  identity, damage, customization and light state. No per-frame install reads
  or GPU uploads are permitted for switching detail.
- Keep detail in graphics profiles, named presets, A/B comparisons and startup
  restoration. Preserve unknown configuration keys and existing user profiles.

The fixed 480-line projection reference remains uncalibrated against the
playable PC reference. No setting is labeled as complete original parity.
Measure route frame times and streaming before selecting extended defaults.

Traffic population is independent of mesh LOD. The current manager may create
whole cars within its forward 150--250 m band (plus speed), including visible
roads. It uses a camera-frustum keep-alive when recycling, but the spawn gate
checks spacing rather than visibility. A future anti-pop policy needs to cover
off-screen/occluded placement and straight-road population; merely enlarging
mesh range or keeping car LOD A cannot prevent population appearances.
