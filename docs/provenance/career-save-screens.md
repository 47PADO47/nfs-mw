# Provenance: the save screens, the memory-card layer and the autosave

Addresses, Ghidra names, method and dead ends for [career-save-screens.md](../specs/career-save-screens.md) and
[career-save-screens-2.md](../specs/career-save-screens-2.md). Ghidra auto names (`FUN_*`, `DAT_*`, `LAB_*`) are used here
only. Image base `0x00400000`; program `/speed.exe` (Black Edition PC v1.3).

## Method

1. A full decompile of every function was searched by regex (strings, constants, offsets, callers). The language table
   (`strings.tsv` in the scratchpad, built from `GLOBAL/LANGUAGES`) gave the `MC_*` labels and the English texts.
2. Many routines of this area are **not defined as functions** in the project: the `MC_Main` and `MC_Bootup` message
   handlers (`0x00564630`, `0x005644F0`), the list-ready step (`0x00563E90`), the pick handler (`0x005436F0`), all the
   select handlers of the menu items (`0x005321C0` safe house Save, `0x005289D0`, `0x00542FD0`, `0x00543030`, `0x00543090`,
   `0x005430F0`, `0x00532740`, `0x00532810`), the main menu back handler (`0x00545500` region) and the pursuit listener
   (`0x00411070`). They were read by disassembling the bytes from the file (PE section map plus Capstone).
3. Callers of an undefined routine were found by scanning the code section for `call`/`jmp rel32` to its address; this
   found seven more callers of the flow starter `0x00527940` than the decompile index (`FUN_00527940` has 21 call
   sites, the index showed 6).
4. The decompile **drops variable arguments**: `FUN_004647c0(buffer, format, ...)` calls look argument-less. The alias
   name that fills the `%s` of the save/damaged dialogs was found in the disassembly (`0x00543F07`, `0x00544292`).
5. The three saves on this machine (`NAME`, `mwo`, `stock`) were only listed (folder layout `<alias>\<alias>`); sizes
   `63,596` agree with `0x1C + 8 + 0xF848`.
6. `MEMCARD/LOCALE_ENGLISH.loc` was parsed (header `LOCH`, id table `LOCI`, strings `LOCL`; offsets are relative to the
   `LOCL` tag).
7. FNG message ids are not strings in the exe. The hash is `h = 0xFFFFFFFF; h = h * 33 + c` per byte (no case folding),
   which reproduces `EXIT_COMPLETE` (`0xE1FDE1D1`), `LEAVE` (`0x033113AC`) and `SAVE_DONE` (`0x40E73793`); the other
   message ids of the flow (`0x461A18EE`, `0x8867412D`, ...) have no known name and are described by role.
8. The screens `MC_Main`, `MC_List`, `MC_ProfileManager`, `MC_DeleteProfile`, `MC_Bootup`, `Autosave_Overlay` were dumped
   with `nfsmw dump-screen` and all 197 packages were grepped for the message ids (the ids of step 7 are handled by C++).
9. No run-time tool was used. Every claim is a static reading; the saves were not modified.

