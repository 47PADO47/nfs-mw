# Local tunnel lighting

F8 > Lighting > Advanced shadows > Local lamp shadows (prototype) controls streamed tunnel fixtures and player headlights.
These are enhanced artistic lights, not recovered original physical-light parameters.

## Fixture coverage

Night mode can additionally illuminate inspected StreetLight C, Cb, Cn, Cc, D, D2, Drift, I, ItalyJG, LightPost A/B, TrafficLight A/B, ParkLamp A and IronLamp hanging/standing
variants. Dual heads are split from their lens geometry into separate lights. Lens positions come from distinct mesh groups; placed props use their current pose and stop emitting
when hidden or substantially knocked over. Park and iron lantern illumination uses their mesh-derived heads;
ordinary road lamps retain their downward-facing lenses. Other streetlight models are not
converted. The fixtures share the existing light and shadow budgets.


Round TUNNELLIGHTA fixtures and LONG/RUNNING tunnel strips are recognized from resident solids.
Each placed instance supplies its transform. The emitter derives from the physical material group's
geometry; glow cards remain artwork. Combined strip meshes retain separate 66-index fixture blocks,
which are expanded into individual emitters. Baked strip normals are not consistently rotated;
those combined fixtures use downward illumination as an explicit artistic approximation.

All recognized fixtures in the loaded world are eligible. The fixed test coordinate is gone.
Up to 128 nearby lights are evaluated, within a configurable 80-600 metres of the main camera (default 160 m). The configurable
active-light limit includes player headlights. Lights fade over 32 metres near the distance/budget
boundary; stable instance identity breaks distance ties. Unrecognized lamp models remain unchanged.

## Appearance

Tunnel intensity and headlight intensity have independent persisted controls. Default tunnel output
is half the first prototype and headlight output is reduced. Streetlights have separate persisted strength and
reach controls (defaults 1.5 and 35 m). Light visibility distance and total shadow/light budgets remain shared.
Recognized street heads also draw depth-tested warm glow sprites at night; a lantern whose drawn model has emissive glass draws none, and other lanterns keep only the camera-facing glow (no lens panel). Their separate 256-sprite budget avoids
tying their appearance to the smaller physical-light budget. Overlapping local contributions
use a smooth compression curve to limit hotspots. Existing baked/prelit illumination remains.
The car-relative headlight upper fade spans roughly 6 to 17 degrees. Nearby traffic beams use the
same markers and damage flags; up to eight traffic cars can be selected (default four), within 150 metres, and fade near their selection
boundary. Local specular highlights are an artistic, shadow-aware approximation. No indirect bounce
is introduced. A receiver walks its light list once: the diffuse term, the highlight and a wet
road's glint share each light's cone, attenuation and shadow lookup (taken along the surface normal).

## Shadow controls

- Active light limit: 8, 16, 32, 64 or 128. Default 128.
- Shadowed lights: 0, 2, 4, 8 or 16. Default 8; player headlights and up to four nearby traffic beams take priority over tunnel fixtures. Additional traffic beams use remaining shadow slots after tunnel fixtures.
- Local shadow resolution: 512, 1024 or 2048. Default 1024.
- Local shadow softness: comparison filter radius, 0 to 3 texels.

The softness filter is a 3x3 tent of hardware bilinear comparisons spaced `radius` texels apart, weights (1,2,1)
per axis. It is taken as four bilinear comparisons: up to one texel the 3x3 pattern covers four texels per axis, and
each pair of texels becomes one tap at its weighted centre, so the result is the same filter (at radius 1 the per-axis
texel weights are (1-f, 2-f, 1+f, f)/4 for the subtexel position f). Above one texel the pattern spans more texels;
there two taps per axis at ±radius/√2 keep the old filter's variance, a smooth edge instead of the nine-tap
pattern's separate copies. Measured on an RTX 3070 at 2560x1440 (synthetic street, 64 lamps, 16 shadowed at 2048,
120-frame batches): 2.00 ms to 1.92 ms per frame dry and 4.57 ms to 4.48 ms wet at radius 1 (the filter's own cost,
against radius 0, from 0.30 ms to 0.22 ms); radius 2 and 3 from 2.01 ms to 1.85 ms. Radius 0.5 and 1 differ from the
nine taps by at most 3/255 and 1/255; radius 2 and 3 differ visibly on surfaces with shadow acne (by up to 12/255 and
23/255), and at radius 2 the four taps show less of that acne. With 8 lamps at 1024 no difference was measurable.
- Local shadow distance: 20 to 240 metres, default 80 m. Lights outside it remain unshadowed.
- Local highlight strength: 0 to 1, default 0.15. Zero skips extra specular shading.

