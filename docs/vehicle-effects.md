# Collision particles and speed trails

Options > Video offers **Collision Sparks: Off / Original PC / Restored
(Experimental)** and a separate **Speed Trails (Experimental)** switch with a **Speed Trail Style**
row: **Speed Streaks** (default) or **Car Only**. Both effects default off; enabling
sparks selects Original PC unless another style was saved. Changes appear
immediately and save when leaving Options.
F8 labels the shared switch **Collision particles**, because it also controls
authored prop dust and vehicle impact debris.

These modes work with an unmodified PC install. No ASI plugin, replacement
texture pack or Xbox addon is required. Only your installed game supplies assets.

**Original PC** reads each vehicle or prop owner's per-surface emitter groups.
World, object, ground, bottom and detachment callbacks select their authored
hit and scrape actions independently of sound. It uses
their spark/glow textures, animated contact sprite, emitter cycles, intensity,
velocity inheritance, gravity, drag and cubic color/size curves. Hit bursts and
continuing scrapes have independent emitter clocks. It adds no restored streaks,
procedural contact glow, particle bounces or car-box exclusion. Ordinary wood and
stone groups retain their own effects; missing profiles do not become metal sparks.

The original particle behavior is the baseline, rather than a promise of identical
final pixels. The port's camera, collision solver, lighting and postprocessing
still differ. Effects run with its fixed 60 Hz simulation; the original count
rounding is frame-dependent. The shared path supports the installed ordinary PC
profiles and rejects unsupported controls. Tracked object scrape transforms
and complete original collision geometry remain limitations. Transparent
particles are not globally depth-sorted.

**Restored (Experimental)** supplements the authored contact particles with the
stock install's extra Xenon definitions as a restoration reference. It renders
two moving spark populations with curved
motion, swept world bounces, time-based streak length and constant lifetime
opacity. Procedural masks replace the restoration's atlas. Narrow cores, head
glows, a short contact flash, distributed births and car-box exclusion are host
enhancements. The box approximates the car mesh; this mode is not a verified
pixel match to the Xbox release or restoration mod. It adds no dynamic wall light.

Speed Trails are experimental high-speed effects, separate from taillight trails and
flames. Both styles use the same curve: nothing shows below 44 m/s (158.4 km/h), then
brightness rises with speed up to twice that speed, and NOS does not bypass the
threshold.

- **Speed Streaks** (default): short lines over the view that radiate from the
  centre, drawn at the strength of that curve. Nothing is drawn while stopped.
- **Car Only**: the wind contrail around the body. It appears in chase camera,
  stops emitting when slowing down, and disconnects on entering free camera.

Stationary wall overlap does not keep emitting sparks. The shared PC budget is
256 particles per owner across at most 64 owners; restored sparks retain at most
2048 and trails 256. GPU vertex buffers
are reused. Particles test opaque depth without writing it. Pause freezes them;
disable, reset and style changes clear spark state and refresh paused buffers.
These are resource bounds, not measured performance improvements.

```toml
collision_sparks = true
spark_style = "original-pc" # or "restored-experimental"
speed_trails = false
speed_trail_style = "streaks" # or "car"
```

CLI: `nfsmw play --collision-sparks --spark-style original-pc`.
Environment: `NFSMW_COLLISION_SPARKS=on`, `NFSMW_SPARK_STYLE=original-pc`,
`NFSMW_SPEED_TRAIL_STYLE=car`.
F12 console: `set collision_sparks on`, `set spark_style restored-experimental`,
`set speed_trails on`, `set speed_trail_style car`. Menu changes save; console changes last for the run.
`vehicle-effects` shows style/counts; `vehicle-effects clear` removes particles.
CLI > environment > file > defaults is the usual priority.

## Tuning (config file only)

The restored sparks and the speed trail have more options in the `[xenon_effects]` table of the config file. They are not
in the menus, the console or the command line. Every default keeps the effects as described above.

```toml
[xenon_effects]
contrail_speed = 44.0          # m/s from which the trail shows (1 to 200)
contrail_min_intensity = 0.1   # strength at that speed (0 to 1)
contrail_max_intensity = 0.75  # strength from twice that speed up
spark_intensity = 1.0          # restored sparks: above 1 means more particles, below 1 dimmer ones (0 to 4)
nos_contrails = false          # also show the trail while the nitrous burns, at full strength, at any speed
nis_contrails = false          # also show the trail on the player's car in cutscenes
contrail_rate = 60.0           # trail emissions per second (1 to 240)
spark_rate = 60.0              # spark emissions per second while scraping (1 to 240)
max_sparks = 2048              # live restored sparks (16 to 10000)
max_trails = 256               # live trail particles (16 to 10000)
shadows = false                # sparks and trail take the sun's shadow (native renderer only)
```

`nos_contrails` and `nis_contrails` apply to the car trail and, for `nos_contrails`, the view streaks. With
`nis_contrails` off, no trail shows in a cutscene. `shadows` needs Shadows on in Options; the Bevy renderer ignores it.
Source of the options: [the restoration mod page](mods/nfsmw_xenoneffects.md).

Behavior and evidence: [PC particles](specs/pc-collision-particles.md),
[source-owned contact effects](specs/source-contact-effects.md),
[effect inventory](specs/vehicle-visual-effects.md),
[restored motion](specs/xenon-particle-motion.md) and
[source credits / validation](provenance/vehicle-visual-effects.md).
