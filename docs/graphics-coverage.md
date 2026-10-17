# Graphics implementation coverage

This ledger describes current rendering, not complete original-game equivalence.
Original assets and original behavior are separate claims. The typed inventory
in `settings/appearance/registry.rs` supplies F8 Diagnostics.

| Feature | Current implementation | Remaining original parity |
| --- | --- | --- |
| Models/textures | Installed original assets | Full material-specific rendering |
| Geometry/texture detail | Adaptive scenery slots, fixed reference projection, full texture mips and car LOD A | Coherent original World LOD and dynamic car LOD; current High is not an exact original maximum |
| World materials | Original common, normal, reflective and window families on native; selectable legacy fallback | Full projected caster/filter parity, remaining states and matched full scenes |
| World normal maps | Original tangent-space normal/specular path on native | Bevy fallback, full projected filter parity, animated bindings and sampler states |
| Car materials | Selectable original PC modern/shadowless arithmetic and gameplay/default frontend rigs on native; retained Custom Glossy | Complete projected shadow classification/filtering, reflection capture, skin composition and exact frontend context; Bevy custom shader fallback |
| Authored overlays | Original SHD scenery meshes | These are not complete dynamic shadows |
| Dynamic/contact shadows | Recovered single-map perspective fit and sun/cloud strength; separate custom cascades, PCSS and contact | Original caster selection, classification, draw strength and vendor filtering/bias |
| Visual Treatment | Original midday profile; optional custom strength and Reference (mod) parameters | Tunnel exposure, pursuit/cinematic look selection/transitions |
| Bloom / ACES | Custom enhancements | Original Over Bright |
| Car reflections | Original-style cubemap, custom budgets | Exact capture policy; separate water path |
| Wet-road reflections | Authored projected scenery/sky, attached vehicles and optional original blur on native; elapsed-time dampness | Complete original eligibility/LOD/effects, sampler state and live reference routes; Bevy unsupported |
| Ambient / coat / AO | Custom probe, clearcoat and GTAO | Not original-lighting parity |
| Sky/fog | Original PC layered sky on native; authored cloud-map/preset atmosphere | Bevy layered sky, reflection-specific textures, scripted weather and full scene comparisons |
| Weather/time | Original sun cycle/cloud map; fixed phase/cloud controls; elapsed-time wetness; custom hour clock/drops | Regional storms, wind/splashes and tunnel exposure; full night is a custom extension |
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

## World/atmosphere reconstruction

The Original PC profile enables Original world materials; F8 Materials exposes
that switch independently. Enhanced and legacy configuration keep the prior
simplified material selection. Quality never changes the family selection.
World/Tree/NoFog/Prelit and native normal-map families have original-shader pixel fixtures; full original rendering
is still not claimed. The native sky uses independent UV channels and three
camera-centered layers with stock textures. Bevy retains its documented sky
fallback. See [material routing](specs/world-material-routing.md),
[normal maps](specs/world-normal-materials.md), [atmosphere](specs/world-atmosphere.md)
and [sky shading](specs/world-sky-shading.md).

Bevy's linear-space blending after its gamma bridge remains different from the
original gamma-space D3D9 surface blending. Its opaque/cutout world families are
validated independently; this does not establish blended-surface equivalence.


The installed city stream contains 46,351 material groups. Common World/Tree
families account for 26,092 groups, normal maps for 8,003, reflective World for
9,054, and GlossyWindow for 3,201; one group uses the sky effect. These counts
measure authored group coverage, not screen pixels or frame time. Reflective
World now uses its original tangent lighting and cloud-attenuated specular path.
GlossyWindow uses its separate sun/ambient/glint equations and the installed
global WINDOWREFLECTION image. Both keep their explicit legacy fallback on
unsupported backends or missing inputs. Wet-road capture/history now follow the
[separate contract](specs/wet-road-reflections.md); complete original capture
eligibility/effects, shadow classification/filtering and full-frame composition remain partial.

The donut-shop reference road was identified by a vertical triangle intersection,
not by its name: it uses reflective effect 1. The adjacent shop windows use effect
5 and alias their authored texture slots to the diffuse image; the global window
reflection must be substituted independently. Selected geometry and texture
payloads match the stock and preferred modded reference installations. Independent
original shader fixtures, installed material/lifetime checks and fixed scene
captures establish distinct layers of evidence; they do not establish full parity.

## Car reconstruction

Original PC selects the regular programmable PC car material family. F8 Materials
and Video > Car Shading also expose the original shadowless family, whose highlight
and reflection equations differ. Quality changes preserve that selection. Custom
Glossy and Simple remain available; existing preferences are not silently migrated.
Save profile keeps the choice. `--car-shading original` and
`--car-shading original-unshadowed` select the same paths for viewers.

The native renderer evaluates original material ramps and light sums per vertex,
retains the original interpolated normal/view behavior, uses composed glass alpha,
and clamps at the original SDR boundary before blending. Gameplay and preview use
their researched three-light rigs. Authored CarSpecScale follows sun/cloud state
through frame uniforms without rebuilding materials. See [car materials](specs/car-material-shading.md)
and [lighting](specs/car-lighting.md) for contracts and evidence boundaries.

Both native APIs need independent original-shader pixel fixtures, installed paint,
glass and customization regressions, and fixed-view smoke captures. These checks
are separate from complete scene matching and a traversal performance benchmark.
Bevy retains its custom glossy shader with an explicit fallback message; original
material arithmetic is not claimed there. Complete original caster selection/filtering,
reflection capture content/cadence, complete skin composition and career frontend
context remain incomplete. Custom sun shadows can be combined with the regular
original material family, but that combination is an enhancement.

## Shadow and condition comparisons

Original PC now selects the recovered single-map perspective shadow camera. F8
Lighting switches it independently from custom cascades. Both reach normal roads
and architectural glazing, preserving their ambient/reflection terms. Higher map
resolutions use existing geometry and textures. Original caster selection,
per-draw classification/strength and vendor sampling/bias remain partial. See
[the projected-shadow contract](specs/original-projected-shadows.md) and
[surface receivers](specs/world-surface-shadow-receivers.md).

F8 Environment can select the original sun cycle or lock raw phase and cloud
blend. No rain is independent of cloudy lighting; dry weather after rain can
still have damp roads. The remake retains this history with an elapsed-time
adaptation of the original render-call recurrence.
The hour clock and Night shortcut are custom. Matching state is necessary but
not sufficient: a modded reference can change the authored TOD colours and keep
planar reflections active. Lighting comes from the selected install; do not
bake another installation's modified colour data into the implementation.
Reference (mod) treatment applies only the supplied six treatment parameters,
not another installation's lighting palette or reflection modifications.

See [graphics detail](graphics-detail.md) for the original master preset and
advanced-slider mapping, and [texture filtering](texture-filtering.md) for
independent sampler quality. Full mip chains and high-detail car geometry do
not imply that every original detail option is at its maximum.