## The manager and the flow

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Manager creation | `FUN_005435d0` | `0x005435D0` | allocates `0xD4` bytes, global `DAT_0091cb20`; sets wide strings "Profile", "Thumbnail", "Image", "NFSMWSD"; calls `FUN_00526660` |
| Manager constructor | `FUN_00517ff0` | `0x00517FF0` | |
| Library setup | `FUN_00526660` | `0x00526660` | builds the library init struct, game info (`0x008F399C`, 19 dwords) and the callback table (`0x0089CD78`) |
| Flow starter | `FUN_00527940` | `0x00527940` | stores mode word `DAT_0091cbac`, screens `DAT_0091cbb0/b4`, callback `DAT_0091cbbc/c0`, messages `DAT_0091cbd0/d4`; pushes `MC_Main.fng` (`DAT_00925e90 == 3`) or `InGame_MC_Main.fng` |
| State step | `FUN_00562c30` | `0x00562C30` | message `0x35F8620B`/`0x3A2BE557` |
| Dialog result | `FUN_00563380` | `0x00563380` | tag in bits 24..27 |
| Alias manager handler | `FUN_00564070` | `0x00564070` | `FEPkgMemcard`, vtable `0x0089EA94` |
| `MC_Main` handler | (undefined) | `0x00564630` | vtable `0x0089EDD0` (`UIMemcardMain`), calls `0x00564070` first |
| `MC_Bootup` handler | (undefined) | `0x005644F0` | vtable `0x0089EDA0` (`MemCardBootScreen`) |
| Exit routine | `FUN_00562ff0` | `0x00562FF0` | |
| Exit message / leave | `FUN_00519730` | `0x00519730` | records `DAT_0091cbc4`; posts `EXIT_COMPLETE` or the leave script |
| Action ("go to step") | `FUN_00544370` | `0x00544370` | cases 1..12 |
| Dialogs | `FUN_00543cc0`, `FUN_00543be0`, `FUN_00543af0`, `FUN_00543eb0`, `FUN_00544210`, `FUN_00544270`, `FUN_00543f90` | `0x00543CC0`... | Yes/No (`0xB8A7C6CC` No, `0xB8A7C6CD` Yes), Continue, save question, damaged notice |
| List ready | (undefined) | `0x00563E90` | jump table at `0x00563FF8`, index table `0x0056400C` |
| Pick handler | (undefined) | `0x005436F0` | sets pending code 5 or 6 |
| Pending dispatcher | `FUN_00563cb0` | `0x00563CB0` | 5 load, 6 delete, 7 list |
| Manager update | `FUN_00563d10` | `0x00563D10` | called from `FUN_005642c0` (`0x005642C0`) |
| Per-frame update | `FUN_005642c0` | `0x005642C0` | |
| Save start | `FUN_005188f0` | `0x005188F0` | size `FUN_0057fd90`, serialize `FUN_00590150` |
| Load start | `FUN_005189f0` | `0x005189F0` | snapshot `FUN_00599500` |
| Delete start | `FUN_00518ac0` | `0x00518AC0` | |
| List start | `FUN_00562b80` | `0x00562B80` | pattern `root\*` |
| Quiet-mode op | `FUN_005268b0` | `0x005268B0` | `7f3c8a` = library op `0x80` |
| Autosave write | `FUN_00526800` | `0x00526800` | applies the masks, then `FUN_005188f0` |
| Autosave run | `FUN_00562ab0` | `0x00562AB0` | |
| Autosave gate | `FUN_005184d0` | `0x005184D0` | |
| Overlay show / hide | `FUN_00518c50` / `FUN_00518df0` | `0x00518C50` / `0x00518DF0` | `Autosave_Overlay.fng`, `LEFT_*`, `RIGHT_*`, `SAVE_DONE` |
| Silent masks | `FUN_00518600`, `FUN_00518710` | `0x00518600`, `0x00518710` | the second has no caller |
| Prompt callback | `FUN_00560980` | `0x00560980` | callback 0 |
| Device ready | `FUN_00559ac0` | `0x00559AC0` | callback 2; calls `FUN_006cbf00` (root folder) |
| Save done | `FUN_00526ac0` | `0x00526AC0` | callback 4 |
| Load done | `FUN_00526c90` | `0x00526C90` | callback 7 |
| Delete done | `FUN_00519050` | `0x00519050` | callback 8 |
| List entry / list done | `FUN_0054f610` / `FUN_0054f6d0` | `0x0054F610` / `0x0054F6D0` | callbacks 10, 11 |
| Error / cancel | `FUN_00526f60` | `0x00526F60` | callback 13 (kind, code) |
| Quiet-mode done | `FUN_005271e0` | `0x005271E0` | callback 15 |
| Device monitor done | `FUN_005191e0` | `0x005191E0` | callback 17; posts `0x54B3AC6C` |
| Get buffers | `FUN_005192f0` | `0x005192F0` | callback 5; accepts descriptor size 8 and `DAT_0091cb20 + 0xc0` |
| Button handler | `FUN_005195d0` | `0x005195D0` | answers a pending library prompt (`FUN_007f3c69`) |
| Prompt-to-state | `FUN_00559e90`, `FUN_00559d60`, `FUN_00559e40` | `0x00559E90`... | autosave-failure question, list request, show stored prompt |
| Autosave-failure open | `FUN_00560940` | `0x00560940` | opens mode `0x91` |

Callback table `0x0089CD78` (index: address): 0 `0x00560980`, 1 `0x00518FE0`, 2 `0x00559AC0`, 3 `0x00519010`, 4
`0x00526AC0`, 5 `0x005192F0`, 6 `0x00519030`, 7 `0x00526C90`, 8 `0x00519050`, 9 `0x00519100`, 10 `0x0054F610`, 11
`0x0054F6D0`, 12 `0x00519120`, 13 `0x00526F60`, 14 `0x00560BD0`, 15 `0x005271E0`, 16 `0x00560CE0`, 17 `0x005191E0`,
18 `0x00519150`, 19 `0x00518FB0`. Callbacks 1, 3, 6, 9, 12, 14, 16, 18, 19 only record or reset flags and were not
needed.

