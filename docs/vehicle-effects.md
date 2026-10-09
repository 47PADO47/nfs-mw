# Collision sparks and speed trails

These optional effects work with the unmodified PC game's assets. No ASI plugin,
modded executable, replacement texture pack or Xbox addon is needed.

Options > Video has independent **Collision Sparks** and **Speed Trails** choices.
Both default off. Changes appear immediately and save when leaving Options.

Collision sparks use the car's actual wall contact, the velocity of that body
point and the surface's hit/scrape effect link. Impacts create short bursts;
active sliding contact emits continuously. Stationary wall overlaps do not
create continuing sparks. Wood effect groups and unknown prop materials do not
fall back to metal sparks. Ground-body scraping and prop debris are not covered.

Speed Trails are the faint wind contrails behind a forward-moving car above
44 m/s (158.4 km/h). Their density increases up to twice that speed. They are
visible with the chase camera; entering free camera disconnects them. Slowing
down stops new trails and lets existing ones fade. NOS does not bypass the
threshold in this preset. These are separate from taillight trails and flames.

The base game's attribute definitions supply the supported spark and contrail
colors and lifetimes. Streak geometry, particle motion, spacing, brightness,
fade and rear-body volume placement are new procedural presentation. They are
informed by the Xenon effects research, without claiming a pixel-matched
recreation. Native size encoding, bounce, ordinary particles/glow and exhaust
effects remain future work.

At most 768 sparks and 256 trail particles are retained. They use reused GPU
buffers, additive RGB blending and opaque-depth testing before the HUD, without
writing depth. They age while the car is parked and clear on disable or reset.
Pausing freezes simulation; changing options refreshes the paused buffers.
This is a resource bound, not a measured frame-rate improvement.

```toml
collision_sparks = true
speed_trails = true
```

Command line: `nfsmw play --collision-sparks --speed-trails`.
Environment: `NFSMW_COLLISION_SPARKS=on`, `NFSMW_SPEED_TRAILS=on`.
F12 console: `set collision_sparks on`, `set speed_trails on`.
`vehicle-effects` shows counts; `vehicle-effects clear` removes current particles.
Menu choices save; console choices last for the run. CLI > environment > file >
defaults is the usual priority.

The research inventory and evidence limits are in
[specs/vehicle-visual-effects.md](specs/vehicle-visual-effects.md), with source
credits and comparison records in
[provenance/vehicle-visual-effects.md](provenance/vehicle-visual-effects.md).
