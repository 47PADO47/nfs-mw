# Career parity menus: the alias list, the alias manager, the career menu, the Quick Race menu, the pause menu

Status: built. Evidence tags: **[exe]** read in the exe, **[verified]** checked in the install or on screen, **[ours]** a
decision of this rewrite. Addresses and Ghidra names are in [../provenance/career-parity-menus.md](../provenance/career-parity-menus.md).
Related: [frontend-menus.md](frontend-menus.md), [save-alias-manager.md](save-alias-manager.md),
[career-save-screens.md](career-save-screens.md), [career-save-screens-2.md](career-save-screens-2.md), [career-flow.md](career-flow.md),
[career-safehouse.md](career-safehouse.md), [career-rapsheet-2.md](career-rapsheet-2.md).

## 1. The model: a loaded profile that is not yet played

A profile (the original's alias) is **loaded** from the profile list and stays loaded across the menus; it is **played** only
from the career menu on. `CareerHandle` (`crates/nfsmw/src/career/mod.rs`) keeps both: `load` loads, `begin_play` and `end_play`
switch playing, `with_loaded` reads the profile for the menus, `with` and `is_active` see it only while it is played. The quick
race, the HUD, the mods and the world therefore see no career outside career mode, as before. **[ours]**

Where the state changes:

| Event | Loaded | Playing |
|---|---|---|
| a profile is picked in the list (start-up, alias manager, career menu) | that profile | off |
| Career menu, Resume Career | unchanged | on |
| Career menu, Start New Career | the new alias | on |
| Quit to any menu, Quick Race, free roam | unchanged | off |
| alias deleted | none, when it was the loaded one | off |

## 2. Start-up and the main menu

**Start-up.** After the title screen's Continue the game shows the profile list (`MC_List.fng`, "load") over the menu backdrop when
at least one alias is saved. Picking one loads it and shows the notice "Load complete." (title "save/load", one button,
Continue); Continue goes to the main menu. Back (Esc) goes to the main menu with no profile loaded. With no alias saved the
game asks to create one before the main menu ([boot-profile-prompt.md](boot-profile-prompt.md), which also lists the original's saved
profiles read only and says how `play` starts). **[exe]** for the screens and the notice (`MC_Bootup` then `MC_Main` in state `0x20`);
**[ours]** for always showing the list: the original loads at once when there is exactly one alias and asks to create one when
there is none (`career-save-screens-2.md` section 9.2). A profile that fails the load checks is not loaded and its notice shows
over the list.

**The main menu.** The scene is unchanged. With a profile loaded the name box at the bottom left shows the alias; without one
the name box and the card are hidden **[exe]**. The footer shows `Quit` (Q) and, with a profile loaded, `Game Stats` (key 1).
Game Stats shows and hides the card: Game Completion, Career Completion, Rap Sheet and Challenge Series (`done/total` and
percent) **[exe]**. The percentages are `career/card.rs` over `nfsmw_career::rapsheet::card`: the milestones banked, the events
the Blacklist bins record as won, the rap sheet ranks, and the 70 races of bin 19 for the series **[ours]**: the build records
no challenge series completions yet, so the series reads `0/70`. The exe skips an event named `19.8.31` in this count, but the
original shows 70 for this install, whose bin 19 holds 70 events including `19.8.31`, so the skip does not apply to this data
**[verified]**.

## 3. The alias manager

The Alias Manager icon of the main menu opens `MC_ProfileManager.fng`, an icon row with four icons in this order, each with its
own picture **[exe]**: Create (`MAIN_ICON_PROFILE_ADD`), Load (`GENERIC_ICON_LOAD`), Save (`GENERIC_ICON_SAVE`), Delete
(`MAIN_ICON_PROFILE_DELETE`). The title is "alias manager", the footer shows Back, and the loaded alias is in the name box.

| Icon | What it does |
|---|---|
| Create | asks for a name (below), makes a new profile with that alias, saves it, loads it (not played) and says "NAME created." **[ours]** |
| Load | opens the profile list; a pick loads that profile and says "Load complete."; the alias manager stays up |
| Save | asks "Are you sure you want to save the alias named NAME?" (Yes/No), then writes the loaded profile and says "NAME saved."; with no profile loaded it says so |
| Delete | opens the profile list ("delete"); a pick asks "Delete the career named NAME? It cannot be undone."; Yes removes the profile and says "NAME deleted."; the loaded profile is unloaded when it is the one deleted |

**Naming (Create, Start New Career).** The original types the name with the keyboard group of `MC_Main.fng` ("Enter name:"); the
PC reads the typed text **[exe]**. The rewrite shows a notice "Enter name: NAME_" over `Dialog.fng`: it starts with the first
free `PLAYER` name, Backspace deletes, typed letters, digits, `_` and `-` are added up to 7 characters (the profile list shows
only names shorter than 8 without a space, `career-save-screens.md` section 2), Enter accepts, Escape leaves. A name that
exists (compared without case) is refused with a notice. **[ours]**

The old rows "Save Alias" and "Load Alias" (the settings alias of `saves.rs`) are gone from the alias manager: it is the career
profile's manager now. **[ours]**

## 4. The Quick Race menu

The Quick Race icon opens an icon row (`MainMenu_Sub.fng`, title "quick race") with three icons **[exe]**: Quick Play
(`MODE_ICON_QUICK_SELECT`), Custom Race (`MODE_ICON_QUICK_RACE`) and Split Screen (`MODE_ICON_2PLAYER_SPLIT`; the exe builds
it from another table, **[unconfirmed]**). Custom Race and Split Screen are listed and greyed out for now **[ours]**; their
screens (`Track_Select.fng`, `Track_Options.fng`) stay in the install readers and the screen factory but no menu opens them.

Quick Play opens the brief (`Quick_Race_Brief.fng`, "quick play brief"): a random car and a random race, with Back (Esc),
Accept (Enter) and Randomize (key 1), and Accept starts that race with that car. See [quick-play.md](quick-play.md).

## 5. The pause menu

The pause menu names its Quit after the menu the game was started from, and in a race offers Resume Race and Restart **[exe]**
(`pm_ResumeRace`, `pm_RestartRace`, `pm_QuitQuickRace`, `pm_QuitMainMenu`, `pm_QuitRaceToFE`, provenance "Pause menu items").

| Started from | Quit item | Confirmation |
|---|---|---|
| the main menu (free roam) | Quit to Main Menu | "Are you sure you want to quit to the main menu?" |
| the Quick Race menu | Quit to Quick Race Menu | "...quit to the quick race menu? All race progress will be lost." |
| the career menu | Quit to Career Main Menu | "...quit to the career main menu? All race progress will be lost." |

Items, in order **[ours]**: Resume Race (Resume Free Roam when no race is running), Restart (a race only), Options, Quit. The
original puts Quit before Options and has a Tuning item; this build keeps its Options-before-Quit order and has no Tuning.
Restart asks "Are you sure you want to restart the race? All race progress will be lost." and then runs the race again from
its grid ([race-restart.md](race-restart.md)). Quit leaves for the menu it names, with the loaded profile still loaded and no
career playing.

## 6. The career menu

The Career icon opens an icon row (`MainMenu_Sub.fng`, title "career main menu", name box and Back) **[exe]**:

| Profile | Icons |
|---|---|
| none loaded | Start New Career, Load |
| loaded, career not started | Start New Career, Load |
| loaded, started, not won (flags bit 11 clear) | Resume Career, Start New Career, Load |
| loaded, started, won | Start New Career, Load |

- **Resume Career** plays the loaded profile. A career still in its prologue (stage 16) starts the world at once behind the
  loading screen; any other goes to the safe house menu, which stands *before* the world (`career-flow.md` section 4).
- **Start New Career** asks for a new alias name, makes the profile, saves it and plays it: the world starts with the prologue.
- **Load** opens the profile list; a pick loads it and says "Load complete."; the career menu is then rebuilt for that profile.

**The safe house menu before the world.** The scene is the safe house interior with the profile's car (section 7). The menu is the
one the world opens (`MainMenu_Sub.fng` with the safe house row): Resume Free Roam (asks "Are you sure you want to leave the Safe
House to resume free roam?"), Blacklist N (the Blacklist screen), Car Select (greyed out here), Rap Sheet and Save, with the
summary of Career Completion, Bounty and Cash. Resume Free Roam starts the world (and asks for the roaming-start autosave, as
leaving the safe house in the world does). Back returns to the career menu. **[exe]** for the items and the confirmation;
**[ours]** for starting the world from here.

## 7. The menu backdrop

The backdrop is chosen per screen (`frontend/look.rs`, `factory::look_for`): the main menu, the options, the alias manager and the
Quick Race menu show the career-start GTR in the stage room (unchanged); the career menu shows the profile's last driven car in the
same room (the starter car, a red BMW M3, when it has none); the safe house menu and the screens it opens show that car in the safe
house interior; the Quick Play brief shows the car it picked. The backdrop is rebuilt when the look changes. How each look is
drawn is [menu-backdrop-looks.md](menu-backdrop-looks.md).

## 8. Footers

The footer hints follow the screen: the main menu shows `Quit` and `Game Stats`; the career menu, the alias manager, the Quick
Race menu, the safe house menu and the profile list show `Back` only; the Quick Play brief shows `Back`, `Accept` and `Randomize`.
