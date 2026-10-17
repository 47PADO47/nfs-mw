# Provenance: the safe house, native side

Addresses, Ghidra names, method and dead ends for [career-safehouse.md](../specs/career-safehouse.md) and
[career-safehouse-2.md](../specs/career-safehouse-2.md). Ghidra auto names (`FUN_*`, `DAT_*`, `LAB_*`) are used here
only. Image base `0x00400000`; program `/speed.exe` (Black Edition PC v1.3).

## Method

1. A full decompile of every function was searched by regex (strings, constants, field offsets, callers).
2. Many routines named in the tables are **not defined as functions** in the project (`LAB_*`), mostly script hooks,
   vtable methods and listeners; they were read by disassembling the bytes from the file (PE section map plus Capstone).
3. Interface lookups (`FUN_005d59f0(table, id)`) were followed by the interface id. Ids seen here: `0x004037E0` (the
   perpetrator/heat interface), `0x00403700` (the pursuit holder), `0x004040E0` (the driver interface), `0x005659C0`
   (the HUD engage component).
4. Virtual tables were read from the file data to resolve the slot numbers (`0x00892770` pursuit interface,
   `0x008A24F8` engage component, `0x00892250` heat interface).
5. Callers of an undefined routine were found by scanning the code section for `call rel32` to its address.
6. No run-time tool was available; every claim is a static reading. Items not closed are tagged **[unconfirmed]** in the
   spec.

## Entering

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Permission flags (recomputed per frame) | `FUN_005eba50` | `0x005EBA50` | called at `0x0060370D` at the end of the race status update (the enclosing routine is undefined); writes manager `+0x2CE`, `+0x2CF`, `+0x2D0` |
| Pursuit/perpetrator lookup for the first player | `FUN_005eb9e0` | `0x005EB9E0` | pursuit via interface `0x403700` slot `+0xA0`, heat interface `0x4037E0` |
| `AllowEngageSafehouse` | `LAB_00605660` | `0x00605660` | returns manager `+0x2CF` |
| Engage component constructor | `FUN_005659e0` | `0x005659E0` | registers interface `0x5659C0`; the HUD owner constructor is `FUN_005a0530` |
| Engage component secondary vtable | data | `0x008A24F8` | slots `+0x04` `0x569010`, `+0x08` `0x569030`, `+0x0C` `0x569170`, `+0x10` `0x57BDE0`, `+0x14` `0x57BD10`, `+0x18` `0x57BEA0`, `+0x1C` `0x5A0640`, `+0x20` `0x569050`, `+0x24` `0x569100`, `+0x28` `0x58C640` |
| `EnterGateZone` / `ExitGateZone` | `LAB_00611fb0` / `LAB_00611ff0` | `0x00611FB0` / `0x00611FF0` | call engage slots `+0x10` / `+0x18` on the first player's HUD (`[player vtable +0x20]`) |
| `DoZoneMenuAction` | `LAB_00612030` | `0x00612030` | attribute `0x2241F4CD` of the zone, manager `+0x108`; slot `+0x0C` with the string at `0x008A055C` (`safehouse`), then `+0x10C`; tail-calls slot `+0x28` |
| `EnterEngagableTrigger` | `FUN_00604f80` | `0x00604F80` | allocates the event (`FUN_00621bc0`, vtable `0x8A6B70`); its destructor `FUN_006407d0` (`0x006407D0`) calls slot `+0x14`; exit event `FUN_00640870` calls `+0x18` |
| Zone-type strings | data | `0x008A055C`, `0x008A0554`, `0x008A0548` | `safehouse`, `carlot`, `customshop` |
| Post `MEnterSafeHouse` (engage) | `FUN_00569100` | `0x00569100` | the three string compares (`FUN_00460d20`, case sensitive), then `FUN_00568fc0` + `FUN_00654af0` on subject `0x20D60DBF` |
| `MEnterSafeHouse` id holder | `FUN_00568f40` | `0x00568F40` | `bStringHash("MEnterSafeHouse")` cached in `DAT_0091dc70` |
| Native listener that forwards it to the scripts | `FUN_00632720` | `0x00632720` | registered by `FUN_006364a0` inside `FUN_00648590` (a list of 20-odd listeners on the subject); queues to `DAT_0091e0d0` |
| Zone screen opener (`+0x28`) | `FUN_0058c640` | `0x0058C640` | safehouse -> platform 2 `MainMenu_Sub.fng`; carlot -> 5 `Car_Select.fng`; customshop -> 3 `CustomizeMain.fng` after `FUN_0056ecc0` |
| Prompt icon by kind | `FUN_0056e100` | `0x0056E100` | case-insensitive compare (`FUN_00460dd0`); ids `0x4EAEE18B`, `0x0ED39F69`, `0x0CF07089` |
| HUD input handler | `FUN_0057c660` | `0x0057C660` | switch on the action code; `0x2E` = engage |
| `EndPursuitEnteringSafehouse` | `LAB_0040b040` | `0x0040B040` | slot `+0x130` of the pursuit interface (vtable `0x00892770`); stores `+0x218 = 4`, `+0xFC = 1.0`, byte `+0x240 = 1`; getter at `0x00433C40` (slot `+0x134`) |
| Evade handler (listener) | undefined | `0x00410F90` to `0x00411236`; next `0x00411240` | the call to slot `+0x134` is at `0x00411172`; `FUN_00580a40` (strike tick, `0x004110F3`), `FUN_00590160` (heat factor, `0x00411211`) |
| Results request constructor | `FUN_00641fe0` | `0x00641FE0` | fills the record at `DAT_0091d3d8` through `FUN_0056a440`; opens `PostRace_MilestoneRewards.fng` |
| Results record | `FUN_0056a440` | `0x0056A440` | `+0x24` is the "enter safe house" byte (global `0x0091D3FC` = record + 0x24) |
| Results screen class | `FUN_005a9430` (ctor), `FUN_0058e320` (dtor) | `0x005A9430`, `0x0058E320` | vtable `0x008A1730`; the dtor posts `MEnterSafeHouse` with the string `safehouse` when the byte is set |
| Front-end request constructor | `FUN_0062fe40` | `0x0062FE40` | platform at `+8`, screen at `+0xC`; vtable `0x008A723C` |
| Front-end request executor | `FUN_00623f80` | `0x00623F80` | the destructor of the request; section 4 of the spec |
| `JumpToSafeHouse` | `FUN_006052b0` | `0x006052B0` | |
| `JumpToCarLot` | `FUN_00605250` | `0x00605250` | |
| `DoSafeHouseIntro` | `LAB_00605560` | `0x00605560` | key from `FUN_00452430` (collection key at `+0x20`); movie string `0x0089CC40` = `storyfmv_saf25`, next screen string `0x0089AFA4` |
| Pause "Jump to Safe House" | `FUN_00603220` | `0x00603220` | node predicates `FUN_005fbd90` (attribute `0xFF5EE5D6`), `FUN_005fc560` (`0x4393F69B`), `FUN_005fbd20` (`0x8CB01ABF`), `FUN_005fbe70` (`0x2B1F54F6`) |

