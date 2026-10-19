# Graphics lab prototypes

**Diagnostics → Performance trace** records general CPU/frame/GPU observations.
**F9** starts/stops recording; **F10** bookmarks a hitch. Choose a capture duration,
copy the saved JSON path and open it in Perfetto. See [profiling.md](profiling.md)
for measurement scope, console commands and how to interpret delayed GPU samples.

Press **F8** to open the resizable graphics lab. It frees the mouse and takes
input like the F6 garage; no modifier needs to remain held. **F8**, **Esc** or
the close button returns to driving and restores the prior mouse capture.
Opening F6 or the console transfers input ownership to that panel.
Changes preview immediately. Save for startup writes the owned graphics request.

The menu and overlay share one persistent graphics request. Choose **Original PC
(incomplete)**, **Enhanced** or **Custom** as the visual style. Quality buttons
**Low (1)**, **Balanced (2)**, **High (3)** and **Very High (4)** change cost without resetting the
colour grade, tone mapping, AA or gameplay preferences. Original PC selects the
available original midday treatment, single-map projected shadows and authored overlays; it does not claim
complete original materials, lighting, shadows or weather. Enhanced uses the same authored world materials with optional modern lighting. Manual edits show Custom while keeping the
base style for later quality changes.

F8 has **Video**, **Look**, **Lighting**, **Materials**, **Detail**, **Traffic**, **Effects**, **Environment** and
**Diagnostics** tabs. Look exposes Off/Subtle/Original/Reference (mod) treatment and a continuous
strength override. Lighting contains independent shadow and car-light controls;
Materials contains car shading, texture filtering, car reflections and road
reflection detail/resolution; Effects contains AO tuning.
Diagnostics lists the source, implementation coverage and missing work for each
feature. Backend downgrades are displayed without erasing stored requests.

**Save for startup** persists owned graphics fields, **Revert saved** reloads them,
**Restore startup (0)** reverts the initial request, **Reset section** resets only
the selected graphics domain, and **Store A / Swap A/B** compares snapshots.
Save/load includes render scale, upscaling quality/sharpness and visual effect
preferences, alongside lighting, materials, treatment and HDR calibration. It
preserves requested values even when the active renderer uses a fallback.
Detail and Traffic controls are shared by both renderers and included in saved snapshots.
Video contains HDR output/calibration and post-process anti-aliasing; Look keeps
tone mapping, bloom and colour treatment. Video includes API (after restart), window mode, monitor, resolution, VSync, frame
limit, performance overlay, upscaler, temporal quality (Native selects DLAA with
DLSS), spatial render scale and sharpening. Native Streamline builds also expose
Reflex Off/On/Boost and hardware-supported frame-generation multipliers. Generation
requires VSync off and DLSS/FSR 3; HDR automatically uses HDR10, with the UI
submitted separately for recomposition.
Only supported upscalers are offered;
unavailable stored requests are retained and their fallback is shown.
Materials includes cubemap/mirror sizes, power-of-two faces, capture detail,
content, distance, reflection brightness and mirror tint. Effects also exposes
motion blur, smoke, skid marks, sparks, speed trails and exhaust flames. Detail
includes custom fonts and HD fonts/cursor/movie toggles. These controls participate
in saves, startup reverts and A/B snapshots. Particle and HD preferences survive
style/quality changes and section resets. Renderer selection, audio and other
gameplay preferences remain unchanged on load. API changes require a restart.
Older profiles leave controls they never stored at their current values.
Select a named preset to preview it immediately, including reselecting the current
name. **Save for startup** (Ctrl+S) makes the current request the startup default, while
**Revert saved** discards unsaved graphics edits. **Save as…** stores a named
snapshot and saves the same request for startup in one action; an existing name
replaces only that snapshot. The header reports unsaved changes and disk errors.
The independent F6 garage presets are unchanged. **Compare / experiments** holds
the session A/B tools and the older lighting experiments. Renderer downgrades
are listed in Diagnostics. Black Box controls are hidden on Bevy; the active
renderer decides the layout even when a different renderer is selected for restart.
Section resets and individual reset arrows use the selected style and quality.
Enhanced and the Black Box High/Very High presets use XeGTAO at 80% strength.
High/Very High preserve deliberately disabled AO; saved method/strength overrides
remain intact on load. Strength changes affect the composition, not the sample count.
HDR calibration has its own reset and survives resetting Look.
Environment overrides remain independent. Freeze time and surface-debug state
are session diagnostics and are not saved. Existing configuration without the
versioned appearance table keeps its previous settings; no style is auto-applied.
The Video menu offers the same style/quality selectors, native enhanced sun
shadows and an **Advanced Graphics** action opening F8. AO intensity/comparison
live in F8. The overlay consumes input before the menus can process clicks.

