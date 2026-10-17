# Reflective world surfaces

- **Specification:** [Original reflective world surfaces](../specs/world-reflect-materials.md).
- **Date:** 2026-10-16; specification written before implementation.
- **Sources:** embedded retail PC WORLDREFLECT vertex/pixel programs and their
  named constant tables, read only for behavioral reconstruction. No original
  source, shader bytecode, decompilation or asset is included.
- **Identity:** stock executable SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`;
  effect SHA-256
  `3c1ef66775787a6930817da2ae8293b36e8d726fb9ae75e631b94d54d3a5bbb6`.
  Prior effect inventory establishes byte identity with the preferred modded
  executable. Active hooks and runtime settings remain separate observations.
- **Independent measurements:** 58 synthetic original D3D9 cases: 29 selected
  modern unshadowed cases and 29 legacy cases. Inputs include synthetic diffuse,
  normal, specular and spatially varying reflected textures; material/light
  constants; vertex colors; raw bases; and controlled projection. Pixel evidence
  remains private; numeric behavior may be used in independent regressions.
- **Verified scope:** sampler roles, homogeneous projected reflection UV,
  reflected red-mask/alpha multiplication, doubled specular term, modern opaque
  shader alpha and selected legacy differences. The inspected basic modern
  shadow shader independently establishes attenuation placement.
- **Runtime input inspection:** the reflective updater attenuates time-of-day
  specular RGB with cloud intensity and takes reflection alpha from per-view
  RoadDampness. The weather update and reflection-capture gates establish that
  settled dry roads suppress planar reflection; recent rain retains wetness.
  These are static findings, not observations of a running gameplay state.
- **Unresolved:** original reflection-scene production and refresh, original
  shadow filtering and higher techniques, runtime sampler/cull/blend state,
  animated UVs, nonuniform model transforms and matched gameplay validation.
