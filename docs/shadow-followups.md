# Custom shadow priorities

These are enhancements, separate from the original PC projected shadow path.
The ordinary original sun cycle never sets. A custom night mode is not missing
retail lighting parity.

## Correctness first

Custom cascades keep stable enclosing spheres and texel-snapped XY projections.
The depth slab includes upstream resident caster bounds overlapping each map's
XY footprint, rather than clipping everything beyond a fixed upstream allowance.
Depth endpoints are quantised in world space; caster order and frame time do not
affect them. Additional depth does not reduce XY texel density. The daylight
basis is continuous through the former near-zenith axis switch.
The custom grid rotates around a persistent local camera pivot rather than city
world zero. This avoids rapidly changing rounding phase at large world coordinates
as the sun moves. After 128 units of camera travel it rebases, preserving each
cascade's current texel phase. It retains texel snapping for camera motion and
does not accumulate visibility or blend previous shadow images. Original projected
shadows keep their separate fitting path.
Only nonempty opaque/cutout non-sky draws contribute caster bounds. Sky and
transparent-only meshes cannot inflate the fitted depth span or its physical bias.

Directional PCSS now searches a fixed twelve-texel footprint, covering the bounded
eight-texel penumbra within the cascade padding. Its footprint no longer changes
when the fitted depth origin moves for upstream casters. This retains sixteen
nearest-depth blocker reads and the existing spatial tent filter. It is a bounded
quality budget rather than a finite area light located at the shadow near plane.
F8 Lighting exposes blocker count (0–16) and filter radius (0–8 texels) diagnostic
views; `NFSMW_LAB_DEBUG=blockers|penumbra` selects them at startup. Original projected
shadows stay separate and report zero for these cascade-only diagnostics. Fixed
PCF remains selectable by disabling contact hardening.

F8 Lighting also offers an opt-in **Dense penumbra filter**, saved with the lighting
profile and A/B snapshots. The sparse 5-by-5 filter spreads taps apart as PCSS grows
its footprint, leaving gaps at wide radii. The dense alternative pairs adjacent
texel weights into bilinear comparisons and integrates a complete separable tent.
It uses up to 81 comparisons at an eight-texel radius, versus the reference's 25.
It leaves radii up to two texels identical and smoothly hands over between two and
three texels. Wider edges use a different tent profile; compare edge width as well
as patterning. It adds no random rotation, frame seed or temporal history, and
does not alter blocker search, cascade fitting or the original projected path.
It is disabled by default because its benefit and GPU cost depend on penumbra size.
An isolated one-texel caster regression checks removal of holes, live off/on/off
round trips, unchanged narrow filters and continuity at the handoff endpoints.

Regressions orbit the camera and translate it around a receiver whose occluder
is behind the camera and beyond the old depth limit, with PCSS on and off.
They also check repeated-frame equality, irrelevant caster rejection, stable
XY density, and continuity across the former sun-basis threshold.
Streaming remains a separate boundary; see [shadow residency](specs/shadow-residency.md).

## Recommended experiments

| Priority | Work | Reason / acceptance gate |
| --- | --- | --- |
| Next | Freeze sun, compare PCF against PCSS on the same route, then repeat a sun-phase sweep | Separates coverage/bias errors from blocker-search shimmer; compare image variance and GPU timestamps |
| Next | Improve blocker search and penumbra estimation | Current 16 nearest-depth blocker reads can change abruptly across texels; measure contact edges, foliage, slopes and cascade overlaps before adding cost |
| Next | Compare fixed, world-anchored Poisson sampling with the current fixed kernel | Retains deterministic high-FPS behavior; add subtexel camera/sun sweeps and silhouette-detail metrics |
| Later | A small player-focused high-detail region, with smooth handoff | Current near splits already favor the player; only add a dedicated map if measured car-shadow quality warrants its draws and memory |
| Later | EVSM or MSM prototype behind a separate filtering mode | Requires moment targets/filter passes and new bleeding/precision tests; it cannot repair missing casters |
| Defer | Animated blue-noise sampling | Current path has no temporal shadow denoiser; animated sampling can replace shimmer with visible noise |
| Defer | Hybrid or full ray-traced shadows | Requires acceleration structures, dynamic/alpha-tested geometry handling, backend capabilities and denoising |
| Defer | Virtual shadow maps | A paging/cache/rendering subsystem, excessive for the current problem; moving sun and objects invalidate cached work |

