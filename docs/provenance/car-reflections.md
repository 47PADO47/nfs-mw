# Car environment captures

- **Spec:** [car-reflections](../specs/car-reflections.md), written before implementation.
- **Sources read:** retail PC v1.3 car effect (the existing shader specification), and
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw/tree/80993f35224c61d2f1b135beba76c6f8234c7940)
  (CC0-1.0 decompilation): `World/CarRender.cpp`, `Ecstasy/eEnvMap.hpp`,
  `Ecstasy/EcstasyData.hpp`. Read for behavior; no reference code was copied or translated.
- **Implementation:** new Rust camera/visibility/resource code drawing straight into the cube faces,
  using the existing renderer's scene pipelines and independently implemented car material shader.
- **Verification:** synthetic GPU captures on the native renderer test six axes, off-screen scenery,
  asymmetric face orientation, gamma-space colour, quality/origin changes, static fallback,
  re-enabling and exclusion of the main-view object and HUD.
- **Limits:** exact retail PC camera placement, capture resolution, update scheduling and reflection
  visibility flags are not fully recovered. The shared-code anchor offset is a reference; the
  rewrite's quality sizes and rates are tuning choices. Captures use streamed scenery only.
  Traffic, world effects, road/water reflection shaders and a separate front-end garage capture
  are outside this change. The optional Bevy backend is not changed to capture reflections.
  Full original-game visual parity has not been established.
- **Additional reference checked:**
  [codepdbh/nfsmw-android](https://github.com/codepdbh/nfsmw-android/tree/a7e6e4c3ca71064d0eea56d91fcc8aaf63418fc7)
  (GPL-3.0, reference only), `docs/native-renderer.md` and
  `docs/android-xclipse-diagnostic.md`. This port executes the Xbox 360 game;
  its notes highlight capture-to-sampling synchronization but do not establish
  PC camera placement or capture cadence. No code, generated shaders or game
  assets from that project are included.
