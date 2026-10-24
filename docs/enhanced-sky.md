# Enhanced sky prototype

The native Enhanced sky replaces the three authored texture layers with a per-pixel
atmosphere and a shallow cloud volume. Original PC keeps the existing shader and
layer arithmetic. The F8 Environment checkbox switches the sky independently;
the selection is saved with appearance settings. Existing custom profiles default
to Enhanced sky; choosing Original PC explicitly selects the authored sky.

F8 Environment > Sky tuning exposes cloud speed (0–4), sun size (0.5–3), sun
brightness (0–3), sky brightness (0.25–2) and extra sunrise/sunset warmth (0–2).
Each control has a reset; Save for startup and named appearance presets retain them.
They affect Enhanced sky only. Cloud speed 0 holds the cloud field. Changing speed
repositions the field. Cloud amount stays under the existing weather controls.

Manual Clock hour changes now seek the Enhanced cloud field as well as the sun.
The seek uses the configured seconds per game hour; it does not advance simulation.
Normal midnight wrapping and weather/speed edits do not seek the clouds. Freeze
time still holds sunlight while wind continues in unpaused gameplay.

## Appearance and integration

- Analytic Rayleigh-shaped colour, forward haze, low-sun reddening and a solar disk.
- The visible disk defaults to 2 times its 0.53-degree baseline. Its size does not
  control shadow softness. SDR sky highlights use a shoulder before adding the
  cloud-attenuated disk, retaining contrast against bright haze/clouds. HDR keeps
  above-white radiance. No lens flare or screen-space sun glare is added.
- A continuous weather coverage field changes cloud density, rather than fading
  separate cloud images. Wind advects periodic 3D gradient/cellular noise. Bounded
  height-dependent wind shear bends the field without stretching it during long
  sessions or hour seeks. Erosion detail moves separately and slowly rises.
  Cloudy weather increases wind speed without changing the update cadence.
- Clouds occupy a 2–6 km slab. A 96-step view march integrates extinction and
  illumination; six sun samples and a four-octave scattering approximation provide self-shadowing and softer interior light.
- Cloud light is integrated in linear space, then converted to the native scene
  and cubemap's display-encoded colour contract. Authored fog is decoded before mixing.
- The main view, mirror, car cubemap and wet-road sky use the same shader, weather,
  sun and world-space density. The horizon blends toward the scene fog colour.
- At the existing orbit endpoints, sunlight on clouds and the sun-facing horizon
  become warmer. The opposite horizon retains a cooler tint. The orbit is unchanged;
  this does not add a below-horizon sunset, sun glare or new world-lighting equations.
- Existing shadowed captures and local car irradiance remain in place. This does
  not add unoccluded global sky light to cars inside tunnels.

The 128-cubed base field, 32-cubed erosion field and horizontal weather map are
generated once on the GPU without game assets. Cloud radiance and transmittance
are rendered into a 1536×384 upper-hemisphere map in linear RGBA16F. Nearby views
share that map, including reflection faces and mirrors. The atmosphere and sun disk
are still evaluated per pixel in each view, keeping the solar disk sharp.

The map updates whenever wind time, cloud coverage or sunlight changes. Wind and
slow sun drift rebuild one of four horizontal row bands per frame, cycling, so each
texel is still a complete single-frame march (at most three frames old, no blending
with history). A visible jump in coverage, sunset warmth or sun direction, or a
camera move of 32 m, rebuilds every row at once, including at frozen time. The light
march through each cloud sample uses four steps. Nearby eyes share one
origin: their cloud parallax is approximated because the cloud base is 2 km high.
Cloud detail has a fixed angular resolution and is bilinearly reconstructed;
longitude wraps across the seam. No previous-frame blending or fixed reflection
cadence is introduced. Car and scenery reflection rendering remains independent.

## Limits

This is an artistic analytic approximation, **not** an implementation of Hillaire's
multiple-scattering LUT atmosphere. It retains the existing shared sun orbit and
world lighting, including the current clock's dusk-based night behaviour. A full
24-hour solar orbit, moon/stars, city night lighting and exposure need separate work.

Clouds do not cast moving shadows onto the city. The authored weather still controls
world sunlight. The flat slab and finite horizon fade approximate a ground-level
sky; flying through clouds is outside the supported camera range. Noise repeats
at large distances. Distant clouds can show noise patterns and blocky detail; these
remain known visual limitations for later refinement. No performance claim is made from shader sample counts alone.

## References

The architecture draws on [Hillaire's atmosphere reference](https://github.com/sebh/UnrealEngineSkyAtmosphere)
(MIT), [SkyGL](https://github.com/kotivas/skygl) (MIT), and
[Godot's shared sky/environment design](https://godotengine.org/article/custom-sky-shaders-godot-4-0/).
Density remapping, cloud-type height profiles and the multiple-scattering approximation
are adapted from SkyGL at `edbbb0148091bd85261dec0a2164bc026ff01a3e`; its MIT
notice is retained in `sky_clouds.wgsl`. Noise generation and renderer integration
are independently implemented. The model follows the base/detail separation
described in [Guerrilla's 2015 cloud presentation](https://www.guerrilla-games.com/read/the-real-time-volumetric-cloudscapes-of-horizon-zero-dawn).
This does not reproduce the complete Nubis or SkyGL renderer.

## Distant cloud sampling

The density textures now contain filtered mip chains. Cloud view and lighting
samples choose their density detail from the ray segment and angular footprint.
A fixed midpoint replaces the interleaved screen pattern; longer horizon paths
use up to 384 samples instead of a fixed 96. This improves the source image
without temporal reconstruction. The shared angular map can still limit distant
silhouettes; this does not add cloud shadows or extend the day/night orbit.
