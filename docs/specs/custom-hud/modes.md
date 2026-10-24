# Custom HUD: contextual modes, free roam and vehicle condition

The Custom HUD changes with what the player is doing. A pure resolver picks the mode; each mode shows a small, distinct
set of elements on top of the always-present cluster. This page covers the resolver, free roam and the vehicle
condition chip; the race and pursuit layouts have their own pages ([race.md](race.md), [pursuit.md](pursuit.md)).
Built in PR 8 (`crates/nfsmw/src/ui/theme/custom/mode.rs` and the layout components). Index: [README.md](README.md).

## 1. The mode resolver

`custom/mode.rs` is a pure function from the frame's `HudState` (plus the time the current mode began) to a `HudMode`:

```text
HudMode { FreeRoam, RaceIntro, Racing, RaceResults, Pursuit(PursuitPhase), Hidden }
```

Resolution order (the first rule that holds wins):

| Rule | Mode |
|---|---|
| the HUD is not visible (`HudState::visible` false), the session `ToggleHud` is off, a menu or the world map covers the frame, or a cutscene plays | `Hidden` |
| a race is set and its result is up | `RaceResults` |
| a race is set and its countdown or intro is up (`RaceHud::count` is `Some`, or the race has not started) | `RaceIntro` |
| a race is set and running | `Racing` |
| the pursuit phase is not `None` ([pursuit.md](pursuit.md)) | `Pursuit(phase)` |
| otherwise | `FreeRoam` |

A pursuit during a race keeps `Racing` as the mode and adds the pursuit panel (the race layout has priority on the
screen; the panel stacks under the leaderboard, [layout.md](layout.md) section 3). The exact race-state fields the
rules read are pinned with the data of PR 3a.

**Transitions** crossfade over `mode_change` (300 ms) on elapsed time. Elements shared by two modes (the cluster, the
map) do not fade. Returning to `FreeRoam` resets every race and pursuit element: a restarted race builds its strip from
the new race state, and no panel is kept from the previous one. The HUD's elements are components rebuilt from the feed
each frame, so a restart cannot duplicate a panel.

## 2. Free roam

The smallest amount of persistent information, as the brief asks.

**Always:** the telemetry cluster ([telemetry.md](telemetry.md)), the map in its mode ([map.md](map.md)), the condition
chip when there is something to say (section 4), notifications ([notifications.md](notifications.md)).

**Contextual:**

- **Discovery chip** (top-left): when the car comes within 300 m of an event start or a discoverable place that the
  player has not discovered yet, a chip shows its name and distance, then the interaction prompt with the live binding
  (`ui/prompts.rs`) once the car is inside the entry zone ([world-markers.md](../world-markers.md) section 4). It fades
  when the car moves away or the place is entered. A discovered place is never announced again.
- **Feedback lane** above the cluster: near misses, bounty deltas ([feedback.md](feedback.md)).

**Optional** (`[hud] free_roam_extras`, on in the Advanced preset): cash (career), bounty (`HudState::career_bounty`)
and the distance to the GPS target, as a single line under the radio card zone. Each value shows only when the game
has it; outside a career the cash and bounty are absent.

**Never in free roam:** a large map (Expanded is on demand only), a leaderboard, a pursuit panel, a large bounty counter,
repeated notifications for discovered events.

The egui cards that today draw in free roam (`hud/interact_prompt.rs`, `hud/message.rs`) are routed through the
discovery chip and the notification queue when the style is Custom (`scenes/world/scene_hud.rs`, PR 8). Original and
Remastered keep the egui cards.

## 3. Minimal, Balanced and Advanced in each mode

| Element | Minimal | Balanced | Advanced |
|---|---|---|---|
| cluster | speed, gear, nitrous | + speed scale, rev strip, speedbreaker, boost | + advanced row |
| map | Compact | Centered | Centered |
| race strip | position and lap or checkpoint | + progress bar and distance | + rival gap source checkpoint |
| leaderboard | hidden | three rows (ahead, you, behind) | up to eight rows |
| pursuit panel | heat and status | + duration and bounty | + cost to state, units involved |
| world markers | off | on | on |
| free-roam extras | off | off | on |
| notifications | important | all | all |

The preset fills only the fields the player has not set ([settings.md](settings.md)). The rows above are what each
preset's field values produce, not separate rules.

## 4. Vehicle condition

Slot `VehicleCondition` (19), bottom-left. A small chip that expands when a real warning needs attention; it is not a
health bar.

**Data** (PR 3a): a `VehicleCondition` summary built from what the simulation has:

| Category | Source | Shown as |
|---|---|---|
| Tyres | per-wheel `TireState` (`blackbox-vehicle`): `None`, `Punctured(seconds left)`, `Blown` | which wheel, punctured or blown; a punctured tyre shows the seconds to blow-out |
| Body | the impact zones of `blackbox_damage::Damage` | `Body damage` with a level: light, heavy |
| Disabled | engine blown or car destroyed (`Vehicle::disabled`, `blackbox-vehicle`), or its input disabled (`InputContext::disabled`) | `Vehicle disabled` |

Engine damage, cooling, suspension and steering damage are not simulated as separate systems, so the chip has no
category for them ([not-shown.md](not-shown.md)). The body level thresholds are set in PR 3a from the zone values and
written here. **[unconfirmed]**

**States:**

- **Hidden** while the car is sound.
- **Compact:** an icon (a car outline) and one word (`Damaged`) once any category has something to report.
- **Expanded** for a real warning (a tyre punctured or blown, heavy body damage, disabled): the icon, the label of the
  worst warning and, for tyres, the wheel. It stays expanded while the warning holds and collapses 4 s after it clears.
- Each warning also raises a notification once (High priority, [notifications.md](notifications.md)); the chip is the
  persistent view, the notification the announcement.

The chip never uses colour alone: the icon and the label carry the state, `caution` and `critical` reinforce it. It
never covers the map or the pursuit panel (it lives in its own zone).

## 5. Tests (PR 8)

- Every mode transition from the table, including `Racing` with a pursuit and `Hidden` over everything.
- Returning to free roam resets the race and pursuit elements; a race restart gives one strip.
- Pursuit elements absent when the phase is `None`.
- The condition chip: hidden when sound, compact, expanded per warning, collapse delay; nothing shown for data the
  simulation lacks (`None` fields).
- The discovery chip never shows for a discovered place.
