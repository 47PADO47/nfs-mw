# Mod API catalogue: the NFS:MW extension

Part of the [modding catalogue](README.md). Everything here is **MW** scope: it lives in
`crates/nfsmw-modding`, is reached with `ctx.ext::<nfsmw_modding::sdk::...>()`, and exists only when the running
game id is `nfsmw`. A mod that uses it lists the interfaces in `[requires]` or `[optional]` and sets
`supported = ["nfsmw"]`. Conventions are in [api.md](api.md#how-to-read-an-entry); **W** marks a write that is
denied in an online session.

Status of the game side: the game offers `nfsmw.pursuit` (heat, the chase, busted, the events) and the mods can use it,
including the cop tables: `cop_table_get` reads a wave (a wave a mod set first, else the install's, read from
`ATTRIBUTES.BIN` once the layers are in place) and `cop_table_set` keeps a wave for the session. A set wave takes effect
on the next chase that starts; a chase that is running keeps its rows. The chase loads the cop car of every name in the
waves it uses, the mods' cars too, before the first wave; a car that does not load is logged and its entries are left out
of that chase. `nfsmw.races` is offered with no state behind it, so every call answers `BB_ERR_UNSUPPORTED` and a mod
can detect it. The roadblocks, spikes, the helicopter calls and the rest of the pursuit tables are not wired yet. The
**career** (cash, bounty, rap sheet, blacklist, milestones, stage, garage, current car) runs over the career being
played: a call outside career mode answers `BB_ERR_NOT_READY`. The calls with no state behind them (safe houses, shops,
giving and taking cars, saving, milestone registration) still answer `BB_ERR_UNSUPPORTED`; see [Career](#career-nfsmwcareer).
The original's career is now specified
([specs/career.md](../specs/career.md)); the rules a backend must follow, and where the original differs from the
sketches below, are in [Original career rules](#original-career-rules). The wire layout is unchanged.

| Interface | Version | Contents |
|---|---|---|
| `nfsmw.pursuit` | 0.1 | pursuit state, heat, cops, support, cop tables |
| `nfsmw.career` | 0.2 | cash, bounty, rap sheet, blacklist, milestones, stage, garage, current car (safe houses and shops not yet) |
| `nfsmw.races` | 0.1 | race events: list, start, control, custom races |

The wire numbers and the status of each entry are in [Wire numbers and status](#wire-numbers-and-status-01)
below (pursuit and races at 0.1, career at 0.2). An entry that is not listed there is not in its interface's version; it
is appended in a later minor once the game can do it.

## Pursuit and heat (`nfsmw.pursuit`)

The pursuit is `scenes/world/ai/pursuit.rs` with `ai/police/` (busted, escape, helicopter), the rules in
`libs/blackbox-pursuit` (`Heat`, `Infraction`, `EndReason`, `Strategy`) and the HUD in `hud/pursuit.rs`.
Heat is a real number from 1 to 10 whose integer part picks the cop wave.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Is chased | `pursuit.active() -> bool` | MW | `scenes/world/ai/pursuit.rs` |
| State | `pursuit.state() -> PursuitState { heat: f32, level: u32, duration: f32, cops: u32, bounty: u32, busted: f32, cooldown: f32, status: Chase or Cooling or Evaded or Busted }` | MW | `ai/pursuit.rs`, `libs/blackbox-pursuit` (`Heat`, `Busted`, `Cooldown`) |
| Start | `pursuit.start(StartDesc { level: u32, near: Option<VehicleId> }) -> Result<()>` **W**; same as the dev `pursuit` command | MW | `ai/commands.rs`, `ai/pursuit.rs` |
| End | `pursuit.end(EndMode { Evade or Bust or Abort }) -> Result<()>` **W** | MW | `ai/police/escape.rs`, `busted.rs` |
| Heat | `pursuit.heat() -> f32`, `set_heat(f32)`, `add_heat(f32)` **W**; clamped to the base and maximum of the pursuit | MW | `libs/blackbox-pursuit/src/heat.rs` |
| Heat limits | `pursuit.set_heat_range(base: f32, max: f32)` **W** | MW | same |
| Heat growth | `pursuit.set_time_per_level(secs: f32)` **W** | MW | same, `nfsmw-data/src/pursuit.rs` |
| Busted | `pursuit.busted() -> f32` (0 to 1), `set_busted(f32)` **W**, `set_busted_scale(f32)` (how fast the meter fills) **W** | MW | `ai/police/busted.rs`, `libs/blackbox-pursuit/src/busted.rs` |
| Cool-down | `pursuit.cooldown() -> f32`, `set_cooldown_scale(f32)` **W** | MW | `libs/blackbox-pursuit/src/cooldown.rs` |
| Evade rule | `pursuit.set_evade_time(level: u32, secs: f32)` **W** | MW | `libs/blackbox-pursuit/src/evade.rs` |
| Sight | `pursuit.set_sight_range(level: u32, near: f32, far: f32)` **W** | MW | `libs/blackbox-pursuit/src/sight.rs` |
| Cops | `pursuit.cops(out: &mut Vec<VehicleId>)` | MW | `ai/cop.rs`, `ai/manager.rs` |
| Spawn a cop | `pursuit.spawn_cop(SpawnCop { car: CarId, pos: Option<Vec3>, behind: Option<VehicleId> }) -> Result<VehicleId>` **W**; joins the pursuit; the generic `vehicles.spawn` with role `Cop` does not | MW | `ai/spawn.rs`, `ai/cop.rs` |
| Dismiss a cop | `pursuit.dismiss_cop(VehicleId)` **W** | MW | `ai/manager.rs` |
| Support | `pursuit.request_support(Strategy { EBrake, CoordinatedEBrake, Ram, HeavyRoadblock, CrossFollow, CrossBrake, CrossPlusVBlock })` **W** | MW | `libs/blackbox-pursuit/src/support.rs`, `ai/pursuit.rs` |
| Roadblock | `pursuit.place_roadblock(pos: Vec3, heading: f32, cars: u32) -> Result<()>` **W** | MW | `ai/roadblock.rs`, `libs/blackbox-pursuit/src/roadblock.rs` |
| Spike strip | `pursuit.place_spikes(pos: Vec3, heading: f32)` **W** | MW | `ai/spikes.rs`, `libs/blackbox-pursuit/src/spikes.rs` |
| Helicopter | `pursuit.helicopter() -> Option<VehicleId>`, `call_helicopter()`, `dismiss_helicopter()` **W** | MW | `ai/police/heli.rs`, `libs/blackbox-pursuit/src/helicopter/` |
| Infractions | `pursuit.infractions() -> Vec<(Infraction, u32)>`, `pursuit.add_infraction(Infraction)` **W** (`Speeding, Racing, RecklessDriving, Assault, HitAndRun, DamageToProperty, ResistingArrest, OffRoad`) | MW | `libs/blackbox-pursuit/src/infraction.rs` |
| Fines | `pursuit.set_fine(Infraction, cash: u32)` **W** | MW | same |
| Bounty in chase | `pursuit.bounty() -> u32`, `pursuit.add_bounty(u32)` **W** | MW | `ai/pursuit.rs` (the bounty counter), `hud/pursuit.rs` |
| Dev toggle | `pursuit.set_enabled(bool)` **W**: cops spawn or not (the `pursuit off` command) | MW | `ai/commands.rs` |
| Pursuit HUD text | `hud.message(...)` from `bb.hud`; the pursuit bar is `hud.set_visible(PursuitBar, ..)` | G | `hud/pursuit.rs` |

### Cop tables

The cop waves are rows of the install's attribute database: `heattable` for free roam and a separate race
table (`nfsmw-data/src/pursuit.rs`, `heat_row` and `race_heat_row`). A row has a list of cop records (a
`pvehicle` collection name, a count and a spawn weight in percent), the patrol-car count, the cops that must
be evaded to end the wave, and the two spawn intervals. The helicopter is the record named `copheli`.
A mod edits the table **before the first pursuit**; after that the new rows apply to the next wave.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Read row | `pursuit.cop_table(Table { Roam or Race }, level: u32) -> Result<HeatRow { cops: Vec<CopRecord { car: CarId, count: u32, chance: u32 }>, patrol_cars: u32, full_engagement: u32, first_four_interval: f32, spawn_interval: f32 }>` | MW | `nfsmw-data/src/pursuit.rs` (`HeatRow`, `CopRecord`) |
| Replace row | `pursuit.set_cop_table(table, level, HeatRow) -> Result<()>` **W** | MW | `modding/cop_rows.rs`, applied by `begin_pursuit` in `scenes/world/ai/commands.rs` |
| Add cop to a level | `pursuit.add_cop(table, level, CopRecord)`; the car must have role `cop` (see `bb.content`) | MW | same |
| Remove cop | `pursuit.remove_cop(table, level, car: CarId)` | MW | same |
| Helicopter | `pursuit.set_helicopter(table, level, Option<u32>)` places `copheli` in a wave or removes it | MW | same |
| Patrol speeds | `pursuit.patrol_speeds() -> (f32, f32)`, `set_patrol_speeds(low, high)` | MW | `nfsmw-data/src/pursuit.rs` (`patrol_speeds_mph`) |
| AI profile | `pursuit.ai_vehicle(car: CarId) -> AiVehicle`, `set_ai_vehicle(car, AiVehicle)` per-car driving parameters | MW | `nfsmw-data/src/pursuit.rs` (`AiVehicle`) |
| Formations | `pursuit.set_formation_weights(level, &[(Formation, f32)])` **W** | MW | `libs/blackbox-pursuit/src/formation.rs` |
| Load from file | a mod can instead ship `content/pursuit/heat.toml` with the same rows; the host applies it at start-up in load order | MW | `modding/pursuit_content.rs` (new) |

A row for a level the install does not have is `Arg`. Level numbers are 1 to 10 (`HEAT_LEVELS`).

## Events

Delivered like the generic events ([api.md](api.md#events)).

| Event | Payload | Cancelable | Raised by |
|---|---|---|---|
| `PursuitStarted` | `heat`, `level`, `cops` | no | `ai/pursuit.rs` |
| `PursuitHeatChanged` | `old`, `new`, `level` | no | `libs/blackbox-pursuit/src/heat.rs` via the pursuit |
| `PursuitEnded` | `reason: Busted or Evaded or Aborted`, `bounty`, `duration`, `infractions` | no | `ai/police/escape.rs`, `busted.rs` |
| `CopSpawnRequested` | `car`, `level`, `kind: Ground or Helicopter or Support` | **yes** | `ai/spawn.rs`, `ai/manager.rs` |
| `CopSpawned`, `CopRemoved` | `vehicle`, `car`, `reason` (wrecked, left, dismissed) | no | `ai/cop.rs` |
| `CopWrecked` | `vehicle`, `by: Player or Traffic or Hazard` | no | `ai/cop.rs`, `scenes/world/events/damage.rs` |
| `InfractionCommitted` | `infraction`, `fine` | no | `libs/blackbox-pursuit/src/rules.rs` via the pursuit |
| `BustedChanged` | `fraction` (sent when it crosses 0.25, 0.5, 0.75, 1.0 and on reset) | no | `ai/police/busted.rs` |
| `BustedRequested` | `fraction == 1.0`, about to end the chase | **yes** | same |
| `CooldownChanged` | `status`, `fraction` | no | `libs/blackbox-pursuit/src/cooldown.rs` |
| `RoadblockPlaced`, `SpikesDeployed` | `pos`, `heading` | no | `ai/roadblock.rs`, `ai/spikes.rs` |
| `SupportRequested` | `strategy` | no | `libs/blackbox-pursuit/src/support.rs` |
| `HelicopterArrived`, `HelicopterLeft` | `vehicle` | no | `ai/police/heli.rs` |
| `BountyChanged` | `old`, `new`, `reason` | no | career module (new) |
| `CashChanged` | `old`, `new`, `reason` | no | career module (new) |
| `MilestoneCompleted` | `milestone: MilestoneId` | no | career module (new) |
| `BlacklistRivalDefeated` | `rank: u8`, `rival` | no | career module (new) |
| `BlacklistRivalChallenged` | `rank` | no | career module (new) |
| `RapSheetChanged` | `stat`, `old`, `new` | no | career module (new) |
| `SafehouseEntered`, `SafehouseLeft` | `safehouse` | no | `scenes/world/entry.rs`, career module (new) |
| `ShopEntered`, `ShopLeft` | `shop` | no | same |
| `PurchaseRequested` | `shop`, `item`, `price` | **yes** | career module (new) |
| `Purchased` | `shop`, `item`, `price` | no | same |
| `CareerLoaded`, `CareerSaved` | `slot` | no | `saves.rs`, `save_format.rs` |
| `RaceStartRequested` | `race`, `kind` | **yes** | race module (new) |
| `RaceStarted` | `race`, `kind: RaceType`, `racers`, `laps` | no | same |
| `CheckpointPassed` | `race`, `index`, `time`, `position` | no | same |
| `LapCompleted` | `race`, `lap`, `time` | no | same |
| `RaceFinished` | `race`, `position`, `time`, `won`, `winnings` | no | same |
| `RaceAborted` | `race`, `reason` | no | same |

## Career (`nfsmw.career`)

The career is the player's progress: cash, bounty, rap sheet, the blacklist of 15 rivals, milestones, owned
cars, and the safe houses and shops they unlock. Storage is the save file (`save_format.rs`, `saves.rs`,
`libs/save-store`); the data it reads is in `nfsmw-data/src/gameplay/`. The stat list, the milestone ids and the
rival table are fixed by the original: see [Original career rules](#original-career-rules).

**Status at 0.2.** The game implements the calls marked **yes** in the [wire table](#career-nfsmwcareer-02). The backend
is `crates/nfsmw/src/modding/backends/career.rs`, over the career being played (`CareerHandle::shared()`). Where the
sketch below differs from the code:

- Bounty is the total over the stable (sold base plus every live car). `bounty_add` and `bounty_set` change the current
  car's career record, so `bounty_set` answers `BB_ERR_ARG` for a total below the other cars' share.
- Rivals are the bins 1 to 15, rank = bin number. `rival_at` lists rank 1 first. A rival's car is handle 0 and its name
  is `Rank N`, since the bin data has neither **[unconfirmed]**.
- Milestone ids are 1 based positions in the career's list. `milestone_complete` pays the milestone's bounty to the
  current car. `milestone_progress` is not offered: a career milestone's progress is the game's statistic.
- Outside career mode every call answers `BB_ERR_NOT_READY`. A call that needs a driven car, with none, answers
  `BB_ERR_NOT_FOUND`.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Cash | `career.cash() -> i64`, `add_cash(delta: i64, reason: &str)` **W**, `set_cash(i64)` **W** | MW | career module (new), `saves.rs` |
| Bounty | `career.bounty() -> u64`, `add_bounty(u64)` **W**, `set_bounty(u64)` **W** | MW | same |
| Rap sheet | `career.rap_sheet() -> RapSheet { pursuits: u32, busted: u32, evaded: u32, infractions: [u32; 8], cops_wrecked: u32, ... }` read | MW | same |
| Rap sheet stat | `career.set_stat(stat: Stat, value: u64)` **W**, `add_stat(stat, delta)` **W** | MW | same |
| Blacklist | `career.blacklist() -> Vec<Rival { rank: u8, name, car: CarId, defeated: bool, locked: bool }>` | MW | same; rival data in `nfsmw-data/src/gameplay/races.rs` |
| Defeat rival | `career.defeat_rival(rank: u8)` **W**; raises the events and unlocks what the game unlocks | MW | same |
| Unlock rival | `career.set_rival_locked(rank, locked: bool)` **W** | MW | same |
| Milestones | `career.milestones() -> Vec<Milestone { id, name, progress: f32, goal: f32, done: bool }>` | MW | same |
| Complete milestone | `career.complete_milestone(MilestoneId)` **W** | MW | same |
| Register milestone | `career.register_milestone(MilestoneDef { key, name, goal: f32, reward_bounty: u32 }) -> Result<MilestoneId>`, where `reward_bounty` is the bounty (rep points, not cash) paid on completion, `career.milestone_progress(MilestoneId, f32)`; mod-owned milestones are saved in the mod's slot data | MW | same, `libs/save-store` |
| Current stage | `career.stage() -> u8` (the blacklist/story stage) | MW | same |
| Owned cars | `career.garage() -> Vec<OwnedCar { car: CarId, paint, parts, performance }>` | MW | `garage/mod.rs` (`Model`), `garage/parts.rs`, `performance.rs` |
| Give or take car | `career.give_car(CarId) -> Result<()>` **W**, `career.remove_car(CarId)` **W** | MW | `garage/`, `saves.rs` |
| Current car | `career.current_car() -> Option<CarId>`, `career.set_current_car(CarId)` **W** | MW | `garage/`, `scenes/world/garage.rs` |
| Save slot | `career.slot() -> Option<SlotId>`, `career.save() -> Result<()>` | MW | `saves.rs`, `libs/save-store` |

### Safe houses and shops

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Safe houses | `career.safehouses() -> Vec<Safehouse { id, name, pos: Vec3, unlocked: bool }>` | MW | `scenes/world/entry_zones.rs` (`ZoneKind::SafeHouse`), `nfsmw-data/src/gameplay/markers.rs` |
| Unlock | `career.set_safehouse_unlocked(id, bool)` **W** | MW | career module (new) |
| Add safe house | an interior of kind `Room` / `safehouse` from `bb.content` plus `career.register_safehouse(InteriorId) -> Result<SafehouseId>` so it counts as one (heat reset on entry, car storage) | MW | `entry_zones.rs`, career module (new) |
| Shops | `career.shops() -> Vec<Shop { id, kind: AutoShop or CarLot or Performance or Paint, name, pos }>` | MW | `scenes/world/entry_zones.rs` (`ZoneKind::AutoShop`), `nfsmw-data/src/gameplay/markers.rs` |
| Stock | `career.shop_stock(ShopId) -> Vec<Item { key, name, price, unlock: Option<MilestoneId> }>` | MW | career module (new), `garage/parts.rs` |
| Add item | `career.shop_add_item(ShopId, ItemDef { key, name, price, apply: CallbackId }) -> Result<ItemId>` | MW | same |
| Remove item | `career.shop_remove_item(ShopId, ItemId)` | MW | same |
| Price | `career.set_price(ShopId, ItemId, cash: u32)` **W** | MW | same |
| Heat in safe house | `career.set_safehouse_cools_heat(bool)` **W** | MW | `ai/pursuit.rs` |

## Original career rules

What the original does, from [specs/career.md](../specs/career.md) (read in `speed.exe`, `gameplay.bin` and its Lua
scripts, the language table and three saves; not tested in game). A backend that implements `nfsmw.career` should
follow these; the places where the sketches above differ are listed last.

**Cash.** One `u32` in the career settings (profile `+0xB4`). Spending clamps at 0 and no cap was found. A race win adds
the event's cash value on every win. A car sells for half its price and is bought at its price.
[career-economy-flows.md](../specs/career-economy-flows.md).

**Bounty.** Stored per owned car (the car's career record) plus one base for cars that were sold or removed; the player's
bounty is the sum. An escaped pursuit adds the pursuit's reputation points to the car driven; a bust adds none.
Milestones pay **bounty**, not cash. `set_bounty` therefore needs a target (the current car's record); the sketch's single
value maps to "set the total by adjusting the current car". [career-economy.md](../specs/career-economy.md).

**Rap sheet.** Eight raw sums over non-busted pursuits, in this stat order: 0 pursuit length, 1 cops involved, 2 cops
damaged, 3 cops destroyed, 4 tire spikes dodged, 5 roadblocks dodged, 6 helicopters involved, 7 cost to state; 8 (total
infractions, both sets) and 9 (total bounty) are computed. Pursuits evaded and times busted are per-car counters summed
over the stable (`pursuits` in the sketch is their sum; `cops_wrecked` is stat 3). Infractions are 8 kinds, counter
index = bit position of the pursuit's infraction mask, each with a fixed fine, in two sets: **unserved** (since the last
payment) and **served**. [career-economy.md](../specs/career-economy.md), [career-text-2.md](../specs/career-text-2.md).

**Busted and impound.** Each car has a strike count, a limit (3, at most 5), a state and a countdown. Paying the fines
with cash adds a strike; a marker pays without one. At the limit the car is impounded and returns after a number of
events or for a release fee (a quarter of its cost). The game is over only when no free car and no release marker is left.

**Stage and blacklist.** The stage is the bin number: 16 is the prologue, then 15 down to 1 (1 is the last rival). Rival
rank = bin number. Each bin is a state graph (`initial`, `unlock_races`, `introduce_rival`, `check_rival`,
`rival_locked` or `rival_unlocked`, `done`), so `defeated` and `locked` are derived from the graph state, not stored
flags. A rival challenge opens when races won, challenges passed (milestones plus speed traps of the bin) and total
bounty all reach the bin's requirements; the counters are recomputed, never stored. Nothing in the exe or the scripts
advances the stage after bin 15 (open question): `defeat_rival` should enter bin N-1. The career-complete flag is set
by winning event 1.8.1. [career-blacklist.md](../specs/career-blacklist.md),
[career-scripts.md](../specs/career-scripts.md).

**Milestones.** 55 milestones and 39 speed traps, fixed by `gameplay.bin`, 30 statistic kinds. States: locked, active,
at stake, done. A reached milestone is at stake until the pursuit ends: an escape banks it and pays bounty, a bust
returns it to active. Milestones are checked in free roam only. [career-milestones.md](../specs/career-milestones.md).

**Garage.** 200 car records, at most 25 with a career record. The car lot refuses a purchase at 10 owned cars; selling
refuses an impounded car and the last car. [career-car-lot.md](../specs/career-car-lot.md).

**Safe house and shops.** The safe house menu is Resume Free Roam, Blacklist, Car Select, Rap Sheet (once unlocked) and
Save. Nothing resets heat, cops or the pursuit on entering or leaving, in scripts or in native code: the whole world is
unloaded when the front end opens and free roam builds a fresh cop manager. Heat survives in the driven car's career
record (written as `min(heat, 5)` at unload, read as `max(1.0, record)` at spawn), so the sketch's "heat reset on entry"
for `register_safehouse` and `set_safehouse_cools_heat` is a rewrite feature, not original behaviour. A safe house is a
zone whose type string is `safehouse` (`carlot` and `customshop` are the other two); it needs no event and either no
pursuit or one in cool-down, where engaging ends the pursuit first. The respawn point is a collection key stored in the
script manager. Autosave runs at three points: free-roam start, event start, and pursuit evaded
([career-save-screens-2.md](../specs/career-save-screens-2.md) section 8). SMS messages arrive only in free roam and sit in a
150-slot inbox ([career-phone.md](../specs/career-phone.md)). [career-safehouse.md](../specs/career-safehouse.md),
[career-flow.md](../specs/career-flow.md).

**Differences to apply to the sketches above**

| Sketch | Original | Action |
|---|---|---|
| `MilestoneDef.reward_cash` | milestones pay bounty | renamed to `MilestoneDef.reward_bounty` (`BbMilestoneDef.reward_bounty`, same `u32` at the same offset; 0.1 kept, nothing shipped yet); the threshold stays the one `goal: f32`; the 55 built-in milestones report their bounty through `BountyChanged` |
| `Rival.rank` 1..15 and `stage() -> u8` | bin number, 16 = prologue | allow 16 in `stage()`; ranks stay 1..15 |
| `Rival.defeated`, `locked` | derived from the bin graph state | compute, do not store |
| `RapSheet.infractions: [u32; 8]` | two sets of 8 (unserved, served) | report the sum; add the unserved set later |
| `add_bounty(u64)` | per-car `u32` | clamp at `u32::MAX` per car |
| `CashChanged.reason`, `BountyChanged.reason` | cash: race win, sale, purchase, fine, release fee; bounty: escape, milestone, speed trap, script award | use these as the reason strings |

## Races (`nfsmw.races`)

Race events are collections of the install with a `racestart` (`nfsmw-data/src/gameplay/races.rs`; types
`Sprint, Circuit, Drag, Knockout, Tollbooth, SpeedTrap, Challenge` and others). The race runtime is not built yet;
the table is the plan.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| List | `races.list() -> Vec<RaceInfo { id, name, kind: RaceType, pos: Vec3, laps: u32, racers: u32, done: bool }>` | MW | `nfsmw-data/src/gameplay/races.rs` |
| Info | `races.info(RaceId) -> Result<RaceDetail { checkpoints: Vec<Vec3>, start: Placement, rewards, difficulty }>` | MW | same |
| Start | `races.start(RaceId) -> Result<()>` **W**, same as driving into the race start | MW | race module (new), `scenes/world/start.rs` |
| Abort | `races.abort()` **W** | MW | race module (new) |
| Current | `races.current() -> Option<RaceState { id, lap, checkpoint, position, elapsed }>` | MW | same |
| Register race | `races.register(RaceDef { key, name, kind: RaceType, checkpoints: &[Vec3], start: Placement, laps: u32, opponents: &[CarId], reward_cash: u32 }) -> Result<RaceId>`; also as `content/races/*.toml`; adds the entry sphere and blip through `bb.content` | MW | `nfsmw-data/src/gameplay/races.rs`, `entry_zones.rs` (`RACE_RADIUS`), `hud.rs` (`map_blips`) |
| Opponent AI | `races.set_opponent_skill(RaceId, f32)` **W** | MW | `libs/blackbox-driver`, `docs/ai-drivers.md` |
| Rewards | `races.set_reward(RaceId, cash: u32)` **W** | MW | race module (new) |
| Heat in races | `pursuit.cop_table(Table::Race, level)` (above) | MW | `nfsmw-data/src/pursuit.rs` (`race_heat_row`) |
| Results | `races.results(RaceId) -> Option<RaceResult>` | MW | `saves.rs` |

## Not planned for the first delivery

- Editing the story or FMV order.
- Changing the physics model itself (only per-vehicle scales in `bb.vehicles`).
- Replacing the AI driving controller (mods can steer a vehicle through `set_ai` goals only).
- Raw access to FEng objects and to the attribute database; both are reachable only through the typed calls.

## Wire numbers and status (0.1)

Code: `crates/nfsmw-modding` (`abi`, `host`, `sdk`). Each table is `#[repr(C)]` and starts with `head: BbIface` and `ctx`.
Its functions are `Option<fn>` slots, so the `n`th function (0 based) sits at byte `16 + 8 n`. Every record and payload
starts with `size`, the bytes its writer filled in. Functions take `ctx` first and return a `BbStatus`; the statuses are
the `BB_ERR_*` of `blackbox-modding`. Pursuit and races are at 0.1, career at 0.2. Those are interface versions, not the ABI version.

| Interface | Id | Version | Table size (bytes) |
|---|---|---|---|
| Pursuit | `nfsmw.pursuit` | 0.1 | 88 |
| Career | `nfsmw.career` | 0.2 | 328 |
| Races | `nfsmw.races` | 0.1 | 104 |

Limits the host checks: heat levels `1..=10`; at most 64 cop records in one wave; at most 256 checkpoints in one race.
Names are inline (`BbName { len: u32, bytes: [u8; 128] }`), so no pointer the host keeps outlives a call.

### Pursuit (`nfsmw.pursuit` 0.1)

| Offset | Slot | Arguments after `ctx` | Writes | Status | SDK |
|---|---|---|---|---|---|
| 16 | `heat_get` | `out: *mut f32` | no | supported | `heat()` |
| 24 | `heat_set` | `heat: f32` (clamped by the game) | yes | supported | `set_heat(heat)` |
| 32 | `cop_table_get` | `table: u32, level: u32, out: *mut BbHeatRow, cops: *mut BbCopRecord, cops_cap: u32` | no | supported (install or a set wave; not ready without an install) | `cop_table(table, level)` |
| 40 | `cop_table_set` | `table: u32, level: u32, row: *const BbHeatRow, cops: *const BbCopRecord, cops_len: u32` | yes | supported (applies to the next chase) | `set_cop_table(table, level, &row)` |
| 48 | `start` | `level: u32, near: u64` (0 for none) | yes | supported | `start(level, near)` |
| 56 | `end` | `mode: u32` (0 evade, 1 bust, 2 abort) | yes | supported | `end(mode)` |
| 64 | `is_pursuing` | `out: *mut u8` | no | supported | `is_pursuing()` |
| 72 | `busted_get` | `out: *mut f32` | no | supported | `busted()` |
| 80 | `cops_in_pursuit` | `out: *mut u32` | no | supported | `cops()` |

A cop wave is a `BbHeatRow { size, cop_count, patrol_cars, full_engagement_cops, first_four_interval: f32,
spawn_interval: f32 }` (24 bytes) followed by `cop_count` records of `BbCopRecord { count, chance, name: BbName }` (140
bytes). The read takes two calls: the first, with `cops_cap = 0`, learns `cop_count`. `table` is `0` (roam, `heattable`)
or `1` (race, `racetable`). A set needs `cop_count == cops_len`, a chance of at most 100, and a name that is UTF-8 and not
empty. The new wave applies to the next pursuit. A stale `near` handle answers `BB_ERR_STALE`.

How the game answers in 0.1:

- The reads are from the chase as the world last published it, once a frame. `heat_get`, `heat_set` and `end` answer
  `BB_ERR_NOT_READY` while no chase is on. `is_pursuing` is 0 then, `busted_get` keeps the meter after a chase (the
  game's own rule) and `cops_in_pursuit` is 0.
- `heat_set` is applied in the next police update, and the game keeps the value in the chase's range. `heat_get` shows
  the value of the frame before, so a mod that sets the heat reads it back a frame later.
- `start(level, near)` runs the console's `pursuit <level>` command at the next frame, the same path the player's console
  takes. A `near` vehicle is not used yet; the cops are placed the way the command places them. A start the command
  refuses (no car to drive, for example) is logged by the game, not returned to the mod.
- `end` applies in the next police update. `Evade` ends the chase as evaded, `Bust` runs the bust sequence and ends it
  busted, and `Abort` ends it with no result (the `pursuit off` outcome).

Not in 0.1: the heat range and time per level, the busted and cool-down scales, the evade and sight rules, the cop list
as vehicles, spawn and dismiss cop, support, roadblock, spikes, the helicopter calls, the infraction list and fines,
bounty in the chase, the dev toggle, formations, patrol speeds, the AI profile, and adding or removing one cop or the
helicopter in a wave. In 0.1 a mod sets the whole wave with `cop_table_set`.

### Career (`nfsmw.career` 0.2)

**Changelog.** 0.2: the career has state. The 19 slots marked supported answer; the rest answer `BB_ERR_UNSUPPORTED`.
The milestone reward is bounty, not cash: `BbMilestoneDef.reward_bounty` (the same `u32` at the same offset as 0.1's
field, which nothing shipped with). The layout is unchanged. Outside career mode every slot answers `BB_ERR_NOT_READY`.
The stat numbers of `rap_sheet_set_stat` and `rap_sheet_add_stat` follow the original's rap sheet index (below); the field
lists of `BbRapSheet`, `BbRival` and `BbMilestone` are [unconfirmed] against the specs.

| Offset | Slot | Arguments after `ctx` | Writes | Status | SDK |
|---|---|---|---|---|---|
| 16 | `cash_get` | `out: *mut i64` | no | supported | `cash()` |
| 24 | `cash_add` | `delta: i64, reason: BbStr` | yes | supported | `add_cash(delta, reason)` |
| 32 | `cash_set` | `value: i64` | yes | supported | `set_cash(value)` |
| 40 | `bounty_get` | `out: *mut u64` | no | supported | `bounty()` |
| 48 | `bounty_add` | `delta: u64` | yes | supported | `add_bounty(delta)` |
| 56 | `bounty_set` | `value: u64` | yes | supported | `set_bounty(value)` |
| 64 | `rap_sheet_get` | `out: *mut BbRapSheet` [unconfirmed fields] | no | supported | `rap_sheet()` |
| 72 | `rap_sheet_set_stat` | `stat: u32, value: u64` | yes | supported | `set_stat(stat, value)` |
| 80 | `rap_sheet_add_stat` | `stat: u32, delta: u64` | yes | supported | `add_stat(stat, delta)` |
| 88 | `rival_count` | `out: *mut u32` | no | supported | `blacklist()` (count, then `rival_at`) |
| 96 | `rival_at` | `index: u32, out: *mut BbRival` | no | supported | `blacklist()` |
| 104 | `rival_defeat` | `rank: u32` | yes | unsupported | `defeat_rival(rank)` |
| 112 | `rival_set_locked` | `rank: u32, locked: u8` | yes | unsupported | `set_rival_locked(rank, locked)` |
| 120 | `milestone_count` | `out: *mut u32` | no | supported | `milestones()` |
| 128 | `milestone_at` | `index: u32, out: *mut BbMilestone` | no | supported | `milestones()` |
| 136 | `milestone_complete` | `id: u32` | yes | supported | `complete_milestone(id)` |
| 144 | `milestone_register` | `def: *const BbMilestoneDef, out_id: *mut u32` | yes | unsupported | `register_milestone(&def)` |
| 152 | `milestone_progress` | `id: u32, progress: f32` | yes | unsupported | `set_milestone_progress(id, progress)` |
| 160 | `stage_get` | `out: *mut u32` | no | supported | `stage()` |
| 168 | `garage_count` | `out: *mut u32` | no | supported | `garage()` |
| 176 | `garage_at` | `index: u32, out: *mut BbOwnedCar` | no | supported | `garage()` |
| 184 | `give_car` | `car: u64` | yes | unsupported | `give_car(car)` |
| 192 | `remove_car` | `car: u64` | yes | unsupported | `remove_car(car)` |
| 200 | `current_car_get` | `out: *mut u64` (0 for none) | no | supported | `current_car()` |
| 208 | `current_car_set` | `car: u64` | yes | supported | `set_current_car(car)` |
| 216 | `save` | none | no | unsupported | `save()` |
| 224 | `slot_get` | `out: *mut u32` (0 for none) | no | unsupported | `slot()` |
| 232 | `safehouse_count` | `out: *mut u32` | no | unsupported | `safehouses()` |
| 240 | `safehouse_at` | `index: u32, out: *mut BbSafehouse` | no | unsupported | `safehouses()` |
| 248 | `safehouse_set_unlocked` | `id: u32, unlocked: u8` | yes | unsupported | `set_safehouse_unlocked(id, unlocked)` |
| 256 | `safehouse_register` | `interior: u64, out_id: *mut u32` | yes | unsupported | `register_safehouse(interior)` |
| 264 | `safehouse_set_cools_heat` | `on: u8` | yes | unsupported | `set_safehouse_cools_heat(on)` |
| 272 | `shop_count` | `out: *mut u32` | no | unsupported | `shops()` |
| 280 | `shop_at` | `index: u32, out: *mut BbShop` | no | unsupported | `shops()` |
| 288 | `shop_stock_count` | `shop: u32, out: *mut u32` | no | unsupported | `shop_stock(shop)` |
| 296 | `shop_stock_at` | `shop: u32, index: u32, out: *mut BbShopItem` | no | unsupported | `shop_stock(shop)` |
| 304 | `shop_add_item` | `shop: u32, def: *const BbShopItemDef, out_id: *mut u32` | yes | unsupported | `shop_add_item(shop, &def)` |
| 312 | `shop_remove_item` | `shop: u32, item: u32` | yes | unsupported | `shop_remove_item(shop, item)` |
| 320 | `shop_set_price` | `shop: u32, item: u32, cash: u32` | yes | unsupported | `set_price(shop, item, cash)` |

Not in 0.2: the career events (`BountyChanged`, `CashChanged`, `MilestoneCompleted`, the blacklist, safe-house and shop
events, the save events) and the cancelable purchase request.

### Races (`nfsmw.races` 0.1)

The race runtime is not built, so every slot answers `BB_ERR_UNSUPPORTED`.

| Offset | Slot | Arguments after `ctx` | Writes | SDK |
|---|---|---|---|---|
| 16 | `race_count` | `out: *mut u32` | no | `list()` |
| 24 | `race_at` | `index: u32, out: *mut BbRaceInfo` | no | `list()` |
| 32 | `race_checkpoint_count` | `id: u32, out: *mut u32` | no | `checkpoints(id)` |
| 40 | `race_checkpoint_at` | `id: u32, index: u32, out: *mut BbVec3` | no | `checkpoints(id)` |
| 48 | `race_start` | `id: u32` | yes | `start(id)` |
| 56 | `race_abort` | none | yes | `abort()` |
| 64 | `race_current` | `out: *mut BbRaceState` (`active` 0 when none) | no | `current()` |
| 72 | `race_register` | `def: *const BbRaceDef, checkpoints: *const BbVec3, checkpoint_count: u32, out_id: *mut u32` | yes | `register(&def, &checkpoints)` |
| 80 | `race_set_opponent_skill` | `id: u32, skill: f32` | yes | `set_opponent_skill(id, skill)` |
| 88 | `race_set_reward` | `id: u32, cash: u32` | yes | `set_reward(id, cash)` |
| 96 | `race_results` | `id: u32, out: *mut BbRaceResult` (`BB_ERR_NOT_FOUND` when none) | no | `results(id)` |

Race kinds are numbers; the list is [unconfirmed] against `nfsmw-data/src/gameplay/races.rs`. The opponents of a custom
race and the race events are not in 0.1.

### Events (0.1)

The kinds are in the MW range from `0x1000`. Each payload starts with `size`, and a reader uses a field only when `size`
covers it.

| Kind | Name | Payload fields after `size` (byte offset) |
|---|---|---|
| `0x1000` | `PURSUIT_STARTED` | `level: u32` (4), `cops: u32` (8), `heat: f32` (12); 16 bytes |
| `0x1001` | `PURSUIT_ENDED` | `reason: u32` (4; 0 evaded, 1 busted, 2 aborted), `bounty: u64` (8), `duration: f32` (16); 24 bytes |
| `0x1002` | `HEAT_CHANGED` | `level: u32` (4), `old_heat: f32` (8), `new_heat: f32` (12); 16 bytes |
| `0x1003` | `BUSTED` | `level: u32` (4), `bounty: u64` (8); 16 bytes |
| `0x1004` | `ESCAPED` | `level: u32` (4), `duration: f32` (8); 12 bytes |
| `0x1005` | `INFRACTION_RECORDED` | `infraction: u32` (4; the `Infraction` number 0 to 7), `fine: u32` (8); 12 bytes |

The game builds the payload and dispatches it with `ModRuntime::dispatch_event` (`nfsmw_modding::host::mw_event`). The SDK
decodes the bytes with `MwEvent::decode`: a mod's `Mod::event` receives an `Event::Unknown` whose `payload` holds the
bytes of the event, and it decodes them with the frame and the kind. The other MW events of the table above (cop spawn
and wreck, the cancelable requests, and the career and race events) are not in 0.1.

When the events go out, and what they carry in the game:

- The chase is read once a frame in the police update. `PURSUIT_STARTED` is sent in the frame the chase is first seen,
  with the cops of its first wave. `HEAT_CHANGED` is sent when the heat level (the integer part) moves, not on every
  growth of the value; the two heat values are the ones either side of the move.
- `BUSTED` is sent in the frame the bust happens. `PURSUIT_ENDED` follows in the next frame, with the reason. A chase
  that is evaded sends `ESCAPED` and then `PURSUIT_ENDED` (reason 0) in the frame after the cops give up.
- `PURSUIT_ENDED` with reason 2 is a chase that was stopped with no result (the `pursuit off` command, or `end(Abort)`).
- `bounty` is the total of the fines of the chase's infractions. The game has no other bounty counter in 0.1.
- `INFRACTION_RECORDED` is sent once for each offence of the chase, in the frame it is recorded.
- The events go out in the `ModsPost` step, after the scene has updated, so the frame number is the one of that frame.

### Open items for 0.1

- **Cop tables.** Delivered for the heat table and the race table as data: the install's wave is read from
  `ATTRIBUTES.BIN` (not from the running chase, which exists only during a chase), and a set wave replaces the row of
  its level in the next chase (`begin_pursuit`, which then hands the rows to `TrafficWorld::start_pursuit`). The cop car
  of every name in the waves is loaded before the chase starts, a mod's car included; one that does not load is logged
  and its entries are left out (`cop_rows::without_models`). The race table is kept but no race runs the waves yet.
  Open: the mod-car path is covered by tests of the selection only; no run has yet spawned a mod's cop car.
- **Data.** `nfsmw-modding` does not depend on `nfsmw-data`, so the install stack stays out of mods. The game converts
  `nfsmw_data::pursuit::HeatRow` into `BbHeatRow` and `BbCopRecord` in its facade.
- **Career and races.** Career is at 0.2 with state behind 19 of its slots (see Career). Races are offered with no state; their table is the contract the race work implements.
