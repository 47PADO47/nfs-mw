# Vulkan reflection prototype

Opt in before renderer creation with `BLACKBOX_RT_REFLECTIONS=1` and select Vulkan.
Requires wgpu 30 experimental ray-query support on the selected adapter. Unsupported
devices and DX12 retain the existing reflections. Original PC rendering stays
independent. This is an Enhanced material prototype, not a denoiser or full path tracer.

Ordinary scenery and capture pipelines have no RT bindings. The wet-road redraw
uses a separate pipeline. RT resources share the stable frame bind group (group 0),
ahead of texture and material groups. wgpu re-emits higher groups when a lower one
changes, so placing the TLAS last would repeat dependency validation per material.
The car and road RT pipelines both use four groups; ordinary pipelines stay independent.
Geometry hit tables retain live mesh addresses, reuse freed spans and upload only
changed meshes. GPU buffers grow only as needed; rare fragmentation triggers compaction.
No RT scene commands are recorded when both road and car RT receivers are inactive.

Puddles trace a deterministic single-bounce triangle ray rather than searching
screen depth. Alpha-tested textures reject transparent candidate intersections.
Single-sided surfaces reject back faces using object-space triangle winding and
the explicit instance winding flag, matching raster culling through mirrored placements.
Two-sided foliage retains both sides. Instance reuse includes the winding flag.
An exact visible hit may reuse current scene colour to retain decals and detailed
material shading. Offscreen hits interpolate texture UVs, vertex colour and normals,
with approximate authored ambient/sun light and existing shadow-map visibility.
Misses and distant hits blend to the existing environment, using the road range/fade.
The existing puddle coverage, Fresnel and rain-ripple response still composites it.

The world supplies resident scenery around the camera, independently of its view
frustum, with current prop poses and enabled scenery groups. Main-view vehicle
meshes add their actual part transforms. Static BLAS are built once on first use;
the instance TLAS grows to fit and is rebuilt when geometry or transforms change.
Identical scenes reuse the exact existing structure. Mesh destruction invalidates
the shading table and releases its cached BLAS. wgpu retains in-flight resources.

F8 Materials exposes a live Hardware RT road reflections switch. Turning it off
uses the existing road SSR path without binding the TLAS in the road pass.
Scene preparation also stops when car RT is inactive.
Advanced wet surfaces contains the common ray range/distance fade and two RT controls:
geometry contribution (RT hit versus probe) and screen colour reuse at exact visible hits.
Both default to 1, preserving the original prototype appearance. F8 Save for startup
persists these choices. The startup opt-in remains necessary to create a capable device;
the overlay disables RT controls in unsupported sessions. Car RT remains independently selectable.

Prototype limits:

- Enhanced car materials can independently trace one sharp geometry ray. Glass keeps
  its existing alpha blending; rough lobes retain filtered cubemaps. Reflected glass
  transmission and broad multi-ray roughness integration are not implemented.
- Reflected textures use a 128x128 fallback and a budgeted detail cache shared by all
  material classes. F8 Materials / RT texture detail selects Low (256x128 layers),
  Balanced (512x192), High (1024x192), or Very High (1024x384). Including fallback
  mips these allocate about 213 MiB, 427 MiB, 1.17 GiB, or 2.17 GiB, respectively;
  source textures and geometry are additional. Balanced is the default.
- Nearby, large surfaces and higher-resolution source textures receive priority.
  Stable ordering, a resident bonus and 32 m ranking intervals reduce cache churn.
  Streaming changes refresh selection immediately. Inactive layers are recycled;
  missing detail falls back to the compact cache. Source resolution remains a limit.
- Normal-material hits retain their shared UV translation and sample the authored
  normal map. The hit tangent basis is reconstructed from triangle geometry and UVs.
  Texture selection is automatic, with no road, vehicle or part-name whitelist.
- Glossy hits use a live material table for diffuse ramps and surface class. Metals
  sample the existing probe instead of emitting raw paint colour. Hit lighting still
  approximates the main shader: exact clearcoat, specular maps, glass transmission,
  particles and recursive ray-traced reflections are absent.
- Traffic outside the main draw, distant unloaded tiles, animated texture redirects
  are not fully represented yet.
- No temporal accumulation or DLSS Ray Reconstruction. Sharp reflection edges and
  animated water can alias; broad rough reflections need further filtering.
- Geometry staging, texture copies, BLAS construction, per-frame TLAS builds and memory
  use need profiling on loaded routes. No performance improvement is promised.

API references: wgpu v30 `ray_scene` and `ray_shadows` examples, under MIT/Apache-2.0:
https://github.com/gfx-rs/wgpu/tree/v30.0.0/examples/features/src/ray_scene
https://github.com/gfx-rs/wgpu/tree/v30.0.0/examples/features/src/ray_shadows
The shader/material adapter here is independently implemented for the native renderer.

## Enhanced car reflections

F8 Materials exposes Hardware RT car reflections plus range and contribution controls.
The car pass shares the scene/atlas with roads and binds RT resources once per material
pass rather than per material. Cars work on dry roads with road RT and SSR disabled.
Sharp RT radiance enters the existing paint, conductor, glass and clearcoat response;
rough lobes retain filtered probes. The reconstructed Original PC families retain their
previous cubemap/SSR equations. Unsupported sessions retain the existing car path.

Car hits use consistent approximate texture/vertex/sun shading on and off screen,
without screen-colour reuse. This avoids the abrupt visibility-dependent shading switch
identified in the road prototype. Misses and range tails blend into the local cubemap.
Transparent receivers retain blend ordering. Rays see opaque/alpha-tested geometry;
transparent objects, fully shaded recursive car hits and accurate rough reflections
remain limitations. This is one-bounce reflection, without RR or temporal accumulation.

## Hit materials and texture detail

Glossy material identity is packed into fourteen triangle flag bits, independently of
alpha-test and sidedness flags. A bounded 112-byte-per-slot table is updated on material
creation/destruction and supports reused slots without rebuilding geometry. Destroyed
materials reject candidate hits. Secondary metal reflection uses the existing environment
probe; it adds no recursive rays. Renderer texture lifetime invalidates both caches.

Texture detail is selected independently of material family. Atlas replacement and
layer eviction invalidate affected hit-table records; existing rays never point at
recycled layers. Cache changes rebind both car and road passes, preserving the stable
group-0 acceleration-structure binding. Detail LOD derives from the actual atlas size.
Tests cover source material colour, live slots, UV translation on shared vertices,
normal sources, deterministic budget selection, cache replacement and texture detail.
