# The world host: the real world behind `WorldSim`

Status: built 2026-10-22 on the LAN branch ([ADR 0011](../../decisions/0011-lan-multiplayer-on-the-player-world.md)). How the
server drives the real world, `WorldScene`, through the `nfsmw_sim::WorldSim` seam, what each trait call does in it, and what
is not done yet. The code is `crates/nfsmw/src/scenes/world/net/`; paths below are relative to `crates/nfsmw/src/` unless they
start with `libs/` or `crates/`.

## 1. Shape

```
nfsmw-server  Server<W: WorldSim>  (fixed 60 Hz loop, inputs per seat, snapshots)
        │ step(&SimInputs, events) · entities · own_state · state_hash · add_player / remove_player
        ▼
WorldSimHost  (scenes/world/net)
        │ owns
        ├─ WorldScene   the single-player world, under SessionRules::session(seed)
        └─ NullBackend  app/null_backend.rs: a renderer that draws nothing
```

- **`WorldSimHost::open(dir, HostConfig)`** opens the world at `HostConfig::at` with the host's car (`HostConfig::car`, the
  lead, `PlayerId(0)`), creates the `NullBackend`, applies `SessionRules::session(seed)` (no time scaling, seeded streams,
  the players' own views, blocking model loads, lockstep streaming), sets the traffic budget if asked, switches the physics
  clock to one step per update, and runs the world until the host's car stands on the road. Those opening ticks are not
  counted: the host's tick is 0 after `open`.
- **`NullBackend`** was the tests' renderer (`app/test_backend.rs`); it is now `app/null_backend.rs`, built in every
  configuration. The tests keep using it through `app::test_backend`, an alias of the same module.
- **Who drives it** (2026-10-22, [game wiring](game-wiring.md)): the listen server's thread and `nfsmw --dedicated` run
  `nfsmw_server::Server<WorldSimHost>` over it; the end-to-end test runs one with a client-mode world and a bot.
- `scene()` and `scene_mut()` give the world and the renderer to the caller between ticks (console commands, a race to
  queue), `catalog()` the car catalogue, `request_reset(id)` a reset on the next tick, `stalled_ticks()` the ticks whose
  streaming wait ran out.

## 2. `step`: one tick

1. **Input.** Every player in the world, parked ones too, gets its `InputState` from `SimInputs` (neutral when it has none)
   through `DriveInput::from_vehicle` (`drive/remote.rs`), the inverse of the mapping the car's physics step uses
   (`DriveInput::to_vehicle`, now the only copy of it). Non-finite axes read as zero, every axis is clamped to its range,
   both shift edges at once become neither (the wire's rule), and the handbrake, a button in the car, is on at half travel or
   more. Speedbreaker and reset are not in `InputState`: the speedbreaker is barred in a session anyway, and a reset comes
   from `request_reset`. The input reaches the car through `Drive::set_remote_input`, which replaces the devices' input in
   `Drive::prepare_frame`; presses (shift edges, a gear, a reset) wait for the next physics step as a device's do. A car with
   no remote input reads its devices exactly as before.
2. **Streaming.** With `HostConfig::settle_streaming` set, the host first asks for the traffic's support tiles (as the
   frame will) and waits (at most that long) until every tile the world wants is resident and none is in flight, as the
   determinism test does. Without the support tiles one host's traffic could wait on the disk where another's did not. Without it the host never waits: while a
   player's surroundings are missing the world's own gate holds the simulation still and the tick passes with no physics.
3. **One tick.** `Scene::update(renderer, idle actions, STEP)`. The physics clock is in one-step-per-update mode
   (`FixedClock::set_one_per_update`, `drive/clock.rs`): any update with time to spend runs exactly one physics step, so the
   wall clock never decides how many ticks run. Everything else the frame does (races, career, co-op, catch-up, effects)
   runs as in a frame of 1/60 s.
4. **Events** (`net/events/`), each from state the world already keeps:

| Event | Source |
|---|---|
| `Contact { a, b, speed }` | The tick's car contacts: player pairs (`players/contact.rs`), the followed player and the guests against the traffic, and traffic pairs (`ai/touches.rs`, filled by `collide` and `collide_guests`). One event per entity pair, the lower id first, at the pair's highest closing speed (measured before the contact), in id order; a car and its trailer are one entity |
| `PursuitStarted`, `PursuitEnded`, `Busted`, `Evaded` | `PursuitTracker` over the chase after the tick: the co-op chase's targets (ADR 0010 section 4) when remote players are in, else the police's one player; the police's bust flag (`TrafficWorld::is_busted`) for the player they follow; `end_reason() == Evaded` when the chase ends in an escape. A player busted on its own meter gets `Busted` then `PursuitEnded` and does not restart the same chase |
| `LapCompleted`, `RaceFinished`, `SpeedTrapPassed` | `RaceTracker` over the race engine's racers: a new lap time is a lap (its own duration), a finish time a finish, a speed trap's stored speed that is new or higher a pass (km/h); places count the computer racers. The first human racer is the lead, then the race's people in grid order (`scene.race_people`; the split-screen guest in a co-op race without a network) |
| `Moved { player }` | `MovedTracker`: a car that was waiting for its road and now stands on it, or one that went further in a tick than its speed allows (2 m slack): a reset, a bust's respawn, a catch-up warp |

The tick count goes up by one per `step` whatever happened, so it is the server's clock.

## 3. Players

- **The host's own player** is the lead, `PlayerId(0)`, in the car of `HostConfig::car`. Its car exists from `open` on:
  `add_player(PlayerId(0), _)` only marks it seated (a second call is `AlreadyJoined`), and `remove_player(PlayerId(0))`
  leaves the car in the world driving with neutral input. The world needs a lead for its traffic, police and streaming.
- **Remote players** (`PlayerId` 1 to 15) are co-op guests (`net/seat.rs`): the car named by `PlayerJoin::car_type` (the
  bChunk hash of the upper-case car folder name, `net::catalog::car_type_id`) comes in through the co-op spawn path
  (`players/spawn.rs`), at `PlayerJoin::spawn` or in a line behind the lead, 12 m per id. An id of 16 or more is `Full`, a
  taken id `AlreadyJoined`, a car the install lacks or that fails to load `UnknownCar`. The installed parts are not applied.
- **The co-op session** (`players/coop.rs`) is opened with the first remote player and closed when the last one leaves. It
  holds the shared chase, so every remote player is a possible target of one pursuit: the police follow a primary, cops are
  split between targets, and each target has its own busted meter while two or more are chased (ADR 0010 section 4). A busted
  remote player is sent to the career's safe house when one was visited, else stays where it is (`career/coop_bust.rs`); a
  remote player has no career car, so no fine or bust record is written for it. A leaving player leaves the chase too.
- **Parked and absent players** behave as in co-op: a parked car (a cutscene, a co-op race seat) is not stepped, not
  exported and not counted; its own-car block says `OWN_PARKED`.

### N-player fixes made for this

Co-op was built for two players; these places assumed it and now take any number:

| Place | Before | Now |
|---|---|---|
| `players/suspend.rs` | Parked player 2 for a cutscene | Parks every guest in play and brings back exactly those (`CoopSession::cutscene_parked`) |
| `players/race.rs` | Brought player 2 back behind the lead | `return_guests_behind_lead(ids)`: each parked player of `ids` in a line, 12 m apart |
| `players/catch_up.rs` | Judged player 2 against the lead; only the lead could be a target | The anchor is the first busy player; every other player in play is judged against it; any chased player is a target and is never offered the warp; `CoopSession::catch_up` is a list; a network seat's reset counts as its Reset press |
| `players/contact.rs` | Player pairs collided silently | The pairs that touched are kept for the contact events |

Already N-player and only checked: residency centres (`focus.rs`, one per player in play), the traffic budget (the spawn
focus turns over every player, `ai/population/guests.rs`), the chase (`players/chase.rs`), player pairs, the garage limits
(`Limits::for_players`, any number over one is the co-op limit) and the tether (a pure distance rule).

## 4. `entities`, `own_state`, `state_hash`

- **`entities`** appends every replicable entity in ascending id order (`net/export.rs`), poses and velocities in render
  space (x forward, y left, z up, metres) after the tick, not interpolated:

| Entity | Id | Kind | `type_id` | Flags |
|---|---|---|---|---|
| A player's car on the road | its `PlayerId` (0 to 15) | `PlayerCar`, `owner` set | the car folder's index in the sorted car folders (`CarCatalog::type_number`) | braking, headlights |
| A traffic car | table | `TrafficCar` | as above | braking, wrecked |
| A police car | table | `PoliceCar` | as above | braking, light bar, wrecked |
| A roadblock's car | table | `Roadblock` | as above | `FLAG_ROADBLOCK_CAR`, light bar |
| A spike strip | table | `Roadblock` | `SPIKE_STRIP_TYPE` | `FLAG_SPIKE_STRIP` |
| The helicopter | table | `Helicopter` | 0 | none |
| A prop body in motion | table | `MovingProp` | low 16 bits of the scenery name hash | none |

  The ids of everything but the players come from `EntityTable` (`net/ids.rs`): 16-bit, from 16 up, round robin over the
  space and skipping ids in use, so an id comes back only after about 65 thousand appearances. New things are numbered in
  the order of their world identity (the traffic's own entity id, the roadblock and strip index, the prop id), so two hosts
  with the same inputs number alike. The table follows the world after every tick.
- **`own_state`**: rpm, gear (wire ids: reverse 0, neutral 1, first 2), nitrous and the speedbreaker meter (0 to 1) from the
  car's HUD state, and the flags `OWN_SPEEDBREAKER`, `OWN_SHIFTING`, `OWN_SHIFT_LIGHT`, `OWN_WAITING`, `OWN_PARKED`
  (`net/flags.rs`). `None` for a player not in the world.
- **`state_hash`**: the tick, then `WorldScene::state_hash` (players' cars, traffic and its streams, police, pursuit, race;
  `state_hash.rs`), through the same FNV-1a hasher.

### `WorldControl` (`net/control.rs`)

| Call | As built |
|---|---|
| `start_race` | Finds the race whose collection name hashes to the track id (`fe_hash_upper`), and begins it with the session's laps and traffic. Every remote player is one of the race's people (section 7). The world's own grid, countdown and finish run; a session skips the race intro (`SessionRules::race_intros`) |
| `stop_race` | Ends the race the world runs; the people's cars come back behind the lead |
| `reset_player` | `request_reset` on the next tick; refused for a player with no car |
| `set_ghost` | Parks the car (co-op parking): out of play, not stepped, not collided, not exported; unparking brings it back where it stood |
| `fill_global` | The world's time of day; for a chased or busted player the heat, its busted meter (its own while two or more are chased), the cooldown bar and the chase state (`Chased`, `Cooldown`, `Evaded`, `Busted`); for a racer its place and lap from the race card |

## 5. Single player is unchanged

The hooks are inert unless a host sets them: `Drive::remote_input` is `None`, the physics clock follows the frame time, and
the contact lists are written but read by nobody (`TrafficWorld::touches` is cleared every tick, `Players::contacts`
replaced every tick). `DriveInput::to_vehicle` is the mapping the physics step had inline. The single-player guards
(`drive/sim/tests/frame_rate.rs`, `ai/pursuit/tests/frame_rate.rs`, the world tests and `determinism_tests.rs`) run
unchanged.

## 6. Tests

| What | Where | Needs the install |
|---|---|---|
| Every wire input code survives the car (steering, pedals, clutch, buttons, edges, gears); the handbrake is on from half travel; hostile input stays in range | `net/tests/input.rs`, `drive/remote.rs` | no |
| One step per update | `drive/clock.rs` | no |
| Entity ids: order, keeping, round robin, overflow, two tables alike | `net/ids.rs` | no |
| Car catalogue | `net/catalog.rs` | no |
| Contact, pursuit, race and moved events | `net/events/*.rs` | no |
| Player contacts are recorded with ids and closing speed | `drive/sim/tests/player_contacts.rs` | no |
| The traffic's export: cops in id order, kind, model, pose | `ai/pursuit/tests/net_export.rs` | no |
| N-player catch-up | `players/catch_up.rs` | no |
| Two hosts from one seed, same inputs: equal events every tick, equal hash and entities every second (40 s, 2 players, traffic and cops) | `net/tests/installed.rs` | yes |
| 16 players join, export in id order with kinds and owners, own-car blocks, leave; `Full`, `AlreadyJoined`, `UnknownCar` | `net/tests/installed.rs` | yes |
| NaN, infinite and huge input for 10 s: no panic, finite entities | `net/tests/installed.rs` | yes |
| A bump between two players is a `Contact`; a console chase gives `PursuitStarted` and `PursuitEnded` | `net/tests/installed_events.rs` | yes |
| A sprint with the lead and two remote players: they are the race's people in grid order, parked from free roam, exported as their player cars, driven by their seats' pedals (20 m in six seconds each), named in the race events, and back on the road after `stop_race` | `net/tests/installed_race.rs` | yes |
| A trap pass comes once, and again only when it beats the best; the person grid's racer numbers | `net/events/race.rs` | no |

## 7. Races with remote players

The server asks for a race with `WorldControl::start_race` when the lobby enters loading (`server/phase.rs`: the race modes with a
track chosen). In the world (`players/race.rs`, `race/start.rs`):

- **The people.** Every remote player (a seat whose input a server holds, `Drive::remote_pedals`) is a human racer on the grid right
  after the lead, in id order, in the car of their loadout: `Opponent`s with `person` set, like the split-screen guest. Their
  free-roam cars are parked for the race and come back behind the lead when it ends (`unpark_guest_after_race`). A person whose car
  does not load or stand is dropped from `scene.race_people`, so the racer numbers stay right. Story races (the scripted chase and the
  final race) are never given people; the lead drives them alone.
- **Pedals.** A person's racer is a car of the traffic world (the AI car's physics, with a person seated): each tick
  `feed_race_people` copies the seat's pedals into it (`set_racer_pedals`). The lead drives its own `Drive` as always.
- **Export.** The racer is exported as the player's car (`PlayerCar`, the player's id and owner) and left out of the traffic ids
  (`net/export.rs`), so the clients, the mode rules' car samples and the spectators see one car per player.
