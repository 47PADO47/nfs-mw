# Vulkan RT sun shadows

Start with `BLACKBOX_RT_SHADOWS=1` (or the existing `BLACKBOX_RT_REFLECTIONS=1`)
and select Vulkan. F8 Lighting exposes **Hardware RT sun shadows**. It is off by
default and requires Sun shadows and Custom cascades. Unsupported backends keep
cascaded shadows. The reconstructed Original PC projection stays independent.

The main view traces deterministic sun visibility per visible depth/normal receiver
within 100 m of the camera. Hard mode uses one central ray; soft mode averages
4, 8 or 16 spatially decorrelated, stratified samples across the angular sun disc. The outer 20% blends back to cascaded shadows.
RT replaces near visibility, rather than multiplying an already shadowed map.
Only direct-light visibility changes; ambient lighting remains independent.
Spatial reconstruction follows the actual receiver plane and projected world-space
penumbra. Four rays are the normal quality target. Ray Reconstruction is not used.

The existing shared reflection BLAS/TLAS provides geometry, with separate instance
masks: reflection rays see reflection instances, sun rays see shadow casters.
Shadow candidates come from the existing dedicated caster feed, including off-camera
traffic, the player and props. A conservative sunward swept receiver sphere includes
blockers up to the configured reach. Instance cast flags and original shadow-overlay
exclusion remain in the caster feed. The fallback without a dedicated list uses the
main instances. Geometry beyond current streaming residency remains unavailable.

Cutouts follow shadow-map material rules: ordinary texture times vertex alpha,
tree/window texture alpha, doubled prelit alpha, and opaque reflective normal materials.
These family rules apply when authored world materials are enabled. Shadow casters
are two-sided, matching the raster depth pass. Reflection-sidedness stays independent.
Incomplete hit storage or instance capacity falls back to cascades instead of
assuming unobstructed sunlight. Freed meshes invalidate shared RT storage.

Advanced controls:

- Soft RT sunlight: optional finite-sun integration. Sun radius is shared with PCSS.
- RT mask resolution: 50% or 100%, default 50%. Hard shadows stay full resolution.
- Sun rays per pixel: 4, 8 or 16, default 4. Hard mode always uses one ray.
- Two-frame stabilization: optional, motion-reprojected blend of the current and
  previous **unblended** spatial estimates. The result never feeds back into history.
- RT spatial smoothing: 0–2, default 0.75. Zero disables the additional filter pass.
- Receiver range: 10–240 m, default 100 m; larger areas trace more pixels.
- Blocker reach: 40–800 m, default 300 m; larger reach includes more geometry.
- Surface bias: 1–50 mm, default 6 mm; excessive values detach contact edges.

Controls persist with F8 Save for startup. The existing Sun visibility diagnostic
shows the selected RT/cascade result. Disable RT to compare against cascades.

## Limits

Hard mode preserves the first milestone exactly. Soft mode produces contact-hardening
from actual sun-disc visibility, with a small depth/normal-aware spatial filter.
The float intermediate retains blocker distance. Smoothing projects a world-space
penumbra kernel onto the receiver plane, protecting contact edges and surface
discontinuities. Unlike camera-depth rejection, coplanar sloping roads remain valid.
The cascade replacement weight is preserved.

Optional stabilization uses two adjacent unblended
spatial estimates, with a maximum 50% previous-frame contribution. Object/camera
motion reprojects the previous surface. Camera cuts, setting changes, large sun
changes, invalid motion, mismatched normals/planes and large visibility changes
reject history. Current-neighborhood clamping limits smaller changes. Fully cleared
regions clear immediately. No resolved estimate is recursively accumulated, so an
old shadow cannot persist through many frames. Motion vectors are generated when
needed even if AO is disabled. This is not NRD SIGMA. Some grain, thin-edge aliasing
and one-frame differences remain possible.

Soft rays cost more than one hard ray; no performance improvement is promised.
The sun radius drives soft RT integration and the distant PCSS fallback together.
Shader-only wind deformation is not yet included in the static RT BLAS geometry.

The current depth prepass excludes blended surfaces, so transparent receivers are
not a separate RT layer. Reflections, mirrors and auxiliary views still use their
existing shadow-map lighting. Screen-space contact detail is bypassed while RT sun
shadows are active; AO remains independent. Cascades are still rendered for fallback
and auxiliary views, so this prototype does not promise a performance improvement.

Reference architecture: AMD FidelityFX Hybrid Shadows, combining rasterized and
ray-traced shadows: https://gpuopen.com/fidelityfx-hybrid-shadows/
API: wgpu v30 ray-query examples, already credited in NOTICE. Implementation here
uses the renderer's own shared scene and direct-light mask integration.

Finite-light sampling reference: Khronos Vulkan ray-query shadow tutorial:
https://docs.vulkan.org/tutorial/latest/Building_a_Simple_Engine/Lighting_Materials/07_shadows.html
Read as documentation only; no tutorial implementation code was copied.

## Lower-cost mask and cascaded reconstruction

Soft RT can trace at 50% linear resolution (one quarter of the pixels). Depth,
normals and receiver planes guide the full-resolution gather. Unmatched small
surfaces retain cascades rather than borrowing an unrelated foreground shadow.
100% remains the reference option. Disabling smoothing keeps full-resolution rays.
Reduced resolution can lose fine edges; it is a quality/cost choice, not a free gain.
Ray directions use a fine world-space seed and no alternating frame phase. The
spatial kernel also follows world position, avoiding a screen-locked rotation.

Cascades blend visibility values through their overlap instead of dithering map
selection. Bilinear, opacity-weighted blocker moments reduce discrete PCSS search
changes. Eight four-texel probes keep that bounded at 32 depth fetches. Receiver
pixel footprints set a minimum antialiasing radius. This does not remove the
underlying depth-map raster limit or make all shadow edges physically exact.

Wide detail can combine with tight near focus: at 8K its closest cascade retains
the old tight 4K density over twice the range. The outer cascade range multiplier
is adjustable from 1 to 5; old distant-coverage settings keep their 3x value.

Optional cascade stabilization resolves only sun/contact visibility, not scene
colour. It uses the same two-unblended-frame history and rejection as soft RT,
works on Vulkan and DX12, and is inactive while RT shadows are selected. Reflections
keep their existing atlas lighting. It adds a normal/depth/motion prepass when
needed and a visibility resolve; its cost must be considered alongside its benefit.