Illumination continues when the shadow budget is zero. Unshadowed lights can illuminate through
occluding geometry; this is a quality/performance tradeoff, not indirect lighting. Nearby fixtures
get the remaining shadow slots; their shadow strength fades as they approach the budget boundary.
This smooths selection changes.

A light that keeps its projection from one frame to the next (a street lamp, a tunnel fixture, a parked car's
beam) keeps the depth of its resting casters in an image of its own. A frame copies that image into the light's map
and draws only the moving casters (cars, swaying trees, props in motion) over it; a map with no moving caster in its
cone is left untouched. The kept image is named by the exact casters, meshes, cutout textures and projection it holds,
so its pixels are those of a full redraw. A caster that starts or stops moving, or leaves, redraws it once. Moving
lights (headlights) redraw in full every frame, as do casters without a persistent key.
`BLACKBOX_LAMP_SHADOW_CACHE=0` disables the kept images for comparison; the "Local lamp shadows" trace counter
reports `reused_maps` and `kept_mib`.

Depth storage is count * resolution squared * 4 bytes for the maps, and up to as much again for the kept images
of still lights, plus instance buffers and other resources. Default eight 1024 maps use 32 MiB of map storage;
sixteen 2048 maps use 256 MiB. A disabled shadow budget uses only a tiny placeholder.
Raster local shadows reuse material/alpha coverage and dynamic caster poses. They do not require RT.

## Hardware RT

Hardware RT sun shadows are separate. Vulkan device creation must request ray-query support at
startup (BLACKBOX_RT_SHADOWS=1); the persisted sun and RT sun switches then control use.
The experimental launcher requests Vulkan RT and enables the saved sun-shadow switches.
Unsupported devices retain the existing capability gate. Local tunnel/headlight shadows remain raster.

## Checks

