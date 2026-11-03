# Provenance: RIDE 6 rider, round 6 (the choices turned into facts)

Spec: [ride6-rider-round6](../specs/ride6-rider-round6.md). Earlier rounds: [ride6-physics](ride6-physics.md),
[ride6-rider](ride6-rider.md). Addresses are image-relative (image base `0x140000000`) of `ride6-Win64-Shipping.exe`, build
`5.6.1-1697004`.

## What was used

- The user's own install, read-only. The shipping executable was decompiled **headlessly in a private copy of the analysed
  Ghidra project** (scratch folder outside the repository); the decompiler output was read, never stored in the repository.
  Constants were read from the executable's data sections and instruction bytes were checked with a disassembler where the
  decompiler hid a register argument (the arguments of the `approach` call).
- The Dumper-7 SDK dump for struct layouts and the table of native function addresses.
- The animation blueprint of the rider (`RiderGP_racing_Master_animBlueprint`), exported as pseudo-code by CUE4Parse's
  Blueprint decompiler: data of the user's install, not native code. The numbers of its functions are quoted as data.
- The physics data assets of the KTM 450 SMR (`Motard_InputHandler`, `Base_RiderHelp`, `Motard_RiderHelp`, `RiderAirDrag`,
  `BaseRiderDragScaleCurve`).

## Functions read

| Topic | Functions | How verified |
|---|---|---|
| Rider drag force: the handler tick that builds it and calls the rider body | `0x50dff70` | field offsets matched to the SDK (`DragCurve` at handler `+0x1108`, `DragScaleCurve` `+0x1190`, `DragScale` `+0x1218`, force `+0x13c8`); the curves' arguments are `Actions.WeightLongitudinal` (vehicle `+0x17f4`) and `Actions.TuckIn` (`+0x1800`) by the layout of `FVehicleActions`; the constant `-0.01` at `0x1490ddc50` |
| Slipstream factor of the drag | `0x5123ee0` | `clamp01(Slipstream * SlipstreamScale)`, the vehicle drag's own |
| Curve evaluation with a default for an empty curve | `0x3ce3160` | the generic rich-curve evaluation; the defaults 0 (`DragCurve`) and 1 (`DragScaleCurve`) are the call arguments |
| Rider body and its spring error | `0x50f5900` (earlier round), `0x50cc510` | the arc formula read line by line; centre = the chassis world transform's translation (`vehicle +0x1610`) |
| `approach` | `0x512cb60`, call in `0x5100f80` | disassembly of the call: first argument the current angle, second the unfiltered target, third `dt` times the filter timescale |
| Rider help tick, tuck action | `0x512b870` | the thresholds at `0x3b0..0x3c0` of the element (SDK offsets); `0.5` at `0x148faa724` |
| Tuck level | `0x5125190` | fields `0x30dc`, `0x30e0`, `0x311c` (SDK); defaults 2 and 2 at `0x50fbfa0` |
| Rider component interface (`TuckedIn`, weights) | `0x5b48910`, `0x5b489b0`, `0x5b474d0`, `0x5a13c08` | tiny accessors of `IIgnitionMotorbikeRider` |
| Animation instance update | `0x5b155c0`, `0x5b14bf0`, `0x5b1fb60`, `0x5b254f0`, `0x5b5e6f0` | the fields written, matched to the SDK layout of `IgnitionRiderAnimInstance` / `IgnitionBikeInfo` |
| Bike info: `Lean`, `Elevation`, `RiderLean` | `0x5d62540`, `0x50d6980` | `acos(right.z) - pi/2` over `MaxLean`, `forward.z`; the vectors are told apart by the unit vectors `(0,1,0)` and `(1,0,0)` at `0x148f950b0`, `0x148fbb060` |
| Lean angle of the bike | `0x50f4b10` | the roll of the chassis rotator; Unreal's roll sign |
| Input table defaults | `0x50a5f20` | stores at `+0x1450 ...`: lateral weight channel linearity 1, deprecated flag 1, rates 10000 |
| Script operators | `0x4f54d60` (Interpolate), `0x4f54ad0` (Constant), `0x4f54800` (Flashback), `0x4f54690` (Decay), `0x4f547e0` (Defuzzify), `0x4f55200` / `0x4f551f0` / `0x4f55220` / `0x4f551d0` / `0x4f55230` (Sum, Max, Min, Not, Product) | read line by line; found through the operator objects' vtables (`0x149d81358`, `0x149d814a8`, `0x149d81198`, ...) |

