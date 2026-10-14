# Speedbreaker visual presentation provenance

- Spec: [speedbreaker-visuals.md](../specs/speedbreaker-visuals.md).
- Sources: original PC visual-target update, cubic evaluator, visual heat update,
  platform effect binding and platform vignette-texture lookup; runtime
  `visuallookeffect` data and global texture packs. Private disassembly and
  twenty unchanged-code replay vectors remain outside publishable source.
- [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
  `VisualTreatment` provides cross-platform structure and field names; the
  original PC instructions independently confirm the world-time target,
  cubic ordinates, radial scale clamp and separate vignette parameter.
- Original PC shader bytecode was recovered independently from the executable
  and the initialized visual-effect object in the running original game. The
  matching high-quality pixel shader uses mask green as additive vignette and
  blue as final gain; red and alpha do not control it. The vertex shader forwards
  coordinates without the invented four-tap displacement. Private bytecode,
  disassembly and object identities stay outside the publishable checkout. The
  prior alpha-darkening and centered radial kernel have been removed.
- The retained pass implements the shader's additive contribution after the
  existing base scene. Ordinary grading, unrelated brightness pulses and other
  blur passes remain separately scoped; matching gameplay sequences are still
  required to claim full image equivalence.
- Profiles and textures are read from the user's install. No original game
  data, code, binaries or shader bytecode are included in the implementation.
