# Optional reflection paths

The native renderer retains the existing local car cubemap and wet-road paths.
F8 Materials adds two independently saved choices, both off by default:

- **Cached local reflection probe** refreshes all six faces together at
  scheduled 10 Hz (Low), 20 Hz (Medium), or 60 Hz (High), capped by base FPS.
  Phase-aligned deadlines avoid rounding each interval up to a whole game frame.
  One metre of movement, streamed scenery, capture settings, and original
  sky/shadow-overlay changes
  invalidate the snapshot immediately. It caches an image, without accumulating
  old frames or blending old and new images. Movement can refresh earlier than
  the scheduled deadline. Switching it off restores the existing quality cadence
  (High captures every base frame). Frame generation does not advance probes.
- **Selective screen-space car reflections** substitutes current visible scenery
  for a probe sample when a glossy-material ray hits. It traces only car-material
  pixels, with bounded steps, hit refinement and an edge/distance fade. Misses
  preserve the existing material's environment response. Original PC and custom
  glossy materials retain their own reflection strengths and brightness controls.

Opaque/cutout world colour is copied before opaque/cutout cars. Blended draws and
world effects keep their later ordering. The matching scenery-only depth is snapshotted before cars, avoiding false self
hits and sampling the depth attachment while a material writes to it. Scene colour keeps
its render format, including HDR, before tone mapping, display encoding, upscaling,
UI composition and frame generation. No UI or generated frame enters reflection
sampling. The mirror and wet-road capture algorithms are unchanged.

These are custom enhancements. Screen-space rays cannot reflect offscreen or
hidden objects, and thin geometry/depth discontinuities can miss. The probe
supplies coverage there. This bounded implementation is not AMD FidelityFX SSSR.
The GPU cost can exceed the old reflection path; compare base-frame GPU time on
the same route, resolution and settings. Probe caching primarily reduces capture
preparation and rendering cost, while selective rays improve local detail.

Synthetic GPU tests exercise a visible coloured wall reflecting in a mirror,
exact probe fallback without a hit, and switching back to the original path.

Car cubemap resolution uses explicit square pixel dimensions per face in F8
Materials and the Video menu. The six faces use the same edge length (16 to 1024);
changing output resolution or upscaling does not change a selected fixed size.
Legacy percentage/Auto configs still load without changing their appearance.
F8 displays their resolved dimensions, including old power-of-two rounding;
selecting an explicit size replaces the legacy rule. Rear-view mirror sizing is
independent and retains its existing options.
