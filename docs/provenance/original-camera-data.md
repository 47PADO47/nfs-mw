# Original gameplay camera data and projection provenance

- **Primary evidence:** the user's installed PC executable, identity checked
  through Ghidra program metadata before inspection. The camera-anchor POV
  getter's seven branches call `ecar` attribute-reference getters, then read
  all eight paired numerical/boolean profile fields. Raw instructions were
  checked where x87 dependencies were absent from decompiler output.
- **Selection evidence:** the PC player-camera-selectability query resolves the
  requested model camera reference and reads `SELECTABLE` at the same viewport
  index. Both the gameplay cycle and frontend camera selector loop over the
  seven indices until this query accepts the candidate. Internal profiles can
  still be requested directly by other camera controllers.
- **Projection evidence:** the PC view-projection producer independently builds
  horizontal and vertical scales. Its viewport rectangle reads, main-view-ID
  branches, half-angle conversions and stock `1.15` constant were inspected.
  Isolated unchanged instruction execution corroborates the mathematical
  contract with synthetic inputs at three FOVs and three resolutions.
  A separate native cubic output span establishes the final tangent/inverse-angle
  round trip. Its inverse-angle endpoints were independently regenerated from
  the mathematical arctangent equation, and synthetic replay verifies final
  angles and the composed projection path without copying original table bytes.
- **Target evidence:** the native cubic target producer distinguishes bumper
  and hood from five outside POVs. Five synthetic unchanged-instruction cases
  check its pitch, lag and target-forward calculations.
- **Playable-reference projection:** the selected reference enables the official
  MIT-licensed ThirteenAG/WidescreenFixesPack FOV correction with Scaling0 while
  disabling its separate rotation module. Its `Rendering.ixx` gameplay-view
  branch establishes the aspect numerator, vertical half-angle factor and
  vertical numerator recorded in the specification. Source pinned to commit
  `708f7528fb8b54c79d4ac3c6071ea7ccc01ae162`:
  https://github.com/ThirteenAG/WidescreenFixesPack/blob/708f7528fb8b54c79d4ac3c6071ea7ccc01ae162/source/NFSMostWanted.WidescreenFix/Rendering.ixx
  Independently written Rust adapts those equations to the existing renderer.
  Nine source-described hook experiments execute the original projection span
  with synthetic FOV/aspect inputs and establish the composed scale vectors.
  They do not claim a live trace of the installed plugin. The unmodified PC
  projection and cubic finalization keep their independent regression groups.
- **Reference declarations:** the private local `nfsmw-reference` headers for
  `ecar`, `camerainfo`, `POV`, `CameraAnchor` and viewport enums identify field
  names and structure purposes. These are behavioral references, not source
  copied into the implementation. PC addresses and instruction evidence were
  used to resolve platform-layout differences.
- **Installed data checks:** ignored tests read camera profiles and their actual
  `ecar` references from the user's own install. No data, binary, decompilation
  text or original lookup tables are distributed.
- **Implementation:** independently written Rust readers follow existing
  `blackbox-attrib` reference-resolution APIs. The recovery fallback for an
  unknown car is explicitly a host policy. Native temporal/visibility behavior
  is not established by these profile-reader tests alone.
