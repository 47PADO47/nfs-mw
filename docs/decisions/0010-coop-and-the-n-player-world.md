# 0010 — Co-op and the N-player world

- **Status:** accepted (2026-10-22). Extends the world model of [ADR 0006](0006-multiplayer-netcode.md) and [ADR 0009](0009-multiplayer-netcode-revisions.md)
  and does not change their netcode. It amends the pursuit rule of MP-07 (see decision 4).
- **Milestone:** split-screen co-op now, on the single-machine path. LAN play (milestone 10) reuses the same roster, input and
  pursuit types with remote seats; nothing in this ADR is rewritten for it.
- **Inputs:** the owner's request for co-op career and free roam on one save; ADR 0006 and ADR 0009; the
  [split-screen spec](../specs/split-screen.md); [multiplayer features](../specs/multiplayer/features.md) (MP-07);
  [multiplayer integration](../specs/multiplayer/integration.md) (section 4).

## Question

The owner wants two people to play career and free roam together on one save. They share a garage. Each has a name
(`<alias> 1` and `<alias> 2`). Rival races have both players and the rival in the field. A police chase is shared, but each
player is busted on their own. Split-screen already exists, but its second player is an AI-slot car, which works for one race
and not for an open world. What world model carries co-op now, and LAN later, without a second rewrite?

## Facts checked on 2026-10-22

- `WorldScene` holds one `drive: Option<Drive>` (`crates/nfsmw/src/scenes/world/mod.rs`). A search of `crates/nfsmw/src` finds
  about 200 `.drive` accesses. The field feeds pursuit, heat, bounty, bust, safe house, entry zones, GPS, map, speed traps,
  streaming, traffic focus, and race index 0.
- Police assume one car. There is one busted meter and one sighting per chase.
- Other systems that assume one car: traffic focus, respawns, the GPS guide, zones (one residency centre), race index 0, and
  career pursuit settlement.
- Split-screen's second player is an AI-slot "Opponent" car. It is driven by `set_racer_pedals` (`crates/nfsmw/src/scenes/world/ai/racers.rs`,
  `split/mod.rs`). It has no `Drive` of its own in the player path, and it cannot be busted, respawned or streamed as a player.
- `blackbox-race` already supports several human seats.
- `blackbox-pursuit` and `nfsmw-career` hold pure rules. Neither depends on Bevy.
- `libs/blackbox-net-core/src/session/` holds `peer.rs` (`PeerId`) and no roster type yet.

## Options

### A. Keep P2 as an AI-slot car and bolt rules on

Every system gets a special case for "the second car". Pursuit checks a second handle, the career code checks two names, and so
on. This is the smallest change for split-screen. It is rejected because every rule that needs a second human becomes a second
code path. LAN adds a third kind of human (a remote one, not an AI), and P2 can never be busted, respawned or streamed like P1
without more special cases.

### B. An N-player world

`WorldScene` holds `players: Players`, keyed by `PlayerId`. Each `Player` owns a full `Drive`, a name, a car handle and its
respawn state. Input arrives per `PlayerId`. P1 is the lead player. Single-player is the one-entry case of the same code.
Pursuit, streaming, traffic and races take lists of players. This is the chosen option.

### C. Extract the headless sim first, then add players

Move the world into `nfsmw-sim` (`WorldSim`, integration.md section 4) and build co-op there. This gives the cleanest end state,
but the extraction is a large move with no visible feature, and co-op would wait behind it. Option B keys the new code by the
same `PlayerId` and `Inputs` that `WorldSim::step` will take, so the later extraction moves code rather than redesigning it.
The extraction stays on the plan.

## Decision

Option B. Seven parts.

### 1. Roster types live in blackbox-net-core

`PlayerId(u8)`, `Seat { Local { slot }, Remote { peer } }`, and `Party` go in `libs/blackbox-net-core/src/session/party.rs`, next to
`PeerId`. `Party` joins a player to the lowest free id, removes a player, and looks up seats and ids.

The types are engine-generic, so they belong in `libs/` with no NFS:MW names. Split-screen's local devices and LAN's remote
peers share one roster, so "who is in the world" has one answer. LAN adds `Remote` seats and does not add a type.

### 2. The world holds `Players`

`crates/nfsmw/src/scenes/world/players/mod.rs` defines `Players` and `Player { id, drive, name, car_handle, respawn_pending }`.
`Players` offers `lead`, `lead_mut`, `get`, `iter`, `push` and `remove`. It replaces `drive: Option<Drive>`.

The lead player is always P1 (`PlayerId(0)`). Single-player runs the same code with one entry, so its behaviour does not change.
The single-player world trace is the regression gate for the change.

