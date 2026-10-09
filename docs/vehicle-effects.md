# Collision sparks and speed trails

Options > Video offers **Collision Sparks: Off / Original PC / Restored
(Experimental)** and a separate **Speed Trails (Experimental)** switch. Both
effects default off; enabling sparks selects Original PC unless another style was
saved. Changes appear immediately and save when leaving Options.

These modes work with an unmodified PC install. No ASI plugin, replacement
texture pack or Xbox addon is required. Only your installed game supplies assets.

**Original PC** reads the ordinary per-car, per-surface emitter groups. It uses
their spark/glow textures, animated contact sprite, emitter cycles, intensity,
velocity inheritance, gravity, drag and cubic color/size curves. Hit bursts and
continuing scrapes have independent emitter clocks. It adds no restored streaks,
procedural contact glow, particle bounces or car-box exclusion. Ordinary wood and
stone groups retain their own effects; missing profiles do not become metal sparks.

The original particle behavior is the baseline, rather than a promise of identical
final pixels. The port's camera, collision solver, lighting and postprocessing
still differ. Effects run with its fixed 60 Hz simulation; the original count
rounding is frame-dependent. Non-live collision emitters are supported; live
motion inheritance, ground-body scraping and prop debris remain outside this
implementation. Transparent particles are not globally depth-sorted.

**Restored (Experimental)** uses the stock install's extra Xenon definitions as
a restoration reference. It renders two moving spark populations with curved
motion, swept world bounces, time-based streak length and constant lifetime
opacity. Procedural masks replace the restoration's atlas. Narrow cores, head
glows, a short contact flash, distributed births and car-box exclusion are host
enhancements. The box approximates the car mesh; this mode is not a verified
pixel match to the Xbox release or restoration mod. It adds no dynamic wall light.

Speed Trails are experimental wind contrails around the body, separate from
taillight trails and flames. They start at 44 m/s (158.4 km/h); brightness increases
up to twice that speed. NOS does not bypass the threshold. They appear in chase
camera, stop emitting when slowing down, and disconnect on entering free camera.

Stationary wall overlap does not keep emitting sparks. The PC budget is 1024
particles; restored sparks retain at most 2048 and trails 256. GPU vertex buffers
are reused. Particles test opaque depth without writing it. Pause freezes them;
disable, reset and style changes clear spark state and refresh paused buffers.
These are resource bounds, not measured performance improvements.

```toml
collision_sparks = true
spark_style = "original-pc" # or "restored-experimental"
speed_trails = false
```

CLI: `nfsmw play --collision-sparks --spark-style original-pc`.
Environment: `NFSMW_COLLISION_SPARKS=on`, `NFSMW_SPARK_STYLE=original-pc`.
F12 console: `set collision_sparks on`, `set spark_style restored-experimental`,
`set speed_trails on`. Menu changes save; console changes last for the run.
`vehicle-effects` shows style/counts; `vehicle-effects clear` removes particles.
CLI > environment > file > defaults is the usual priority.

Behavior and evidence: [PC particles](specs/pc-collision-particles.md),
[effect inventory](specs/vehicle-visual-effects.md),
[restored motion](specs/xenon-particle-motion.md) and
[source credits / validation](provenance/vehicle-visual-effects.md).