Road Reflection Detail selects Off, World, World + vehicles, or the same content
with the original two-pass blur. It requires native rendering and Original world
material support; unavailable controls explain the backend limitation without erasing preferences.
320×240 is the source image size; 640×480 and 1280×960 are independent enhancements.
These choices persist with Save for startup and participate in A/B/startup restores.
Environment offers Rain history, Dry, Damp and Wet comparison overrides. Overrides
are session-only, while natural dampness builds during rain and lingers after it.
Its 60 Hz reference recurrence is adapted to elapsed time, not rendered frames.
`NFSMW_ROAD_REFLECTIONS`, `NFSMW_ROAD_REFLECTION_RESOLUTION`, and the session-only
`NFSMW_ROAD_WETNESS` expose the same choices for controlled captures.

Quality budgets are 1024/120 m, 2048/240 m and 4096/320 m for Low/Balanced/High
custom shadow maps/range, with progressively higher reflection/AO budgets. The original perspective fit retains its .01 to 50 source frustum at every resolution. Quality keeps
the chosen lighting family and manual colour treatment. Original-style shadow
rendering remains incomplete; `SHD_` overlays are original scenery art rather
than an original dynamic-shadow pass. See [coverage](graphics-coverage.md) and
[the shared-settings contract](specs/graphics-profiles.md).

The preferred comparison may use a modded PC game for effects reproducible with
stock assets. Extra mod textures, shaders and packs are not runtime requirements.
Configured mod presence alone does not prove which effect rendered a frame.

The native Vulkan and DX12 renderer offers these independent prototypes:

| Control | What changes |
| --- | --- |
| Sun shadows | Original PC (partial) fitted perspective map, or custom directional cascades with 5×5 tent PCF |
| Distant coverage | A fourth cascade extending coverage to three times the near range |
| Contact-hardening sun | Optional directional PCSS: sixteen blocker samples and spatial filtering |
| Contact shadow detail | A short, twenty-four-step depth/normal ray march, applied to direct sunlight |
| Original shadow overlays | Shows or hides the game's separate `SHD_` scenery meshes, including in reflections |
| Car clearcoat experiment | An approximate Fresnel/GGX highlight and environment coat over existing glossy materials |
| Car sky and ground light | Diffuse illumination projected from the current environment cube; replaces approximate fill/back lights |
| Authored world materials | Shared world baseline; legacy prelit ambient experiments are internal fallbacks |
| Freeze sun | Holds sun direction and its lighting phase while driving; physics, traffic and other clocks continue |
| Surface debug | Sun visibility, ambient visibility or coloured cascade coverage, without colour effects |

The quick buttons are **Off**, **Sky**, **Sun**, **Far**, **Detail**, **Coat**.
Sky enables car and world ambient lighting. Sun through Coat also enable it alongside
their shadow experiments. AO remains an independent selection, with intensity and
split comparison in this overlay. The startup preset `max` selects 4096 shadow
maps, contact detail and clearcoat. Reflections, AO and post-processing are
independent settings; ACES, half-strength Visual Treatment, SMAA and bloom off
can be selected alongside it. The preset keeps ordinary brightness, penumbra
and AO-intensity defaults rather than maximizing
artistic strength sliders. Experiment presets do not change saved settings.
Map resolution, near range, filter width and contact reach can be adjusted live.
The sun presets start at **2048** pixels per cascade and hide original shadow
overlays to avoid overlapping the old static and new dynamic shadows. The Off
preset restores those overlays. **4096** is available for quality comparisons;
sun angular radius controls the PCSS penumbra. Vertex lighting remains intact.
**Store A** saves the current graphics selection, **Swap A/B** exchanges it with
the live selection, and **Restore startup** restores the initial graphics settings
including whichever prototypes were enabled at startup. Gameplay settings are excluded from these restores.