Recognized street fixtures use a lens-shaped glow, bright camera-facing core and halo. The inspected `_1B_` iron
(standing and hanging) and park lantern meshes and the `_1C_` iron lantern meshes instead emit from their actual glass
triangles, identified by the original texture's UV band; only that group draws a masked copy of the texture (full mip
chain, same texture-detail setting), and the rest of the mesh keeps its original material. The `_1Z_` iron and
`_1C_`/`_1Z_` park lanterns share pane and roof triangles, so they are not split; while the main view draws such a
LOD, the lantern keeps its glow sprite. The split applies only while street lights are enabled when the scenery loads.
Raster environment captures can include the emissive material. Ray-traced reflection hits do not yet evaluate
its emission. The F8
Street lamp glow control changes their appearance without changing emitted light. Their illumination and
glow switch off together when the placement moves or its scenery model is hidden or damaged;
restored placements regain their light. Detached fragments do not carry working lamps.
The bulb core can exceed display white in HDR scenes and feed the existing bloom. Bulb visibility
uses its own longer distance and budget, subject to scenery streaming and depth occlusion. Wet
surfaces receive an additional local-light highlight, including from lights outside the view.
An optional fog grid evaluates up to 64 local lights in shared camera-aligned cells. A screen-tile
light list bounds the work. Each grid column integrates scattering and extinction once; the
half-resolution pass looks up that result at scene depth. Extinction limits the scattered light within
the volume; the composite adds it to the scene without applying a second layer of atmospheric fog.
The quality options use 64, 96 or 128 depth slices, reducing flicker around small distant lamps when
the camera moves.
The covered track zones reduce weather-driven density inside tunnels, with soft portal transitions.
Selected local shadow maps also shadow the fog. Unshadowed lights can still leak through geometry.
Reprojected history is rejected on camera cuts, large camera movements and abrupt lighting changes; moving
lights are not guaranteed to be completely trail-free. Light selection crossfades at its budget boundary.
F8 controls strength, grid quality, light count and visibility distance up to 600 metres; zero strength
or count disables it. Fixture distance, the active-light limit and world streaming also limit coverage.
High settings can add substantial GPU work. The track-zone shelter approximation is limited to the
nearest 24 covered polygons with at most 16 vertices; it is not a complete geometry-derived weather mask.
Streetlight lens geometry is drawn into screen and environment captures, while camera-facing glare stays
in the main view. Hardware RT hit materials do not yet reproduce that lens emission.
The shared-medium design follows the general approach described in [Frostbite's volumetric
lighting presentation](https://www.advances.realtimerendering.com/s2015/index.html). Wet-road
highlights are informed by [artist-directable rain rendering](https://advances.realtimerendering.com/s2006/Chapter3-Artist-Directable_Real-Time_Rain_Rendering_in_City_Environments.pdf).
At night architectural windows suppress daytime reflection and in-scattering while retaining the
authored texture pattern and local light response. Daytime window shading stays unchanged.
Opaque apartment, hotel, city tower and numbered modern-building families fade toward a darker
nighttime response at distance; nearby lighting and daytime appearance are preserved. The existing distant
panorama response uses the same distance transition. Other building families are not classified.

Synthetic GPU tests cover moving blockers, exact off restoration, raised headlight receivers and
live shadow-budget/resolution changes with more than four lights. Installed checks cover all nine
recognized model variants, combined-mesh splitting and fixed-view GPU/capture comparisons.
A conservative 16 x 16 world-space light grid, fitted each frame to the selected light spheres (cells of at
least 4 metres), restricts per-pixel work to overlapping lights. It covers every light at any selection distance,
so receivers outside it test none, and surfaces skip local shading entirely when no light is selected. Streaming and the light limit
can still cause transitions in dense areas; increasing the limit is not a guarantee of zero pop-in.

At night, opaque unlit world surfaces and architectural glass are dimmed and can receive local light.
Blended unlit artwork remains unchanged, but opaque emissive artwork cannot be identified reliably
and may also be dimmed. No material albedo is reconstructed.

Night is an experimental F8 Environment switch. It reduces and cools existing world lighting,
darkens the enhanced sky (independently adjustable in F8) and suppresses sun glare while reusing the current sun direction. It enables
the local light options and turns on player headlights; the prior player switch is restored when
night is disabled. It does not provide a visible moon, stars, full city lights or remove baked daylight.

Settings round-trip checks cover the persisted local-light controls. Fixed-view timing is not a
4K driving-performance prediction. No frame-rate target is assumed.

## Full day/night cycle

F8 > Environment > Full day/night cycle enables an artistic 24-hour clock-driven cycle. Sun and
moon directions follow separate paths, and a night blend transitions world, car, fog and sky response.
A simple moon disk and direction-stable stars appear through the enhanced sky; cloud transmittance
occludes them. The sun and moon are artistic, not an astronomical simulation. Cached reflections
refresh in coarse clock increments rather than every frame. The original sun cycle is still selectable.
The clear-night sky uses a darker horizon gradient. Recognized panorama world surfaces retain their
daytime material and get additional night dimming; this does not identify every distant asset or emissive sign.

Automatic player lights change at dusk/dawn; manual changes are retained until the next transition.
Traffic beams fade with night. Tunnel lights remain available by day and night. The optional player
tail-light spill uses intact rear markers and follows engine, headlights and braking. It is unshadowed
and consumes the shared light budget. With enhanced lamp surfaces, the player's outer brake-lamp ON
textures act as running tails when available; the centre brake lamp still follows the brake.

## Police light-bar spill

F8 > Lighting > Local lights > Police light bar spill (off by default, persisted with a strength from 0 to 5,
default 1) lets the flashing roof bars of nearby police and roadblock cars light the road and the cars around them.
It is an artistic enhancement with untuned values; it was written without the game install and has not been seen
in the game or measured.

- What lights it: the bar's own siren lamp markers (`COPLIGHT*`) and the flash level the authored emergency flares
  already use (`car_light_flares::intensity::samples`, from the car's lamp state and lamp clock). Light and flare
  pulse together; there is no second timer. Only cars whose bar runs (`emergency_lights`) cast light. The
  simulation, AI, pursuit and audio state are only read.
- Shape: per car, one light per colour lit this frame (red, blue, white), not one per bulb, so at most three. It
  sits 0.15 m above the lit lamps of that colour (weighted by their level) and points down the car's own up axis in
  a wide cone (outer cosine 0.1, inner 0.55) with a 14 m reach, so the road out to about 14 m and the sides of
  neighbouring cars receive it. Its colour is scaled by the brightest lamp's level, the strength, and the same
  night fade as traffic beams.
- Budget: the four nearest such cars within 150 m of the camera, nearest first, faded over the last 24 m before the
  distance or count boundary like traffic beams. The lights are unshadowed (they never take a shadow slot and do not
  make the shadow casters needed), and come out of the shared active-light limit after the player's lamps and the
  traffic beams, before the fixtures.
- What it does not do: the flare sprites themselves are still not mirrored in screen-space road reflections. The
  moving coloured highlights on wet roads, puddles and car paint come only from the local-light diffuse, highlight
  and wet glint terms. Being unshadowed, the light can leak through geometry, and the bar's own car is lit from
  above.

Only the listed streetlight variants are covered. Park and iron-lamp fixtures were identified from the original installed meshes; their light parameters are independent approximations. Their physical lights and sprites can be disabled independently of tunnel fixtures;
not all original lamp artwork illuminates itself. Baked daylight, an incomplete streetlight catalog,
unshadowed-light leakage, light-budget transitions, and 4K driving performance remain limitations.

## Where the local-light time goes (4K, 2026-11-16)

Measured in game on an RTX 3070 (Vulkan) at 3840x2160, night rain at the street lamp (2298,3415), the user's night
settings (8 lamp shadows at 1024, tail lights, fog), by switching one part off at a time (median frame time, two
12 s benches each; base 27.9 ms before the wet-road change of `weather-gpu-tuning.md`):

| Off | Saving |
| --- | --- |
| The per-pixel light walk (`local_lights` returns at once) | about 1.3 ms |
| Only its lamp-shadow lookups | about 0.5 ms |
| Street lights (`local_street_lights = false`) | 0.85 ms |
| Car lights (head, tail, traffic) | 0.45 ms |
| Lamp shadows (`local_shadow_count = 0`) | 0.6 ms |
| Light fog (`local_light_fog = 0`) | 0.7 ms; by timestamp 0.62 ms: volume build 0.13, half-resolution integrate 0.06, full-resolution composite 0.17, the rest tile masks, column integration and uploads |
| Street haze volumes | nothing measurable |

`local_lamps = false` alone does not switch the lamps off at night with the night lighting on, and cutting the light
limit to 16 or the fog count to 2 changed nothing: few lights reach any one pixel. A street-light reach of 20 m
instead of 35 m saved 0.5 ms, so the cost is the lit area times the lights on it, not the light list walk.

Tried and not kept, as they did not pay: leaving the walk as soon as a light's cone is zero (no change), skipping the
shadow lookup when a light's contribution is under 0.002 (0.15 ms, within the noise, and it changes how lights
outside their shadow frustum are culled). Fog quality 1 instead of 2 changed nothing measurable. No change was made
to the lamps.
