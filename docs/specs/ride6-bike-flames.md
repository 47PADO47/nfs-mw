# RIDE 6 bikes: the exhaust flames

What a bike's tail pipe shows at a gear change and at a backfire, and how the two relate. Code:
`crates/nfsmw/src/scenes/world/exhaust/` (`trigger.rs`, `step.rs`, `load.rs`, `scale.rs`, `enhanced/burst.rs`). The general
flames (cars, nitrous, the setting) are in [exhaust-flames.md](../exhaust-flames.md); the sound that decides when a bike pops is in
[ride6-engine-sound.md](ride6-engine-sound.md) (sections 5, 6 and 8). Everything here is a modelling choice: RIDE 6's own flame
effects are authored in particle assets this repository does not read, so no value of a bike's flame is data. Each choice is a named
constant.

## 1. The request

"The backfire flame is too much, it should be the same as the changing gear flame but multiple times and just a little bigger."

## 2. What was drawn before

A bike is a mod car (`is_mod_car`), so two things flamed it:

| Event | Flame |
|---|---|
| A gear change (an up-shift under power, a down-shift with the pedal up) | The group's orange emitters at `BIKE_FLAME_SCALE` (0.4), for the pitch time `BIKE_SHIFT_TIMING` (8 degrees per second over 1 degree: 0.125 s), full strength |
| Every pop the sound reports (`note_pops`) | The **same lanes** at the same size, for `MOD_BACKFIRE_SECONDS` (0.15 s), full strength, and **each pop restarted it** |

So a pop was a shift flame (a little longer), and a lift-off with a bang and one to three pops (`own_sfx/overrun.rs`, a lift-off pops
by `lift_pop_chance`, the table's 0.8 times the engine speed over the redline, commit 43c60cd7a) kept the pipe lit without a break
for 0.15 s after the *last* of up to four notices: one unbroken flame of 0.3 to 0.8 s, on up to four lift-offs in five. That is the
"too much": the pops merged into one long flame, not several flicks, and it was not distinguishable from the shift flame it was
meant to resemble except by being longer. (The enhanced exhaust, off by default, drew a pop as its own smaller burst of 0.085 s
against the shift burst of 0.14 s: a third, different flame.)

## 3. What is drawn now

A bike's **backfire flame is the shift flame, a flick, fired once for every pop the sound plays, and a quarter bigger.**

- **Same flame.** The pop lanes (`Active::pop`, made in `load.rs`) are the group's orange emitters through the same
  `compact_bike` as the shift lanes, only with `BIKE_BACKFIRE_SCALE` in place of `BIKE_FLAME_SCALE`: same textures, colours,
  rate, life and shape. The pop lasts the shift's pitch time (`Active::timing.duration()`, 0.125 s) at the same strength (1.0). The
  enhanced exhaust draws a pop as a *shift* burst (`BackfireEvent::Shift`) instead of its smaller pop burst.
- **A little bigger.** `BIKE_BACKFIRE_SCALE_OVER_SHIFT = 1.25` (`trigger.rs`), the user's request ("a little bigger", 20 to 30
  percent), the middle of the range. `BIKE_BACKFIRE_SCALE = BIKE_FLAME_SCALE * BIKE_BACKFIRE_SCALE_OVER_SHIFT` = 0.5 is the factor
  on the group's emitters. It scales the sprites, the speed they leave the pipe at, the accelerations and the start volume
  together (`compact_bike`), so the flame keeps its shape. The enhanced burst gets the factor on its length and width
  (`Bursts::trigger_sized`).
- **Several times.** Each pop notice is one flick. The **number and spacing of the flicks are the sound's**: a bang (when its
  effect level lights the pipes) and the one to three pops after it, or the one to three pops of a lift-off, each
  0.12 s after the lift for the first and 0.05 to 0.25 s after the one before (`logic::pops_of`). The `[car_sound.overrun]`
  table has no pop count or spacing (it has `flame_delay` 0.2 s, the delay before the *continuous* flame, which the KTM 450 never
  reaches: its `flame_effect_threshold` is 1.0), so a burst of N is the N pops the sound draws. The flames therefore stay in step
  with the sound by construction: a notice is due when its recording plays (`Bangs`, `take_sputter_pops`) and the flick starts in the
  physics step that takes the notice.
- **Overlap.** A pop less than a flick (0.125 s) after the last restarts the flame, so the two run together as one flame of
  their combined length; with the spacing uniform over 0.05 to 0.25 s that is about a gap in three. A longer gap leaves the pipe
  dark between the flicks.
- **A pop wins over a shift flame** of the same moment (`bike_flame_levels`): the bigger flame replaces the smaller one rather than
  piling on top of it.
- **The console's `exhaust-flames pop`** is a flick of the same length.

## 4. When it flames (and when it does not)

The flame follows the pops that play and nothing else; the exhaust adds no rule of its own to a bike. It fires:

1. At a pop notice from the sound (any throttle, forward gear; the bangs are due 0.12 s or more after the lift, when the pedal may
   be down again).
2. At a gear change: an up-shift made under power, a down-shift with the pedal up (unchanged).

A plain throttle lift does **not** flame: it only draws pops through the sound, with the table's own chance (`lift_pop_chance`: 0.8
times the engine speed over the redline for the KTM 450, so about 0.8 near the redline, 0.4 at half speed, and no pop for a lift that
the sound did not rate as a lift at high engine speed: `engine_off_sweetener`). A car without a pop does not light the pipes.
The chance is the sound's (it decides what is heard); lowering it is a one-constant change in `own_sfx/overrun.rs` and the flame
follows.

## 5. Cars are unchanged

A car (stock or mod) keeps `MOD_BACKFIRE_SECONDS` / `MOD_BACKFIRE_INTENSITY` and its single set of lanes. The pop lanes exist only
for a bike (`Active::pop` is empty otherwise), and so does the larger enhanced burst.

## 6. Tests

`exhaust/trigger.rs` (`bike_flame_levels`), `exhaust/mod_car_tests.rs` (scale ratio 1.25 of the pop spec to the shift spec on size,
speed and volume with the same rate, life and colours; a pop spawns only the pop lanes and a shift only the shift lanes; a pop flicks
exactly as long as a shift flame; three pops 0.2 s apart are three equal flicks, each in the step of its notice; two pops
closer than a flick merge into one restarted flame; a lift-off without a pop draws nothing) and `exhaust/enhanced/burst.rs` (a
sized burst is the same burst at 1.25 of its length and width and lasts as long).

## 7. Offscreen check

```sh
nfsmw view-world --drive KTM450 --at 985,4430 --muted --drive-script "6:throttle=1;0.1:throttle=1,up;1:throttle=1" \
  --screenshot C:/r6/shots/flame-shift.png --screenshot-delay 7   # the shift flame
nfsmw view-world --drive KTM450 --at 985,4430 --muted --drive-script "6:throttle=1;0.5:throttle=0;1:pop" \
  --screenshot C:/r6/shots/flame-pop.png --screenshot-delay 7     # the pop flame (the script's `pop` stands for a notice)
```

See the images listed in the round's report for the measured sizes. The look against RIDE 6 is not verified; nothing of RIDE 6's
flame is read.
