# The pursuit HUD

The current integration is recorded in [pursuit-system](../pursuit-system.md).
The detailed radar and signed meter contract is
[hud-pursuit-state](hud-pursuit-state.md). The chase simulation is described in
[ai-pursuit](ai-pursuit.md) and [ai-pursuit-heat](ai-pursuit-heat.md).

## Sources and evidence

- **[verified]** Installed `HUD_SingleRace.fng` in `GLOBAL/InGameB.bun`:
  object identities, parentage, original sizes, pivots, scripts and messages.
- **[decomp]** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0,
  `Frontend/HUD/FePursuitBoard.cpp`, `FeRadarDetector.cpp` and `FeHeatMeter.cpp`.
  The implementation follows the behavior contract; no decompiled code or
  extracted game data is shipped.
- **[binary]** The PC radar update and target feeder confirm the two backing
  states, 300-metre 3D range, 45-degree forward cone and per-update LED ramp.
  The pursuit-board update confirms the signed segment formulas and anchors.
  Matched gameplay captures and timing/listening comparisons remain necessary
  before claiming original-game visual and behavioral parity.

## Bound objects

| Object/hash | Current behavior |
|---|---|
| `PursuitBoardGroup` / `0xde89e070` | Board shown while the pursuit feed exists, including the arrest display. |
| `PursuitTime` / `0xfc39cb0a` | Elapsed time in `mm:ss.cc`. |
| Cops group / `0xe0b1430b` | Live count, damaged count and destroyed count. Ground engagement counting still needs original comparison. |
| Engaged / `0x814918ca` | Ground units reported by the world; no template `99`. |
| Damaged / `0x5fa70d4c` | Distinct damaged police vehicles in the current pursuit. |
| Destroyed / `0xa16f9f1e` | Police vehicles destroyed in the current pursuit. |
| Meter group / `0x7b422ba3` | Signed busted/evade meter with authored entrance and cooldown transition tracks. |
| Five meter images / `0xe0e0169b`, `0x8eeebd33`, `0x47a4e2a9`, `0x8e3653a6`, `0x30f39a6f` | Piecewise widths, fixed outer edges and flashing states from the signed display value. |
| `Cooldown_Meter_Group` / `0x84a226ec` | Shown on actual cooldown state, independently of fill fraction. |
| Cooldown fill / `0xcf817638` | Installed width times cooldown progress, keeping its left edge fixed. |
| `Heat_Meter_Group` / `0xc46a80a9` | Retained vehicle heat, including outside a pursuit. |
| Duplicate heat strings / `0xa81a2028` | Both string objects updated to the integer multiplier. |
| Radar backing / `0x839e7d77` | Standalone layout selected through authored `APPEAR`; hidden when the mirror supplies the backing. |
| Mirror backing / `0x9ee06631` | Mirror layout selected with `APPEAR`; retained during pursuit when enabled. |
| `RADAR_GROUP` / `0x062743f5` | Authored `INIT` in mirror mode or `APPEAR` standalone. Suppressed during active pursuit. |
| `Radar_DirectionArrow` / `0x24196ad4` | Relative target direction, hidden without a target and during active pursuit. |
| `Radar_Icon` / `0x6ccbbbf1` | Authored idle state instead of distance-dependent host alpha. |
| LEDs / `0x69aa01e7`, `0x9f59065a` | Both installed mask windows updated; zero width without a target. |

The radar backing and group no longer receive the earlier +75/+15 unit shifts.
Those shifts aligned the mirror-oriented initial poses in the standalone view,
leaving the detector floating in the upper quarter of the scene. The authored
standalone entrance places it near the top of the screen instead.
The presenter preserves the standalone backing's authored alpha 200 rather
than forcing it opaque with the old mirror compensation.

The fill geometry is applied again after FEng script evaluation. In particular,
restarting a flashing segment must not briefly restore its authored position or
size over the newly calculated fixed edge.

## Remaining presentation gaps

| Feature | Status |
|---|---|
| Heat arcs / `0x862824c9`, `0x4b2cbe1b` | Held hidden; the original mask-to-heat mapping is not implemented. |
| Bounty row / `0x8674e6d4` | Held hidden until the complete bounty feed is bound. |
| `CostToState` / `0x3ff5f33c` | No complete cost-to-state presentation. |
| Backup timer / `0xbee44775`, `0x6a144066` | Original finite backup scheduling and countdown are not implemented. |
| Hiding / `0x60b45523`, `0x38074901`, `0x5c2a4f20` | Hidden pending complete hiding timing/presentation. |
| Infraction notifications | Original notification queue and authored actions remain incomplete. |
| Radar audio/targets | Native ground selection, range, cone and per-update ramp are bound. Detector click and speed-camera selection remain absent; roll-sensitive lateral orientation remains unverified. |
| View mode | The rear-view setting selects the detector layout. Original view-specific rear-view eligibility still needs comparison. |
| Fades/sounds | Radar/meter transforms and transitions run from installed scripts; full HUD event and sound dispatch remains incomplete. |

The board and cops groups still have host-managed alpha. This is not a claim
that every original HUD message, fade, counter animation or sound is reproduced.

## Verification

The installed HUD regression checks both duplicate heat labels and live
counters, standalone and mirror transforms, repeated state changes, stale target
suppression, 4:3/16:9/ultrawide centering, and the five meter segments retaining
fixed edges after their scripts run. Synthetic tests cover the signed threshold
values and explicit cooldown phase. These checks establish asset binding and
state integration; a matched original route/capture remains a separate check.
