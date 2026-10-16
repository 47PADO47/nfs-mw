# Mod API catalogue: the NFS:MW extension

Part of the [modding catalogue](README.md). Everything here is **MW** scope: it lives in
`crates/nfsmw-modding`, is reached with `ctx.ext::<nfsmw_modding::sdk::...>()`, and exists only when the running
game id is `nfsmw`. A mod that uses it lists the interfaces in `[requires]` or `[optional]` and sets
`supported = ["nfsmw"]`. Conventions are in [api.md](api.md#how-to-read-an-entry); **W** marks a write that is
denied in an online session.

Status of the game side: the pursuit (heat, cops, roadblocks, busted, evade, helicopter) exists in code. The
**career** (bounty, rap sheet, blacklist, milestones, cash, shops, safe houses) has no runtime state yet; the
tables below are the contract the career work will implement. Field lists marked [unconfirmed] must be checked
against the specs (`docs/specs/`, `docs/formats/`) when each part is built.

| Interface | Version | Contents |
|---|---|---|
| `nfsmw.pursuit` | 1.0 | pursuit state, heat, cops, support, cop tables |
| `nfsmw.career` | 1.0 | cash, bounty, rap sheet, blacklist, milestones, safe houses, shops |
| `nfsmw.races` | 1.0 | race events: list, start, control, custom races |

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
| Replace row | `pursuit.set_cop_table(table, level, HeatRow) -> Result<()>` **W** | MW | same, applied in `scenes/world/ai/data.rs` |
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
`libs/save-store`); the data it reads is in `nfsmw-data/src/gameplay/`. [unconfirmed]: the exact stat list and
milestone ids until the career spec exists.

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
| Register milestone | `career.register_milestone(MilestoneDef { key, name, goal: f32, reward_cash: u32 }) -> Result<MilestoneId>`, `career.milestone_progress(MilestoneId, f32)`; mod-owned milestones are saved in the mod's slot data | MW | same, `libs/save-store` |
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
