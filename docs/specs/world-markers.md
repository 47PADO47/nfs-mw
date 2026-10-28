# World entry markers: the ground circle, the drive-over zone and the interact prompt

What the world shows at an auto shop, a safe house or a race start, when the car drives into it, and what the
Interact button does there. The destinations (the races, the safe house, the auto shop) are not built yet, so the
entry is a stub. The stored zones are in [formats/gameplay-races.md](../formats/gameplay-races.md) sections 3, 6 and
7; the minimap blips of the same places are in [hud-minimap.md](hud-minimap.md).

**Evidence tags** as in [docs/README.md](../README.md#evidence-tags). **[verified]** = read from the install's
gameplay data (the reader `crates/nfsmw-data/src/gameplay`) or checked in the running game; **[unconfirmed]** = a
choice of this rewrite with no evidence yet from the original. Nothing here is read from `speed.exe` or the decomp
in this pass: the box test's axes were **not** looked up in Ghidra. World icon assets have since been checked separately.

## 1. World icon presentation

The placeholder coloured discs have been replaced by installed world icon
geometry and particle groups. The retail type-to-asset table, current
placement rules and remaining uncertainties are documented in
[world-activity-icons.md](world-activity-icons.md). Trigger radius still
controls the entry test; it does not scale the world icon.

## 2. Which zones there are

| Zone | Shape | Source | Status |
|---|---|---|---|
| Auto shop (8) | sphere, stored radius | `ZoneType = customshop` | **[verified]** count; radius per zone |
| Safe house (4) | sphere, stored radius | `ZoneType = safehouse` | **[verified]** count; radius per zone |
| Race entry | sphere, authored radius and position | `engagetrigger`, linked by `TargetActivity` | **[exe]** icon placement; synthetic data without engage records retains the old 6 m grid fallback |

The stored box triggers are **not** entries. The install has 2,332 box triggers, and every name read is a race's own
route: checkpoints (`checkpoint`, `ai_cp`, `time_bonus_checkpoint`, the tollbooth `challenge_tollbooth_`), finish lines
(`finishline`, `finishline_reverse`, `finishline_bk`, `finish_line`, and names that start with a route number such as
`10_4_2_finishline`), the `gpstarget` and the `lose_trigger` **[verified]** (a count of the names in the `boxtrigger`
class, this pass). None is a place to enter, so the zone list holds spheres only.

The race grid and free-roam event entry are distinct positions. The latter
supplies the icon and drive-over zone; the grid is used when the race starts.

## 3. The drive-over test

Each frame, while the car drives in the chase view, the car's position is tested against every zone. A sphere holds
the point if its distance from the centre is at most the radius (the surface counts as inside). If several zones hold
the car, the one whose centre is nearest is the zone (`entry_zones.rs`, `EntryZones::entered`).

The car's position is its body centre (`Drive::position`), not its footprint **[unconfirmed]**. The spheres are tested
in render space, the space the car and the effects use (x forward, y left, z up): the gameplay reader gives physics
space, and `scenes/world/space.rs` converts it.

**The box test** is implemented in `nfsmw-data` (`BoxTrigger::contains`, full sizes) and, for the races, in
`libs/blackbox-race` (`Gate`, which the race code builds with the stored extents as **half** sizes, see below). The
boxes are the race route's, which the game does not use as entries. The rule:

- The box is centred on `Position` (physics space) and turned by `Rotation` about the up axis; the heading is
  `world_heading` (`(-sin R, 0, cos R)`).
- The first extent `Dimensions[0]` lies **along the heading**, the second `Dimensions[1]` across it (horizontal),
  the third `Dimensions[2]` up. The extents are full sizes, so a point is inside when its offset from the centre is
  at most half of each extent along the matching axis.
- **[unconfirmed]** The axis choice is fitted to the data, not read from the trigger code. The format doc records that
  the first extent is the thin one (2 to 4 m for 1,296 of the 1,446 checkpoints; finish lines `(2 or 3, 25..65,
  3..40)`) **[verified]**. A checkpoint or finish line is a thin gate across a road, so its thin extent is its
  thickness along the direction of travel, which is the heading. The alternative (the thin extent across the road)
  would make a 2 m wide strip with 40 m along the road, which no checkpoint in the data is shaped like.
- **Full or half sizes.** `BoxTrigger::contains` still reads the three extents as full sizes. The trigger constructor
  of `speed.exe` reads the two horizontal ones as full sizes and the height as a half size **[exe]**
  ([race-wrongway.md](race-wrongway.md) section 2), and the race code (`setup::gate_of`) does the same. The height
  was first found from the data, because the first checkpoint of `10.2.1` (`checkpoint1`, stored `(2, 60, 40)`) is
  centred 22 m above the road that the AI racers drive through (the road is at y 212, the box centre at y 234):
  a 40 m high box centred there (full size) does not reach the road, so a racer that passes it never counts; with half
  sizes (80 m high) it does, and every checkpoint and the finish line of that race are reached in order (measured with
  AI racers on the install). Open question 1 of the format doc remains open until the trigger test is read from
  `speed.exe` (`EGTriggerInternal`, no symbols in the Ghidra project).

## 4. The interact card

A card at the top left of the screen says what the car is in, and how to enter. It is drawn with egui by the world
scene (`Scene::ui`), not with FEng: the original card's package is not located yet.

- **Text.** The title is the place (`Auto shop`, `Safe house`, `Race`); the line is `Press Enter to go in`
  **[unconfirmed]**. The game's own shop prompt copy is `Enter Shop / Engage Event` (language table, label
  `0xab1032fa`, read from the install) [verified]; the card does not use it yet.
