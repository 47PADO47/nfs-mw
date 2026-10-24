# Custom HUD: the telemetry cluster

The horizontal instrument in the bottom-right of the content box: speed, gear, a segmented speed scale, a rev strip,
nitrous, speedbreaker and boost, and an optional advanced row. Built in PR 6 (`crates/nfsmw/src/ui/theme/custom/cluster/`
`{mod,speed,scale,revs,nitrous,advanced}.rs`); its data comes from PR 3a. Index: [README.md](README.md).

## 1. Shape

A horizontal block about 236 by 68 HUD units at scale 1, anchored bottom-right ([layout.md](layout.md)). Top to bottom:

```
 [advanced row: 4200 rpm · 12.4 psi · 78% · 0.9 g · ●●●●]   (Advanced only)
 [rev strip ▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▯▯▯]
 [gear 4] [  187 ] KM/H
 [speed scale ▮▮▮▮▮▮▮▮▮▮▮▯▯▯▯▯▯▯▯▯▯▯▯▯]
 [N2O ▮▮▮▮▮▮▯▯▯▯]  [SB ▮▮▮▮▮▮▮▮▮▮]  [BOOST ▮▮▮▯▯]
```

One `surface` plate sits behind the gear and speed row only; the strips and bars draw on the road with `rule`-coloured
empty segments. The speed readout is the most prominent permanent instrument, as the brief asks.

## 2. Data

All values come from `HudState` through `CustomFeedCell`. Fields added in PR 3a are marked.

| Value | Source | Missing |
|---|---|---|
| speed | `HudState::speed`, shown with `display_speed()` (whole units, floored) | never missing while driving |
| unit | `speed_units` setting (`SpeedUnits::Metric` / `Imperial`) | |
| rpm, `max_rpm`, `red_line` | `HudState` | |
| `shift_up_rpm` (PR 3a) | `Powertrain::shift_points()` for the current gear | `None`: the amber band starts at 0.85 of `red_line` |
| gear, `shifting`, `shift_light` | `HudState` | |
| nitrous | `has_nos`, `nos`; `nos_burning` (PR 3a) | hidden when `!has_nos` |
| speedbreaker | `speedbreaker`, `speedbreaker_active` | |
| boost | `has_turbo`, `boost_psi` | hidden when `!has_turbo` |
| `throttle` (PR 3a) | engine load, `Powertrain::throttle()` (`blackbox-vehicle`) | `None`: not shown |
| `lateral_g` (PR 3a) | derived from the vehicle's velocities | `None`: not shown |
| tyres (PR 3a) | per-wheel `TireState` (`blackbox-vehicle`) | `None`: not shown |

## 3. Speed

- **Digits:** numeric key, size 44, right-aligned on their last cell, tabular ([typography.md](typography.md)
  section 5). The value is the raw speed floored, exactly as `display_speed` does today, so the digits never lag.
- **Unit:** label-wide key, size 10, `text_secondary`: `KM/H` or `MPH` from the catalogue (`hud.ftl`).
- **Unit conversion** follows the existing `SpeedUnits` rules; tests check km/h to mph on known values (PR 6).

## 4. Speed scale

A row of 24 segments under the digits. The lit count is `presented / top × 24`, where `top` is 360 km/h (metric) or
225 mph (imperial). Past `top` all segments are lit. **[ours]**

- **Smoothing:** the presented speed follows the raw speed through a critically damped spring with a 2% settling time
  of about 60 ms (angular frequency about 97 s⁻¹), stepped on the frame time. It removes the jumps of the physics step
  without a visible lag. The digits do **not** use the spring.
- A lit segment is `text`; an unlit one is `rule`. The segment under the current value is drawn at a fraction of its
  alpha for sub-segment resolution.

## 5. Revs and gear

- **Rev strip:** 20 segments from 0 to `max_rpm` above the speed row.
  - Below the amber band: `text_secondary`.
  - **Amber** (`caution`) from `shift_up_rpm` up to `red_line`. Without `shift_up_rpm` it starts at 0.85 of `red_line`.
  - **Red** (`danger`) past `red_line`.
  - Colour is not alone: the amber and red segments are also taller (1.4 times), so the bands read by shape.
- **Gear box:** a square left of the digits, numeric key size 26: `R`, `N` or the number. Edge in `accent`. While
  `shifting` the digit is drawn at 0.45 alpha.
- **Shift cue:** shown when the gearbox wants the next gear (`shift_light`, the box's shift-up wish), or the player
  drives a manual box (`Transmission::Manual`) and rpm is at or past `shift_up_rpm`. The cue fills the gear box with
  `caution`, draws the digit in `surface` colour and adds an up-chevron above the box. It is steady while the condition
  holds (no blinking) and one `pulse` marks its start.

## 6. Nitrous

- A 10-segment bar labelled `N2O`, hidden entirely when the car has no nitrous (`!has_nos`).
- Charge changes animate with the `speed_spring` spring, so refills and burns glide.
- **Activation:** one `pulse` (400 ms brightening of the bar and label) on the rising edge of `nos_burning`. While the
  burn continues the bar shows depletion only; the pulse does not repeat until `nos_burning` has been false. With
  motion reduction the pulse is a static highlight of the same length.

## 7. Speedbreaker and boost

- **Speedbreaker:** a 10-segment bar labelled `SB` (catalogue text), `nav` while active (`speedbreaker_active`), `text`
  otherwise. In free roam it hides 3 s after it is full and not active, and returns as soon as it is used; in races and
  pursuits it stays. **[ours]**
- **Boost:** shown only with `has_turbo`. A 5-segment bar from 0 to 30 psi, a display scale, not a limit (the same scale
  as today's Custom meter). The number in psi is in the advanced row.

## 8. Advanced row

Shown with the Advanced preset or `[hud] advanced_telemetry = true`. Each field is drawn only when its value exists;
absent metrics leave no gap and no placeholder.

| Field | Format | Shown when |
|---|---|---|
| rpm | `4200 rpm` | always while driving |
| boost | `12.4 psi` | `has_turbo` |
| engine load | `78%` | `throttle` is `Some` |
| lateral acceleration | `0.9 g` | `lateral_g` is `Some` |
| tyres | four dots, front-left, front-right, rear-left, rear-right | tyre states known |

The tyre dots use shape as well as colour: a full dot is sound, a dot with a notch is punctured, a hollow ring is blown.
Engine temperature, brake temperature and traction aids are not shown: the simulation does not compute them
([not-shown.md](not-shown.md)).

## 9. Tests (PR 6)

- The rev colour bands at rpm just below the amber start, inside amber and past the red line; amber from 0.85
  of the red line without `shift_up_rpm`.
- The shift cue for the automatic box (`shift_light`) and for manual past `shift_up_rpm`; none in manual below it.
- Nitrous and boost hidden without the part; the activation pulse once per rising edge of `nos_burning`.
- The spring settles to within 2% of a step in about 60 ms and never overshoots.
- Speed digits in km/h and mph match `display_speed`.
- Golden node counts per state (idle, cruising, shift cue, nitrous burning, advanced), as Remastered's `tests.rs` does.
