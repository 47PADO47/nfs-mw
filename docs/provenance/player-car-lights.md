# Player car light provenance

Spec: [player-car-lights](../specs/player-car-lights.md).

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
