# Custom HUD: the pursuit layout and the pursuit phase

The pursuit HUD is driven by an explicit pursuit phase taken from the police simulation, never from the heat value or
the presence of a cop. This page defines the phase (PR 3b), its transitions, and the compact panel that shows it
(PR 8, slots `PursuitMeter` 6 and `BountyRow` 15). Index: [README.md](README.md).

## 1. The phase

```text
PursuitPhase { None, Active, Search, Cooldown, Escaped, Busted }
```

It is derived in `crates/nfsmw/src/scenes/world/ai/` (PR 3b) from the pursuit's own state:

| Phase | Condition (provisional, pinned in PR 3b) |
|---|---|
| `None` | `is_pursuing()` is false and no end card is pending |
| `Active` | a chase runs and a cop sees the player (the sighting, `has_contact()`), or the sight grace has not run out |
| `Search` | a chase runs, the cool-down status is still `InitialChase`, and no cop has seen the player for longer than the grace: contact is lost but the cool-down meter is not shown yet |
| `Cooldown` | the cool-down status is `CoolDown`: the meter is shown and filling |
| `Escaped` | the cool-down status is `Evaded`, or `end_reason()` says the player got away; held for the escape card |
| `Busted` | the bust flow has started (the busted overlay, `HudState::busted_overlay`); held until it ends |

The cool-down rules are in `libs/blackbox-pursuit/src/cooldown.rs` ([ai-pursuit.md](../ai-pursuit.md) sections 4.3
and 6): a chase counts as lost after `grace` seconds unseen (7.0), and the meter comes up only after `mode_gate`
(2.5 s) more. The window between "contact lost" and "meter shown" is what `Search` names.

**If the simulation does not separate them.** PR 3b reads `cooldown.rs` and the pursuit's search (`ai/search.rs`,
which keeps `Pursuit.last_seen`) and pins the mapping. If `Search` and `Cooldown` turn out not to differ in a way the
player can see, `Search` is dropped from the enum and this section says so. **[unconfirmed]**

### 1.1 Transitions

| From | To | When |
|---|---|---|
| `None` | `Active` | a chase starts |
| `Active` | `Search` | contact lost past the grace |
| `Search` | `Active` | a cop sees the player again (reacquired) |
| `Search` | `Cooldown` | the cool-down meter is shown |
| `Cooldown` | `Active` | reacquired (the meter drains back through the gate) |
| `Cooldown` | `Escaped` | the meter reaches 1 (`Evaded`) |
| any running phase | `Busted` | the bust flow starts |
| `Escaped`, `Busted` | `None` | the card or the bust flow ends |

The table is unit-tested in PR 3b (active to search to cooldown to escaped, reacquire from search and from cooldown,
busted from each running phase). The phase is computed every frame from state, so a repeated identical state cannot
produce a second transition.

## 2. The panel

Top-right, compact. It shows only while the phase is not `None` and `[custom_hud] pursuit_info` is on (with it off, only the
status label and the cooldown bar remain: the minimum needed to play).

```
 HEAT 3 ▮▮▮▯▯    02:14    ACTIVE
 BOUNTY 12,500
```

| Field | Source | Shown |
|---|---|---|
| heat | the vehicle's heat (`HudState::vehicle_heat`): the integer level and a 5-segment bar | every running phase |
| duration | `PursuitHud::secs`, `mm:ss` | Balanced and Advanced |
| status | the phase: `ACTIVE`, `SEARCH`, `COOLDOWN` | every running phase |
| bounty earned in this chase | the bounty row (`BountyRow` slot), from the career (PR 3b) | career only, Balanced and Advanced |
| cooldown progress | `PursuitHud::evade` as a bar | `Cooldown` only |
| hiding | `PursuitHud::hiding`: a `HIDING` label and its latch progress | while `Some` |
| bust meter | the positive part of `PursuitHud::busted`, a thin bar labelled `BUSTED` | while above zero |
| cost to state | the police's `cost_to_state()` | Advanced |
| units involved / disabled | `HudState::cops_engaged`, `PursuitHud::cops_destroyed` | Advanced |

Heat levels above 5 (possible with tuning or mods) fill the bar and show their number.

**Per phase:**

- **Active:** the status in `danger`, the heat bar in `danger`. Escalation (a heat level up) is shown by one `pulse`
  of the heat field, a heavier weight for the number, and a High notification; never by flashing or a red overlay.
- **Search:** the panel turns down its decoration: `danger` becomes `text_secondary`, the status reads `SEARCH`, and
  the map and edge arrows switch from tracking to last-known marks ([fair-knowledge.md](fair-knowledge.md)). No
  cooldown bar.
- **Cooldown:** the status reads `COOLDOWN` in `nav`, and the cooldown bar appears and fills. It appears only when the
  meter is actually shown, never earlier.
- **Escaped:** the panel leaves; the `Banner` slot shows an escape card for 3 s: `ESCAPED`, the duration, the bounty
  earned (career), in `success`. Then the mode returns to free roam.
- **Busted:** the panel leaves; the busted overlay (slot 11, unchanged) and the existing bust flow take over. The
  Custom `Banner` shows a short `BUSTED` card where the flow leaves room for it.

The egui end-of-pursuit card (`hud/pursuit_end.rs`) is routed through the banner when the style is Custom (PR 8).
Original and Remastered keep it.

## 3. Police awareness

Contextual notifications for what the player can know, from `HudEvent` (PR 4), never polled and redisplayed:

| Event | Priority | Text (catalogue, `hud.ftl`) |
|---|---|---|
| roadblock reported or in sight | Critical | `Roadblock ahead` |
| helicopter tracking (in sight) | High | `Helicopter overhead` |
| heat up | High | `Heat level 3` |
| visual contact lost | Medium | `Visual contact lost` |
| cooldown started | Medium | `Cooldown` |
| escaped | (banner) | the escape card |

An "interceptor approaching" warning is shown only if the sighting tells which unit it is; until PR 3b exposes a
sighted unit's kind, there is no such warning ([not-shown.md](not-shown.md)). Search zones are not shown: the
simulation has no search-area shape.

## 4. Directional threats

Sighted cops outside the view get an edge arrow ([world-markers.md](world-markers.md) section 3) under the
fair-knowledge rules; last-known positions get a hollow arrow that fades with age. At most three police arrows are
drawn, the nearest first.

## 5. Tests (PR 3b and PR 8)

- The phase transitions in section 1.1; `None` when no chase runs; never derived from heat.
- The panel absent at `None`; the cooldown bar only in `Cooldown`; the escape card for 3 s then free roam.
- Search reduces the panel and switches marks to last-known.
- No cop position reaches the HUD without a sighting ([fair-knowledge.md](fair-knowledge.md) section 4).
