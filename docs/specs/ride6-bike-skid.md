# A bike's tyre squeal, smoke and skid marks

Why most bikes screeched and left marks at speed, and what a bike's tyres tell the effects now. Sources: the model's own tyre
(`ride6-tyre.md`, `ride6-tyre-mf94.md`) and the game's effects (`tire-effects.md`, `car-audio-tyres-transmission.md`); nothing
here is read from a game binary. Evidence tags as in [README](../README.md): **[I]** inferred, **[M]** a modelling choice of this
project, **[T]** measured on the model. Code: `crates/nfsmw/src/scenes/world/drive/sim/bike_slide.rs`, `blackbox_vehicle::bike`
(`TyreOutput::slide`).

## 1. The report and its cause

The user's playtest: "most bikes still play screeching sound/skidmarks present when going faster". Reproduced by riding 8 of the
imported bikes (BMW S 1000 RR, Panigale V4, Ninja H2R, KTM 450, CB 1300, Challenger, RS 250, CRF 450 SM) and the example bike
straight and in a gentle slalom at a held 30, 50 and 80 m/s, reading what the effects read (`bike_skid_tests.rs`, the report
test) **[T]**:

| Before | 30 m/s | 50 m/s | 80 m/s |
|---|---|---|---|
| rear slip speed the sound reads (m/s) | 0.3 | 1.0 to 1.7 | 1.4 to 3.3 |
| strongest skid loop volume | 0 to 0.025 | 0.03 to 0.09 | 0.06 to 0.19 |
| rear skid (mark strength 0 to 1) | 0 | 0 | 0.14 to 0.47 on 6 of 9 bikes |

A bike that does nothing but hold its speed screeched (8 of 9 bikes at 30 m/s, all of them from 50) and left marks at 80 m/s, with no lean at all.

**Cause [I].** The effects take the wheel readouts of a car: the car sound's skid loop plays from a slip *speed* in m/s with a
dead zone of 0.1 m/s (loop volume above 0.01 from 0.15 m/s on), and the marks and the smoke ramp over 2 to 5 and 3.5 to 9 m/s. A
car's tyre slips about nothing until it is near its limit. The tyre of a bike never does: the driven rear tyre slips a few percent
to push the bike against its drag (3 percent of 77 m/s is 2.3 m/s), a leaned tyre needs a degree or two of slip angle to carry the
bike, and both are *speeds* that grow with the speed of the bike. Read as a car's they are a screech from 0.15 m/s and marks from
2 m/s, a speed the bike passes at 50 m/s with its tyre at 5 percent of its force peak. Not a wrong slip (the body double reports
the tyre model's own slip ratio and slip angle, not the body's lateral velocity in the world frame, and lean does not enter), not
the wheelie, and not a threshold of the sound; the unit of the readout was wrong for a tyre that is always slipping.

## 2. What a bike reports now

A tyre skids when its slip is past the peak of its force curve, and the model already knows where that peak is: the normalised
slip of the combined force (`TyreOutput::normalised_x`, `normalised_y`, -1 and 1 at the peaks, the peaks taken at the tyre's
load and camber) **[I]**. `TyreOutput::slide` is its length, with the slips not scaled down below 1 m/s of patch speed (that scaling keeps the
force quiet at a standstill; a rear wheel spinning on the spot in a burnout has to read as the skid it is). 1 is the peak of the
combined force, a tyre below it grips, a tyre above lets go. `WheelState::slide` carries it (`normalised_y` too).

`bike_slide.rs` turns it into the car's readouts of a wheel (`WheelState` of the body double):

- **`skid`** (the marks, and the car sound's loop level through the wheel's traction): 0 up to a slide of 1.05, 1 from 1.5 **[M]**.
- **`smoke`**: 0 up to a slide of 1.3, 1 from 2.2: a tyre well past its peak **[M]**.
- **`slip`** and **`lateral_speed`** (m/s, what the squeal and the tyre sound read): the slip speed along the wheel and the
  sideways slip speed, times the part of the slide that lies past the peak, `1 - 1/slide` (0 up to the peak, 0.5 at twice it, towards 1
  for a locked wheel). The squeal's own dead zone does the rest: a tyre that holds reads 0.

Cars are not touched: their readouts are those of `Vehicle::wheel`. A car tyre's slip speed is already about nothing until the
limit, and a car still screeches in a hard turn (tested).

## 3. Marks

The marks follow the two contact patches of the two wheels: the body double's corners 1 and 3 repeat the front and the rear wheel
and leave no marks of their own (`tire_contacts`), and a bike's marks start from the real contact point of the tyre (`wheel.contact`,
which sits on the shoulder of a leaned tyre), not from the bottom of the drawn tyre, which a lean moves sideways from the road
patch. A mark is thin: half the width of the tyre (6 cm in front and 9 cm behind for a 120 and a 180 tyre) **[M]**, a car's
`0.24` m or the width of its drawn tread before.

## 4. After **[T]**

The same rides (9 bikes, straight and slalom at 30, 50 and 80 m/s): **no skid loop asked of the sound, no skid, no smoke, on every
bike and every speed** (before: loops up to 0.19 and marks up to 0.47, section 1). The hard cases (the report test, 4 s each):

| Case | What the tyres do | What the effects read |
|---|---|---|
| both brakes from 35 m/s (7 of 9 bikes: the rear locks) | rear slide 2.4 to 8.2 peaks | skid 1, smoke 1, loop 0.14 to 0.3; the front: 0 |
| both brakes (KTM 450, CRF 450: the bike stands on its front wheel) | rear off the road, front slide 0.3 | nothing |
| rear brake alone from 35 m/s | ABS keeps the slide at 0.79 to 1.13 | nothing (the example bike, which has no ABS to speak of, reaches 0.18) |
| burnout (throttle and brake from rest) | rear slide 127 to 435 | skid 1, smoke 1, loop 0.3 on all 9 |
| corner, stick over, 30 m/s, lean 0.88 to 0.94 rad (50 to 54 degrees) | slide 0.2 to 0.43 (the lean controller keeps the tyres well inside their peaks) | nothing |
| a car (default stats, 35 m/s, hard turn with the brake) | unchanged | loop and marks as before |

Screenshots (`BMW_S-1000-RR-2019`, `view-world --drive BMW_S-1000-RR-2019 --at 985,4430 --camera-mode chase --no-traffic --muted --drive-script "4:throttle=1;3:brake=1" --screenshot-delay 7.5`):
the rear mark after a lock is a thin line of the width of the tyre's contact patch (before: 24 cm wide, a car's); a burnout shows the
smoke and the marks of the rear wheel alone.

## 5. Tests

`bike_skid_tests.rs` (the example bike always; imported bikes too when `NFSMW_SKID_BIKES` lists their manifests, `;` separated):
straight and in a gentle slalom at 30, 50 and 80 m/s no screech (the sound's own skid mixer is fed the wheels of every step), no
marks, no smoke; a locked rear wheel and a burnout mark, smoke and screech; the tyres that grip do not; a corner at 50 degrees of lean
reads nothing while both tyres hold their peak; a car still screeches in a hard turn. `bike_slide.rs`: the mapping from the slide.
`blackbox_vehicle::bike::tyre`: the slide of a rolling, a driven, a locked, a sliding and a spinning-on-the-spot tyre at 5, 30 and 80 m/s.
