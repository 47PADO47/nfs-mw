# Enhanced bloom and camera glare

In the native F8 Look tab, **Physical bloom** and **Sun glare**
both start at zero. Checking a box starts that effect at 30%. Its slider adjusts the strength. Console equivalents
are `set physical_bloom 30` and `set sun_glare 25`,
in percent. Zero physical bloom restores the existing Bloom setting. F8 saves
these controls with appearance settings. Original PC suppresses them.

Physical bloom decodes scene samples into linear light and selects highlights with
a soft threshold. Thirteen samples feed the first downsample, with five
inverse-luminance-weighted groups to suppress fireflies. A four-read tent filter
reconstructs the pyramid. The composite redistributes up to 45% of the selected
light, subtracting it at its source and blending the normalized blur back in.
Uniform fields keep their brightness, and nearby dark pixels receive a glow.

Sun glare draws a procedural veil at the projected sun and shares the existing artwork's solar-disc depth and cloud
visibility. Foreground objects can hide the entire veil. Sun glare applies only to the world camera.

The bloom composite precedes Visual Treatment and tone mapping. Spatial
upscalers follow it. Temporal upscalers run first so bloom stays out of their
motion history. The effects belong to the scene image before HUD drawing.

At 4K the six RGBA16F pyramid levels occupy about 21 MiB. Bloom uses six
downsample draws, five upsample draws and one composite. Sun glare adds
one bounded quad to the sun pass.

The filter follows Jorge Jimenez's [2014 SIGGRAPH presentation](https://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare/)
and the five-group downsample in [Bevy 0.20](https://github.com/bevyengine/bevy/tree/v0.20.0/crates/bevy_post_process/src/bloom)
(MIT OR Apache-2.0).
