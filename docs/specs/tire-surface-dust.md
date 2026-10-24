# Dry tire surface effects: dust, grass, sand and leaves

Extends the wet tire spray of [rain-pc.md](rain-pc.md) ("Tire surface water") to dry weather. Evidence tags as
in [docs/README.md](../README.md). Provenance and the executable findings:
[provenance/tire-surface-dust.md](../provenance/tire-surface-dust.md).

## Original behaviour [decomp]

- Every ground contact of every car runs three persistent particle groups per tire: drive, slip and slide. Each
  group is chosen from the contacted `simsurface` arrays `TireDriveEffects`, `TireSlipEffects` and
  `TireSlideEffects`. The three groups are always evaluated; rain only changes which record is picked.
- Record choice, per tire: index 2 when the tire is flat and the array has three records; else index 1 when the car
  is wet and the array has two or more; else index 0 (the dry record). A missing wet record therefore falls back to
  the dry one, never to another material.
- **Wet** is a car-level flag recomputed every frame. It is set when either of the two player views reports
  precipitation (non-zero rain amount, car within 60 m of that view's camera, and a shelter flag on the view's rain
  object clear). It does not look at the surface, the time of day or any puddle.
- Intensity is the clamped linear ramp of an input between the record's minimum and maximum. Drive uses the car
  speed. Slip and slide use the tire's slip and lateral speed, each multiplied by a per-axle (front, rear) factor from the
  car's `ecar` record: `SlipFX` for slip, `SkidFX` for slide. Stock cars use slip 1/1 and slide 0.1/1.0 (the front
  axle barely produces slide effects); 29 trucks and vans use slip 0.1/0.1 and slide 0.1/1.0. A ramp of zero disconnects the group; otherwise the group is placed
  at the tire's ground point in the car's frame, inherits the car's velocity and takes the ramp as its intensity.
  A record whose maximum does not exceed its minimum always ramps to zero.
- Emission is gated to wheels on the ground, and to cars near a view: the followed car, or a car within about 80 m
  of the camera that passes a visibility counter (not decoded). The original does not test the surface type anywhere; the data alone decides.

## Installed data [verified data]

Unmodified 1.3 attributes. Records are `[dry, wet, flat]`; ranges are the ramp minimum..maximum in m/s.

| Surface | Drive (car speed) | Slip | Slide |
|---|---|---|---|
| `grass`, `golf_fairway`, `golf_rough`, `golf_green` | `fxtd_dr_grass` 10..15; wet `fxtd_dr_grass_wet` 3..15 | grey-brown smoke plus grass flecks, 0.5..1 | the same, 1..2 |
| `sand` | `fxtd_dr_sand` 1..2: one large tan dust cloud; wet record has a zero range (silent) | dust cloud plus low drifting dust, 1..2 | the same, 1..2 |
| `dirt`, `mud` | `fxtd_dr_dirt` 1..10: smoke plus brown clods; wet record silent | `fxtd_dr_dirt` 0.5..5 | the same |
| `gravel`, `cobble` | `fxtd_dr_cobble` (0.1..5 and 6..10): smoke plus clods | cobble skid group | cobble skid group |
| `water` | its only record is the wet spray group, so it sprays in dry weather too | wet spray | wet spray |
| `asphalt`, `concrete`, `stone`, `rooftile`, `wetpaved` | none (null record) | `fxtd_sl_asphalt`: faint smoke | `fxtd_sk_asphalt`: smoke **and leaves** |
| default, `asphalt_no_leaves`, `oilslick`, ... | none | faint smoke | `fxtd_sk_asphalt_no_leaves`: the same smoke without the leaves |

Textures (all in the global packs): `FX_GRASS01_BLEND_ANIM` (2x2 grass flecks), `FX_SMK03_BLEND` (sand dust),
`FX_SMK06_BLEND`, `FX_SMK01_BLEND`, `FX_BEAN_BLEND` (dirt clods), `FX_DEBRIS02_BLEND` and
`FX_LEAF01_BLEND_ANIM` (4x4 leaf animation at 15 fps, one leaf per emission, rising 1.5 m/s, three second life).

Leaves exist only as the emitter `emtd_dr_leaves` inside `fxtd_sk_asphalt`. A leaf-strewn road is therefore a
`simsurface` of `asphalt`, with `asphalt_no_leaves` marking the clean ones; no separate leaf field exists in the
attributes. Where a park's leaves come from a different collision surface than `asphalt` is a data question for the
track, not a rule here.

## This port

- The groups run in dry weather as well. `wet` selects the wet record; its existing sources (rain, or enhanced
  dampness) are unchanged. Nearby traffic and police use the same selection with the player's weather.
- **Loose ground** is any surface whose dry drive record exists. It runs every dry record.
- **Paved ground** (no dry drive record) runs only `emtd_dr_leaves`. Its other dry groups are tire smoke, which this
  port already draws procedurally ([tire-effects.md](tire-effects.md)); running both would double it. Loose
  surfaces keep their authored smoke next to the procedural smoke for now.
- A blown tire picks the flat record only when wet, as before. Its dry sparks stay with the damage effects
  ([destroyed-cop-tires.md](destroyed-cop-tires.md)) to avoid doubling them.
- The enhanced puddle and dampness scaling applies only to wet records; dry records run at the ramp intensity.
- Particle appearance is the installed emitter data and the reference brightness of 2 from rain-pc.md.
- The per-axle factors scale the input before the ramp, for the player and for traffic by model name. Physics wheels
  0 and 1 are the front axle.

## Not covered

- The 80 m camera gate (traffic uses the existing 60 m) and the
  second-view rain test.
- Procedural smoke on loose ground, which could be replaced by the authored smoke.
- Pixel comparison with the original; the groups are driven by this port's wheel slip and lateral speed.
- Whether the golf-club park uses `grass`, `golf_*` or `asphalt` for its leaf-strewn ground.

## Verification

Installed-data tests resolve every dry loose-surface group and its textures, assert that paved ground keeps only
leaves and that rain still selects the wet group. Emission tests drive a car model over grass, fairway, sand and
dirt without rain and over sliding asphalt, and require particles while in contact and none afterwards.
