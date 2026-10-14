# Cached body terrain reference

- **Spec:** [vehicle-ground-reference](../specs/vehicle-ground-reference.md).
- **Sources:** original PC 1.3 executable (proprietary behavior reference),
  inspected locally on 2026-10-13; [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw),
  CC0 files listed in the spec. No code copied.
- **Identity:** SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- **Implemented:** generic collision candidate/region/face cache from the
  separately written specification. Tuning and group state are caller inputs.
  Independent wheel-face histories consume the body's shared candidate list;
  the adapter selects original per-driver masks. The sleeping body uses the
  global point/instance path, including full rotated instance transforms.
- **Evidence:** PC triangle-list builder, instance/grid ordering, face selection,
  cache-preservation and normal-update instructions; physics-object constructor
  confirms the 0.025 terrain tolerance.
- **Original numerical validation:** ten complete cache/update instruction
  replay vectors and fourteen direct containment vectors, with synthetic faces;
  corresponding public fixtures cover reuse, miss retention, forced selection,
  continuation order and first-edge winding boundaries.
  Ten original wheel-position/tolerance/cache vectors, seven original driver
  mask vectors and eleven original global upright/tilted query vectors extend
  the validation. The wheel replay stops after the completed world-position
  query; subsequent surface-object construction is outside that replay.
- **Installed validation fixture:** 64 deterministic resident ground queries;
  requires explicit installation input and at least 48 valid selected faces.
  This is a query smoke check, not an original gameplay comparison.
- **Bounds:** awake cached and sleeping/global paths have isolated numerical
  coverage. The original enabled-group table has no scenario/event producer in
  the current game adapter; its group-zero-only policy matches the original
  initial/unload state, established by table bytes and group-state routines.
  Live animated collision transforms and complete
  streaming/group behavior still need comparison.
