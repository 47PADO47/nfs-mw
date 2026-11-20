# Dynamic soft sun shadows

Build with `--features renderer-bevy` and run with `--renderer bevy --shadows medium --post-aa smaa`.
The Video menu's **Dynamic Soft Shadows** row and console `shadows off|low|medium|high`
change quality immediately. Config key `shadows` and environment variable `NFSMW_SHADOWS`
use the same values. Default: off. Native and OpenGL backends explicitly downgrade to off.

| Quality | Map edge | Cascades | Receiver distance (world units) |
|---|---:|---:|---:|
| Low | 1024 | 2 | 80 |
| Medium | 2048 | 3 | 160 |
| High | 2048 | 4 | 240 |

## Pipeline

The Bevy backend fits cascaded depth maps to the active camera and updates them each frame.
One shadow-only directional light follows `FrameParams.light_dir`; it adds no radiance.
The custom colour shader reads sun visibility from Bevy's fixed Gaussian PCF sampler. Moving cars, scenery,
and physical props use the same instance transforms in the colour and depth passes.
Alpha-tested depth uses the colour pass's texture/vertex alpha cutoff. Sky, blended and additive
draws never cast opaque silhouettes; sky and additive draws receive no sun attenuation.
Particles and HUD stay in their existing separate passes.

Nearby resident scenery is submitted even outside the player frustum, with an 80-unit caster
margin beyond the receiver range. Swept bounds along the sun ray reject casters that cannot
reach the camera frustum. Visible objects retain the original camera LOD; only off-camera
casters use a camera-independent projected-size LOD. Explicit local mesh bounds let Bevy cull
those objects separately for each camera and shadow cascade. Hidden or unloaded instances
stop casting in the frame they disappear.
The feature does not load extra world sections: only resident, zone-eligible scenery can cast.

The spatial Gaussian filter uses nine hardware comparison samples for an effective 5x5 kernel.
There are no random sample rotations, blocker searches, temporal shadow samples or shadow history.
It works with AA off, FXAA and SMAA; enabling TAA does not change the shadow filter. Bevy's cascade
fitting snaps to texels for stability. The final cascade fades smoothly from 75% of its far bound
back to unshadowed shading at the boundary (Medium: 120 to 160 units). Filtering hides map edges,
not all subpixel movement: very small distant detail may still shimmer without temporal AA.

## Lighting boundary

This adds geometric sun visibility, not original environment parameters or lighting rigs.
The game's existing car key light points toward the same sun as the frame; its fill and back
lights remain artistic approximations. Only key diffuse and the sun highlight are shadowed;
fill, back, ambient and environment reflections remain. Lit materials similarly retain ambient.

Prelit scenery stores baked lighting with no separate direct-sun term. Its shadow response is
an explicit approximation: retain 35% of baked radiance and attenuate the remaining 65% in
linear light before fog.
Applying that attenuation to gamma bytes would darken the already baked scene much too strongly.
Shadows off with tone mapping off preserves the previous shading, including its brightness cap.
Recovering original lighting inputs or splitting authored direct and indirect lighting is separate work; the shadow
visibility calculation can be reused when those inputs are available.

## Sources and verification