- **Events.** `racer_states` names the lead and then the people, so laps, finishes and speed-trap passes reach the mode rules by
  player id; the co-op DNF timer (30 s after the first human finishes) ends a race whose lead nobody drives, such as a
  dedicated server's idle car.
- **Not done.** The race car is not a `Drive`: it has no own-car block (rpm, gear, nitrous read zero, `OWN_PARKED` stays set), and a
  client that predicts its car with its own `Drive` physics meets the AI car's, so corrections are larger than in free roam. The
  post-race screen is skipped; the race ends 13 s after the finish (the race-over message and the post-race wait).

## 8. Known gaps

- **Races with remote players.** See section 7 for what is built. The people race in one grid with the race's own computer
  opponents; the server's `RaceStart` countdown and the world's own countdown are not aligned.
- **Reset.** `InputState` carries no reset; the server calls `request_reset` through `WorldControl::reset_player` when a
  client sends `ResetRequest` (the client's reset key).
- **Speedbreaker.** Barred in a session (`SessionRules::time_scale_allowed`); a per-car speedbreaker waits for
  [features](features.md).
- **The host's car** cannot be changed by `add_player(PlayerId(0))`, and it stays in the world when its player leaves. A
  dedicated server without a host player still has this car: the first player to join (slot 0) drives it, whatever car
  the loadout names.
- **Parts.** `PlayerJoin::parts` is not applied; remote cars are stock.
- **Streaming.** Without `settle_streaming` a missing tile freezes the whole world for the tick (the single-player gate),
  which is safe but not what [integration](integration.md) section 2 wants (only the stalled client's car should wait).
  With it, a server tick can wait on the disk. The wait is the lockstep rule the determinism test uses; a server that streams
  only collision is the extraction's work.
- **Wall clock.** The car's 10 s road wait (`drive_input.rs`) is still wall clock ([determinism](determinism.md) D2); the
  host waits for streaming instead, so it does not trigger in the tests.
- **Moving props** carry a 16-bit hash of their scenery name, not their prop id, so a client matches a moving body to its own
  resident prop by type and position.
- **No `Send`.** The scene holds `Rc`s, so the host runs on the thread that made it (integration.md section 7, question 1).
- **Contacts with props** are not events; only car pairs are.
