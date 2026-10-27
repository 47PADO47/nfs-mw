# Enhanced world ambient lighting

This optional enhancement replaces part of the ambient response on world,
tree, prelit prop and normal-mapped scenery materials. Original PC style and
unlit/no-fog artwork retain their equations. It does not add global illumination.

The CPU projects a deterministic analytic sky/ground field into nine RGB
spherical-harmonic coefficients. The atmosphere follows the existing enhanced
sky's broad gradient, sun direction, sunset warmth and cloud-coverage colour.
It excludes the sun disk: direct sunlight already supplies that energy.
With the authored sky, ambient and fog colours supply the broad sky estimate.
Ground reflectance is a neutral estimate, not a road texture or car-local capture.

Cosine convolution uses band factors 1, 2/3 and 1/4. The field is normalized
to unit mean per colour channel in the existing display-encoded material domain.
The authored ambient already contains weather/time colour; this avoids tinting
blue dusk shadows blue a second time while retaining directional colour variation.
Each surface's authored ambient colour, intensity, baked vertex colour and AO
remain inputs. The directional ratio is bounded to 0.45–1.65 and blended with
uniform ambient (default 65%). Normal-mapped roads evaluate the perturbed normal;
wet films use their water normal. Direct sunlight and specular are unchanged.

This is not a physical radiometric conversion of original artwork. It avoids
adding a second ambient light and preserves weather-dependent authored intensity.
Cloud positions, wind and camera orientation cannot flicker the field. Coverage
and sun changes are continuous. There is no history, noise, GPU readback, extra
geometry pass, shadow map, texture allocation or per-pixel texture sample.
The small CPU projection and nine-term shader evaluation still require profiling.

The global field cannot detect a tunnel or nearby wall. Baked occlusion remains
essential; this does not reconstruct local bounce or remove baked lighting.
The existing cloud-shadow pass stays independent; this field uses mean coverage.
Cubemap/mirror captures share the same world field without feeding their own
reflections back into ambient light. Primary raster materials are covered;
off-screen RT-hit material shading remains an approximation.

Enhanced style enables it. F8 > Lighting > Directional sky and ground light
provides an immediate comparison; Advanced world lighting controls the balance.
The game menu's existing Enhanced style is the normal entry point. No new preset
family or redundant quality switch is introduced. Older custom settings keep it
off until selected. Settings save the override and balance independently.

## References

The SH diffuse irradiance principle is described in
[Google Filament](https://google.github.io/filament/Filament.html#lighting/imagebasedlights/irradianceenvironmentmaps)
(Apache-2.0), already credited in NOTICE. Implementation is independent, reusing
this renderer's SH basis and atmosphere equations. No external code or assets
were imported.

Compare fixed views in sunny/cloudy weather and at low sun. Inspect road fill,
wall and rock faces, tree crowns and shaded props. Baked dark crevices should
remain dark. Drive into tunnels and check for overly blue fill or washed-out
surfaces. Traffic bodywork should have broader reflections than glass or chrome.