## Which class each fact belongs to

The KTM's input handler is `Motard_InputHandler_C` -> `Default_InputHandler_C`, native parent `UMotorbikeInputHandler`
(constructor `0x50fbfa0`; the motocross `UMXBikeInputHandler` has other offsets and was not used). The vtables of
`UBikeInputHandler` and `UMotorbikeInputHandler` were compared slot by slot (the Motorbike one overrides slots 0, 91, 93, 101,
110, 111, 117, 118, 135 to 137, 143, 146, 149, 155 and 156).

| Fact | Class and function | Shared with the Motorbike handler? |
|---|---|---|
| Rider drag force (`Rider` at `+0xF00`, curves inside `FRiderComponent`), `MaxLeanAngle` (`+0x3a8`) | `UBikeInputHandler` slot 95, `0x50dff70` | yes, same slot entry in `UMotorbikeInputHandler` |
| Target lean (`+0xc18`) from the stick, rider-lean normalisation `0x50d6980` | `UBikeInputHandler` slot 128 `0x50d8f10`, helper `0x50d6980` | yes (slot 128 is not overridden) |
| Normalised target lean (`+0xc3c`) | `UBikeInputHandler` slot 118 `0x50d7520` | the Motorbike override `0x5124d90` calls it first, then the stability and block parts |
| Tuck level (`TuckInRaiseRate +0x30dc`, `TuckInFallRate +0x30e0`, `TuckInLevel +0x311c`, defaults 2 and 2) | `UMotorbikeInputHandler` slot 117 `0x5125190`, constructor `0x50fbfa0`; the fields are `UMotorbikeInputHandler`'s | Motorbike only (not in the base or the MX class) |
| Tuck action, the auto-tuck thresholds, `WeightLongitudinalInputDeadzone` | `URiderHelpVehicleElement` tick `0x512b870`, constructor `0x50fda20` | separate element, listed in the KTM's vehicle elements |
| `approach` | `UWheelieControl`, `0x512cb60` called from `0x5100f80` | separate element |
| Arc error of the rider spring | `FRiderComponent`, `0x50cc510` (called from `0x50f5900`) | struct-level, no handler |
| Input table defaults (lateral weight rate 10000) | `FVehicleData`/`UIgnitionVehiclePhysicsSetup` constructor `0x50a5f20` | not a handler |
| `Actions.TuckIn`, `.WeightLongitudinal` offsets | `FVehicleActions` inside `UPhysicsVehicle` (vehicle `+0x17d8`) | not a handler |
| `TuckedIn` input of the animation | `IIgnitionMotorbikeRider` of `UIgnitionMotorbikeRiderComponent`, `0x5b48910` (reads `UMotorbikeInputHandler +0x311c`) | Motorbike component |
| `BikeLean`, `Elevation` | `UIgnitionBikeComponent`, `0x5d62540` | bike component |

## Not read, and why

- The script's `FuzzifyRamp` evaluator, `LoadFuzzy` (assumed a clamp to 0..1; the blueprint only feeds it values of that range)
  and `DefineFloatRule` without `Ex`: the semantics used are the ones of the readable neighbours.
- The device-to-weight mapping (`UIgnitionPlayerCustomizableInputComponent::EvaluateRiderWeight*`): virtual calls into
  Blueprint data, no native code to read.
- The constructor default of `TuckInWeightLongitudinalRange` and `bAutomaticLateralWeightControl` (the element constructor
  `0x50fda20` does not store them; zero-initialised is the reading).
- The head inertia multipliers' unit and the blueprint graph's pin bindings (cooked binary data).
- The bike component's `MaxLean` (`+0xf98`) and the rider asset's `BikeLeanWeight`.

Nothing here was tested in the running game (RIDE 6 must not be launched).
