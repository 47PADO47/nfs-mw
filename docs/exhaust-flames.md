# Exhaust flames

Flames at the car's tail pipes while nitrous burns, for a moment after a gear change on an upgraded engine,
and as a short backfire each time the engine sound's sputter pops after you lift off the throttle.
Everything is read from your own install at run time (the pipe markers of the car, its particle group and the
flame textures); nothing of the game is stored in this repository. How the original does it, and what the
rewrite decided for itself, is in the [spec](specs/exhaust-flames.md).

Engaged nitrous also draws the PC taillight smear: 24 extra sets of the installed
brake flares over three historical car poses. It retains normal brake brightness
and centre-lamp state, stops immediately on release or depletion, and is independent
of Exhaust Flames and experimental Speed Trails. History uses the shared 60 Hz
simulation clock for consistent length across display rates; the stock PC history
advances per rendered view. See the [nitrous presentation spec](specs/nitrous-light-trails.md)
for the original algorithm, timing choice and remaining visual-parity limits.

## Switching it on and off

**Options > Video > Exhaust Flames: On / Off**, in the main menu and in the pause menu. It is **on by
default**: a car at rest pays one check per physics step, and a flame is at most 4 pipes x 2 emitters x 96
particles. Turned **off**, nothing of it exists: the pipe markers, the particle definitions and the textures are
not read, no particle or vertex buffer is built and the physics step skips it. Turning it on while driving loads
it at the next frame; turning it off frees it. Menu changes save when you leave Options; the other ways last for
the run, or are saved in the config file.

```toml
exhaust_flames = true
```

CLI: `nfsmw play --exhaust-flames` or `--no-exhaust-flames`. Environment: `NFSMW_EXHAUST_FLAMES=off`.
F12 console: `set exhaust_flames off` (also `exhaust_flames off`, and `set exhaust_flames` flips it).
The usual order applies: command line, then environment, then the config file, then the default.

The existing blue nitrous tint from main is retained: it swaps red and blue
of each particle. This is a host approximation, not a measured original-game colour.

The burning nitrous is drawn **smaller than the group's data** (`exhaust/scale.rs`): sprites at half the width and
leaving the pipe at 15 % of the speed, so the glow ends under a metre from the pipe instead of filling the view behind the
car. The 2012 game's nitrous is a small glow in each pipe, 0.3 m wide growing to 0.7 m at its brightest
([chameleon-nitrous.md](formats/chameleon-nitrous.md)); the 2005 group (a 0.45 m fire and a glow growing to 1.5 m, thrown out at
16 m/s) is far bigger. The two factors are the rewrite's own, set by eye; the blow-off and the backfire keep the group's numbers.
The "NFS Most Wanted 2012" nitrous variant keeps its glow to 0.7 m, the width its effect file gives at the peak of the alpha.

## When it flames

| Trigger | What you see | Needs |
|---|---|---|
| Engaged nitrous | The car's installed NOS particle group at full intensity until release or depletion | nitrous actually burning; holding the key with an empty tank does nothing |
| Gear change (up or down) above 10 m/s | The full flame for 0.06 to 0.3 s (the car's own pitch time) | an upgraded engine, or a car whose engine cannot be upgraded |
| A sputter pop with the throttle released | A half-strength flame for 0.06 s per pop | the sound running (not `--no-sound`) |

The engine upgrade level follows the installed garage performance package; `exhaust-flames engine 1`
can override it for diagnostics. Nitrous and backfire do not depend on that level. The backfire follows
the pops the sample layer's sputter module plays
([engine sound](specs/engine-sound-aems.md)): about five over a couple of seconds after a lift-off at high
revs, none at a steady throttle. A car a mod added (the converted 2012 cars) has no sample layer and so no sputter module:
the backfire and the pops its own sound logic decides at a lift-off (`audio/car/own_sfx/logic.rs`: a backfire at 3 to 25 % of
the lift-offs by revs, one to three pops after it, now and then a lone pop) report the same pop notices, each when its sound
plays, with or without a recording for it. A pop with the pedal down is not shown. The original draws no flame for
the sputters; this link is the rewrite's own. Cars with no pipe markers (some cop and service vehicles) show
nothing.

## Console

`exhaust-flames` (or `exhaust-flames status`) prints the pipes, the engine level, the live particles against
the budget and whether it is flaming; `exhaust-flames engine <n>` sets the engine upgrade level;
`exhaust-flames pop` fakes one sputter pop (a backfire whatever the throttle is).

## Budgets

At most 4 pipes and 96 live particles per pipe emitter (the group needs about 40 fire and 25 glow
particles), so at most 768 sprites (4,608 vertices) per frame. Emitter storage is reserved when the
car loads; stepping does not allocate; emitters with nothing alive and nothing to spawn are not stepped; the
vertex buffers go back to the flames after each upload and the renderer reuses its GPU buffers. Flames are
drawn depth-tested without writing depth, like the other world effects. The fire's alpha is multiplied by 4 (spec 8.5): its data value is too faint to see. These are resource bounds, not
measured speed-ups.

## Reproducible checks

```sh
D="3:brake=1;1.5:throttle=1,steer=0.5;0.5:throttle=1,steer=0.12"   # back out of the bus bay, pull away
nfsmw view-world --drive --drive-script "$D;0.05:pop" --screenshot pop.png
```

A scripted screenshot run has no sound, so the `pop` key of the script stands for a sputter pop; the blow-off needs a gear change above 10 m/s. The log prints
`exhaust flames ...` once a second of a scripted run. Unit tests cover the shift timer, the backfire gate,
the budgets, the setting's layers, the console and the menu rows; the real-install tests read the pipes,
the group and the textures of every car (`cargo test --release -p nfsmw-data -- --ignored`) and check the
option screens. The look of the flame against the original game is unverified.
