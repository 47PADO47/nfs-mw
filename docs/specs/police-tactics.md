# Police tactics: the aim of each cop, and the pursuit state

Status: **implemented** (pure rules and game wiring). Tags as in [docs/README.md](../README.md#evidence-tags).
The install's tactic tables are in [ai-pursuit-tactics.md](ai-pursuit-tactics.md) and
[ai-pursuit-formations.md](ai-pursuit-formations.md); this spec covers what the rewrite does with them.

## 1. What a cop aims at

Every think, each cop in a chase gets one of four tactics from `blackbox_pursuit::tactics::choose`:

| Tactic | When | Aim |
|---|---|---|
| Pit | the cop is alongside the player (1 to 4 m to its side, within 6 m along it) and pits is allowed | the player's rear quarter on that side (`pit_point`) |
| Block | the cop is ahead of the player, within 60 m, and blocks are allowed | a slot of a road block across the road, 30 m ahead (`block_slots`) |
| Ram | within 6 m | the player itself |
| Follow | otherwise | the player itself |

The planner (`tactics::plan`) takes the cops' positions and the player's and returns one aim per cop. Blocking cops
take the slots nearest first, with at most 3 cars across a 14 m road (`ROAD_HALF_WIDTH` 7 m, assumed two lanes).

The game (`scenes/world/ai/cop_aims.rs`) allows blocks from heat level 3 and pits from heat 4.
**[unconfirmed]** Those two levels are our choice; the install's formation rows decide the real gates
(`CopFormations`, `AirSupport`, `HeavySupport`), which are not yet read.

A cop with an aim drives to it with the same path and speed code as a chase: the aim replaces the player's
position as the goal (`cop::Target::aim`). Cops with no aim are unchanged.

## 2. What the rule does not do

- No formation timers (`Duration`, `Frequency`), no PIT versus ram choice by heavy support, no spike strips.
- A ram does not steer harder into the car; the cop chases the player's position as before.
- The aim's height is the player's height; the cops use the road below it.

## 3. Checks

- `cargo test -p blackbox-pursuit tactics` (block spread, pit side, follow when neither tactic is allowed).
- A live run (`pursuit 5` in the console, then watch the cops) is still needed to see the blocks form.
