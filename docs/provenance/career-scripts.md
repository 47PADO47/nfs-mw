# Provenance: career scripts

Method, addresses and dead ends for [career-scripts.md](../specs/career-scripts.md) and
[career-scripts-events.md](../specs/career-scripts-events.md). Ghidra auto names appear only here. Image base
`0x00400000`; program `/speed.exe`.

## Method

1. **Extract.** A throwaway Cargo project (outside the repo, path dependencies on `blackbox-attrib`, `ea-compress`,
   `blackbox-chunk`) loads `GLOBAL/ATTRIBUTES.BIN` and `GLOBAL/gameplay.bin`, lists every `gameplay` collection with
   its attributes and writes every `bytecode` blob (HUFF/JDLZ unwrapped) to a file named after the collection. Field
   names are not in the files, so they were recovered by hashing words (lookup2 with initial value `0xABCDEF00`)
   taken from the Lua constants and from the strings of the executable. About 60 of the 220 fields are named; the
   rest are given as hashes in the specs.
2. **Disassemble.** A small Lua 5.0 chunk reader: header `1B 4C 75 61 50`, little-endian, 4-byte int and size, 4-byte
   float numbers; function layout source, line, upvalue count, parameter count, vararg, stack, then line info,
   locals, upvalue names, constants (type byte 0 nil, 3 number, 4 string), prototypes, code. Instruction word:
   op in the low 6 bits, then C (9), B (9), A (8, top); Bx is 18 bits from bit 6; sBx = Bx - 131071; operands of 250
   or more address constants. Opcode order is the Lua 5.0 one (MOVE .. CLOSURE, 35 opcodes). A second pass prints
   registers as expressions and branches as `if cond goto`. Handler bodies are hashed to find duplicates: 203
   distinct non-empty bodies among 258 handlers.
3. **Owners.** The handler field `0x857FE432` names the collection a copy belongs to; the `_N` suffix is only a
   name collision counter. Mapping every copy to its owner gave the per-bin and per-event tables.
4. **Native side.** Linear disassembly with capstone (skip-data mode) of the file for functions Ghidra has not created
   (all registry handlers are bare labels), plus the decompiler dump for named functions.
5. **Saves.** The two local save files were parsed with the loader's own layout (`0x005F81D0`); 94 of 94 milestone
   and speed-trap keys resolve to collection names when hashed with lookup2.

**Story-flow answers pass (differences from the steps above).** The reader was a Python script in a scratch folder, not a
Rust crate: the VPAK, vault, class and collection layout of `blackbox-attrib` (legacy layout), HUFF and JDLZ, and a Lua
5.0 reader. The Lua 5.0 chunk layout was checked against the 5.0 loader source: source, line, counts, lines, locals,
upvalues, constants, nested functions, then code. The 35 opcodes match the Lua 5.0 header, and register/constant operands
of 250 or more name constants. The owner field `0x857FE432` is a 4-byte key of the owning collection: the generic
`basicrace` copies name the gpcore base, per-race copies name the race collection. The `bytecode` field is `0x9A4A020A`
(an 8-byte blob value: size and pointer). All 259 `bytecode` collections were read (253 HUFF, 5 JDLZ, 1 empty).

## Addresses

