# World atmosphere provenance

- Date: 2026-10-15.
- Specification: [world-atmosphere.md](../specs/world-atmosphere.md), written
  before the implementation changes.
- Sources: the retail PC executable inspected with Ghidra; generated
  `timeofdaylighting` names/layout and `SkyRender`, `rain`, `ParameterMaps`
  in [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
  reference only. The implementation is written from the specification.
- Identity: baseline PC SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- Lighting blend: preset preparation at `0x00768EE0`, mixing at
  `0x00769190`, selection and dusk factor at `0x00769580`.
  The last function's final store forces ambient alpha to 4.0.
- Normal-mapped world material lighting uses the blended specular RGB from
  the lighting object's colour block. PC material setup `0x006D6AA0` reads
  the exponent at `0x008FADC0` (initial float 11.0); its other checked use is
  `0x006D71F0`. Per-group setter `0x006C76E0` is gated to shader IDs 4/2.
  See [world-normal-materials.md](../specs/world-normal-materials.md) for
  the shader/material evidence. Addresses are provenance, not runtime dependencies.
- Fog coefficient setup: `0x006D6CF0`; shader expressions require separate
  resource-disassembly checks, not inference from constant names.
- Clouds: initializer `0x0088C5D0` associates the Clouds string with the
  accessor used by target query `0x00753AC0`. The spatial reader is
  `0x0074B5C0`, with quadrant selection `0x0073D560` / `0x0073D660`.
- Precipitation update `0x00758100` supplies cloud intensity to the renderer:
  desired rain takes priority, cloud easing divisor is ten seconds.
- Original functions were read only; original binaries, mod files and
  decompiler output are not included in the repository.
- Sky selection: factory `0x006D6000` populates the effect table;
  its ID19 entry is the programmable skybox effect. `0x006D1CF0` selects
  `SKYDOME_XENON` when that effect's programmable-technique flag is enabled;
  otherwise it selects `SKYDOME` and `0x006D1220` substitutes ID21 legacy
  skybox. The ID19 declaration at `0x008FA610`, constructed by `0x006D5C10`,
  has 44-byte vertices with UV1 at byte36. Installed shared X0 geometry
  corroborates the Xenon dome's 141 vertices, independent UV1 and authored
  `SKY_NEXGEN_CLOUDS_A` diffuse. The ordinary dome has 36-byte vertices.
  This corrects the earlier assumption that the ordinary model supplied the
  three programmable passes. `0x006DB310` starts from identity, centers on
  the active camera and uses distinct main/rear/environment scales; its
  `0x006C0D50` texture replacement is gated to the legacy branch.
- PC sky state `0x006C60F0`, corroborated with all FPU stores: named
  miscellaneous texture bindings, cloud-weight polynomial/clamps and
  .02-per-second scroll. Decompiler output loses two weight components;
  instruction-level stores establish these components instead.
- FPU instructions at `0x0073C980` return constant one second in free roam;
  `0x0073CA50` adds an X-dependent angular term. The earlier specification's
  claim that this implies a neutral cloud gate is corrected here.
- High-FPS exponential easing and input validation are explicit remake
  adaptations. Free-roam spatial weather gate is the initial scope;
  scripted weather, full sky compositing and original-game visual parity
  remain separately unverified until recorded captures confirm them.

## Environment comparison controls (2026-10-16)

- The specification's comparison-control section was written before these edits.
- Reused the matched PC sun integrator at `0x00769360` and phase limits recorded
  in [time-of-day.md](../specs/time-of-day.md): free-roam rate 1/1200 per second,
  reversal at .05/.95. The custom hour clock is retained as an explicit remake
  option; fixed phase and cloud presets are comparison tools, not retail menus.
- Reconfirmed Ghidra program identity against the baseline SHA-256 above.
  Precipitation update `0x00758100` and cloud target `0x00753AC0` establish that
  cloud amount, desired rain and road dampness are different inputs. Positive
  desired rain precedes the spatial cloud target, before any remake override.
- The preferred playable reference's installed configuration was read only.
  It enables Xbox 360 shader enhancements and persistent planar captures;
  those options must not be mistaken for stock PC wetness or a complete night
  pipeline. No mod assets or install edits are needed by the comparison tools.
- CPU regressions cover automatic/forced cloud transitions, zero-time holds,
  original-cycle reversal and equivalent states at 30/60/144/360 Hz, explicit
  phase selection, and paused forwarding. Rendered reference matching remains
  a separate validation requirement.
