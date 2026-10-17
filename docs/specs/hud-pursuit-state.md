# Pursuit HUD states and authored geometry

This contract corrects the free-roam radar placement and binds the installed
busted/evade meter. It is written before the corresponding implementation.

## Evidence

- **[decomp]** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0,
  `Frontend/HUD/FeRadarDetector.cpp` and `FePursuitBoard.cpp`: radar backing
  selection, script states, signed meter formulas and fixed fill edges.
- **[verified]** Installed `HUD_SingleRace.fng`: object identities, parentage,
  original widths, pivots and script keyframes. No extracted assets are shipped.
- **[binary]** The original PC radar update, its local-player target feeder and
  pursuit-board update were checked against their native instructions. They
  confirm the backing/script split, target geometry, LED threshold and signed
  meter formulas below. These are static/native-isolation results; matched
  gameplay comparison remains required.

## Radar target and sweep

The native feeder selects the nearest eligible live ground police vehicle
within 300 metres and a 45-degree cone about the player's forward direction.
Distance and the normalized direction use the complete 3D displacement, so
an elevated road is eligible when it remains inside that cone. The feeder has
neither a separate deck-height limit nor a line-of-sight ray test. Helicopters
and inactive/destroyed units are excluded. The ground-vehicle distance test
includes the 300-metre boundary.

The forwarded range is `clamp(distance / 300, 0, 1)`. Direction magnitude is
`acos(clamp(dot(normalized_forward, normalized_delta), -1, 1))`, signed with
the player's lateral vector. The HUD feed uses left-positive radians and the
presenter converts to its clockwise screen orientation. The remake currently
derives lateral orientation from forward and world-up; full roll-sensitive
side-vector parity remains unverified.

Each detector HUD update increases LED reveal by 0.1, capped at 1. The first
cycle starts when a target appears. When its elapsed time exceeds
`max(normalized_range, 0.1) * 1.5`, restart the timer at the current time and
reset reveal to 0.3. Do not retain overshoot or replace this with a fixed
units-per-second ramp: the original reveal ramp depends on HUD update rate.
With no target, both masks reveal zero and the timer is unset; the stored
reveal amount is retained until the next update/cycle. The detector click and
speed-camera target selection remain unimplemented.

## Radar placement

The radar has two authored layouts. With the rear-view mirror visible, hide the
standalone backing `0x839e7d77`, show mirror backing `0x9ee06631` with `APPEAR`,
and select `INIT` on radar group `0x062743f5`. Without the mirror, hide the mirror
backing and select `APPEAR` on the standalone backing and radar group.

The standalone group's `APPEAR` track settles at `(5,-178,53)` in parent space;
its backing settles at `(6,-181,93)`. Both inherit parent `0xab9c1d4d` at
approximately `(-6.5,-19.9473,92.5227)`. On the 480-unit canvas this places the
standalone detector near the top, around y=40. The mirror layout deliberately
uses another position. Do not add offsets to these installed transforms.
The standalone backing's settled authored alpha is 200; the presenter must
preserve it rather than applying the old mirror-opacity compensation.

While actively pursued, hide the detector group and standalone backing using
their authored exit/hide scripts. Retain the mirror backing when the mirror is
enabled. Free roam and actual cooldown restore the appropriate layout. An
absent target hides the direction arrow and resets both LED mask widths to zero.
An active pursuit suppresses the target even if a stale target arrives in the
HUD feed. Detector sound and full view eligibility remain separate open issues.

## Signed busted/evade meter

The world supplies a signed display value `t` in [-1,1], plus an explicit
cooldown state and the separate cooldown fill fraction. Positive values fill
the busted side; negative values fill the evade side. The cooldown fill is not
the signed display value and must not be used to infer the pursuit phase.

The five installed images are `0xe0e0169b`, `0x8eeebd33`, `0x47a4e2a9`,
`0x8e3653a6`, `0x30f39a6f`, under meter group `0x7b422ba3`. Their width shares:

| Segment | Share of installed width | Fixed edge |
|---|---|---|
| 0 | `max(2*(t-0.5),0)` | right |
| 1 | `0.9` above 0.5; `2*t-0.1` above 0.1; otherwise zero | right |
| 2 | full width; pulse only for `-0.1 < t < 0.1` | unchanged |
| 3 | `1` below -0.5; `-2*t-0.1` below -0.1; otherwise zero | left |
| 4 | `max(-2*(t+0.5),0)` | left |

Segments 0 and 4 select the authored flashing script `0x026ded57` beyond their
respective +/-0.5 thresholds, otherwise `INIT`. Segment 2 uses `0x03826a28` near
zero and `SHOW` elsewhere. The meter and cooldown groups use the installed
`APPEAR`, `LEAVE`, and paired transition tracks `0x92975065` / `0x13f51124`.
The paired exit retains alpha 255 and collapses the group's local Y scale to
zero; it does not clear the runtime visibility flag. The paired entrance
restores Y scale to one. Settled phase checks therefore inspect presented
triangle area as well as opacity, rather than assuming a hidden flag.
Apply dynamic fill geometry after evaluating scripts so a restarted track
cannot overwrite a newly calculated width or fixed edge for one frame.

## Acceptance

- Installed transforms settle at the measured standalone and mirror positions;
  mirror toggles and chase/cooldown transitions accumulate no offsets.
- Presenter vertices retain the standalone backing's authored alpha 200.
- At 4:3, 16:9 and ultrawide sizes the standalone detector remains centered near
  the top; it does not inherit left/right HUD context offsets.
- Stale radar targets do not resurrect the detector during active pursuit.
- Targets behind the player or outside the 45-degree cone are rejected, while
  elevated targets inside the cone and exactly 300 metres away remain eligible.
- LED reveal advances once per HUD update and resets to 0.3 only after the
  native period is crossed, including restart and no-target behavior.
- Signed values -1,-0.75,-0.5,-0.1,0,0.1,0.5,0.75,1 preserve the correct fixed
  edges and authored original widths, including after flash scripts restart.
- The cooldown group follows explicit cooldown state and fills from its left
  edge. A small evade share alone does not activate the cooldown layout.
- Settled active meter/cooldown descendants present nonzero-area triangles;
  the outgoing group's descendants present no nonzero-area triangles after
  the authored scale-collapse transition.
