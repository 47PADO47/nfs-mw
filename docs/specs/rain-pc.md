# PC precipitation and camera water

Sources: read-only analysis of the original PC executable (proprietary), SHA256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
The [CC0 reference](https://github.com/dbalatoni13/nfsmw) supplied class names only;
the PC instructions and shader resources supplied the behavior below. No code or assets are copied.
This corrects the primary-pool, base-roll and missing-thunder conclusions in `weather.md`.

## Falling rain [decomp]

The primary pool contains 350 points, not 200. The separate 200-point pool draws
rain curtains near sheltered regions. The two must not share their counts or tuning.
The primary texture is `RAINDROP`, loaded from the installed global texture packs.
Each point selects one of ten horizontal atlas cells, using its index modulo ten.
The original vertex color is neutral RGBA (128,128,128,128).

The camera-relative primary box is 20 units along the horizontal view direction,
7 across it, and 6 vertically. Its forward offset is 8 units. World vertical is Z.
Ten shared velocity variants have horizontal components in [-0.01,0.01] and vertical
components in [-0.38,-0.35] per original update. Wind contributes with a factor of 0.2.
The original stretches camera-facing quads along particle motion, using a half-width
of 0.01 and length term 0.45 plus camera motion. Active population follows intensity.

An elapsed-time port should retain the 60 Hz reference speed, rather than making
rain fall faster on a high-refresh monitor. Simulation pauses must not advance it.
Teleports must reseed around the new camera rather than leave rain at the old location.
Shelter classification is spatial and height aware. Rain outside a tunnel must remain
visible from inside it; suppressing all distant rainfall is not curtain parity.

## Regional weather [decomp]

The `Rain` parameter-map layer has intensity in field 0 and comparison threshold in
field 1. Rain is selected when the gate does not exceed the threshold. Forced rain
overrides the regional selection; zero amount disables precipitation.
The original gate converts `(0.71428573*t + 0.02*x)*65536` to an integer, divides by
360, then evaluates a 16-bit cosine. The result is `1-max(cosine,0)`.
Free roam uses t=1; other gameplay modes use the original wrapping clock. Earlier
notes omitted FPU operands and incorrectly reduced this gate to zero.

Original rain easing is dt/1, snapping dry below 0.01. Elapsed-time integration may
use the equivalent 60 Hz recurrence for stable high-refresh behavior. Road dampness
has its own slower recurrence and must not be replaced by the visible particle count.

## Camera droplets [decomp]

There are 20 independent screen droplets in ordinary single-player views. Shelter
or zero intensity disables them. Their initialization uses random lifetime [0.3,5.3],
size [0,1] and downward speed [0.02,0.12]. Expired drops restart at random UV positions.
Positive lifetime decreases with simulation time. Y advances by `0.2*speed*dt`.
Camera velocity contributes a horizontal outward drift from UV (0.5,0.1), scaled
by 0.0001. The original applies that velocity contribution per update, so a 60 Hz
elapsed-time adaptation must be explicit. The four-shape counter is not supplied
to the ordinary PC draw, so do not invent an animated texture atlas for it.

The displacement texture is `RAINDROPPC`, distinct from the falling-rain atlas.
The PC pixel program samples its green/blue channels, scales and offsets those to
scene UV, then samples scene color. Its RGB is multiplied by 1.2. Alpha is texture
alpha times the droplet decay multiplier (3 times remaining/max lifetime).
The nominal screen size is randomized size times 0.12; the draw uses a quarter-size
offset in its displacement coordinates. Draw after scene processing, before HUD.
The lens effect belongs only to the player view, never cubemaps or road captures.
PC draw disassembly resolves the exact convention: for d=size*0.12, the normalized
quad spans `(x,y-d/2)` to `(x+d/2,y)`. Texture V increases upward. The displacement
transform is `(x-d/4,y+d/4,d,-d)`. Thus neutral GB=(0.5,0.5) samples the quad center,
not its random initialization anchor. These values supersede a centered/aspect-correct
quad approximation.

## Rain sound [decomp]

`CARSFX_Rain` uses the installed `FX_Rain_MB.abk` bank and `FX_Weather` class.
Member order is hood flag, hood-volume subtraction, volume, width, pitch offset,
rain-off flag, low-pass, high-pass, dry filter, wet filter. Initial filter values
are 25000,0,32767,0. While active, width is 7000, pitch offset and hood subtraction
are zero. Hood and bumper views select WEATHER output 1; other views select output 0.
The rain object (11, distinct from weather-wind object 12) input 2 is 32767
while the module is active and not fading. The output slots above belong to object 11.
Intensity above 0.01 starts playback. Falling below that threshold sends rain-off=1;
the original retains the module for a 10-second release. Do not substitute generic
generated white noise for the installed authored module.

Stock PC also queues distant/close thunder moment streams below/above intensity 0.5.
The intensity threshold is 0.25; subsequent delays are 25 plus random [0,20) seconds,
with streaming/speech guards. Static presence does not establish audible runtime parity.
The installed selector masks each permit nine thunder recordings. The port selects
among matching recordings with its own random sequence; it does not claim to match
the original variation order. Existing single-recording moment callers retain their
strict ambiguity checks.

## Remaining confirmation

The shelter-bound curtain transitions, tire water emitter selection, exact camera
quad orientation and stock in-game audio mix still require comparison. Neither unit
tests nor offscreen rendering establish full original-game visual or audible parity.

### Shelter curtain geometry [decomp]

PC helpers 0x723330/0x722e90 select a boundary edge of the active enclosure polygon,
then another edge with the most parallel direction. The initial query is ten units
ahead of the camera. Each frame selects the near or opposite boundary according
to view direction relative to the followed vehicle. The existing PC path only draws
this curtain for polygons with fewer than six vertices. The 200-point curtain
fills an edge-width by ten-unit outward by ten-unit vertical volume, with its base
at the followed vehicle's height. Its outward direction points away from the zone
bounding-box center. The atlas and streak dimensions are shared with primary rain.
The remake uses the camera position as the height anchor for free-camera views;
the exact original camera-mover selection and transition hysteresis remain unverified.

## Tire surface water [decomp]

The CC0 reference's `CarRenderConn.cpp` and `MWAttribUserTypes.h` describe the
surface dispatcher separately from the primary rain emitter. Each simsurface
contains `TireDriveEffects`, `TireSlipEffects`, and `TireSlideEffects` arrays.
A `TireEffectRecord` is 20 bytes: a 12-byte emittergroup reference followed by
float minimum and maximum speeds. Index 1 selects wet weather when present;
index 0 is dry, and index 2 takes priority for a flat tire when present.
Missing wet variants fall back to the surface's dry record, not another material.

Each tire has independent persistent groups. Drive input is car speed; slip and
slide use the physical tire's longitudinal slip and lateral speed. Group intensity
is the clamped ramp between the record's speed endpoints. A zero ramp disconnects
the group. The emitter frame uses the car's orientation and the tire's ground
position, and inherits the car velocity. Particle appearance comes from the
installed emitterdata profiles, including textures and animation. Do not create
arbitrary white spray or substitute collision sparks. This section's dispatcher
is reference-derived and still needs independent PC runtime comparison.


### Reference particle response [runtime, compatibility choice]

The preferred modded reference's installed shadowed particle program was matched
byte for byte at draw time. Its brightness constant was 2.0. It multiplies the
sampled texture times the vertex colour in all four channels, before blending;
this changes both radiance and opacity, not sprite size or emission rate.
The unshadowed program contains the same brightness multiplication, but its
runtime draw constant has not been measured. The observation covers the shared
particle program and is not an exhaustive identification of every tyre-water draw.

For the native tyre-water path use a brightness multiplier of two. Keep authored
colours, sizes, lifetimes and emission intact. Apply the multiplier after texture
sampling, preserving bright colour values and clamping opacity to [0,1]. Other
textured effects retain a neutral multiplier. This is a measured modded-reference
compatibility choice; stock-PC runtime calibration remains a parity boundary.

## Camera-water size control [enhancement]

Expose `rain_droplet_scale` in F8 Environment, persisted with graphics profiles.
Default to 0.5; 1.0 retains the reconstructed PC footprint, zero hides lens water
without changing falling rain, wetness, spray or audio. Accept finite [0,2].
Scale both destination dimensions and source sampling offsets about the same
centre, so changing size does not move the droplet or change its refraction ratio.
This mirrors the purpose of Widescreen Fix's `RainDropletsScale` setting;
its configuration documents size adjustment. It is an enhancement, not a claim
of pixel-identical Widescreen Fix geometry.
