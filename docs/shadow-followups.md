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
Only nonempty opaque/cutout non-sky draws contribute caster bounds. Sky and
transparent-only meshes cannot inflate the fitted depth span or its physical bias.

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
