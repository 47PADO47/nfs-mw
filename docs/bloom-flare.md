# Enhanced bloom and camera glare

In the native F8 Look tab, **Physical bloom**, **Lens flare** and **Sun glare**
each start at zero. Checking a box starts that effect at 30%. Its slider adjusts the strength. Console equivalents
are `set physical_bloom 30`, `set lens_flare 20` and `set sun_glare 25`,
in percent. Zero physical bloom restores the existing Bloom setting. F8 saves
these controls with appearance settings. Original PC suppresses them.

Physical bloom filters the linear HDR image with thirteen taps per downsample.
The first level averages five overlapping groups with normalized inverse-luminance
weights to reduce fireflies. A nine-tap tent reconstructs the pyramid. Dividing
by the level count preserves a uniform field. The composite mixes up to 12% of
that blur into the original colour, preserving a uniform paint highlight.

Lens flare samples the reconstructed pyramid at mirrored positions on the
screen-centre axis. Three displaced colour samples form the halo. An off-screen
source disappears at the image boundary. Sun glare draws a procedural veil at
the projected sun and shares the existing artwork's solar-disc depth and cloud
visibility. Foreground objects can hide the entire veil. Neither effect places
light sprites on vehicles or scenery.

The bloom composite precedes Visual Treatment and tone mapping. Spatial
upscalers follow it. Temporal upscalers run first so ghost images do not enter
their motion history. The effects belong to the scene image before HUD drawing.
Lens ghosts are screen-space and cannot follow off-screen headlights.

At 4K the six RGBA16F pyramid levels occupy about 21 MiB. Bloom uses six
downsample draws, five upsample draws and one composite. Lens flare reuses
those levels and adds five filtered reads per composite pixel. Sun glare adds
one bounded quad to the sun pass.

The filter follows Jorge Jimenez's [2014 SIGGRAPH presentation](https://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare/)
and the five-group downsample in [Bevy 0.20](https://github.com/bevyengine/bevy/tree/v0.20.0/crates/bevy_post_process/src/bloom)
(MIT OR Apache-2.0). The independent ghost and halo implementation follows
[John Chapman's algorithm description](https://john-chapman.github.io/2017/11/05/pseudo-lens-flare.html).