## The library (EA memory card, PC hard-disk driver)

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Wrapper API | `FUN_007f3b96` ... `FUN_007f3ca0` | `0x007F3B96`... | `3b96` init+list, `3bd0` save, `3bfa` load, `3c2b` delete, `3c41` list, `3c69` answer, `3c74` device, `3c8a` quiet op, `3c95` monitor, `3ca0` masks, `3cc5` update |
| Driver constructor | `FUN_007f4cb7` | `0x007F4CB7` | `0x304` bytes; controller `FUN_007f63c2` (`0x138` bytes) |
| Controller queue | `FUN_007f6455`, `FUN_007f66a8` | `0x007F6455`, `0x007F66A8` | op codes 1, 2, 4, 8, `0x10`, `0x20`, `0x40`, `0x80`; 3 queue slots of `0x54` bytes |
| Op update | `FUN_007f5b1d` | `0x007F5B1D` | dispatches `0x10` read, 1, 2, 4/8 write, `0x20` delete, `0x40` list, `0x80` quiet |
| Save prepare | `FUN_007f4e57` | `0x007F4E57` | name copy (31), CRCs, header `FUN_007f4104` |
| Save run | `FUN_007f4f11` | `0x007F4F11` | states 9..0xE; `open(0x8301, 0x180)` |
| Check op | `FUN_007f4414` | `0x007F4414` | op 4; no caller |
| Read prepare | `FUN_007f445f`, `FUN_007f451d` | `0x007F445F`, `0x007F451D` | |
| Read run | `FUN_007f5255` | `0x007F5255` | states `0xF..0x12`; `open(0x8000)`; the CRC mismatch is overwritten at `0x007F5690` |
| Delete prepare / run | `FUN_007f45fe`, `FUN_007f56bc` | `0x007F45FE`, `0x007F56BC` | states `0x13..0x18`; remove files then `FUN_0085e6b0` |
| List run | `FUN_007f492d` | `0x007F492D` | skips `.` and `..`; no kind filter in mode 0 |
| Disk space | `FUN_007f48a4` | `0x007F48A4` | `_getdiskfree`; op 1 only |
| Prompt filter | `FUN_007f4b97` | `0x007F4B97` | mask bits `0x7C`/`0x80`, quiet `0x6A` |
| Prompt build | `FUN_007f5ac3`, `FUN_007f5a37`, `FUN_007f5a71`, `FUN_007f41b3`, `FUN_007f5e04` | | message id, string id lookup, buttons, `%1` expansion |
| Prompt to game | `FUN_007f5da9` | `0x007F5DA9` | calls callback 0 |
| Answer | `FUN_007f5ae8` | `0x007F5AE8` | 1..4 |
| Mask setter | `FUN_007f5bd2` | `0x007F5BD2` | 1 set, 2 force-ask, else clear |
| Header builder | `FUN_007f4104` | `0x007F4104` | magic `0x4D433032` |
| CRC | `FUN_007f73e1` (via `FUN_007f4c98`) | `0x007F73E1` | see career-save.md section 1 |
| Directory | `FUN_007f43e4` (mkdir `FUN_0085e150`), `FUN_007f4275` (find) | | |
| String lookup | `FUN_007f0f48` (`bsearch` id table `FUN_007f0f1b`) | `0x007F0F48` | |
| Locale load | `FUN_00518260` | `0x00518260` | `memcard/LOCALE_<language>.loc`; called from `0x0057E836`; English for the two languages 14 and 15 |
| Root folder | `FUN_006cbf00` | `0x006CBF00` | `SHGetFolderPathA(0x8005)`, `CreateDirectoryA` x2 |

