# Custom paint response

This is an optional reconstruction enhancement [rewrite], not original PC
shader parity. Custom Glossy paint opts in; Original material families retain
their equations. Other surface layers and indirect lighting are described in
[Custom car materials](custom-car-materials.md). The earlier matte reflection
tuning described here remains separate from those new layers.

F8 > Lighting > Car material response toggles this extension independently of
clearcoat. Vulkan and DX12 support this extension; limited compute backends
retain the legacy response. It defaults on for Custom Glossy; older saved settings inherit that
default. Disabling it restores the legacy response without reloading a car.

The game-specific layer selects perceptual roughness and coat strength from
the installed paint family. These are tuning choices, not measured calibration or a decoding of
the original GLOSS byte. Keep authored GLOSS unchanged in the data model.
The renderer receives generic surface parameters with an explicit opt-in.

Use a GGX normal distribution, Smith masking and Schlick Fresnel for the paint
sun highlight. Its width and peak change together; widening the distribution
must not arbitrarily increase integrated energy. Existing authored material
ramps remain inputs to the hybrid model. The installed MATTE01 material has
a zero environment ramp [file]; Custom paint receives a minimum dielectric
reflection with 4 percent normal-incidence gain and a roughness-limited grazing
response [rewrite]. This is an approximate hybrid, not split-sum PBR. This does not make the whole car PBR.

Filter the current environment into a separate cube with roughness levels.
Each level integrates reflected directions using GGX importance sampling and
cosine weights. A fixed sequence avoids temporal noise. Regenerate only when
the source environment changes and a Custom paint surface needs it. Preserve
the original cube, including its mip zero, for legacy and Original draws.
Reduce screen reflection contribution as roughness increases; rough paint
must not recover sharp details through the optional screen-space path.

Rough paint (perceptual roughness at least 0.45) combines the legacy diffuse
result, reflected environment and sun specular in linear light, then encodes
the result for the legacy scene pipeline. The cube is display-encoded, so its
4 percent dielectric reflectance must not multiply encoded bytes directly.
Filtering still averages encoded probe samples; this remains a hybrid rather
than a fully linear capture/IBL pipeline. There is no ambient floor.

Global clearcoat remains an enable switch. Opted-in paint scales that coat by
its own finish strength; matte uses roughness 0.52 and coat strength 0.25.
Rough paint's coat uses its own broad highlight and filtered environment,
with coat roughness 85 percent of the surface roughness (minimum 0.35).
Gloss, metallic, pearl, chrome and recognized non-paint surfaces now use the
separate layered response. Matte retains its tuned reflection model; indirect
lighting uses the shared linear-light correction. No global exposure or fixed
ambient lift is introduced.

References for the mathematical models: [Filament material and IBL theory]
(https://google.github.io/filament/main/filament.html), sections 4.4 and 9.2.
Equations are independently implemented; no source code is imported.

Validation must cover constant-environment energy, roughness-dependent detail
filtering, environment replacement/restoration, highlight shape, material
isolation and Original pixel regressions on Vulkan and DX12. Compare actual
installed paint at identical camera, lighting, color and exposure.

The standalone car viewer accepts `paint <installed part-name> [RRGGBB]` to
compare actual installed paint at a fixed orbit angle. Its existing model parts
and camera stay in place. This is a developer preview command.
