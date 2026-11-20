# Sun shafts

In the native renderer, enable **Sun shafts** in F8 lighting and adjust **Sun shaft strength**
(0–4, default 1). The toggle defaults off. Console equivalents are `set sun_shafts on` and
`set sun_shaft_strength 1`. Fog grid quality and visibility distance are shared with local light fog.
The lamp count and lamp strength can both be zero.

Each fog cell samples the shadow cascade for its camera depth, including the split-resolution detail atlas.
When cloud shadows are active it also reads the existing cloud transmission map. Beyond cascade coverage
only cloud shade remains. The active directional light supplies direction and diffuse colour, including
the moon when the day/night cycle uses it. The Henyey–Greenstein phase function (g = 0.65) favours looking
toward the light. Rain, shorter fog range and low solar elevation increase scattering.

The volume uses spatially jittered samples, existing per-view reprojection with camera-cut rejection,
and depth-aware upsampling. Its additive composite preserves the scene's existing atmospheric attenuation.
Narrow gaps and thin foliage can disappear at coarse grid quality.

At 4K, medium quality covers 160 × 90 × 96 cells (1.38 million), including columns without lamps.
Each covered cell takes up to one cascade comparison and one cloud read. History and integration reuse
the local-light volume buffers. The published local-light fog timing measures a sparsely illuminated volume
and does not predict the cost of full-screen directional lighting.

Algorithm reference: Bart Wronski, *Volumetric Fog: Unified Compute Shader Based Solution to Atmospheric
Scattering*, SIGGRAPH 2014. The shader implementation is original to this project.

## Verification

A native Vulkan capture on an RTX 3070 in zone G15 showed subtle daylight scattering
behind the parked car. A second pair looked toward the treeline at (2361, 3352), at
07:00 with cloud coverage 0.8, a 130 m fog range and shaft strength 3. Switching the
shafts on brightened the air above the distant road while leaving shadowed bands
beneath the treetops. The GPU regression also checks the forward scattering peak
with zero local lights and the return to a black background when switched off.
