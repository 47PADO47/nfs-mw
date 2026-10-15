# Graphics lab prototypes

Press **F8** to open the resizable graphics lab. It frees the mouse and takes
input like the F6 garage; no modifier needs to remain held. **F8**, **Esc** or
the close button returns to driving and restores the prior mouse capture.
Opening F6 or the console transfers input ownership to that panel.
Changes preview immediately. Save profile writes the owned graphics request.

The menu and overlay share one persistent graphics request. Choose **Original PC
(incomplete)**, **Enhanced** or **Custom** as the visual style. Quality buttons
**Low (1)**, **Balanced (2)** and **High (3)** change cost without resetting the
colour grade, tone mapping, AA or gameplay preferences. Original PC selects the
available original midday treatment and authored overlays; it does not claim
complete original materials, lighting, shadows or weather. Enhanced preserves
the existing modern art baseline. Manual edits show Custom while keeping the
base style for later quality changes.

F8 has **Look**, **Lighting**, **Materials**, **Effects**, **Environment** and
**Diagnostics** tabs. Look exposes Off/Subtle/Original treatment and a continuous
strength override. Lighting contains the existing shadow/ambient controls;
Materials contains car shading/reflections; Effects contains AO tuning.
Diagnostics lists the source, implementation coverage and missing work for each
feature. Backend downgrades are displayed without erasing stored requests.

**Save profile** persists owned graphics fields, **Load saved** reloads them,
**Restore startup (0)** reverts the initial request, **Reset section** resets only
the selected graphics domain, and **Store A / Swap A/B** compares snapshots.
Environment overrides remain independent. Freeze time and surface-debug state
are session diagnostics and are not saved. Existing configuration without the
versioned appearance table keeps its previous settings; no style is auto-applied.
The Video menu offers the same style/quality selectors, native enhanced sun
shadows and an **Advanced Graphics** action opening F8. AO intensity/comparison
live in F8. The overlay consumes input before the menus can process clicks.

Quality budgets are 1024/120 m, 2048/200 m and 4096/320 m for Low/Balanced/High
shadow maps/range, with progressively higher reflection/AO budgets. Quality keeps
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
| Sun shadows | Three texel-snapped, overlapping directional cascades with 5×5 tent PCF |
| Distant coverage | A fourth cascade extending coverage to three times the near range |
| Contact-hardening sun | Optional directional PCSS: sixteen blocker samples and spatial filtering |
| Contact shadow detail | A short, twenty-four-step depth/normal ray march, applied to direct sunlight |
| Original shadow overlays | Shows or hides the game's separate `SHD_` scenery meshes, including in reflections |
| Car clearcoat experiment | An approximate Fresnel/GGX highlight and environment coat over existing glossy materials |
| Car sky and ground light | Diffuse illumination projected from the current environment cube; replaces approximate fill/back lights |
| World ambient lighting | Preserves baked shade when adding dynamic shadows, with shade-fill and ambient-colour controls |
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
Visual Treatment. Native Vulkan/DX12 now support genuine **SMAA 1x**, using the
MIT-licensed [smaa-rs](https://codeberg.org/fintelia/smaa-rs) dependency (0.21.0).
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
sampling at cascade edges. Depth centres are snapped as well as the two map axes.
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

F8 exposes the active world's hour, Dawn/Morning/Noon/Dusk/Night shortcuts,
clock rate and Freeze time (the same clock as Freeze sun). Changes apply live,
including through the pause menu, without advancing physics. Manual time
changes refresh headlights, lighting and car reflection captures. Time and rate
are session-local. Clear/Rain and rain amount use the existing rain settings;
rain eases during simulated time and remains paused with the game. Quality
profiles do not change the chosen hour, rate or weather.

The clock is a custom host feature, not original-game day/night parity. Night
currently holds dusk lighting; the sky and street lights are not time driven.
Rain uses the existing drop pool; regional storm selection and cloud animation
are not implemented. No new weather assets or speed-line effects are added.
