# `--drive-script`: a scripted driver for offscreen runs

`nfsmw view-world --drive [CAR] --drive-script "<script>"` drives the player's car without a keyboard, for screenshots and
repeatable checks. It is a debug facility; no key does what the script does.

## Syntax

A script is segments separated by `;`. Each segment is `seconds:key=value,key=value`, and holds that input for that many seconds
(simulated time, 60 steps a second). A segment without keys coasts. After the end the driver lets go of everything.

```
8:throttle=1;3:fall=high;12:throttle=0
```

| Key | Value | Meaning |
|---|---|---|
| `throttle`, `brake` | 0 to 1 | pedals |
| `steer` | -1 to 1 | left to right |
| `weight` | -1 to 1 | a bike's rider weight lever, back (-1) to forward (1), held for the segment: what V or the pad's stick pulled back give; a wheelie needs about -1; a car ignores it |
| `handbrake`, `nos`, `speedbreaker` | 0 or 1 | held for the segment |
| `up`, `down` | 0 or 1 | one gear shift at the start of the segment |
| `reset` | 0 or 1 | put the car back on the nearest road at the start of the segment |
| `pop` | 0 or 1 | one sputter pop for the exhaust flames at the start of the segment |
| `fall` | `low`, `high` or `generic` | a bike throws its rider at the start of the segment (below) |
| `look`, `lookup` | -1 to 1 | the right stick's X (right positive) and Y (up positive) for a bike's helmet camera (`--camera-mode hood`), held for the segment; the rider's head springs to it and back; not a drive input |

A key with no value is 1 (`up` is `up=1`); `fall` alone is `fall=generic`.

## `fall=`: crash a bike on cue

`fall=low` (a low side), `fall=high` (a high side) and `fall=generic` (anything else: a collision, a lean in the air) throw the
rider of a bike at the first step of the segment, through the path every fall takes: the bike's own `throw_rider`, which hands the
host a release (the bike's velocity at the rider, the kind) exactly as the fall tests and a hard collision do. The host then builds
the rider's ragdoll and plays the fall animation of that kind; the bike lies on its side, waits for the rider, and is stood up when
he has got up ([the get-up spec](../specs/ride6-rider-getup.md)). A bike that has already fallen, and a car, ignore it.

Run it in a still world and look at the fall and the get-up frame by frame (the screenshots run offscreen; `--muted` keeps the
engine quiet):

```
nfsmw view-world --drive KTM450 --at 985,4430 --muted --camera-mode chase \
  --drive-script "8:throttle=1;3:fall=high;15:throttle=0" \
  --screenshot out/f.png --screenshot-delay 11 --screenshot-count 12
```

`--screenshot-delay` is the simulated time of the first frame (here 3 s into the fall), `--screenshot-count` the number of frames
and `--screenshot-interval` the time between them.
