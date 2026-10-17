# Original graphics detail controls

- **Specification:** [original-graphics-detail](../specs/original-graphics-detail.md),
  written before any implementation of these original detail controls.
- **Date:** 2026-10-16.
- **Source:** matched retail PC executable, read-only Ghidra inspection of main
  and advanced Video option construction/callbacks, registry read/write, preset
  application, capability clamps, texture upload, scenery selection/submission,
  reflection dispatch and sampler setup. Proprietary artifacts are behavioral
  references only. No code, decompiler output or assets are distributed.
- **Identity:** SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`,
  confirmed with the active program identity before address-based queries.
- **Additional reference:**
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw/tree/80993f35224c61d2f1b135beba76c6f8234c7940),
  CC0-1.0 decompiled shared game code, `World/CarRender.cpp` and
  `World/CarInfo.cpp`, read for the distinction between allowed geometry range
  and projected-size body/tire selection. No code copied or translated.
- **Corroboration:** independent call paths agree on texture-filter choice;
  original option UI, writer and consumers distinguish master/world/car detail.
  Raw instructions confirm the world-detail helper returns integer two despite
  the decompiler's inferred boolean type. Existing source shadow/reflection
  specifications provide the separately researched target dimensions.
- **Related pending work:**
  [PR 121](https://github.com/47PADO47/nfs-mw/pull/121), inspected at head
  `9f13b32229e0723b1c6c1419cc0fdb06888ce986`, contains reflection sizing,
  distance, LOD, content and brightness controls. Its source implementation,
  rather than every historical proposal in its spec, establishes overlap.
- **Limits:** static evidence only in this audit. No original process, hook,
  runtime setting change or GPU comparison was performed. The preferred
  enhanced reference's exact effective option values are not measured. Car LOD
  usage mapping, some reflection content and scenery projection scale remain
  unresolved. Existing custom quality levels must not be advertised as an exact
  translation of original slider positions.