Keep deterministic PCF as the baseline and PCSS as an optional extension. A new
algorithm must beat the baseline at the same measured frame budget, with fixed
routes and matching sun/cloud state. No automatic quality change should depend
on rendered-frame count. Keep original and custom modes independently testable.

## Research sources

- Microsoft's [cascaded shadow map guidance](https://learn.microsoft.com/en-us/windows/win32/dxtecharts/cascaded-shadow-maps)
  describes stable projections, cascade blending, filter padding and receiver-plane bias.
- NVIDIA's [PCSS integration guide](https://developer.download.nvidia.com/whitepapers/2008/PCSS_Integration.pdf)
  describes blocker search, penumbra estimation and PCF. It is a reference for the
  algorithm, not evidence that a particular sample count fixes this renderer.
- AMD's [Hybrid Shadows sample](https://gpuopen.com/fidelityfx-hybrid-shadows/)
  combines raster and ray-traced visibility with a shadow denoiser. It is not a
  replacement filter that can be dropped into the existing depth sampler.
- Epic's [Virtual Shadow Maps documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/virtual-shadow-maps-in-unreal-engine)
  explains page caching and invalidation by lights and geometry.

The priorities above are engineering judgments for this renderer, not measured
performance claims about those alternative algorithms.

## Related upstream work

At review, [PR #121](https://github.com/47PADO47/nfs-mw/pull/121) is open and adds
Bevy Solari lighting, upscaler wiring, reflection controls, native radial blur
and frontend shadow controls. Its native cascade fit changes only accommodate
the extended frame structure; they do not repair caster-depth clipping.
Solari uses opaque world stand-ins and explicitly omits alpha-masked foliage,
so it does not replace the original-material native rendering path or establish
original lighting parity. Reuse that optional backend when evaluating ray tracing
after integration instead of building a second experimental RT stack here.
# Shadow density comparisons

F8 Lighting offers 8192 maps as an optional density experiment. Four Depth32F
cascade layers consume 1024 MiB at 8192, versus 256 MiB at 4096. The renderer clamps
the request to the device's texture-dimension limit. Raster bandwidth and cache
pressure can increase; compare the GPU sun timing in milliseconds while driving.

**Focus nearest cascade** instead halves the first split (8% to 4% of Near range).
At range 320, it ends at 12.8 units instead of 25.6. This approximately doubles
nearest-cascade linear density without increasing map allocation. Later cascades
retain their ending distances, but the second starts earlier. The camera can cross
the near transition sooner; inspect that transition as well as the car contact.

Both controls preserve continuous sun motion and remain independent of PCSS and
screen-space contact detail. They do not remove all raster aliasing from moving
light. Filter width is in shadow texels: double it when doubling linear density
to compare a similar world-space footprint. PCSS's eight-texel maximum also covers
a smaller world-space area at higher density. Dense penumbra filtering remains
separately selectable. Existing defaults and saved values are preserved.

## Wider detail coverage

**Wide-detail cascades** is an independent alternative to tight near focus. It
overrides the legacy focus split while enabled. The first two splits use 16% and
42% of Near range rather than the focused layout's 4% and 28%. At range 320 this
gives endpoints 51.2 and 134.4. Each overlap begins at 65% of its ending distance;
the first transition therefore spans 33.28 to 51.2 camera-depth units instead of
10.24 to 12.8. These are view-depth intervals, not circles around the car.

The first two depth layers have their own **Detail map resolution**. The regular
Map resolution controls the two distant layers. Separate texture arrays allocate
only those sizes: two 8192 maps plus two 4096 maps consume 640 MiB, versus 1024 MiB
for four 8192 maps. Equal 4096 settings consume 256 MiB. The detail resolution is
never lower than the distant resolution, and both respect the device limit.
Existing layouts retain their original single array allocation; the original PC
projected path stays separate. No temporal history or noise is added.

In this mode Filter width and the PCSS penumbra limit use the nearest cascade's
world-space texel size, converting that footprint to each sampled map. This
avoids deliberately increasing filter softness at each split. It cannot recover
detail lost to a coarse raster grid. PCSS blocker search remains a fixed 12-texel
pattern in each map, so different blocker estimates can still affect overlaps.
Adjacent maps are sampled together only inside an overlap, preserving the
existing blended result while avoiding the second sample elsewhere.

This is a quality/cost experiment, not a claim of seamless or shimmer-free
shadows. Moving-sun stability, foliage and driving views need separate checks.
The mixed-resolution routing and continuous overlap weights have GPU probes;
frustum coverage includes both ends of every overlap and both texture sizes.