## Inside the safe house

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| `MainMenu_Sub.fng` registration | data | `0x008F432C` | (name, constructor) pair: constructor `FUN_00570660`, class `FUN_00549280`, vtable `0x0089E750` |
| Menu item builder | `FUN_00532220` | `0x00532220` | item classes with vtables `0x0089C428`, `0x0089C430`, `0x0089C438`, `0x0089C440`, `0x0089C448` |
| Item select handlers | undefined | `0x00532080` (Car Select), `0x005320E0` (Rap Sheet), `0x00532150` (Blacklist), `0x005321C0` (Save) | strings at `0x0089D998`, `0x0089D9A8`, `0x0089D9BC` |
| Screen handler (accept / back) | `LAB_00531ea0` | `0x00531EA0` | `0xD05FC3A3` -> `0x00531F7B`; `0xE1FDE1D1` with `[this+0x138] == 0x911AB364` -> `0x00531EF3` |
| Blacklist stepper | `FUN_005302c0` | `0x005302C0` | screen table at `0x008F322C`; object created in `FUN_0054ea80` (counter `-1`); callers `FUN_00562ff0`, `FUN_00622600` |
| Memory-card flow starter | `FUN_00527940` | `0x00527940` | opens `MC_Main.fng` (`InGame_MC_Main.fng` in game); next screen stored at `0x0091CBB4` |
| Autosave possible | `FUN_005184d0` | `0x005184D0` | |
| Rival bio constructor | `FUN_00561320` | `0x00561320` | registered for `SafehouseRivalBio.fng` and `InGameRivalBio.fng` (`FUN_005713e0`); posts enter-bin when bit 17 is set and the stage is 16 |
| Enter bin | `FUN_0062c2d0` | `0x0062C2D0` | event vtable `0x008A7AB4`; reward routine `FUN_0062aa90` |
| Markers screen | `FUN_007b9a80` | `0x007B9A80` | wrapper `FUN_00571560`; popup queue accessor `FUN_00576b50`; marker table at `0x00905EE0` (21 rows of 7 words) |
| Region unlock screen | `FUN_0055b020` | `0x0055B020` | wrapper `FUN_005714a0`; loads `Global\Rivals.bin` |
| Reputation overview | `FUN_00561130` | `0x00561130` | wrapper `FUN_00571320` |
| Infractions screen | `FUN_0058cef0` | `0x0058CEF0` | not part of the safe house; it grants four Get Out of Jail Free markers the first time ([career-economy-flows.md](../specs/career-economy-flows.md)) |
| Preview car | `FUN_005a3b60` | `0x005A3B60` | |