## Profile side

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Profile save | `FUN_0058fd10` | `0x0058FD10` | see career-save.md |
| Profile load | `FUN_005acb70` | `0x005ACB70` | settings, stable hash, data, MD5 last; returns bool |
| Load wrapper | `FUN_005acdd0` | `0x005ACDD0` | tail-calls the above; player index selects profile 0 or 1 |
| Snapshot restore | `FUN_005ace40` | `0x005ACE40` | |
| Snapshot take | `FUN_00599500` | `0x00599500` | |
| Unsaved-change test | `FUN_00599570` | `0x00599570` | **no caller** |
| Settings save / load | `FUN_0057f7b0` / `FUN_005ac950` | | blob first |
| Blob save / load | `FUN_005f7410` / `FUN_005f81d0` | | load returns silently on a bad MD5, tag or version |
| Race-bin flush | `FUN_005f6ba0` | `0x005F6BA0` | per tracked collection |
| MD5 | `FUN_0057f920`, `FUN_0057f9f0` | | |
| Profile reset | `FUN_005ab9c0` | `0x005AB9C0` | new alias / deleted alias |
| New career settings | `FUN_005a3950` | `0x005A3950` | |
| Resume | `FUN_0057f570` | `0x0057F570` | sets manager `+0x2B` |
| Set name | `FUN_0056da30` | `0x0056DA30` | default "Player 1/2" labels |
| Stable changed | `FUN_0057fc90` | `0x0057FC90` | compares FEDatabase `+0x124` copy with the live stable |

## Triggers and options

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| EAutoSave constructor | `FUN_00620920` | `0x00620920` | type `0x10`, vtable `0x008A6820` |
| EAutoSave dispose | `FUN_00620930` | `0x00620930` | runs the gate, sets `+0x1F`; called by `FUN_0062a9f0` |
| `SetRoaming` | `FUN_005fe3e0` | `0x005FE3E0` | posts at `0x005FE4xx`; caller `0x006038D9` (constructor `FUN_00603720`) and the race graphs |
| Event start | `FUN_00602010` | `0x00602010` | the old docs named it "race end"; its only caller is `FUN_0060dbd0` (`Game_StartRace`) |
| Complete nodes | `FUN_00602770` | `0x00602770` | param 1 posts `EAutoSave` when `+0x1960 == 0`; caller `0x004110EC` in the pursuit listener |
| Pursuit listener | (undefined) | `0x00411070` | vtable slot at `0x00891910`; strings `pursuits_in_a_row`, `pursuit_evasion_time` |
| HUD arm | (inside an undefined routine) | `0x0058CE99` | `if (+0x1F) +0x2A = 1` |
| Node flags | `FUN_005fbd20`, `FUN_005fbd90`, `FUN_005fc180`, `FUN_005fba70`, `FUN_0056dc00` | | `0x8CB01ABF`, `BossRace`, `ChallengeSeriesRace`, node name, "is `1.8.1`" |
| Idle test | `FUN_005edfc0`, `FUN_005ec040`, `FUN_00542670` | | |
| Stepper | `FUN_005302c0` | `0x005302C0` | step 3 at `0x005304F5` |
| Main menu back | (undefined) | `0x00545508` | message `0x911AB364`; calls `FUN_00529230` |
| Leave to title | `FUN_00529230` | `0x00529230` | mode `0x400B3` |
| Shop exit | (inside `FUN_007be3f0`) | `0x007BE59D` | mode `0x2000B3` |
| Option row | (inside `FUN_00528ea0`) | `0x00528F0E` | mode `0xA1` |
| Option page exit | (inside `FUN_00545730`) | `0x00545C1F` | mode `0xA1`; also writes FEDatabase `+4` |
| Unsaved flag writers | `FUN_00545730`, `FUN_00545f50` | | `0x00545B72`, `0x005461DE`, `0x0054624B` |
| Option availability | `FUN_00510530` | `0x00510530` | |
| Option defaults | `FUN_0056d500` | `0x0056D500` | `[0] = 1` is the autosave byte |
| Quit | `FUN_005a4820` | `0x005A4820` | `QUIT_GAME`, no save |

## Dead ends and uncertain readings

- **Flag `0x8CB01ABF`.** The accessor reads bit 4 of a cached word or the attribute; no collection in the gameplay dump
  sets it explicitly. Its use for the icon side and for the event-start autosave suggests "race-type node". The older
  blacklist notes treat the same attribute as an exclusion in the race table. Not resolved.
- **Prompt order in a manual save.** Derived from the library state machine and the masks (all clear after the quiet-mode
  operation). A trace of the file operations would confirm whether the overwrite and "save complete" panels really appear.
- **Two-click behaviour.** `FUN_005195d0` answers a pending library prompt and returns, while `FUN_00563380` also handles
  the same button for the game's own dialog; which handler wins when both are pending was not followed.
