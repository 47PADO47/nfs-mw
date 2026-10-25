# Custom car material layers [rewrite]

This enhancement extends Custom Glossy. Original PC arithmetic, authored data,
light rigs, exposure and global sky/ground slider values remain unchanged.
F8 > Lighting > Car material response controls the extension; the saved key
remains `paint_response` so existing profiles retain their choice.

## Shadowed diffuse response

Keep direct diffuse lighting unchanged. Combine indirect irradiance with
linear albedo, applying baked occlusion and screen AO as linear energy factors.
Encode the sum of direct and indirect light for the legacy scene pipeline.
The existing artistic rig and SH values remain irradiance intensity controls;
this does not make those inputs calibrated radiance. Zero incident light must
produce black: no fixed ambient floor, lifted exposure or shadow bypass.
This helps retain body colour under shade without erasing panel crevices.

## Explicit material families

Generic renderer surface kinds select separate layers; game-specific code
maps known installed material identities. Unknown materials and lamp lenses
retain legacy response. Only Custom opts in. Do not infer glass from every
material whose name contains GLASS, since headlamp lenses share that wording.

- Glass retains authored tint and alpha, adds neutral angle-dependent Fresnel
  reflection, and increases coverage towards grazing angles. It has no extra
  experimental clearcoat. Existing alpha blend modes remain authoritative.
  This is a thin-surface approximation without refracted scene transmission.
- Carbon keeps its installed weave texture. A dielectric base and a smoother
  optional resin coat have independent highlight/reflection widths.
- Bare metal/chrome have coloured conductor reflections and no diffuse lobe.
  Polished chrome, silver and aluminium differ in roughness. Painted rims use
  dielectric paint and an optional coat. Synthetic rim identities stay limited
  to eligible wheel solids, including driving LODs; tyres and brakes are not
  recoloured. Rubber uses rough weak reflection without clearcoat.
- Gloss has a dielectric base and optional smoother coat. Metallic adds a broad
  tinted flake lobe and subtle UV-stable variation. Subpixel variation fades to
  its mean using derivatives. Pearl retains its authored facing-colour ramps
  and adds a broad tinted layer whose strength varies with viewing angle.
  These are artistic approximations, not measured flake/thin-film optics.
- Interior, underside and plastic hubcaps receive corrected indirect diffuse
  lighting without inheriting a glossy coat.

The global clearcoat option remains the coat enable switch. Its per-material
strength attenuates the underlying reflected/diffuse energy. Reflection
brightness scales reflected linear energy independently of angular response.

## Probe filtering and matte compatibility

Generate two views of the same bounded 64-pixel, seven-level filtered cube:
the existing encoded average for the tuned matte response, and linear-light
averages for the new layers. Decode before accumulation in the second view.
Original and legacy materials retain the unmodified raw source cube.
Both views share dirty/source replacement/restoration handling and a compute
dispatch. Storage is about 0.5 MiB. The new layered family adds filtered lookups;
no claim of free rendering cost is made. Compute-limited devices use legacy.
The earlier matte roughness (0.52), coat strength (0.25) and reflection filtering
stay intact; corrected indirect lighting also applies to matte.

## Evidence and validation

Installed material names and constants are read through the existing readers
[file]. Layer selection and tuning are enhancement choices [rewrite].
Mathematical reference: [Filament material theory]
(https://google.github.io/filament/main/filament.html), sections 4.2, 4.4, 4.9,
5.3 and 5.6. Equations are independently implemented; no code is imported.

Validate both native backends: zero-light black, indirect colour retention,
unchanged direct diffuse, linear probe energy, glass tint/grazing response,
coat isolation, stable flakes, distinct paint layers, legacy toggle restoration,
and Original family pixel equality. Installed tests must verify material and
surface routing, paint preview validation and resource lifetimes. Compare matched captures
in sunlight and shade. Outdoor probes cannot establish complete tunnel/weather
lighting parity or compensate for missing environmental illumination.

## Coloured body chrome

Custom chrome body paint uses `TintedChrome` independently of bare-metal `Conductor`
rims, trim and brake discs. Its reflection F0 mixes 60% linear composite skin colour
with the neutral metal baseline; bare metal retains its 35% mix. The body finish reads
the paint/vinyl texture before authored diffuse ramps, which can be black on chrome.
There is no added diffuse pigment layer: selected colour and vinyl contrast modulate
the conductor reflection. Original PC shading and disabled material response retain
their existing equations. GPU coverage checks saturated colours with zero authored
diffuse, composite stripe contrast and the absence of invented light in a black probe.