- **Animation.** Fades in over 0.2 s while it slides in from 24 points to the left, and fades out over 0.2 s when the
  car leaves the zone or another zone takes over (the old card goes out before the new one comes in)
  **[unconfirmed]**.
- **Where.** 24 points from the top left of the window, the same corner as the radio card; not placed by the HUD
  layout setting **[unconfirmed]**.
- **Scope.** Only while the car drives in the chase view. In the fly view or with no car, the card is hidden.

### 4a. The enter icon (not located)

The original also shows an animated enter icon (a pulsing circle with cars and an up arrow) while the car is in a zone.
It is not drawn yet. Its FEng object, package and script are **not located**: no dumped screen (all 197 of the
install's catalogue) has a matching object, and about 40 candidate names hash to nothing. The zone code that drives it is
partly read: the handler opens the front-end screens of each zone (`MainMenu_Sub.fng`, `Car_Select.fng`,
`CustomizeMain.fng`). What was checked and what is left is in [provenance/world-entry-zones.md](../provenance/world-entry-zones.md).
The card and the ground disc stay as the only entry feedback until it is found.

## 5. The Interact button

`Action::Interact` is Enter on the keyboard and the pad's South (A) button. Entering is allowed only when all of
these hold this frame (`entry.rs`, `enter_wanted`):

1. The car is in a zone (chase view, driving).
2. The card is up: its target is the zone's prompt and it is at least half faded in. A press during the fade is
   ignored.
3. The card is not holding a placeholder (a flashed prompt, see section 6).
4. Interact went down this frame (`just_pressed`).

**Overlaps.**

- **Alt+Enter** (fullscreen, `app/window/hotkey.rs`) is Enter with Alt held. `ActionState` does not read Interact from
  Enter while Alt is held (`input/state.rs`, `is_alt_enter`), so the chord does not enter a zone.
- **The pad's South** is also Handbrake. Entering does not suppress the handbrake: a press that enters a zone also
  pulls the handbrake for that step. Known; not changed here.
- **The console** takes the keyboard (`ui_focus`), and `ActionState` reads every game action as released then, so no
  entry happens while typing.

## 6. The stub entry

Until the destinations exist, entering logs and shows a placeholder:

- Log: `entering <kind> <collection name> (stub: the destination is not built yet)`, at info level.
- Card: the title `Entering <kind>` and the line `Not built yet` for 2 s. While it shows, the zone's own prompt does
  not change the card and a second Interact does nothing.

A **race** zone is no stub any more: Interact starts the race (`entry.rs`, `enter_race_zone`; `docs/specs/race-mode.md`),
in the career when the Blacklist has opened the race, and while a race runs every other card and zone is held. A race
the Blacklist lists and has not opened shows no start icon on the minimap, and no start icon shows during a race. The
safe house and the auto shop keep their own handlers. During a race the rewrite also hides the garage, car lot and safe house
icons, the free-roam speed-trap icons and every entry disc on the road (`hud.rs`, `effects/world.rs`) **[ours]**.

## 7. How to check it

- The disc: drive to a safe house or an auto shop (the positions are in the blip test, `hud.rs`
  `the_install_has_race_and_interior_blips`). The disc should sit on the road under the car, not floating and not
  buried; its edge should match the zone's radius when the car drives through it.
- The card: the title and the line appear as the car crosses the disc and leave when it does, with the slide.
- The original's marker: take a screenshot in the game at a safe house and compare the disc's size, colour and
  fade with this rewrite's. The size is verified by the radius; the colour, fade and pulse are not.
- The box test: a checkpoint's gate in the game should be crossed by a car going along the heading. Check the first
  extent by driving through a 2 m checkpoint and one of the 40 m cubes.

## 8. Open questions

1. The trigger test in `speed.exe` (which extent lies along the heading; full or half sizes; a body centre or a
   footprint). Until read, section 3 is **[unconfirmed]**.
2. The original marker's colour, fade, pulse and size per kind (the disc is a guess; section 1).
3. The race entry's own radius (6 m is a guess) and whether the race start is the entry or the event's own trigger.
4. The original card's package and placement (section 4), and the enter icon's object, package and script (section 4a).
   The copy is partly known: `Enter Shop / Engage Event` is the shop prompt's label.
5. Which button the pad's South should do when both Handbrake and Interact are bound to it (section 5).
