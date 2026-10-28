# Chameleon engine on/off: the ignition sound and the standstill auto-stop

What NFS: Most Wanted (2012) holds about an engine that is switched off and started, for the mod setting **2012 engine
auto-stop** ([api-engine.md](../modding/api-engine.md), the mod's [README](../../mods/nfsmw-2012-content/README.md)). Measured on
the 2012 install (v1.4.0.0) with the bundle reader ([chameleon-bundle.md](chameleon-bundle.md)), the sound graph reader
([chameleon-sound-graph.md](chameleon-sound-graph.md)) and strings and references in `NFS13.exe`. Evidence tags as in
[README.md](../README.md#evidence-tags): **[verified]** is read from the data, **[exe]** is a string or a table of the
executable, **[unconfirmed]** is a reading or a design value that the data does not give. Reader and model:
[`libs/chameleon-engine-state`](../../libs/chameleon-engine-state).

## Short answer

The 2012 game has an **engine on/off state** and an **ignition sound** for the player's car. This work **did not find the rule
that switches the engine off when the car stands still**: no idle time, no speed threshold and no restart input is named by any
string, sound graph, state machine or table that was searched. The rule in the mod ([The model](#the-model)) is therefore a
design built from the description of the 2012 game's behaviour, with numbers that are all **[unconfirmed]**. What is read from
the data is the part around it: that the engine has an on/off state, what silences the engine sound, and the start-up sound.

| What | Where | Status |
|---|---|---|
| `Players.LocalPlayer.IsEngineOn` ("Is the engine turned on atm?") | `NFS13.exe` property table, read by the HUD behaviours | [exe] name and meaning; [verified] that the HUD reads it |
| HUD reaction to an engine that is off | `GAMEMODES.BNDL` behaviours `Engine Off Group`, `Wait For Engine On`, `Exit When Engine Is Off`, widget groups `Engine On Off` and `Engine On Off Freedrive` | [verified] names; what the widgets show is not decoded |
| `RaceCarEngineSound.EngineOn` ("Is the Engine on (as a [0,1] value)") | `NFS13.exe`; read by the engine sound graphs | [exe]; [verified] that the graphs use it as a gain |
| The engine goes silent through `EngineOn` | player and opponent engine sound graphs of every car | [verified] |
| `Player Vehicle Ignition` sound sequence | 40 of the 93 car bundles | [verified]; the roles of its two items [unconfirmed] |
| Standstill auto-stop: idle time, speed, restart input, conditions | nowhere found | **not found** |

## The engine state in the executable **[exe]**

`Player.IsEngineOn` is registered with the player type (the table that holds `GetPlayerRPM`, `GetPlayerGas`, `IsInChase`, ...;
registration function at `0x006d1830`, the getter at `0x006d8ab0`). The getter finds the player's race car, tests that its
slot is active, and answers the function at `0x00572c20`, which is
**false when bit 2 of the race car's flag word (offset `0x1a40`) is set**, and otherwise true when a vehicle state value at
offset `0x1bd8` of the car is neither 0 nor 1. `RaceCarEngineSound.EngineOn` (getter `0x0059ba00`) is `0` when the same bit is
set and a float read from the car otherwise. Two facts limit what this says about a rule:

- Every instruction in the executable that touches offset `0x1a40` of a race car is a load, except the stores of the car's
  construction (the flag word is set to the value passed in when the car is created) and of a copy of the car. Nothing sets or
  clears bit 2 later with an `or` or an `and`. So the flag is a property the car is created with, not a state that changes while
  the car stands still. (The executable has little analysis: ~50,000 functions and few references, so a store through a pointer
  that Ghidra did not follow is possible. **[unconfirmed]**)
- The value at `0x1bd8` is read as a three-way state (0, 1, other) by a getter at `0x008f5330`, and compared with 6 to 8 and
  10 to 12 by the car's presentation code, so it is a wider vehicle state than an engine switch. It is written only when the car
  is created and when it is reset to 0.

## How `EngineOn` silences the engine **[verified]**

The player's engine load sequence (`Players.LocalPlayer.GetPlayerEngineLoad`, 12 mixer items, see
[chameleon-sound-graph.md](chameleon-sound-graph.md)) has two mixer items whose gain modulator is the expression
`RaceCars.EngineSound[#].EngineOn` with no curve (input 0 to 1, output 0 to 1, so the gain is `EngineOn` itself) on the Porsche's
mixer channels `73504` and `b381c`, the channels the engine voices play on. The idle loop of the opponents' engine sound has
`RaceCars.EntityEngineSound[#].EngineOn` as its gain input (output 0 to 1.8). An engine with `EngineOn = 0` is silent; the
gearbox whine, the tyres and the wind have their own channels and are not gated by it. The mod's host switch does the same:
engine voices, the sample layer, the spark chatter and the turbo are muted, everything else plays.

## The ignition sequence **[verified]**

A car bundle can hold a sound sequence (kind `0x15`, class `Genesys.Gen.Sequence`) whose name string is **`Player Vehicle
Ignition`**. It has no live value (range 0 to 0), so it plays once from its start. It holds two items:

| Item | Window (s) | Content |
|---|---|---|
| wave item | 0 to the sample's length | one sample, gain = the modulator's base (no mixer channel); a fade out of 0.2 s with shape 4 on the Porsche |
| effect item (class `RaceCarEffectSequenceItem`) | 0.25 to 1.5 on the Porsche | what it does is **not decoded** (a camera or controller effect is likely) |

`libs/chameleon-engine-state` reads it (`read_ignition`). On the install, 40 of the 93 car bundles have the sequence (13 distinct
samples) and 53 do not. The sample is in the car's bundle or, for most, in the samples the cars share (`VEHICLES/WAVES.BNDL`;
the Porsche's `15734a` is also in `SOUND/GLOBAL.BNDL`).

| Sequence | Cars | Sample | Length (s) | Gain | Effect window (s) |
|---|---|---|---|---|---|
| `15734c` | 8 | `15734a` (shared) | 0.591 | 0.8 | 0.25 to 1.5 |
| `b3975` | 5 | `15a1bf` | 0.767 | 2.8 | 0.43 to 1.25 |
| `228c65` | 5 | `228c58` | 1.410 | 0.5 | 0.5 to 1.5 |
| `228c0a` | 5 | `228c15` | 1.410 | 0.5 | 0.5 to 1.5 |
| `17f6e3` | 3 | `17f6d3` | 0.633 | 1.0 | 0.38 to 1.25 |
| `17f547` | 3 | `17f527` | 1.000 | 1.0 | 0.5 to 1.5 |
| `157317` | 3 | `1572a1` | 0.754 | 0.6 | 0.25 to 1.17 |
| `1d1ada` | 2 | `1d1ade` | 0.997 | 0.75 | 0.62 to 1.53 |
| `17f672` | 2 | `1d1b21` | 0.716 | 0.5 | 0.25 to 1.1 |
| others (4 sequences) | 1 each | | 0.77 to 1.0 | 0.4 to 1.5 | one has none |

(`cargo test -p chameleon-engine-state --lib -- --ignored --nocapture` with `CHAMELEON_GAME_DIR` prints the table per car.)
Nothing in the data says **when** the game plays the sequence (a trigger is not named in the car bundle); that it is the sound of
the engine starting is its name. The mod plays it when its model starts the engine.

## What was searched and not found

- Strings (case-insensitive) `ignition`, `stall`, `starter`, `engine start/off/stop/kill/cut`, `idle`, `auto stop`, `shut off`,
  `turn off`, `restart`, `stationary`, `standstill` in every block of the dumped bundles `GAMELOGIC/GAMEMODES`, `GAMEPLAY`,
  `HL_31208`, `HELICOPTERSOUNDS`, `SOUND/GLOBAL`, `METALCAR`, `MIXER_CHANNELS`, `REVERB`, `SPEEDSOUNDS`, `GLOBALCONFIG`,
  `PHYSICS/PHYSICSCONFIG`, `VEHICLES/COMPETITORSOUNDS`, `VEHICLELIST` and the Porsche's bundle: only the names in the table
  above. The word `ignition` alone was also searched in the debug name tables of all 2,021 `.BNDL` files of the install (no hit),
  and `Player Vehicle Ignition` in the blocks of the 93 `VEH_*_HI` bundles.
- Strings in `NFS13.exe`: `IsEngineOn`, `EngineOn` and the property descriptions only; nothing like `IdleShutoff`, `EngineOff`,
  `Stall`, `AutoStop`, `Restart` for an engine.
- Hashed Genesys fields (the physics and car attributes name their fields by hash) were not searched for a number, since no
  name or neighbour points at one.

So either the rule is in code without a name in the data, or the shipped game does not have it. The mod ships the rule as a
setting that is **off by default**, so nothing changes for a player who does not ask for it.

## The model

`EngineMachine` (`libs/chameleon-engine-state/src/machine.rs`) has three phases: **Running** (with the time the car has stood
still), **Off**, and **Starting** (cranking, with the time left).

| Rule | Value | Status |
|---|---|---|
| Stands still | speed below `still_speed` = 0.5 m/s either way, and the throttle below `restart_throttle` | [unconfirmed] |
| Idle time before the engine stops | `idle_seconds` = 5.0 s | [unconfirmed] ("a few seconds") |
| The brake pedal | does not count: a car held on the brake still stops | [unconfirmed] |
| Restart input | the throttle at or above `restart_throttle` = 0.1 | [unconfirmed] |
| Restart by reverse | a **fresh** brake press (at or above `restart_brake` = 0.1, after a step below it) with no throttle and a speed below `reverse_speed` = 2.5 m/s starts it like the throttle; 2.5 is where the game's own brake-to-reverse takes over. A pedal held since the stop does not, or a car held at a light would restart at once | [unconfirmed] |
| Cranking | `crank_seconds` = 0.6 s from the press to the running engine (the Porsche's ignition sample is 0.591 s) | length [verified] for the Porsche, the wait [unconfirmed] |
| Start-up sound | the car's own ignition sample if it has one, else the first car's in the install, else a generated whirr | see above |
| Where it applies | only in free driving: no race (running or queued), no pursuit, no arrest, no cutscene, no garage | [unconfirmed]; the 2012 game's behaviour in races and chases is not known, so the mod never stops the engine there |
| Rule stops applying while off | the engine runs at once, with no sound | design |

A step reports `Stopped`, `Cranking { seconds }`, `Running` or `Resumed`; tests are in `tests.rs`. The model knows nothing of
the gearbox or the lights: the game does both from the engine switch (below).

### What the game does with the engine off **[design]**

The phases `Off` and `Starting` (anything but `Running`) have three effects besides the silence, all in the game and none
read from the 2012 data:

- **Lights dark.** Headlights, tail, brake and reverse lamps, running and glow lamps, the flares, the lamp surfaces (the lit
  pieces of the lamp circuits) and the optional indicators are all off; the headlights switch keeps its position and the lights
  come back when the engine runs.
- **Neutral.** The gearbox goes to N (the HUD shows it) and is held there through the stop and the cranking: no drive torque,
  and no gear is taken, neither from a gear key, a shift button, the automatic box nor the brake's automatic reverse (the
  brake only brakes). When the engine runs the hold ends: an automatic box takes first by itself, or reverse if the brake is
  still held (it was the reverse that started the engine); a manual box **stays in N** until the driver picks a gear (the
  brake's automatic reverse still works from N, as in the stock game).
- **No throttle and no nitrous** (see [api-engine.md](../modding/api-engine.md)).

Not covered: a reverse that is asked for with a gear key or a down-shift (only the brake pedal starts the engine; `bb.engine`
does not report gear requests).

## Reading the data again

`MW2012_GAME_DIR=<install> cargo test -p chameleon-engine-state --lib -- --ignored --nocapture` lists the ignition sequences. The
searches above were made with a scratch scanner over all blocks of the bundles and with Ghidra on `NFS13.exe` (the references to
the `IsEngineOn` and `EngineOn` strings, and every instruction that reads or writes offsets `0x1a40` and `0x1bd8` of a race car).
