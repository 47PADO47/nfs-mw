# Spatial parameter-map reader and car environment context

- Date: 2026-10-14.
- Specification: [spatial-parameter-maps.md](../specs/spatial-parameter-maps.md).
- Implementation: `libs/blackbox-paramap`, game audio `environment_context`.
- Source read: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw),
  CC0-1.0, `ParameterMaps`, `TrackPath`, `SFXCTL_Tunnel`; reference only.
- Independently inspected the matching PC executable's quad8/quad16 lookup,
  city environment update, enclosure lookup and zone selection.
- Wrote the specification before implementing the reader/provider. No source,
  executable, installed parameter sets or extracted quad nodes were copied.
- Validation includes synthetic malformed-tree and quadrant tests, and
  install-backed spatial samples independently executed through original PC
  lookup instructions using isolated synthetic controller memory. Enclosure
  elevation vectors also execute the complete native polygon lookup without
  stubbing its world-query or containment calls.
- A complete native reflection update replay, including original ramp and
  Q15 conversion calls, independently confirms fade-out endpoint at tick18
  and patch-ready at tick19 for a300ms transition at60Hz.
- Scope limits: decoding/context lookup does not implement the FXEDIT wet
  kernels, original occlusion tests, tunnel drive-by state or mutable gameplay
  zone visits beyond values provided by the world. Query validation does not
  establish audible output equivalence.
