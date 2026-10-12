# Speedbreaker

Status: the action, the meter and the HUD bar are built. The meter's constants are placeholders, the slowdown is not
applied, and the bar's geometry is not checked on screen. Decided by the user: the Speedbreaker is a new action with a
control of its own, and it is wired to the speedometer HUD.

## 1. The action [built]

- `Action::Speedbreaker` (`input/action.rs`), a button. Its name is `speedbreaker`, and the menu calls it "Speedbreaker"
  (`input/describe.rs`).
- Default keyboard binding: **Left Ctrl** (`input/bindings.rs`). The original's Speedbreaker is on Ctrl in its own
  keyboard screen (the first screenshot), so the default follows it.
- The controls list's "Speedbreaker" row changes this action. It used to change `Boost` (Shift), which was wrong:
  Boost is the free camera's "fly fast" and is not the original's Speedbreaker.

## 2. The meter [built; constants are placeholders]

`scenes/world/drive/speedbreaker.rs`: `SpeedBreaker { meter }`, from 0 to 1, a new car starts full.

- While the action is held and the meter is above zero, the meter drains: `DRAIN_SECONDS` (5 s) to empty.
- Otherwise, while the car is below `RECHARGE_BELOW_MPS` (100 km/h), it recharges: `RECHARGE_SECONDS` (10 s) to full.
- `step` returns whether the breaker is in use (held, with charge).

These three constants are **our placeholders**. The mod's keys (`SBRechargeTime`, `SBRechargeSpeedLimit`,
`SBMassMultiplier`, see [extra-options.md](extra-options.md)) are not the game's attribute names, and a probe of
`GLOBAL/ATTRIBUTES.BIN` by those names found nothing. The original's values are not read.

The meter steps once per physics tick (`scenes/world/drive/simulation.rs`), with the input flag `DriveInput::speedbreaker`
and the car's speed.

## 3. The slowdown [not built]

The breaker is "in use" but nothing slows yet. The effect the original has is a slower world clock (the physics clock,
which `game_speed` multiplies). Which objects slow, and by how much, is not read yet.

## 4. The HUD bar [built; not checked on screen]

The speedometer's gauge is in `HUD_SingleRace.fng`. From the package's names (found with Ghidra, from the strings
`Speedbreaker_Meter_Multi_Image`, `Speedbreaker_Meter_Group`, `SPEEDBREAKER_METER_ICON_GROUP`, and their FEng name hashes
from `blackbox_feng::fe_hash_upper`):

| object | name | hash | role |
|---|---|---|---|
| icon group | `SPEEDBREAKER_METER_ICON_GROUP` | `0x11448B5C` | the stopwatch group (group `0xE9C1E`) |
| meter group | `Speedbreaker_Meter_Group` | `0x8991BD54` | the group (group `0xEC1BD`) |
| meter bar | `Speedbreaker_Meter_Multi_Image` | `0x5DF2F5B4` | the MultiImage that rotates (`0xE3778`, `Meter_Backing2.tga`) |

`hud/bind.rs` binds the bar by `SPEEDBREAKER_BAR` and rotates it by the meter, like the nitrous bar
(`SPEEDBREAKER_BAR_EMPTY_DEG`, which is assumed to equal the nitrous bar's 175°). The meter reaches the HUD through
`HudState::speedbreaker`, which the drive scene fills from `Drive::breaker`.

Not checked: the bar turns the right way and the right amount on screen, and the icon group shows and hides as the
original does (the bar is only driven, not shown or hidden). The next step is a run with a car, and a look at the bar.

The race timer ("00:00.00", `0xE1AA7`) is a different timer, at the top right of the screen.

## 5. Open

- **The original's name is "GameBreaker".** Ghidra's strings for the Speedbreaker are `GAMEACTION_GAMEBREAKER`
  (`0x008A6600`), `GAME_ACTION_GAMEBREAKER` (`0x008A5A24`), `StartBreaker` (`0x008A78E8`) and `TIP_SPEEDBREAKER`
  (`0x008A005C`). Neither action string has a direct cross-reference, so the code reaches them through a table.
  The function that reads the constants and runs the meter is not found yet.
  Searched the code (Ghidra `search_byte_patterns`) for the FEng hashes of these names (`GAMEBREAKER` `0x266E53F5`,
  `SPEEDBREAKER` `0x1731352C`, `GAMEACTION_GAMEBREAKER` `0x1970C22C`, `StartBreaker` `0xCAE8D0C9`): no match. So the game
  does not reach these names through the FEng hash in code; it uses another key (the attribute or string hash). That
  key is the next thing to find.
- The original's constants (`DRAIN`, `RECHARGE`, the speed limit) and its recharge rule.
- The slowdown: what it slows, and how much.
- The bar's geometry against the screen, and whether the icon group should show only while the car has the breaker.