- **`FUN_005ec040`** is only summarised by its effect; its many conditions were not named.
- **The pair of bytes at `DAT_0091cae0`** are set to hide the HUD while an `MC_Main` screen is up in a world; not
  identified.
- **State `0xD0`, the check op, the device-select op, `FUN_00518710`, `MC_DeleteProfile.fng`, `FUN_00599570`,
  `FUN_00543e40`, `FUN_00543ea0`, `FUN_00543f90`** have no callers; they are old console paths.
- **The `0x6A02D7C6` offset hook** (`FUN_005ac7d0`) in the settings load was not needed; the saves load with a zero
  displacement ([career-flow provenance](career-flow.md)).
- **`COMMON_*` and `MC_*` PS2/Xbox texts** are shown on PC for the autosave-failure question and the damaged notice; no
  PC wording exists in the table.

## Corrections to existing documents

Made in place (small):

1. [career-flow.md](../specs/career-flow.md) section 0 item 6 and section 8: the autosave gate and the triggers. The old text
   said the event is created "when a race ends" (it is the event *start* hook plus `SetRoaming` plus pursuit evaded), and that the
   gate depends on "no profile loaded (unless the manual save flag is on)" (the gate has no such test; the run checks the
   loaded flag, and the "manual save flag" is the switched-off-by-error byte). Open question 3 marked answered.
2. [career-save.md](../formats/career-save.md) open question 6: answered (the descriptor/profile CRC mismatch is ignored).

Not edited (other agents' or older files); for the maintainers:

- [career-safehouse.md](../specs/career-safehouse.md) section 5.1: the Save item does not send message `0x2251` to the
  starter; `0x2251` is the **mode word** it passes (state `0x50`, return to the caller). The `[unconfirmed]` can go.
- [career-safehouse.md](../specs/career-safehouse.md) section 5.3, "autosave possible": also true when the manager's
  switched-off-by-error byte is set; `BossRace` is the field `0xFF5EE5D6`.
- [save-alias-manager.md](../specs/save-alias-manager.md) sections 7 to 9 describe the file as "16-byte digest + body" and the
  writer as `FUN_005ACB70`; both are superseded by career-save.md (the file is `20CM` header, descriptor, profile; `FUN_005ACB70` is
  the loader). Section 2's note that `MC_CORRUPT_PROFILE` is "a corrupt slot" is not reached on PC; the damaged notice uses
  the Xbox label. Section 5's list of the four buttons: Save `0x00542FD0` (`0x2251`), Create `0x005430F0` (`0x61`), Load
  `0x00543030` (`0x411`), Delete `0x00543090` (`0x31`).
- [career-flow provenance](career-flow.md): `FUN_00602010` is the event start, not the race end.

## Node state bits

Pass of 2026-10-21 (story mode, Ghidra on `speed.exe` 1.3): `search_instructions` for `OR [reg + 0x4], imm` in the
race-database code, then the writers decompiled. The node list lives in the race database (`DAT_0091e004`):
count at `+0`, records of 16 bytes from the pointer at `+0x3C`.

| Function | Address | Role |
|---|---|---|
| race-finished record | `FUN_00602320` | finds or adds the node of the current race (`+0x1968`), keeps the best result, writes the two u16 statistics, and on a win (argument 1) ORs 2 when the play mode (`DAT_0091e000 + 0x1964`) is 2 (career), else 1 |
| caller | `LAB_006119f0` (`NotifyRaceFinished`), call at `0x00611CB4` | the same routine also walks the won race's unlock list (`0x00611C80`): OR 4, then OR 8 unless `FUN_005fc110` (attribute `0xA4E6FCFD`) is set |
| unlock request | code at `0x00612170` | same pair: OR 4, then OR 8 unless `0xA4E6FCFD` |
| reset | `FUN_00600b20` | clears the list and re-seeds it from the race database: skips nodes with `0x8CB01ABF`; ORs 4 when `0xB39ED8C3` or `0xEA855EAF` (not when `0xFF5EE5D6` `BossRace`), ORs 0x10 when `0x39509746` |
| attribute readers | `FUN_005fbd20` / `FUN_005fbbd0` / `FUN_005fbc40` / `FUN_005fbcb0` / `FUN_005fbd90` / `FUN_005fc110` | `0x8CB01ABF` / `0xB39ED8C3` / `0x39509746` / `0xEA855EAF` / `0xFF5EE5D6` / `0xA4E6FCFD`; each reads a cached bit word at `+0x18` of the node's record or the attribute |

Not found: the name of `0x39509746` and what reads bit 0x10.
