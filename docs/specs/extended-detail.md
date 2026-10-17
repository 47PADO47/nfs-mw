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
- Highest-world is a diagnostic: choose slot zero where it exists, within
  eligible visible scenery. Do not substitute arbitrary slots or draw hidden
  variants. It does not mean loading the entire map.
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
