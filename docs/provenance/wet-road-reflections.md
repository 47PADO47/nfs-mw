# Wet-road reflection producer

- **Specification:** [wet-road-reflections](../specs/wet-road-reflections.md),
  written before capture implementation.
- **Date:** 2026-10-16.
- **Source:** matched retail PC executable, read-only Ghidra inspection of road
  texture/depth allocation and release, view initialization/projection, scenery
  culling/submission and authored model-matrix changes, target dispatch/clearing,
  material-family substitution, road blur configuration, weather and the render
  caller. Embedded material programs and their existing independent synthetic
  fixtures are documented in [world-reflect-materials](world-reflect-materials.md).
- **Identity:** executable SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`,
  confirmed against the active Ghidra program before address-based inspection.
- **Additional reference:**
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw/tree/80993f35224c61d2f1b135beba76c6f8234c7940),
  CC0-1.0 decompiled shared source, `World/Scenery.cpp`, `World/Rain.hpp`,
  `World/rain.cpp`, `World/CarRender.cpp` and `World/CarRenderConn.cpp`. Read to
  interpret fields and compare behavior, not copied or translated. PC addresses
  and offsets were checked independently rather than importing another
  platform's layouts.
- **Ownership:** original binaries, assets, shader bytecode, decompiler output
  and detailed address evidence remain private. The public implementation uses
  newly written code and assets read from the user's install at runtime.
- **Fixed-function capture verification:** the matched embedded ID 30 effect was
  applied using the original D3D9 effect runtime. Both programs are disabled;
  stage-zero RGB/alpha operations and lighting/fog state were queried directly.
  Five independent asymmetric texture/vertex fixtures measured the predicted
  doubled RGB and ordinary alpha products, including both alpha-zero cases.
  Their private inputs/results stay outside source control. The public native
  tests use newly authored synthetic geometry and independently stated expected
  pixels; no original bytecode is embedded in the renderer.
- **Limits:** producer findings are static, not a captured original-game run.
  Source recurrence is render-coupled; a reference-cadence high-FPS adaptation
  must be labeled as such. Complete car/effect reflection geometry, blur effect
  sampler states, preculling and modded-reference behavior are not proved here.
  A shared estimated mirror plane would be an approximation and must not be
  described as the original authored-geometry mechanism.
