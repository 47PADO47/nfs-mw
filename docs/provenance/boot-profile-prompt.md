# Provenance: the profile step of the boot flow

Addresses, method and dead ends for [boot-profile-prompt.md](../specs/boot-profile-prompt.md). Ghidra auto names (`FUN_*`, `DAT_*`)
are used here only. Image base `0x00400000`; program `/speed.exe` (Black Edition PC v1.3), read through the Ghidra MCP bridge
(read only: nothing was renamed or retyped). It builds on [career-save-screens.md](career-save-screens.md) (the memory-card layer,
`MC_Main` and `MC_Bootup`) and [career-parity-menus.md](career-parity-menus.md).

## Method

1. Searched the string table for the boot screens (`MC_Bootup`, `MC_Main`, `MC_List`, `BootFlowManager`, `LS_EAlogo`, `LS_PSA`,
   `MW_LS_Splash`) and followed the data references of `MC_Bootup.fng` (`0x008A0138`).
2. Five of them are in one function, the constructor of the boot flow's screen list. Its reads of the pointer tables were dumped
   with a read-only inline script (a loop over the words of `0x008F3C58..0x008F3CD8`, printing each pointer and the string it points
   at).
3. The list-ready step (`0x00563E90`, undefined as a function in the project) was read by disassembling from its address, and
   matched to the table of [career-save-screens-2.md](../specs/career-save-screens-2.md) section 9.2.
4. Label keys were hashed with `h = 0xFFFFFFFF; h = h * 33 + c` (no case folding) and compared with the language table
   (`nfsmw strings`): `MC_CREATE_NEW_PROFILE_MENU` gives `0xBE97590F`, `MC_AUTOSAVE_WARNING_PC` `0xB01BE5DD`,
   `MC_AUTOSAVE_WARNING2_PC` `0xB380362F`, `MC_INFORM_NO_PROFILES_FOUND` `0xBA373453`, `MC_SAVE_PROMPT` `0x83F4BB3E`,
   `MC_CONFIRM_SAVE_THE_PROFILE` `0x39B3CCBA`. The first three were new; the texts are in the specification.
5. The save folders were listed on the machine the screenshots came from; the exe was not run.

## The boot screen list

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Boot flow screen list, built | `FUN_0058E9C0` | `0x0058E9C0` | vtable `PTR_FUN_008A17D0`, a linked list of `0x10`-byte nodes (vtable `PTR_FUN_008A0F48`, name at `+0xC`); the only caller is `FUN_00597C10`. Its RTTI string is `BootFlowManager` (`0x008A1D1C`) |
| PC list, the one the game takes with `FUN_0064A110() == 0` and `FUN_004FB000() == 0` | table | `0x008F3C58` | `MC_Bootup.fng`, `LS_EAlogo.fng`, `LS_PSA.fng`, `MW_LS_AttractFMV.fng`, `MW_LS_Splash.fng`, **`MC_Main.fng`**, **`MainMenu.fng`**, `DiscErrorPC.fng` (the terminator the loop stops at) |
| Other lists | tables | `0x008F3C78`, `0x008F3C98` (no attract movie), `0x008F3CB4` | the same screens with the language select (`LS_LangSelect.fng`) and the wide variants (`WS_LS_EA_hidef.fng`) swapped in by `FUN_00460DD0`; their conditions were not followed |
| Checks of the boot screens | `FUN_005864B0` | `0x005864B0` | true when the screen is none of `EA_TRAX_JUKEBOX`, `MC_MAIN`, `FEANYMOVIE`, `LS_PSA`, `LS_EALOGO`, `MW_LS_ATTRACTFMV`, `MW_LS_SPLASH`, `LOADING`: `MC_Main` is treated as a boot screen |
| Flow starter | `FUN_00527940` | `0x00527940` | pushes `MC_Main.fng`; the start-up mode is `0x20` (career-save-screens-2.md section 9.1) |
| List-ready step | (undefined) | `0x00563E90` | jump table `0x00563FF8`, index table `0x0056400C` |

Reading of `0x00563E90`, state `0x20` (the code at `0x00563EFC`): the entry count is in `EAX`; `CMP EAX,1` then

- count above 1: jump to `0x00563FBE`, which shows the list (clears the pending state and requests the device monitor);
- count equal to 1: copies the alias name and calls the load start `0x005189F0` at once;
- count 0: `CMP word [manager+0x4A],0xB`: equal posts the "back" message `0x8867412D` (leave); otherwise calls the action
  `0x00544370(2)`, the create question (dialog tag 1, wording `MC_CREATE_NEW_PROFILE_MENU` when no other bit asks for another).

State `0x10`/`0x30`/`0x70` with 0 entries shows the "no aliases found" notice (`0xBA373453`, `MC_INFORM_NO_PROFILES_FOUND`)
instead. These agree with [career-save-screens-2.md](../specs/career-save-screens-2.md) sections 9.2 and 10, which were read before from the same
code.

## The save folders on the test machine

`Documents\NFS Most Wanted` holds `NAME`, `mwo` and `stock` (each `<alias>\<alias>`, 63 596 bytes) and `Settings.ini`: the list in
the user's screenshot of the original. This build's own folder (`%APPDATA%\nfsmw\config\career`) holds `PLAYER` and `bust`. Before
this change the rewrite listed only its own folder, so on a clean install the list was empty and the main menu came straight up.
`NAME` loads in this build ("Load complete."), so the original's files pass `ProfileDir::read`.

## Why the prompt did not appear (the cause on the reporting machine)

The config file has `skip_intro = true`. `play` mapped `skip_boot || skip_intro || ui_script` to the main menu, which skipped the
movies, the title screen and the profile step together. The profile step also had no screen for the no-profile case (it went to
the main menu), and Back on the list left an empty window. All three are changed ([the specification](../specs/boot-profile-prompt.md),
sections 2, 5 and 6).

## Dead ends and open

- The conditions that choose between the four screen lists (`FUN_0064A110`, `FUN_004FB000`, `FUN_0064A140`) were not followed.
- The notice title of the two autosave notices is the library's "save/load" in this build; the original's was not read.
- How `FUN_00597C10` walks the list (which screen it opens on each "done") was not read; the order is the table's.
