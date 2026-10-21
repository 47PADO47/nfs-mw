# The profile step of the boot flow: load a saved profile, or create one

Status: built. Evidence tags: **[exe]** read in `speed.exe` (Black Edition PC v1.3), **[verified]** checked in the install, on
this machine or on screen, **[ours]** a decision of this rewrite. Addresses and the method are in
[../provenance/boot-profile-prompt.md](../provenance/boot-profile-prompt.md). Related: [career-parity-menus.md](career-parity-menus.md)
(section 2, the alias list), [career-save-screens-2.md](career-save-screens-2.md) (sections 9 and 10, the load and create flows),
[save-alias-manager.md](save-alias-manager.md), [../app-flow.md](../app-flow.md).

## 1. What the original does

The boot flow of the original (`BootFlowManager`) is a table of screens. The PC list is **[exe]**:

| # | Screen | What it is |
|---|---|---|
| 1 | `MC_Bootup.fng` | the memory-card boot screen: lists the save root, then sets the start-up mode (`0x20`) |
| 2 | `LS_EAlogo.fng` | the EA logo movie |
| 3 | `LS_PSA.fng` | the public service announcement movie |
| 4 | `MW_LS_AttractFMV.fng` | the attract movie |
| 5 | `MW_LS_Splash.fng` | the title screen ("press Enter") |
| 6 | **`MC_Main.fng`** | **the profile step**, in start-up state `0x20` |
| 7 | `MainMenu.fng` | the main menu |

So the profile step comes **after the title screen and before the main menu**, on a black screen: the front end's stage scene is
not up yet. Step 6 does, by the number of aliases that step 1 listed (the alias is a folder `<alias>\<alias>` under
`Documents\NFS Most Wanted`, listed when its name is shorter than 8 characters without a space) **[exe]**:

| Aliases | What happens |
|---|---|
| none | the question "Create a new alias? If you do not create a new alias, changes may be lost." (Yes/No, No focused) |
| one | it is loaded at once, with no list |
| two or more | the list `MC_List.fng`, titled "load" (blackletter), eight rows, the first focused, Back at the bottom |

**Create (Yes).** Two notices, each with Continue (the autosave feature, and "do not reset or switch off the computer while the
autosave icon is displayed"), then the name entry ("Enter name:"), then "Do you want to save the alias NAME?". No at the question
leaves for the main menu **[exe]**.

**Load.** Picking a row loads that alias; the library says "Load complete." (Continue); the screen then closes and the main menu
follows. A file that fails the load checks is not loaded and its notice shows over the list. **Back** closes the list: the main
menu opens with no alias loaded **[exe]**.

## 2. What this build does

After the title screen (or straight away when the movies and the title screen are skipped the way section 5 says) the game runs
the same step over a black backdrop **[ours]** for the shape, **[exe]** for the screens and texts:

| Saved profiles (both folders of section 4) | Screen |
|---|---|
| none | the question (below), then the main menu |
| one or more | the list "load", then the main menu. The original loads a single alias at once; this build shows the list for one as well, as the user asked (`career-parity-menus.md` section 2) |
| a profile is loaded already | the main menu |

**The question.** `MC_CREATE_NEW_PROFILE_MENU` (`0xBE97590F`, the text above) in a dialog titled "selection", Yes and No, No focused
as in the original. Yes shows the two autosave notices (`MC_AUTOSAVE_WARNING_PC` `0xB01BE5DD`, `MC_AUTOSAVE_WARNING2_PC`
`0xB380362F`, title "save/load", one Continue each), then the name prompt of the alias manager's Create (`name_entry.rs`: first free
`PLAYER` name, up to 7 characters, Enter accepts, Escape leaves). Accepting makes the alias, **saves it at once** and loads it, and
the notice says "NAME created." Continue goes to the main menu. **[ours]** for saving at once: the original asks "Do you want to
save the alias NAME?" after the name; this build has no unsaved alias, so the question is left out. No, or Escape on the name prompt, leaves
for the main menu with no alias loaded (Escape on a notice is Continue, so the chain goes on).

