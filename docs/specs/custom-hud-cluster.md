# Custom HUD: the telemetry cluster as built

What PR 6 of the Custom HUD 2026 built: the horizontal drive cluster of the Custom theme, in
`crates/nfsmw/src/ui/theme/custom/cluster/`. The design is the telemetry spec on the PR 1 branch
(`docs/specs/custom-hud/telemetry.md`; read it with `git show hud/p01-spec:docs/specs/custom-hud/telemetry.md`), with
the layout, tokens and type in `layout.md`, `visual-identity.md` and `typography.md` beside it. This page records what
the code does and where it departs from that design. Decision: ADR 0013. The retired first drive card is in
[ui-themes-custom.md](ui-themes-custom.md) sections 8 and 9.

## 1. Slots

One component, `Cluster` (`cluster/mod.rs`), draws the whole cluster from the `Speedometer` slot. The Custom theme
claims `Tachometer`, `GearIndicator`, `Nitrous`, `Turbo` and `Speedbreaker` with `Empty`, so the Original pieces do not
draw under it. The arms are in `hud_component` (`custom/mod.rs`), so a mod theme that declares tokens gets the cluster
too.

## 2. Data and state

- The cluster reads `CustomFeedCell` every frame: the `HudState`, `[custom_hud]`, the speed unit and the frame clock.
  `CustomFeed` gained `transmission` (`Settings::transmission`, filled in `hud/plugin.rs`) for the manual shift cue.
- The theme builds a new component every frame, so what animates lives in `ClusterMemory` (`cluster/memory.rs`), which
  the feed cell carries and every clone shares: the speed spring, the nitrous charge spring, the nitrous and shift-cue
  pulses, and the speedbreaker's rest. The memory steps once per frame, keyed on `elapsed_s`, so a frame laid out
  twice does not run the springs twice.
- **Spring:** critically damped, integrated with its exact solution, so any frame time is stable and a step from rest
  never overshoots. Its angular frequency is `5.834 / Motion::speed_spring` (60 ms), which settles a step to 2% in
  60 ms. The first frame starts on the target.
- **Pulse:** fires on the rising edge of its condition and not again until the condition has been off. With motion its
  level falls from 1 to 0 over `Motion::pulse` (400 ms); under motion reduction it is 1 for that long.

## 3. Shape

Rows, bottom to top, in HUD units at scale 1 and text size 1, anchored bottom-right through `HudBox` (`cluster/frame.rs`):

| Row | Height | Contents |
|---|---|---|
| bars | 10 | nitrous, speedbreaker, boost: label and segments, closed up to the right edge |
| speed scale | 4 | 24 segments |
| speed | 48 | the plate; the gear box (30 square), the digits (size 44, right-aligned), the unit (size 10) |
| revs | 4 (5.6 for amber and red) | 20 segments |
| advanced | 12 | the advanced fields, closed up to the right edge |

The cluster is 236 wide; rows are 4 apart. It is about 80 high, 96 with the advanced row, rather than the spec's 68: the
rows above the speed row are separate strips. Text rows grow with `text_size` and the rows above them move up.
Scale, opacity, high contrast (the plate's outline) and the palette come through the layout, the plate and the tokens.

## 4. What each part does

- **Speed:** `HudState::display_speed`, raw and floored; the unit from `CustomFeed::units` (`hud-cluster-kmh`,
  `hud-cluster-mph`).
- **Speed scale:** the smoothed speed over 360 km/h or 225 mph. Lit `text`, empty `rule`, the segment under the end
  mixed between them.
- **Rev strip:** fixed bands by each segment's top end. Amber from `telemetry.shift_up_rpm`, or 0.85 of the red line
  when the gearbox reports none; red past the red line. Lit normal segments are `text_secondary`, empty ones `rule`.
  Amber and red segments are 1.4 times as tall, lit or not.
- **Gear box:** `R`, `N` or the number (catalogue text for the letters), accent edge, digit at 0.45 alpha while
  `shifting`.
- **Shift cue:** `shift_light`, or a manual box with rpm at or past `shift_up_rpm`. The box fills with `caution`, the
  digit turns `critical_text` and an up-chevron (a two-stroke `Polyline`) sits above the box. Its start pulse turns the
  edge `text`; with motion the chevron also lifts 2 units and settles.
- **Nitrous:** 10 segments, hidden without `has_nos`. The charge follows the speed spring. The activation pulse
  brightens the lit segments from 0.8 to full alpha and the label to `text`; with motion the segments also grow by
  half and settle.
- **Speedbreaker:** 10 segments, `nav` while active and `text` otherwise. In free roam (no race, no pursuit) it hides
  after resting full and unused for 3 s, and returns as soon as it is used or drains.
- **Boost:** 5 segments from 0 to 30 psi, only with `has_turbo`.
- **Advanced row** (`advanced_telemetry`): rpm to the nearest 10, boost psi (turbo only), engine load from
  `telemetry.throttle`, lateral g (its size, one decimal), and the tyres when `condition` is known. Fields that are
  drawn close up; nothing is a placeholder.

## 5. Deviations from the design

- **Amber band.** The spec starts it at 94% of `shift_up_rpm` and draws none without a shift point. The cluster starts
  it at `shift_up_rpm`, or at 0.85 of the red line without one, as the PR 6 brief asked.
- **Engine load and lateral g** are always shown in the advanced row: PR 3a made `throttle` and `lateral_g` plain
  numbers the simulation fills every frame, not options.
- **Tyre marks** are small squares, not dots: sound is solid, punctured is notched (the top-right quarter cut away),
  blown is a hollow square in `danger`. A round dot would cost a `Ring` of about 120 quads each.
- **The cue's digit** is `critical_text` (the surface colour, opaque) rather than `surface`, which is translucent and
  would read weakly on the amber fill.
- **Height:** about 80 units, not 68 (section 3).
- The foundation modules keep their `#[allow(dead_code)]`: other items in them are still unused until PRs 7 to 9.

## 6. Tests

`cluster/logic_tests.rs`: the spring settles within 2% in about 60 ms without overshoot and does not depend on the
frame rate; the pulse fires once per rising edge, and is static under motion reduction; the memory steps once per
frame; the speedbreaker's rest; the rev bands with and without a shift point; the shift cue for automatic and manual;
the gear letters; km/h and mph digits and scale; the boost fill; the advanced row's missing fields.

`cluster/tests.rs`: golden node counts per state (idle, cruising, shift cue, nitrous burning, advanced, high contrast);
the rev strip's colours and heights; the speed scale's lit share and its glide while the digits jump; the cue's fill
and digit, manual and automatic; the nitrous pulse with and without motion; the cluster's place on the safe area.

Owed: a screenshot check on a GPU machine (the unit's baseline against the digits, label widths at text size 1.5).
