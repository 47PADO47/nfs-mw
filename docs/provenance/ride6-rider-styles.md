# RIDE 6 rider styles and reverse: provenance

What the styles and reverse work read, and from where. Spec: [ride6-rider-styles](../specs/ride6-rider-styles.md). Code:
`mods/ride6-bikes/src/rider/{graph_sets,graph_reverse,style}.rs`, `mods/ride6-bikes/src/physics/reverse.rs`,
`libs/blackbox-vehicle/src/bike/reverse.rs` (generic law, takes its numbers as parameters),
`crates/nfsmw/src/scenes/world/drive/rider/{style,reverse}.rs`.

## Sources

| Source | What was taken | Status |
|---|---|---|
| The animation blueprint of the user's install, as pseudo-code printed by CUE4Parse's Blueprint decompiler | The function `SET Animations`: the clip variables assigned for each `BikeType` (0 to 5) and `RidingStyle` (0 to 2); the function `Retro` and its one call (`0.8, 1.567, -1, -1, -20, 1, 4`); the blueprint variables of the retro state | Facts about the user's data asset (Kismet bytecode), not native code. The branch structure was followed by a scratch script; only the final clip names per type are kept (`graph_sets.rs`) |
| `RiderAnimationCorrection_DT`, `RidingStyle_DT`, `RIDING_STYLE` (the user's data tables, dumped as JSON) | The per-vehicle `EBikeTypeOverride`; the riding style rows (no clip names) | Data facts; not stored in the repository |
| The vehicles' physics assets (the physics dump) | The input handler class each vehicle runs; `BackwardForce`, `BackwardMaxSpeed` of the eight input handler assets (200000 and 125) | Data facts |
| The camera container folders of the install (`data/cameras/Containers/*`) | The camera kind of each vehicle, compared with the handler | Names only |
| The Dumper-7 SDK dump | Layout of `UBikeInputHandler` (`BackwardForce` at 0x634, `BackwardMaxSpeed` 0x638, `ReverseTimeThreshold` 0x63c, `RequestingReverse` 0x266, `ReverseTimer` 0x294), `EBikeTypeOverride` | Names and offsets only |
| RIDE 6 native binary (`ride6-Win64-Shipping.exe`, build `5.6.1-1697004`), headless Ghidra on a private copy of the project | The reverse logic of `UBikeInputHandler`: the request (`0x50d8240`, the pressed flags set at `0x50d7fb0`), the force and the brake release (`0x50d8620`, `0x50d8a80`), the force on the body (`0x50ff580`), `HasReverse` (`0x50d96a0`), the constructor's defaults (`0x50bbf80`: 100000, 250, 0.5), the constants 0.1 (pressed), 100 (speed, cm/s), 0.036 (cm/s to km/h) | Read to understand the behaviour; no decompiled code is stored. The spec states it in words and one equation |

## What is a choice or an inference

See section 6 of the spec. The ones to remember: the bike type is the override, else the input handler (an inference that the engine's
variable follows the handler), riding style 0 everywhere, the rider keeps both feet down in the walk, the fade of the walk, the
single brake input as the reverse request, and `HasReverse` read as "more than two forward gears".

## Exported for this

The clips of the race, naked, scooter and enduro sets, the motard's reverse clips and the ACL ones among them (the standing wheelie
and the drift poses of each set, the enduro lean and poses, the enduro reverse) were exported with the user's scratch tool
`R6ToolRider` (`anims` and `acl` modes) into the user's own export folder, as before. Nothing is committed.
