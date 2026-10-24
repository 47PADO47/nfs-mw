# Custom HUD: not shown, and why

The brief lists several things a modern HUD could show. The rule is to show only what the game computes: a metric the
simulation does not have is hidden, never faked, and there is no placeholder for it. This page lists what the Custom
HUD deliberately leaves out, the reason, and what would let it appear later. Index: [README.md](README.md).

A code search of `libs/blackbox-vehicle/src`, `libs/blackbox-damage/src` and `crates/nfsmw/src/scenes/world/drive/`
(2026-10-24) finds no drafting, slipstream or temperature model; the rows below that say "not simulated" or "not
modelled" rest on that search and on the specs named.

| Item | Why it is not shown | What would change it |
|---|---|---|
| **Drafting** indicator | Not simulated: the vehicle model has no slipstream force. | a drafting model in `blackbox-vehicle` exposing a strength |
| **Drift score** and multiplier | Not a scored mechanic in NFS: Most Wanted: no race type scores drifts (`RaceType` in `nfsmw-data`). | a drift event type with its scoring rules |
| **Engine temperature** | Not modelled. The original's drag HUD has a temperature gauge, but the rewrite's drive simulation computes none (see [architecture.md](../../architecture.md), "The HUD", hidden elements). | an engine thermal model |
| **Brake temperature** | Not modelled. | a brake thermal model |
| **Tyre temperature** | Not modelled. Tyre *condition* (punctured, blown) is shown ([modes.md](modes.md) section 4). | a tyre thermal model |
| **Cooling-system damage** | Not modelled: damage is impact zones, punctures and the disabled flag (`blackbox-damage`, `blackbox-vehicle`). | a component damage model |
| **Suspension or steering damage** | Not a separate system in the simulation; body zone damage is shown as body damage only. | per-component damage |
| **Traction or stability assistance** | The vehicle step applies traction control and a stability boost (`blackbox-vehicle` `vehicle/drive_assists.rs`, `vehicle/assists.rs`), but exposes no intervention state the HUD can read. PR 3a checks whether one can be read without changing the simulation; if so it joins the advanced row. **[unconfirmed]** | an intervention value exposed read-only |
| **Search-area shape** | The police search has no area shape: there is the pursuit's last-seen point and the helicopter's search call, not a region ([fair-knowledge.md](fair-knowledge.md)). | a search region in `blackbox-pursuit` |
| **Continuous rival time gap** | The race records passage times per checkpoint only (`Racer.checkpoint_times`), so only checkpoint splits are real ([race.md](race.md) section 5). | a continuous progress-time model |
| **Interceptor approaching** (by unit type) | Shown only if the sighting tells which unit it is; until `PoliceIntel` exposes a sighted unit's kind (PR 3b), no unit-type warning. | the kind on the sighting |
| **Event location name** in the race intro | Shown only if the race data has one; it is not invented. | a location field in the race data |
| **Exact occlusion** of world markers | Depends on a collision ray the scene may not expose to the HUD; without it markers fade by distance only ([world-markers.md](world-markers.md) section 6). | the ray query exposed to the HUD |
| **Custom HUD in split screen** | The split-screen HUD bypasses the themes today: each player has their own FEng package ([split-screen-hud.md](../split-screen-hud.md)). Custom in split screen is a follow-up. | per-player theme composition |

Where an item becomes available, it gets a row in the matching spec page and is removed from this list in the same
change.
