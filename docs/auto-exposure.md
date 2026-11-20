# Adaptive tunnel exposure

Enable **Auto exposure** in the native F8 Look tab or run `set auto_exposure on`. Use `off` for the authored image. Selecting either visual style resets the saved toggle to off.

A fixed 64 × 64 grid samples the internal HDR image before grading or bloom. The compute shader bins log₂ luminance in quarter-stop steps, discards the darkest and brightest 10%, and averages the remainder. Headlights and small tunnel exits therefore have limited influence.

The target stays at authored exposure for mean log₂ luminance from −4 to −1. Outside this range the correction grows toward the nearest boundary, limited to −0.4 or +0.85 stops. Compensation defaults to zero. The exponential response takes about 0.63 seconds to brighten halfway toward a new target and 0.23 seconds to darken halfway. Cuts and the first frame snap to the target. These are artistic settings rather than a reproduction of the original tunnel tables.

The fullscreen draw applies the correction after temporal reconstruction and before Visual Treatment, bloom and tone mapping. Frame generation receives the finished scene. Mirror views skip the meter and HUD drawing follows the chain. Dark roads at night receive the same bounded lift as tunnels.

Metering uses 4,096 reads per rendered frame. At 3840 × 2160 a separate RGBA16F fullscreen read and write represents about 133 MB of nominal image traffic before compression and caching. The pass requires native compute support.

The histogram and percentile reduction adapt Bevy 0.20's auto-exposure approach to one WGSL workgroup. See [Bevy's source](https://github.com/bevyengine/bevy/tree/v0.20.0/crates/bevy_post_process/src/auto_exposure) and [Bruno Opsenica's luminance histogram explanation](https://bruop.github.io/exposure/).
