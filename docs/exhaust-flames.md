# Exhaust flames

Flames at the car's tail pipes: while the nitrous burns, for a moment after a gear change on an upgraded
engine, and as a short backfire each time the engine sound's sputter pops after you lift off the throttle.
Everything is read from your own install at run time (the pipe markers of the car, its particle group and the
flame textures); nothing of the game is stored in this repository. How the original does it, and what the
rewrite decided for itself, is in the [spec](specs/exhaust-flames.md).

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

## When it flames

| Trigger | What you see | Needs |
|---|---|---|
| Nitrous burning | The full flame at every pipe for as long as it burns | a car with pipes |
| Gear change (up or down) above 10 m/s | The full flame for 0.06 to 0.3 s (the car's own pitch time) | an upgraded engine, or a car whose engine cannot be upgraded |
| A sputter pop with the throttle released | A half-strength flame for 0.06 s per pop | the sound running (not `--no-sound`) |

The engine upgrade level will come from the career; until then it is 0, so cars with upgradable engines
show the gear-change flame only after `exhaust-flames engine 1` in the F12 console. The nitrous and the
backfire do not depend on it. The backfire follows the pops the sample layer's sputter module plays
([engine sound](specs/engine-sound-aems.md)): about five over a couple of seconds after a lift-off at high
revs, none at a steady throttle. A pop with the pedal down is not shown. The original draws no flame for
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
nfsmw view-world --drive --drive-script "$D;0.3:throttle=1,nos=1,steer=0.12" --screenshot nos.png
nfsmw view-world --drive --drive-script "$D;0.3:throttle=1,nos=1,steer=0.12" --no-exhaust-flames --screenshot off.png
nfsmw view-world --drive --drive-script "$D;0.05:pop" --screenshot pop.png
```

A scripted screenshot run has no sound, so the `pop` key of the script stands for a sputter pop; the car must be moving for the nitrous (it burns only in gear, on the throttle and above a minimum speed). The log prints
`exhaust flames ...` once a second of a scripted run. Unit tests cover the shift timer, the backfire gate,
the budgets, the setting's layers, the console and the menu rows; the real-install tests read the pipes,
the group and the textures of every car (`cargo test --release -p nfsmw-data -- --ignored`) and check the
option screens. The look of the flame against the original game is unverified.
