# Filtered reflection quality [rewrite]

The local capture remains sharp for Original PC materials, polished chrome and
clear puddles. Enhanced materials use a separate GGX-filtered environment.
This extends the existing paint filter rather than adding another reflection
technique. There is no new temporal history, reprojection or extra geometry ray.

## Spatial filtering

The integration source has nine spatial mip levels at up to 256 texels per cube
face. The first level averages a bounded source footprint (up to 8x8 samples),
then subsequent levels average four child texels. GGX samples the resulting
pyramid through seamless cube views across face boundaries. Encoded and linear source pyramids keep the two existing material
contracts separate; linear samples are decoded before the first average.

GGX integration selects source LOD using the sample probability and its solid
angle, so rare samples over broad lobes represent an area rather than isolated
sharp texels. It retains 64 deterministic samples and cosine normalization.
The output has eight roughness levels with a 128x128 base face (previously 64).
The integration and source texture storage totals approximately 10 MiB, versus
0.5 MiB for the previous filter, independent of the capture resolution. This is
additional to the raw capture, RT caches and other renderer resources.

Resources and bindings persist between updates. Rebuild only after the source
changes and a consumer needs filtering; a cached frame records no filtering
dispatches. Partial cube refresh still waits for a complete face cycle before
refreshing the filtered copy and diffuse lighting. Scheduling and reprojection
remain separate follow-ups.

## Consumers and boundaries

Custom paint, carbon, glass and metal retain their material roughness and blend
between raw sharp reflections and the filtered environment. Legacy matte paint
retains its encoded averaging/finish tuning; layered materials use linear IBL.

Enhanced wet-road redraws also bind the filtered environment. The probe fallback
uses roughness 0.32 for a water film, approaching 0.07 for standing water. Clear
puddles retain the raw capture. Exact SSR/RT geometry hits are unchanged. Rough
secondary metal hits in road RT also use the filtered probe. Capture passes bind
the static fallback, avoiding a read/write feedback loop with their own cube.

This does not fix missing reflected geometry, approximate RT hit shading,
transparency, or single-ray roughness integration. Original PC sampling and
capture texels are unchanged. Unsupported compute devices keep the raw fallback.

## Validation and references

GPU regressions cover constant energy, a sub-texel checker moving between phases,
continuous colors across cube edges, probe replacement/restoration, material
isolation and original material fixtures. Separate ignored timestamp diagnostics
measure filter rebuild/cache cost and synthetic six-face/one-face/cached captures.
Those diagnostics are not a gameplay benchmark or a frame-rate guarantee.

The mathematical reference is Filament's [pre-filtered importance sampling]
(https://google.github.io/filament/main/filament.html#annex/importancesampling/pre-filteredimportancesampling),
under Apache-2.0. Sampling, footprint reduction and the Rust/WGSL implementation
are independent; no code was copied. The earlier source review of rustyAC's
[CSP research](https://github.com/thesupersupersigma/rustyAC/blob/master/docs/csp/rendering_config.md)
(GPL-3.0-or-later, reference only) motivated reviewing the existing filter.
Its CSP notes are research, not an implemented dependency.