| Role | Ghidra name | Address |
|---|---|---|
| Script registry | `FUN_0061e750` | `0x0061E750` |
| Table-selecting registration helpers | `FUN_0061a270` .. `FUN_0061d1d0` | `0x0061A270` ..; the target Lua table (`Game`, `Bin`, ...) is held in a register set by the code before each group (`Bin` at `0x0061E7CE`, `Game` at `0x0061E8BF`) |
| Lua global lookup for `E*` names | `FUN_0061b970` | `0x0061B970` |
| Event hash search | `FUN_00633760` / `FUN_006337A0` | tables `0x008A5418` (hashes), `0x008F61C8` (constructors), `0x008F6450` (Lua wrappers); handlers `0x008F5F40` |
| `EEnterBin` | wrapper `LAB_0062c4c0`, handler `FUN_0062c470`, constructor `LAB_00621bb0` (bare `ret`) | `0x0062C4C0`, `0x0062C470`, `0x00621BB0` |
| Bin-enter routine | `FUN_0062c2d0` | `0x0062C2D0` (only constructor of vtable `0x008A7AB4`) |
| `ChangeState` request | `FUN_005ed7d0` -> `FUN_00621070` | vtable `0x008A6940` |
| Persistent activity start | `FUN_00603500` | `0x00603500` (loops the manager's root instances with `FUN_005fa520`) |
| Load an instance | `FUN_005fa780` / `FUN_005fa860` | `0x005FA780` |
| Reward request | `FUN_0062aa90` | `0x0062AA90` (upgrade type `0x0E0113FE`, marker kind `0x06A077D5`) |
| Reward marker apply | `FUN_005a62d0` | `0x005A62D0` (kind 0x12 pink slip, 0x13 cash bonus) |
| World race icon gate | `FUN_00601790` tail | race icon shown when CurrentBin <= the race's bin |
| Zone menu action | `FUN_0058c640` (HUD engage object vtable `0x008A24F8`, slot 10) | `0x0058C640` |
| Engage zone enter / exit | `LAB_00611fb0`, `LAB_00611ff0` | call slots 4 and 6 of the same object |
| Resume career | `FUN_0057f570` | `0x0057F570` |
| New career init | `FUN_005a3950` | `0x005A3950` |
| Final race test | `FUN_0056dc00` | `0x0056DC00` |
| Post-race front-end return | `FUN_00623f80` | `0x00623F80` |
| Rival-defeated screen sequence | `FUN_005302c0` | `0x005302C0` |
| Game save / load | `FUN_005f7410` / `FUN_005f81d0` | `0x005F7410` / `0x005F81D0` |

## Script-side facts used

- 258 handlers by class: `basicrace` 116, `race_bin` 62, `pursuit_mini_game` 19, `drive_to_target` 17, `bin_race_engage` 8,
  `world_engage` 8, `cell_engage` 7, `menu_gate` 6, `milestone_challenge` 3, `speedtrap_activity` 3,
  `speedtrap_group_monitor` 2, `speedtrap_challenge_engage` 2, `reputation_unlock` 2, `completion_unlock` 2,
  `pursuit_zone` 1.
- Global and field names used by the scripts are the API of the spec tables. Of the `E*` events they use: `EAddSMS`, `ECellCall`, `EBailPursuit`, `E911Call`, `ECinematicMoment`, `EDispIntroRace`,
  `EFadeScreenNoLoadingBarOn/Off`, `EJumpToStrategyFlow`, `EPlayRaceMovie`, `EPlayRaceNIS`, `EResetProps`,
  `EShowMessageScreen`, `EShowPostPursuit`, `EShowRaceCountdown`, `ETerminateMusic`, `ENISNukeSmack`,
  `ENISWolrdGeometry` (sic), `EDebugScreenMessage`. Never `EEnterBin`.
- State-name hashes seen in saves (lookup2): `initial` `0x904ECCE4`, `done` `0x32956993`, `rival_locked` `0x2ABB39C2`,
  `rival_unlocked` `0x76F21CB8`, `monitor` `0x28E9FA29`. No code compares them.

## Negative search record for `EEnterBin`

Hash `0x2AF7332B` (lookup2 of `EEnterBin`), also big-endian, `EnterBin` (`0x3660D692`), the multiplicative hash of both names
(`0x38BE1D7B`, `0x62C1C856`) and the literal text, searched in: every file of the install raw; every file after
unwrapping, as a whole and per chunk (including bare JDLZ chunks); the NIS scenes (which do contain other event
hashes, for example `ENISBurnout` `0x6A5265B2`) and `TRACKS/L2RA.BUN` (which contains `ESpawnFragment`
`0x2554C48A`, the control for the scan). No hit except the literal in `speed.exe`. Native: the only references to the
routine are its three callers; the only writers of the CurrentBin byte are that routine, the new-career reset (16),
the debug start (15) and `DoSafeHouseIntro` (16).

## Dead ends

- `Game.IntroduceRival` (`0x00605550`), `Demo.SetRaceCompleteForFE`, `Demo.StorePursuitRepForFE`,
  `ShowWinningPostRaceScreen`, `ShowLosingPostRaceScreen`: bare returns.
- `milestone_challenge`, `speedtrap_activity`, `speedtrap_challenge_engage`, `cell_engage`, `pursuit_zone`,
  `reputation_unlock`, `completion_unlock`: handlers exist, no instance in `gameplay.bin`.
- `Game.SetWorldHeat` target interface (`0x004037E0`) is the heat interface of the player car (write slot `0x00409060`);
  see [career-safehouse.md](career-safehouse.md). The two call sites use 1 and 5.
- The phone and message table behind `EAddSMS` / `ECellCall` ids: read in the phone pass, see
  [career-phone.md](../specs/career-phone.md) and [its provenance](career-phone.md).
- Attributes still unnamed: `0xB39ED8C3`, `0x39509746`, `0x8CB01ABF`, `0xA4E6FCFD`, `0xE10FB7A3`, `0xA07AE814`,
  `0x1823B89E`, `0x8F186AC4`, `0xE8C24416` (the last four sit on `race_bin` and are zero or small floats).