The same panel exposes existing car reflections, AO, tone mapping, bloom, AA and
Visual Treatment. Native Vulkan/DX12 now support genuine **SMAA 1x**, using `libs/blackbox-smaa`, our own WGSL
port that follows the MIT-licensed [smaa-rs](https://codeberg.org/fintelia/smaa-rs) crate (0.21.0) and uses the
pipeline cache.
It detects edges, computes spatial blend weights and resolves the current frame
before drawing the UI. It has no jitter, reprojection or frame-history input.
OpenGL retains its existing FXAA fallback. Native TAA requests report their backend downgrade; temporal AA is not implemented there.

## Pipeline and costs

The world supplies shadow candidates from all resident geometry, independently of
camera-visible colour instances. Each cascade then culls candidates against its
light-space volume and batches adjacent instances of the same mesh. Alpha-tested
casters respect both texture and vertex opacity; blended surfaces and sky do not
cast. Shadows attenuate scenery lighting and the main light and highlight of
glossy surfaces, while preserving fill and environment lighting.

Static shadow casters keep their most detailed available silhouette for their
resident lifetime. They do not use the colour view's projected-size thresholds,
missing coarse slots or camera-frustum culling. Prop hierarchies use their current
physical model. Authored shadow overlays never enter the physical caster list.
This can draw more triangles than the former view-dependent caster LOD policy;
the cascade volume still culls each mesh. Streaming residency remains the limit.

Cascade extents use minimum enclosing spheres and snapped light-space centres.
Sixteen texels of additional boundary coverage protect snapping, PCF and PCSS
sampling at cascade edges. Depth endpoints include upstream resident caster bounds
and are quantised in world space without changing XY texel density. A continuous
daylight basis avoids the former near-zenith axis switch. Nearby resident tiles
survive custom shadow zone transitions within a bounded retention range; this
does not request never-loaded shadow-only tiles. See [shadow residency](specs/shadow-residency.md)
and [custom shadow priorities](shadow-followups.md).
The first two splits receive eight and twenty-eight percent of the near range,
concentrating resolution near the player. Receiver-plane depth adjustment follows
fragment derivatives of actual geometry, while authored smooth normals still
drive surface lighting. A smoothly regularised slope replaces the former hard
angle cutoff; hardware PCF's half-texel footprint receives a matching bias.
PCSS compares blocker depth at the sampled texel centre. This reduces false
self-shadowing on slopes without requiring image history. PCSS estimates caster/receiver separation in
world units; the resulting penumbra has a bounded filter radius and a fixed
spatial sampling pattern. The blocker-distance concept follows
[NVIDIA's PCSS description](https://developer.nvidia.com/gpugems/gpugems3/part-ii-light-and-shadows/chapter-8-summed-area-variance-shadow-maps).
The last
20 percent blends into the next cascade; the final cascade fades to full light.
At the default near range of 160, far coverage fades between 384 and 480 world
units. This replaces an abrupt range boundary with a controlled transition.
The fitting concepts follow Microsoft's
[cascaded shadow map guidance](https://learn.microsoft.com/en-us/windows/win32/dxtecharts/cascaded-shadow-maps).

Depth map memory is 4, 16, 64 or 256 MiB at 512, 1024, 2048 or 4096 pixels per cascade.
Disabled sun shadows use a tiny placeholder and skip all shadow draws. Changing
resolution reallocates the atlas and rebinds materials, so a short toggle-time
hitch is possible. Steady frames reuse the maps, instance buffers and pipelines.
The displayed shadow setup time measures **CPU command preparation**. Optional
hardware timestamps report GPU elapsed time for the main frame, sun maps and
shared geometry/contact and AO passes. Three asynchronous readback slots skip samples when busy and
never block rendering. The main-frame interval includes post processing, SMAA
and UI, but excludes environment captures and diffuse probe updates submitted before the main frame.
It also excludes presentation/driver frame limiting. Timestamps can be unavailable
on a backend; FPS is not a substitute for them. FPS and frame-time readings
include streaming and gameplay costs.
SMAA uses separate spatial edge and weight targets and additional queue submissions.

AO and contact shadows share one camera depth/normal prepass when either is enabled.
At native texture LOD, the colour pass also loads that depth. Identical invariant
clip positions and cutout tests let it reject hidden surfaces before shading,
without changing coverage or introducing frame history. Mip-biased upscaling
retains separate depth because its colour alpha test can choose a different mip.
Reflection captures always clear their own depth. Sharing saves four bytes per
scene pixel (31.6 MiB at 3840x2160) in addition to reducing hidden shading.
The prepass and colour pass share one instance upload. AO caches immutable bindings
until an input view changes, even when a replacement has the same dimensions.
Sun-cascade preparation computes caster bounds once per frame and reuses scratch
storage; it retains the previous intersection tests and caster order.
Spatial GTAO resolves an R8 ambient visibility mask before colour shading. It affects
ambient/fill lighting, preserving direct sunlight, highlights and reflections; fog
is applied afterward. Blended surfaces do not consume the screen-space AO mask.

Car ambient lighting projects the current environment cube into nine diffuse spherical
harmonic coefficients for each hemisphere. The upper hemisphere supplies sky and
surrounding light; the lower hemisphere supplies road/ground bounce. Separate strength
sliders control them. The projection runs once when the cube changes, using 384 fixed
direction samples, stays on the GPU and uses 288 bytes of coefficients. It reuses the
existing local reflection capture and falls back to the static environment when those
captures are off. Paused captures keep their probe; toggling sky light on refreshes a
dirty probe before rendering. The mathematical method follows
[NVIDIA's irradiance environment-map discussion](https://developer.nvidia.com/gpugems/gpugems2/part-ii-shading-lighting-and-shadows/chapter-10-real-time-computation-dynamic).
No pixel history, TAA, animated sampling or CPU readback is used.

World ambient calibration retains the authored prelit output in unoccluded
regions. The shade-fill control reserves at least 60 percent of that illumination
for ambient light by default, increasing smoothly to 90 percent for already dark
vertex lighting. AO attenuates the ambient share; additional sun shadows attenuate
the remainder. A bounded tint from the scene's time-of-day ambient colour applies
only to additional shade, weighted by surface orientation. Ambient-colour strength
defaults to 0.35. This is an artistic estimate of baked illumination, not recovered
albedo, an unbaking process, new bounced-light tracing or a full PBR conversion.

Surface debug shows masks before fog and bypasses Visual Treatment, bloom, tone
mapping and the post-chain upscaler. The final spatial SMAA pass and UI remain.
Returning debug to Off restores the selected effects without changing settings.
White in a visibility view means unoccluded, black means occluded. The cascade
view colours the four regions red, green, blue and yellow, including their blend
bands; dark grey marks uncovered regions. Sky art is excluded from surface masks.
Freeze sun resumes from the held phase without catching up elapsed frozen time.

## Prototype limits

- Distant coverage uses geometry already resident in the streaming system. It
  does not load an entire shadow region or solve all scenery streaming pop-in.
- Prelit art retains its authored vertex lighting and brightness cap. Its direct
  versus ambient split is an approximation, rather than recovered material data.
- Contact detail only sees visible depth and can miss offscreen occluders. It
  rejects coplanar samples, unlit normals and excessive depth thickness, and
  defaults to a short 0.35-world-unit reach. A dedicated depth/normal prepass and
  R8 contact visibility mask add five bytes per scene pixel when sharing main depth,
  or nine with separate depth, plus additional geometry
  work. Direct lighting consumes the mask before fog; ambient, fill and environment
  light stay intact. Combining it with the sun map uses minimum visibility to
  avoid multiplying the same occlusion twice. It starts disabled in the Far preset.
- The diffuse probe shares the reflection capture's origin, visibility coverage
  and refresh schedule. All glossy objects use that one local probe. It is broad
  environmental illumination, not a spatial probe grid or traced multi-bounce GI.
  The capture and car shaders retain the original artistic display-space colour
  convention; this is not a conversion of all materials to physical HDR lighting.
- The original-overlay toggle removes only identified shadow-only models.
  Lighting embedded in ordinary textures or vertex colours is still present.
- Clearcoat layers an approximate lobe onto existing glossy materials; it is
  not a complete physically based material conversion. Glass and rims also use
  the glossy pipeline, so the coat applies there too.
- No new ray tracing, virtual shadow maps or GI probe system is included. The
  previous experimental renderer branches remain independent.

For automated comparisons, `NFSMW_GRAPHICS_LAB=baseline|ambient|shadows|distance|detail|modern|max`
selects a preset at startup, and `NFSMW_LAB_OVERLAY=0` hides the panel. These
developer switches do not extend the saved settings schema.
`NFSMW_LAB_FROZEN_SUN=1`, `NFSMW_LAB_DEBUG=sun|ao|cascades` and
`NFSMW_LAB_WORLD_AMBIENT=0` provide deterministic diagnostic startup overrides.

## Time and weather

F8 offers Original cycle, Fixed phase and Custom clock. The ordinary original
sun traverses phases .05 to .95 in 18 minutes each way and never sets. Fixed
phase locks its raw input for comparisons. Custom clock retains hour shortcuts
and rate control; **Custom night (dusk lighting)** is an optional clock extension.
The ordinary original cycle does not reach full night. Freeze time works with each mode.

Sunny/Cloudy/Overcast supply immediate comparison cloud amounts (0/.5/1).
Location restores the installed spatial/rain target and easing; Settle clouds
jumps to that target. Readbacks show phase, sun direction, dusk blend, actual
and target cloud amounts, and rain intensity. No rain does not force sunny
lighting. The rain slider adjusts the existing custom particle density.

Selections work while paused without advancing physics, invalidate cached
reflections and remain session-only. Regional storms, wet-road history/planar
reflection, tunnel exposure and wind/splashes remain missing original behavior.
A full-night renderer would be a separate custom feature, not ordinary PC parity. A
modded reference may change authored lighting colours or restore persistent
reflections independently of these inputs. Treatment Reference (mod) changes
only its six grading parameters; it does not replace the installed TOD data.

## Authored world rendering

F8 Materials includes **Original world materials**, also selected by the
Original PC profile. This restores distinct ordinary world, tree, unfogged and
prelit shading. Native Vulkan/DX12 also restore normal-mapped/reflective dry-road
lighting and architectural glass with its stock global reflection texture, plus the camera-centered
three-layer PC sky. Enhanced shares these authored materials; custom lighting
and quality controls operate on that common baseline.
World atmosphere now reads authored fog scalars and spatial cloud blending.
Wet-road projected capture/history, complete projected shadow classification/filtering, scripted weather and full scene equivalence remain
open; see [coverage](graphics-coverage.md). Bevy supports the four surface families
and keeps its legacy sky until that backend implements the layered sky interface.

Car shading in Materials offers Original PC, Original PC (shadowless), Custom
Glossy and Simple. The two original choices reproduce distinct PC material
families on the native renderer; choosing one does not enable original projected
shadows. Original PC style selects the regular family. Graphics quality preserves
the selected family, and Save for startup persists it. Bevy reports its custom shader
fallback explicitly. See [graphics coverage](graphics-coverage.md).


## Active material dependencies

F8 greys out car clearcoat and sky/ground lighting unless Materials selects
**Custom Glossy**. Original PC, Original PC (shadowless), and Simple bypass
these added terms. Sky strength and ground bounce also require the sky/ground
toggle. Stored requests remain unchanged for switching back.

Legacy world ambient, shade fill and ambient colour requests are retained only
for internal prelit fallbacks. Their F8 controls are retired. Authored families
supply their own ambient terms on the shared material baseline.

Contact detail remains available as a custom screen-space enhancement on
supported direct-lit world surfaces. It also works without a sun map; only
its reach slider requires the contact toggle. Original PC cars bypass contact
when using the original projected map, and shadowless original cars never
receive it. With custom cascades, Original PC cars use the ordinary visibility
path and can receive contact detail. F8 explains this scope instead of disabling
world contacts based only on the selected car family. Bright flat receivers or
missing offscreen occluders can make a short reach visually subtle.

Authored world materials are the common style baseline. Legacy material profile
requests migrate on load while other tuning is retained. The old world-material
and prelit ambient experiment controls are retired; simple shading remains an
internal fallback. Background panorama scenery remains visible but does not cast
sun shadows. See `specs/scenery-shadow-casters.md`.

## HDR output

Video offers Auto / On / Off display output and independent peak, scene-white and
UI-white calibration in nits. Auto requires positive OS HDR confirmation and a
supported native scRGB surface; other outputs retain SDR with an explanation.
HDR uses pinned PsychoV and preserves extended highlights through material and
treatment output. The SDR tone-mapping choice is stored separately and restored
when HDR is disabled. Save for startup, A/B and startup restores include calibration.
PNG screenshots are SDR previews. See [the HDR contract](specs/hdr-output.md) and
[PsychoV provenance](provenance/psychov.md). This is a remake enhancement.

## Revised quality budgets

All quality levels retain 16× anisotropic filtering, the selected colour treatment,
AA, HDR calibration and gameplay settings. Quality does not switch AO on if it
was deliberately disabled. High uses wide 4K cascades and dense penumbra filtering;
Very High raises the first two maps to 8K while retaining 4K distant maps. Its
640 MiB atlas and higher shadow time are explicit costs, not a different art style.
Balanced uses 4K detail / 2K distant maps. Low uses regular 1K maps without PCSS
or contact detail. Legacy tight focus is disabled by presets. Road capture sizes
are original / 2× / 2× / 4×; reflection content remains independently selectable.
Enhanced now defaults to SMAA, XeGTAO with depth prefiltering, 0.55° sun, filter
width 1.0 and the tested car sky/ground lighting. Bevy quality changes only shared
budgets and preserves Black Box experiments. Original PC retains its projected
shadow family and independently selected road captures.

## Resolution and performance readings

**Game / output resolution** is the final image and UI size. **Desktop / monitor
native** follows the display mode; the live pixel dimensions are shown separately.
**Internal 3D render** is the scene size before upscaling. Temporal **Native**
quality renders at output size (DLAA with DLSS); internal render scale controls
spatial upscalers. Changing internal resolution leaves output/UI resolution intact.

The performance overlay shows measured **Base / rendered** FPS and **Presented**
FPS. Base counts successful main renders; presented uses actual Streamline
presentation counts when available, including generated frames. It never multiplies
base FPS by the requested generation mode. Missing presentation data is explicit.
These are rolling rates, not a monitor scanout measurement.

Advanced metrics keep base FPS separate from SDK-counted presented FPS. CPU
elapsed update/submission time and GPU base-render timestamps remain diagnostic
spans, not utilization: submission can block, and the GPU span omits auxiliary
views, present-time FG and the DX12 presentation copy.

On Windows, an owned background worker samples the renderer-owning thread's
kernel/user CPU time and this process's `GPU Engine(*)\\Utilization Percentage`
PDH counters once per second. The busiest GPU engine across adapters is shown;
overlapping engines are not added. This includes this process's SDK and auxiliary
queues, so frame generation is covered without assuming a multiplier or deriving
GPU load from base-render timestamps. It is engine scheduling activity, not
shader occupancy, and includes no other process's GPU usage. The owning thread's
percentage includes driver CPU execution and excludes blocked/sleeping time;
CPU worker bottlenecks can therefore remain uncertain. The worker never waits
on a graphics fence. Headless renderers do not start it. Other platforms and
missing counters retain unavailable status, never fabricated idle values.

The bottleneck estimate uses fresh OS samples (at most 2.5 seconds old): at least
90% on a GPU engine indicates GPU pressure; at least 85% on the main thread with
less than 75% GPU activity and little explicit pacing/wait time indicates likely
CPU-main-thread pressure. Significant waits/pacing suppress CPU attribution even
when a driver busy-spin consumes CPU time. Both busy
means mixed pressure. A reached base-frame cap takes priority. Pacing and waits
are reported when neither processor is proven saturated; Reflex sleep alone is
not a CPU/GPU diagnosis. Each label has a profiling hint. Let the counters settle
for a few seconds after changing settings. No unsaturated engine or CPU worker
is guaranteed to have unlimited throughput; use a CPU/GPU profiler for uncertain
cases and compare FG off to isolate generation cost.

API references (implementation is original): Microsoft's
[GetThreadTimes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getthreadtimes),
[PDH formatted arrays](https://learn.microsoft.com/en-us/windows/win32/api/pdh/nf-pdh-pdhgetformattedcounterarrayw)
and [Task Manager busiest-engine convention](https://devblogs.microsoft.com/directx/gpus-in-the-task-manager/).