**The list.** `MC_List.fng` in load mode (`career_screens/save.rs`), the same screen as the alias manager's Load. A pick loads the
profile into the session (the loaded profile, not yet played: `career-parity-menus.md` section 1), shows "Load complete.", and
Continue goes to the main menu. Back goes to the main menu with no profile loaded. **[exe]** for both.

**The loaded profile reaches the rest of the game** through `CareerHandle::shared()`: the main menu card and name box, the career
menu, the alias manager, and Career, which resumes it. The settings that the original keeps in an alias are not part of this
build's profile files: they stay in the config file.

## 3. The player's name in race standings

The alias is the player's name in the game: the post-race standings and the lap table name the racer by it. The loaded alias is
`CareerHandle::shared().with_loaded(|career| career.alias.clone())`; the quick race and Quick Play also run with a profile that is
loaded and not played, so a race **outside career mode must read `with_loaded`, not `with`**. The rewrite's race report still names the
player "player" (`scenes/world/race/start.rs`, `plan_racers`), so the standings do not show the alias yet: the race and results code
has to take the name from the loaded profile **[open]**. Nothing in the profile step has to change for that.

## 4. Where the profiles come from

Two folders are listed, each alias once, sorted without case (`career/sources.rs`) **[ours]**:

1. **This build's folder**: `career` next to the config file (`NFSMW_CAREER_DIR` overrides it). Read and written.
2. **The original's folder**: `Documents\NFS Most Wanted` under the user's profile folder (`NFSMW_ORIGINAL_SAVES_DIR` overrides it;
   an empty value turns it off). **Read only.** This is the list the original shows (`NAME`, `mwo` and `stock` on the
   machine the screenshots came from) **[verified]**.

A pick reads this build's copy when it has one, else the original's. Saving always writes to this build's folder, so a profile
loaded from the original is copied there on its first save and the copy then hides the original's. The original's folder is never
written, so the original keeps working. The same load checks run on both (`ProfileDir::read`: magic, MD5, the profile hash).
Loaded checked on this machine: `NAME` from the original's folder loads ("Load complete.").

## 5. How the game starts

`play` (`commands/start.rs`):

| Switches | Start | Profile step |
|---|---|---|
| none | movies, title screen, profile step, main menu | yes |
| `skip_intro` (setting, `SkipMovies`, `NFSMW_SKIP_INTRO`) | title screen, profile step, main menu | yes |
| `--skip-boot` | main menu | no |
| `--ui-script` | main menu | no |
| `--aliases` | profile step, main menu | yes |
| `--drive` | free roam | no |

`skip_intro` used to start at the main menu, which skipped the title screen and the profile step too; it now only drops the movies,
as its name and the original's `SkipMovies` say **[ours]**. `--skip-boot` and scripted runs (screenshots, tests) keep starting at the
main menu with no prompt. **`--aliases` is the explicit way to run the profile step in a script**: it works with `--ui-script` and
`--screenshot`, and with `NFSMW_CAREER_DIR` and `NFSMW_ORIGINAL_SAVES_DIR` pointing at empty folders it shows the question.

```
nfsmw play --aliases --ui-script "wait 3" --screenshot list.png
NFSMW_CAREER_DIR=empty NFSMW_ORIGINAL_SAVES_DIR= nfsmw play --aliases --ui-script "wait 3;up;wait 1;accept;wait 2" --screenshot create.png
```

A screenshot run saves into a temporary profile folder, never the player's (`Frontend::begin`).

## 6. The step's end

The step is over when its last screen closes, whatever closed it (Back, No, Continue after a load or a create). The front end holds a
flag while the step runs; each frame of the menus asks whether the screen stack is empty, and when it is the main menu opens
(`flow_startup.rs`, `poll_startup`). Before this, Back on the start-up list closed the only screen and left an empty window
(found with `play --aliases --ui-script "wait 3;back"`): fixed.

## 7. Open

- The attract movie (`MW_LS_AttractFMV.fng`) is not part of this build's boot list.
- The single-alias auto-load and the save question after the name are not built (section 2).
- The two original notices' title is the library's "save/load"; the original's notice title for them is not read.
- The race standings take the alias only when the race code reads the loaded profile (section 3).
