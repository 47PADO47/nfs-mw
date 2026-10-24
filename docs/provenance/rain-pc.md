# PC rain reconstruction

Date: 2026-10-23. Specification: [rain-pc.md](../specs/rain-pc.md).

Read-only original PC executable analysis supplied counts, atlas layout, initialization,
camera-water shader behavior and sound-module parameters. The proprietary executable
and extracted shader remain outside the repository. The CC0
[nfsmw reference](https://github.com/dbalatoni13/nfsmw) supplied names; no decompiled
source is included or translated line by line.

PC evidence: primary initialization 0x73c6b0, main update 0x757770, weather update
0x758100, screen initialization 0x73cd30, screen update 0x73cdb0, screen draw 0x6d4620,
audio initialization 0x4dce50, release 0x4dcf30, activation 0x4dd070, mixer input
0x4dd180, thunder stream 0x4dcf90. The `IDI_RAIN_DROP_FX` resource supplied the
green/blue displacement and 1.2 scene-color multiplier.

Validation and remaining parity limits are recorded as implementation proceeds.
No live original-game visual/audio comparison has been completed for this revision.


A 2026-10-23 modded-reference follow-up matched the installed particle program
bytecode at actual draw calls and queried its brightness constant as 2.0.
Static shader inspection established multiplication of texture times vertex
RGBA by that scalar. Proprietary effects and detailed live traces remain private.
This supports the explicit tyre-water compatibility response in the specification,
not a claim of stock-PC parity or proof of the complete Xbox-pack modification.

## Droplet size enhancement [community]

Widescreen Fix's public configuration describes `RainDropletsScale` as an
on-screen droplet size adjustment and supplies 0.5. The remake exposes an
independent size multiplier with the same default. No plugin code was ported.
Reference: <https://github.com/ThirteenAG/WidescreenFixesPack/blob/master/data/NFSMostWanted.WidescreenFix/scripts/NFSMostWanted.WidescreenFix.ini>.
The reconstructed PC size remains available at 1.0.
