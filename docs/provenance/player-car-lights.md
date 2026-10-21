# Player car light provenance

Spec: [player-car-lights](../specs/player-car-lights.md).

## Traffic adapter audit (2026-10-21)

The identity-matched PC `RenderFlaresOnCar` inspection at `0x742950`
reconfirmed usage-type 2's brake-flare type change. The model loader already
retained that installed classification, but the world AI adapter bypassed the
authored renderer: it drew generic circles and procedural beam wedges, omitted
trailer flares, and left brake surface selection at its default released state.
The adapter now shares the installed directional/range-limited vehicle flare
path, loads `brakelight_traffic_inner` separately from the racer brake profile,
and selects cached brake surfaces from actual physical controls. Trailer brake
lights therefore clear after release instead of retaining tractor input.

Authored cop particles and their existing emergency-state gate remain in use;
the extra fixed-period generic siren circles no longer enter the world layer.
Optional steering indicators retain their explicitly separate rewrite setting.
Normal traffic beam wedges are removed consistently with the already-audited
reflection-only native footprint caller. Traffic flares also reach the world
effect layer when there is no placed player drive.

Installed CPU regressions cover traffic/police model upload, authored marker
output, actual braking, cached mesh selection and distance rejection. Stock
`TRAILERB` supplies no tail-flare markers: its regression first requires empty
native flare output, then adds an explicitly synthetic marker to exercise the
real trailer adapter and brake clearing before/after uncoupling. It does not
claim that stock trailers should acquire inferred flare positions.
These are adapter/data checks, not a live original-game
optical comparison. The follow-up below resolves destroyed-lamp admission and
narrows the occlusion/night claims. Tractor-to-trailer reverse propagation
remains unverified; no new guessed blink rates or marker positions were added.

## Lamp-damage, night and occlusion follow-up

The same PC identity was checked again before inspecting `0x742950`,
`0x737E30`, `0x505380`, `0x744E20`, `0x6BE8F0`, `0x6BE750` and `0x6D1A40`.
The PC flare consumer applies independent broken bits after light-on admission;
the brake texture consumer also rejects broken brake flags. The CC0
`Physics/Behaviors/{DrawCar,DamageVehicle,DamageRacer}` and `World/VehicleFX`
references establish the producer/interface and marker mapping: explicit
destruction reports all mapped lamps broken, whereas cosmetic zone levels do
not directly set that mask. The implementation therefore propagates explicit
destruction to ordinary and historical flare copies and clears it on repair.
Individual `DamageLight` event producers and per-side broken emissive texture
selection remain open; no invented panel-damage threshold is introduced.
The bounded `0x737E30` recheck keeps head replacements independent of the
broken mask; only the three brake replacements test their respective bits.
The existing shared mesh cache therefore selects brake OFF when all three
brake bits are broken while preserving the existing head replacement state.
This does not claim partial-side brake surface selection.

The inspected PC base-headlight branch has no hour threshold. Its suppression
global belongs to the cinematic `INIS` singleton, cross-checked against the
reference call; explicit on flags can restore the head base. A guessed dusk
cutoff would mix the custom day-clock extension with original behavior.

The supposed occlusion candidate `0x6BE8F0` is a six-plane point-frustum test.
The inspected normal flare chain submits textured polygons, without a collision
world ray or asynchronous occlusion query. The current renderer already uses
scene reverse-Z depth testing and disables depth writes for textured flares.
Existing `textured_particles_vulkan` / `textured_particles_dx12` GPU tests
explicitly verify opaque-depth occlusion, no depth writes, authored additive
alpha and fog. This audit does not justify a new CPU visibility ray. Exact
original screen rotation, clipping and native render-state/pixel comparisons
remain separate optical limits.

Read-only analysis of the PC `speed.exe`, SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`:
`CreateCarLightFlares` (`0x74df10`), `RenderFlaresOnCar` (`0x742950`),
`UpdateLightFlareParameters` (`0x502640`), `eRenderLightFlare` (`0x505380`),
and `RenderTextureHeadlights` (`0x738180`). Program identity was checked first.
The type table distinguishes racer and traffic brake profiles. Installed
AttribSys records and texture packs independently confirm the profile fields
and texture names. No original data or decompiler output is distributed.

The later caller audit also checked `VehicleRenderConn::RenderFlares`
(`0x744e20`), `RenderVehicleFlares` (`0x750b00`) and `___eDisplayFrame`
(`0x6de300`). The normal player scene passes reflection zero; only the
reflection-marked auxiliary call can invoke the textured footprint. The former
normal-scene projection was incorrect and is removed. This is static caller
evidence, not a live observation of the original auxiliary render output.

The focused lens audit checked the same flare renderer's arithmetic and
texture blend setup: sizes are quad half-extents and additive textures use
source-alpha/one blending. Its projection setup reads current window size;
the previous fixed reference-size interpretation was incorrect. The PC
`UpdateLightStateTextures` (`0x737e30`) and `GetUsedCarTextureInfo` (`0x751860`)
also establish headlight-glass ON replacement without the other-platform
reference's front-window overwrite. Original surface-shader bloom and exact
flare rotation/occlusion still require separate optical validation.

The CC0 dbalatoni13/nfsmw reference's `World/CarRender.cpp`,
`Ecstasy/Ecstasy.hpp`, and generated `light_flares_cg` field definitions supplied
names and cross-checks. Existing NOTICE credits that reference. The implementation
is independently written from the behavioral spec. Runtime texture replacement
is tested separately from optical parity; a synthetic render cannot establish
pixel-exact agreement with the original game.
