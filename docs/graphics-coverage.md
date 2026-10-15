# Graphics implementation coverage

This ledger describes current rendering, not complete original-game equivalence.
Original assets and original behavior are separate claims. The typed inventory
in `settings/appearance/registry.rs` supplies F8 Diagnostics.

| Feature | Current implementation | Remaining original parity |
| --- | --- | --- |
| Models/textures | Installed original assets | Full material-specific rendering |
| World materials | Simplified prelit/lit shader | Original effect families, normal/detail maps, blending |
| Car materials | Original parameters, custom rig | Exact light rigs and remaining paint/glass behavior |
| Authored overlays | Original SHD scenery meshes | These are not complete dynamic shadows |
| Dynamic/contact shadows | Custom cascades, PCSS and screen-space detail | Original shadow behavior |
| Visual Treatment | Original midday profile; optional custom strength | Tunnel exposure, pursuit/cinematic look selection/transitions |
| Bloom / ACES | Custom enhancements | Original Over Bright |
| Car reflections | Original-style cubemap, custom budgets | Exact capture policy; road/water paths |
| Ambient / coat / AO | Custom probe, clearcoat and GTAO | Not original-lighting parity |
| Sky/fog | Original assets/formulas with approximations | Sky composition and dynamic location blend |
| Weather/time | Partial original weather, custom clock/drops | Regional rain, wind/splashes; custom night sky is separate |
| Player lights | Original profiles and lamp textures | Occlusion, damage flags, auxiliary passes |
| Speedbreaker | Original profile and mask | Full composited comparison coverage |
| Particles | Original-derived collision/exhaust, custom tire visuals | Complete authored smoke/skid/particle behavior |
| AA/upscaling | Modern FXAA/SMAA/spatial FSR1 | Original PC AA; optional backend temporal paths |

## Reference policy

The preferred reference may include modded PC effects and compatibility fixes
when their behavior can be reproduced using stock assets. Classify observations
as stock behavior, asset-independent restored behavior, or mod-asset-dependent
appearance. Do not silently relabel an Xbox/restored effect as retail PC parity.
Mod-specific textures, replacement shader packs and geometry are optional
reference material, not required game assets. Runtime effect selection still
needs evidence; an installed plugin/configuration is not proof it was active.
Private reference manifests retain executable/mod/config identities. Public
source ships neither original/mod assets nor extracted bytecode or data.

## Next implementation batches

1. Establish matched reference shots and pass coverage; pin installed identities.
2. Original world materials, sky and atmosphere.
3. Original car materials, lighting, shadows and reflection paths.
4. Complete grading/exposure/Over Bright, then weather and particles.
5. Retain custom implementations as explicit enhancements with independent tuning.

Each batch needs a behavior spec, provenance and matched captures. Run shared
settings/persistence tests, both native APIs and real/simulation-time transition
checks. FPS screenshots alone are not a performance benchmark or parity proof.