Bevy 0.20 (MIT OR Apache-2.0), used as a dependency, provides cascade fitting and spatial filtering:
[directional light](https://docs.rs/bevy_light/0.20.0/bevy_light/struct.DirectionalLight.html),
[filter methods](https://docs.rs/bevy_light/0.20.0/bevy_light/enum.ShadowFilteringMethod.html),
[shadow sampling](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_pbr/src/render/shadow_sampling.wesl).
No upstream shader code is copied into the custom material.

`shadow_tests` uses procedural geometry and GPU readbacks; run
`cargo test --release -p blackbox-bevy-render shadow_tests -- --include-ignored --test-threads=1`.
It covers Vulkan and DX12 dynamics, runtime quality changes, caster removal, off restoration,
alpha cutouts, off-camera casters, glossy indirect light, final-cascade fading, deterministic
spatial PCF with AA off, SMAA/FSR 1 and calibrated ACES midtones/clear/fog, exposure and above-white
detail. CPU tests cover settings layering, unsupported backends, off-camera caster rejection
and spatial caster selection.

## Display-authored colour and ACES

The original shaders shade in display gamma space; applying scene-referred ACES directly to their
sRGB decode applies contrast a second time and crushes shadow detail. Only when ACES is selected,
`material/color.rs` and the material shader calibrate the decoded neutral axis to Bevy's fitted
RRT/ODT. Reference white maps to 90% display-linear output, leaving room for brighter lighting.
Above-white prelit values are retained in HDR, and glossy highlights retain their extra range.
Clear and fog colours use the same calibration, including changes made while running.
Tone mapping off keeps native byte parity. This is an artistic bridge for SDR-authored assets;
it does not turn the legacy gamma-space lighting into physically based scene-linear shading.
Saturated colours still receive ACES's colour mixing and highlight desaturation.

The neutral fit is documented by [Bevy's tone mapping shader](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_core_pipeline/src/tonemapping.wesl).
The inverse calibration is derived algebraically; no upstream shader is copied.

## Vehicle identities

Repeated traffic models require independent placement identities in persistent render pools.
That shared traffic correction is maintained separately from this shadow feature. Shadow and
colour passes consume the same instance keys and transforms supplied by the game.

## Profiling

Instance-pool batch aggregation and apply timing run only when their debug logging is enabled.
Otherwise periodic diagnostics must not walk thousands of objects or allocate batch tables.
`--bench-seconds` waits for assets/roads and traffic to load, then measures a scripted drive
while it is moving. Screenshot readiness still waits for the script to finish. Previously both
used capture readiness, so a scripted benchmark accidentally measured only the final view.
These are application frame times, not isolated GPU timings or compositor present measurements.

## Native renderer: cheaper sun filter at night (tried, not adopted)

The native renderer's cascade receiver (`shadow_receiver.wgsl`) runs a 32-load blocker search and a
12- or 20-tap disk filter. Night and cloud cover were tested as a reason to spend less there: a uniform
(a spare `debug` lane) blended the PCSS result towards a four-tap fixed PCF, reaching it at night blend 1.
Measured on an RTX 3070 (Vulkan), synthetic street with sun casters, 2560x1440, batch timing:

| Case | Shadow cost before | With the cheap night path |
|---|---:|---:|
| Soft, 2048, day | 1.37 ms | 1.58-1.61 ms |
| Soft, 2048, night blend 1 | 1.35-1.37 ms | 0.26-0.31 ms |
| Dense + wide detail, 4096, day | 1.56-1.77 ms | 2.10-2.11 ms |
| Dense + wide detail, 4096, night blend 1 | 1.75 ms | 0.37-0.41 ms |

It was reverted for two reasons. The night tint scales ambient and direct light alike, so a shadow keeps
its daytime contrast at a third of the brightness: the cheap kernel changed the final night image by up to
41/255 at penumbrae (lamp heads shadowing their poles, contact lines), which is visible. Cloud
transmission never falls below 0.35 and varies across the map, so overcast skies are not a safe bound
either. Second, carrying both paths in the receiver made daytime frames slower (the shader's other work
included), although daytime images stayed bit-identical. A saving here needs to leave the image
unchanged, for example cheaper depth fetches for the same blocker texels, not a reduced kernel.

## Native renderer: the blocker search fades out with distance

The contact-hardening blocker search (32 depth loads per pixel) widens the filter by the penumbra a blocker
casts. Far from the camera that penumbra spans a pixel or two, so the search now fades out between 80 and
120 m from the camera (`near` in `cascade_sample`): its added radius is scaled down through the band and the
search is skipped beyond it, where the fixed filter width and the half-pixel antialiasing floor remain. With
hardware RT sun shadows (receiver range 100 m by default) the cascades only show from 80 m on, so most of the
search's work there was spent on these distant pixels.

Measured in the game on an RTX 3070 at 3840x2160 (SMAA, 4096 maps, PCSS and dense filter on, 12 s medians):

| Scene | Before | After |
|---|---:|---:|
| Clear morning, RT sun shadows | 17.3 ms | 16.1 ms |
| Clear morning, cascades only | 16.7 ms | 15.5 ms |
| Overcast rain by day, RT | 21.7 ms | 20.6 ms |
| Overcast rain at night, RT | 28.7 ms | 27.8 ms |

The 4K frame changes by 0.014/255 on average (two runs of the old build differ by 0.003/255); the changed
pixels are distant shadow edges, which are slightly crisper. Two attempts at an identical-image saving in the
same pass did not pay: gathering the blocker texels (8 gathers for 32 loads) was 1.3 to 1.5 ms slower at 4K,
and skipping redundant surface tests in the RT reconstruction filter saved nothing measurable.
