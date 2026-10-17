# Nitrous exhaust and taillight trail provenance

Specification: [nitrous-light-trails](../specs/nitrous-light-trails.md).
Research date: 2026-10-17. Specification written before implementation; no
implementation, Cargo build or original-game runtime validation was performed
by this research pass.

## Sources and identity

- PC `speed.exe`, SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`,
  MD5 `c0516b485065fabdd69579816b5df763`. Ghidra `get_program_info` confirmed
  the PE/x86 target before analysis. Proprietary original, read-only behavioral
  reference. No executable, assets or decompiler output is distributed.
- [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw/tree/80993f35224c61d2f1b135beba76c6f8234c7940),
  CC0 decompilation, `World/VehicleRenderConn.cpp`, `CarRender.cpp`,
  `CarRenderConn.cpp`. Used for names and independent-platform cross-checks,
  not copied implementation. Existing NOTICE credits this project.
- [ThirteenAG/WidescreenFixesPack NOS correction](https://github.com/ThirteenAG/WidescreenFixesPack/blob/708f7528fb8b54c79d4ac3c6071ea7ccc01ae162/source/NFSMostWanted.WidescreenFix/NOSTrailFix.ixx),
  MIT, used to understand frame-rate normalization and distinguish correction
  behavior from the stock routine. Existing vehicle-effect provenance credits
  this source; source code was not copied.
- Prior [exhaust research](exhaust-flames.md) and
  [player car light research](player-car-lights.md) establish installed
  profiles, particle definitions and flare arithmetic. Those measurements were
  not independently rerun in this focused pass.

## PC findings checked directly

`VehicleRenderConn::RenderFlares` at `0x744e20`: visibility and camera-owner
gates, normal flare call, non-reflection/player-view ring update, strict
speed-above-0.2 and NOS-state gate, three directed ring edges, translation-only
interpolation, stored edge-start matrices, and brake-only extra calls. Program
memory at `0x903694` initializes the division count to 8; the speed constant at
`0x895074` contains IEEE-754 0.2. This is 24 extra sprite sets, including the
newest-to-oldest closing edge, rather than a newly inferred ribbon emitter.

`CarRenderInfo::RenderFlaresOnCar` at `0x742950`: the extra-call flags exclude
all flare types except 1; traffic brakes become type 2; forced-light state 8
does not force braking; ordinary brake intensities and light flags survive;
each copy routes through the normal installed-profile flare renderer. Marker
role names are cross-checked against the CC0 constructor reference and prior
PC car-light research. No lifetime fade or unique trail texture is introduced
by these calls.

The bounded exhaust-frame follow-up inspected `CarRenderConn::UpdateEffects`
at `0x7561c0`, its `VehicleRenderConn::Effect::Update` callee at `0x744a50`,
and group-particle dispatch at `0x507770`. The persistent update multiplies
the stored local frame by the car render frame and sets it directly on the
emitter group; particle draw dispatch supplies identity. This is the actual
continuous NOS caller chain and is distinct from `World_OneShotEffect`
at `0x779460`, discussed speculatively in the older exhaust research. The
reference `CarRenderConn` loader independently constructs pipe effects from
exhaust marker matrices. No new transform correction is justified by the
older one-shot-handler trace.

## Implementation and remaining verification

The Rust implementation was written independently from the specification on
2026-10-17 in `scenes/world/tail_light_trails.rs` and the existing car flare /
exhaust paths. History advances after shared physics ticks and is cleared on
placement, model replacement and leaving the driving view. The main scene draws
brake-only copies through the existing installed-profile appearance calculation;
auxiliary reflection rendering does not invoke this path.
The fixed-60-Hz sampling contract, safe initial history, reset/camera lifecycle,
resource bounds and supported-player-only scope are host choices, separately
identified from native behavior. Preserving the existing 4x flame alpha gain
does not establish original brightness. Nitrous exhaust reuses the documented
installed group and existing emitter limitations.

The native game was not launched or modified. No frame capture, driving route
or side-by-side comparison was made. The static PC findings establish routing
and arithmetic, not pixel parity. Before publication, record the actual Rust
checks/captures separately and retain the limitations in the specification.