## Leaving and the new world

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Resume branch | `LAB_00531f7b` | `0x00531F7B` | uses `FUN_00600c30` (first unfinished prologue name), `FUN_005fb710` (node by hash), `FUN_005fab40` (node name hash), `FUN_005dca70` (state flags), `FUN_00603440` (node state object), `FUN_00600ab0` (set pending node and mode), `FUN_005e39e0`, `FUN_0056c5b0` (leave the front end) |
| Prologue name table | data | `0x008A37E4` | `16.1.0`, `16.2.2`, `16.2.3`, `16.1.1`, `16.2.1`, stride 8; `16.2.1` string also at `0x0089B0C8` |
| Race status constructor | `FUN_00603720` | `0x00603720` | allocated by `FUN_00603900` (size `0x4558`); sets `+0x1964 = 2`, `+0x1960 = 1`, calls `FUN_00602bb0` and `FUN_005fe3e0` when the pending node is none |
| `SetRoaming` | `FUN_005fe3e0` | `0x005FE3E0` | script hook registration in `FUN_0061e750`; callers: constructor `FUN_00603720` and the race graphs |
| Camera/HUD director reset | `FUN_004d0d70` | `0x004D0D70` | unreferenced details |
| Events | `FUN_00624700` (`EReloadHud`, vtable `0x008A72DC`), `FUN_00620920` (`EAutoSave`, vtable `0x008A6820`) | | strings read from the vtable neighbours |
| Set current bin | `FUN_00602bb0` | `0x00602BB0` | |
| Load-complete callback | `FUN_00603500` | `0x00603500` | caller `FUN_006f9ea0`; clears manager `+0x110`, sets `+0x118`, starts the pending node (`FUN_005fd370`) |
| World loader | `FUN_006f9800` | `0x006F9800` | start point from `FUN_00733710`; other caller `FUN_006f9bb0` |
| Start point | `FUN_00733710` | `0x00733710` | track name at `DAT_009a3a20 + 0x40`; node start `FUN_005fad10` / `FUN_005fae00` |
| Respawn key selection | `FUN_005e4cf0` | `0x005E4CF0` | `career_start_%s` built with `FUN_004647c0`, hashed with `FUN_004549b0` |
| Respawn position | `FUN_005e4df0` | `0x005E4DF0` | attributes `0x9F743A0E` and `0x5A6A57C6`; constant `0x008A3874` = -1/360 |
| Gameplay blob save / load | `FUN_005f7410` / `FUN_005f81d0` | `0x005F7410` / `0x005F81D0` | the two keys at the tail (offsets 0 and 16); load accepts a key only when `FUN_005e2580` finds its collection |
| Player car creation | `FUN_006f47a0` | `0x006F47A0` | heat default `1.0` at the start, `max(1.0, record)` from `FUN_0056f7b0` and `FUN_00401dc0`; heat write at `0x006F4BD9` |
| Heat interface vtable | data | `0x00892250` | `+4` `0x409400` (get), `+8` `0x409060` (set) |
| Player car destructor | `FUN_00434fe0` | `0x00434FE0` | `min(heat, 5.0)` (static float `0x00890DA4`), writes `FUN_0056f7a0` |
| Race start hook | `FUN_0060dbd0` | `0x0060DBD0` | registered as `Game_StartRace`; event attribute `0xE4211F4F` |
| `SetWorldHeat` | `LAB_00612660` | `0x00612660` | first player's heat interface slot `+8` |
| Evade heat factor | `FUN_00590160` -> `FUN_00581ff0` | `0x00590160` | multiplies each record's `+0xC` by the `+0xA8` field of `pursuitlevels/default` (hash `0xEEC2271A`) |

## Corrections to the earlier career pages

1. [career-flow.md](../specs/career-flow.md) 7.3 listed `SafeHouseMarkers.fng` as autosave-gated; the code gates the
   `MC_Main.fng` step (step 3), and the markers step is unconditional. Fixed in place.
2. [career-scripts.md](../specs/career-scripts.md) 4 said the front end sends the enter message when the player
   confirms; the senders are the engage component and the results-screen close. Fixed in place.
3. The first-visit step that lowers the stage from 16 to 15 is the rival bio screen constructor (enter bin 15).

## Dead ends and limits

- The "back" branch of the screen handler (`0xE1FDE1D1`) was read but its role in the game was not confirmed.
- No writer of manager `+0x11C` was found; no reader other than `FUN_005e4cf0`.
- `FUN_00403800`-style interface objects have no names in the project; the interface ids were matched by their users.
- The engage component's property `0x0280164F` and the HUD event types (0 to 3) are not understood.
- The first-visit continuation of the stepper (which step runs after the bio) was not traced.
- `SafeHouseRegionUnlock`, the rap sheet screens and the Save item were read only as far as their constructors or entry
  handlers.
- Whether the time-of-day object (`DAT_009b392c`) is rebuilt per world was not followed.
- The two numeric tables reset by `FUN_007652f0` (`DAT_009b38c0`, `DAT_009b38c8`) were not identified.
- Cheat Engine was not used (no running game).