The mod API keeps `player_car` pointing at the lead player. A `players()` call for mods is a follow-up (see "Not decided here").

### 3. Input is keyed by PlayerId

`Inputs` maps each `PlayerId` to an `ActionState`. Split-screen fills it from two local devices, and `SplitInput` keeps the
device side. LAN fills it from remote seats. The same shape becomes the `&Inputs` argument of `WorldSim::step(tick, &Inputs)`,
so that signature does not change when the sim moves out of the scene.

### 4. One pursuit, several targets

`libs/blackbox-pursuit/src/targets.rs` adds three things:

- `TargetId`, one per player in a chase.
- `Targets`, the state of each target: `Chased`, `Busted` or `Left`, plus `is_over`.
- `assign(cops, targets)`, a balanced greedy split of cop cars between targets, preferring the nearest target.

The split of state is as follows.

- **Shared per pursuit:** heat, cooldown, helicopter and roadblocks.
- **Per target:** busted meter, infractions and sighting.
- **Start:** the chase starts at the higher carried heat of the cars involved.
- **End:** the pursuit is over when no target is still chased.

This amends MP-07. MP-07 says heat is tracked per player. Here heat is shared, and only the busted state is per player. The
owner chose a shared chase in which each player is busted on their own.

### 5. Career rules for co-op

`crates/nfsmw-career/src/coop/` holds the Bevy-free rules.

- **Names:** `<alias> 1` and `<alias> 2`.
- **Places:** `credited_place` credits a rival race with the best human place, once.
- **Bounty:** a bounty is banked once into the shared career, so Blacklist progress is not counted twice. Each car keeps its own heat.
- **Garage:** `Limits::for_players(n)` gives the garage a cap of 25 record slots (`CAREER_SLOTS`), the same as today. `Limits::COOP`
  is the two-player value. The save format does not change.
- **Session:** co-op state (who is P2 and in which car) lasts for the session only.

### 6. Streaming: one residency centre per player

Each player has its own residency centre. `Zones` loads the union of all centres. The simulation waits until both players'
areas are resident.

Distance is handled by a soft prompt, not a forced warp. Past 500 m, the trailing player gets a "Catch up" prompt. The owner chose
no forced warp. The fallback, if memory runs out, is a forced warp past about 1500 m. That fallback needs a new owner decision.
Residency bytes are logged, so the memory cost is measured and not guessed.

### 7. Presentation reuses split-screen

These pieces are reused unchanged: `SplitInput`, `SplitOverlay` (the two half captures), `SplitHuds` and the `Scene` split hooks.
One piece is new: a per-half pursuit strip (heat, busted and cooldown), in `hud/split/pursuit.rs`. The game's split HUD files have no
pursuit widgets.

Native viewports (one window with two real viewports) are out of scope. They need their own ADR.

## Why not the alternatives

- **A** keeps the second player as a special case in every system. Each new rule doubles, and LAN adds a third kind of player.
- **C** is the right end state, but it puts a large move ahead of a visible feature. B uses the same keys, so C can follow later
  without changing types.

## Consequences

- **LAN reuses this unchanged.** `PlayerId`, `Party`, `Inputs`, the pursuit targets and the career rules all carry over.
  LAN changes only where `Inputs` comes from (the network, not devices) and what `Seat::Remote` holds.
- **Mods.** `player_car` stays the lead player. Mods see P1 only.
- **Single-player behaviour.** After the rename from `drive` to `players`, the single-player world trace must match line for line.
- **Performance.** Co-op renders two half captures per frame. If a half falls under 30 Hz, a later task adds a second capture slot
  in `libs/blackbox-render/src/gpu/view_capture.rs`. The cost is measured with `--bench-seconds`.
- **Memory.** Two residency centres can roughly double resident scenery in the worst case. Residency bytes are logged.
- **Code removed.** Once the split custom race runs on `Players`, the AI-slot path goes: `seat_person`, `set_racer_pedals`,
  `split::SECOND` and `SplitState.camera`.
- **Size.** The 500-line rule applies. The `players/` folder is split by concern, one concern per file.
- **Rule text.** The full co-op rules belong in a separate spec, `docs/specs/coop-career.md`. This ADR records the decisions only.

## Not decided here

- **Native viewports** (one window, two real viewports). Own ADR.
- **Mod API `players()` call.** A follow-up. `player_car` stays the lead player until then.
- **Human cops.** A player driving a police car is out of scope.
- **Persistence of P2's car.** Session only. The save format does not change.
- **Forced warp past about 1500 m.** A fallback only, and an owner decision if memory runs out.
- **Timing of the `nfsmw-sim` extraction.** Stays on the plan in integration.md section 4.
