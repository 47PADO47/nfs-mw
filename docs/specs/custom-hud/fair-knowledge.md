# Custom HUD: fair knowledge

The HUD must never tell the player something they could not reasonably know. This page fixes what the map, the edge
arrows, the markers and the notifications may reveal about the police, and how timings are claimed. The data comes
from `PoliceIntel` (PR 3b); the rules are applied where it is built, so no consumer can leak more than it holds.
Index: [README.md](README.md).

## 1. Information types

The brief asks to keep these separate; each has its own look:

| Type | Source | Look |
|---|---|---|
| **Confirmed sighting** | a cop that sees the player now: `sighting().seen` per unit; the helicopter's `in_sight` (`helicopter_status()`) | solid mark, exact position, tracks the unit |
| **Radio report** | dispatch: a roadblock that stands (`PursuitFeed.roadblock_active`, `audio/interactive/state.rs`), the helicopter called in | notification; a roadblock mark at the reported site |
| **Last-known position** | the pursuit's `last_seen` (`Pursuit.last_seen`, `ai/pursuit.rs`, exposed read-only in PR 3b) and its age | hollow mark at a fixed position, fading with age |
| **Search area** | none: the simulation has no search-area shape | not shown ([not-shown.md](not-shown.md)) |

## 2. Rules

1. **A cop is placed** on the map, or as an edge arrow, **only while it is sighted**: the unit's sighting says the cop
   sees the player (mutual line of sight is the simulation's own test, `CanSeeTarget`), or, for the helicopter, its
   status says it has the player in sight.
2. **When the sighting ends,** the mark does not follow the unit. It becomes a last-known mark at the position of the
   last sighting and stays there.
3. **Last-known marks fade** linearly from 0.8 to 0 alpha over 20 s of age, and are dropped at 20 s or when the phase
   returns to `None`. A new sighting of the same unit replaces its last-known mark with a confirmed one.
4. **The pursuit's own last-seen point** (`Pursuit.last_seen`: where the police believe the player was) is not a cop
   position and is not drawn as one. In `Search` it may be drawn as a single "last reported" ring on the map, the place
   the search centres on, because the dispatch radio says it aloud. **[ours]**, confirmed against the speech events in
   PR 3b ([ai-pursuit-speech-events.md](../ai-pursuit-speech-events.md)).
5. **A roadblock is announced** only when dispatch reports it (`PursuitFeed.roadblock_active` rising) or a roadblock site
   (`roadblock_sites()`) is within sight distance of the player. Sight distance is the same visibility the cops use
   against the player, pinned in PR 3b. **[unconfirmed]**
6. **No count reveals hidden units.** "Units involved" (Advanced) is the number the game already shows the player
   (`HudState::cops_engaged`, the pursuit board's figure), not the number of cops alive in the world.
7. **Pursuit breakers** are marked only within sight distance ([world-markers.md](world-markers.md)).
8. **Heat** is the game's own number, which the original also shows; it is not intel.

## 3. Timings the HUD may claim

- The **rival gap** is a checkpoint split: the difference of the two racers' passage times at the latest checkpoint
  both passed, shown as `1.42 s AHEAD` or `0.85 s BEHIND`, never signed, hidden before the first shared checkpoint and
  in drag, speed-trap and tollbooth races. The full convention is [race.md](race.md) section 5.
- No continuous rival time gap, no predicted finish time and no estimated time to escape: the simulation does not
  compute them. The cooldown bar shows the meter the simulation fills, not a time.

## 4. Enforcement and tests

- `PoliceIntel` (PR 3b) is the only source of police positions for the HUD. It is built from the sightings, so a unit
  that is not sighted has no position in it. The Custom map skips the cop entries of `HudState::blips`: those follow
  the original minimap's rules ([hud-cop-blips.md](../hud-cop-blips.md)), which Original and Remastered keep.
- Tests (PR 3b): with no sighting, `PoliceIntel` holds no cop position; a sighting gives one; the end of the sighting
  turns it into a last-known mark that stays where it was while the unit moves; the mark expires at 20 s.
- Tests (PR 7b, PR 9): the map and the edge arrows draw nothing for a cop without a sighting.
