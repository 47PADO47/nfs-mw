# Advanced controls: keyboard remap screen

Status: wired, not yet checked in the game. The keyboard slots (`input/slots.rs`) take a `Control` (an action, or one
direction of an axis). `frontend/controls_screen.rs` maps the 16 buttons to the eight rows. A press sends
`Command::Remap`, the frontend takes the next key (Escape cancels) in `plugin.rs` and saves it with
`Bindings::save_to`. The Controls icon of the option categories opens this screen directly (`icon_menu.rs`); the Control Options list (`input_options.rs`) has no bindings row.

**Two packages, one logic.** The main menu's list is `Options_PC_Controller.fng`: `icon_menu.rs` (`open_controls`, not
paused) switches to it from the Controls icon of `MainMenu_Sub.fng`. The in-game list is `Pause_PC_Controller.fng`,
opened from the pause menu's categories. `factory.rs` gives both packages `ControlsLogic`, and the package's name picks
its GUID table (`controls_layout.rs`: `OPTIONS` and `PAUSE`). Before that, the pause package had no table, so its
titles and key fields stayed at the package's placeholder `option_data`. The pause GUIDs come from `nfsmw dump-screen
Pause_PC_Controller.fng` (see [controls-packages](../provenance/controls-packages.md)).

**Drawn (checked offscreen, `nfsmw view-screen Options_PC_Controller.fng --screenshot`):** the title "Keyboard
Configuration" and the "Primary" and "Secondary" headers come from the package itself. The eight row titles are the
package's `option_data` text objects at x = -298 (`OPTIONS.title`); `ControlsLogic::start` sets them to the control
names (Accelerate, Brake / Reverse, Steer Left, Steer Right, Handbrake, Speedbreaker, N2O, Shift Down). The pause
package's titles are at x = -289 (`PAUSE.title`), and it is not yet drawn or checked live.

**Row limit (ours).** Both packages draw exactly eight rows (16 buttons). The list therefore cannot show every action:
Map (`M`, the world map) and ShiftUp have no row. They stay rebindable through the bindings file. A ninth row would
need a new package object, which the rewrite does not create at runtime. Giving Map a row is a decision for the user
(drop a row, or add a second page).

**Drawn (checked offscreen): the key names.** `plugin.rs` sets `Screens::set_key_texts` from the bindings every frame
(`controls_screen::key_texts`). The screen context (`Cx::key_texts`) carries them to `ControlsLogic::tick`, which writes
only the fields that changed. A key shows its code name without the `Key` prefix (`W`, `SPACE`, `SHIFTLEFT`,
`ARROWUP`); an empty slot shows `---`. The original shows its own names (`Freccia SU`, `CTRL`), so these differ in
wording, not in which key is shown.

**Speedbreaker (decided, by the user):** it is its own action, `Action::Speedbreaker`, bound to Left Ctrl by default,
and the row changes that action. The earlier mapping to `Boost` was wrong. The mechanic and the HUD gauge are not built
(see [speedbreaker.md](speedbreaker.md)).

**Checked live (scripted):** a remap. `nfsmw view-screen Options_PC_Controller.fng --ui-script "wait 2;accept;wait 1;key I;wait 2"` sets Accelerate's primary to `I`. Accept reaches the field as `BUTTON_RELEASED` (`0x936A6A7F`), not `PAD_ACCEPT`, so the field listens for that; the script's `key NAME` step supplies the key. The remap writes the real config file, so restore it after a test. Not checked: a real keyboard press. The two option widgets (`frontend/widgets.rs`) are tested but not drawn yet. Read from the install with `nfsmw dump-screen` (the English
build `Options_PC_Controller.fng`, FEng version 0x20000). Tags follow [frontend-menus.md](frontend-menus.md).

## 1. Which package is the screen [verified]

- `Keyboard.fng` is the **key picker**: a 6 × 10 grid of key images and buttons that the original shows when a
  binding is changed. It is not the list of actions.
- `Options_PC_Controller.fng` (frontend, 140 objects, 16 buttons) is the **controls list**: eight action rows, each
  with a Primary and a Secondary button. The screenshot's "Tastiera" selector and "Primary / Secondary" headers
  belong to the same screen family (`Pause_PC_Controller.fng` is the in-game copy, 116 objects, 16 buttons).

## 2. Layout of `Options_PC_Controller.fng` [verified]

The 16 buttons are image objects with the flag `BUTTON`. Each row is one button in each column:

| row | y | Primary button (x = -14) | Secondary button (x = 146) |
|---|---|---|---|
| 0 | -38 | name `0x62C1BBD0`, GUID `0x000FA349` | name `0x62C1BC33`, GUID `0x000FA366` |
| 1 | -8 | `0x62C1BBD1`, GUID `0x00104689` | `0x62C1BC34`, GUID `0x001046DC` |
| 2 | 22 | `0x62C1BBD2`, GUID `0x0010468E` | `0x62C1BC35`, GUID `0x001046E4` |
| 3 | 52 | `0x62C1BBD3`, GUID `0x00104693` | `0x62C1BC36`, GUID `0x001046ED` |
| 4 | 82 | `0x62C1BBD4`, GUID `0x0010469A` | `0x62C1BC37`, GUID `0x001046F5` |
| 5 | 112 | `0x62C1BBD5`, GUID `0x0010469F` | `0x62C1BC38`, GUID `0x001046FE` |
| 6 | 142 | `0x62C1BBD6`, GUID `0x001046A4` | `0x62C1BC39`, GUID `0x00104707` |
| 7 | 172 | `0x62C1BBD7`, GUID `0x001046A9` | `0x62C1BC3A`, GUID `0x00104710` |

The package has one text object: the footer hint "Press new button to remap this function" (label
`0x5FD43704`). The action names and the "Primary" / "Secondary" headers are not objects in the package, so the
rewrite has to draw them (or find where the original sets them) before the list reads like the screenshot.

## 3. What the rewrite needs (open)

1. **Row labels and headers.** The screen has no text objects for the rows (see above). Draw them from the
   language table, or from the `Action` names, in the rows' positions.
2. **Key capture.** A remap waits for a key. The frontend's `update` system (`frontend/plugin.rs`) reads
   `ActionState` only; it needs to read `ButtonInput<KeyCode>` while a row is waiting, and pass the key on.
3. **Logic.** `ControlsLogic` for `Options_PC_Controller.fng` maps the 16 buttons above (by GUID) to (row, column).
   A press on a button starts the wait for that slot; the next key sets it with
   `Bindings::set_keyboard_slot` and saves with `Bindings::save_to`. Esc cancels the wait, not the screen.
4. **Defaults and clear.** The screen's "Defaults" (1) and "Clear" (2) keys map to `Bindings::reset` and
   `Bindings::clear_keyboard_slot`.
5. **Header selector.** The "Tastiera" arrows are `widgets::Selector` over the input devices (keyboard, mouse,
   pad). Only the keyboard column has a backend yet.

## 4. The pieces that exist

- `input::Bindings::keyboard_keys(action)`: an action's keys in file order. Slot 0 is the primary, slot 1 the
  secondary (`SLOTS` = 2).
- `Bindings::set_keyboard_slot(action, slot, key)`: puts a key in a slot. The slot keeps the sign of the binding it
  replaces (S stays -1). A key moves away from any other action, and a key is never in both slots of one action.
  This is our choice; the original's conflict rule is not checked yet.
- `Bindings::clear_keyboard_slot(action, slot)`: removes a slot and moves the next one up.
- `frontend::widgets::Selector<T>` and `DualValue<T>`: the two shared widgets (choice with arrows; a row with a
  value per column). Their state is tested; they are not drawn yet.

## Playtest follow-up: field focus and pause footer

The 2026-10-27 playtest reports accumulated field highlights (UI-025) and an empty pause controls footer (UI-027).
Installed package inspection shows both field scripts, HIGHLIGHT and UNHIGHLIGHT, but no field response to
BUTTON_UNHIGHLIGHT. The host must apply the unselected script to departed fields, including focus moved by native
pad or pointer handling. Only the current field keeps HIGHLIGHT; a repeated tick must not restart its script or
request a new focus sound. Keep keyboard slot values and remap targets independent of this presentation repair.

The pause controls footer uses the same animated group and bar as pause settings. The replacement hints must
inherit that frame and expose the existing Select, Column, Remap and Back actions with their current bindings.
Suppress the complete old PC Back hint when composing its replacement, including its separate key image.
Controller reassignment/page switching now has a separate remake implementation contract in
[controller-remap-ui](controller-remap-ui.md), with its own physical-input and original-page acceptance limits.

## 5. Open questions

- Does the original reject a key bound to another action, or move it as this rewrite does?
- The order of the eight rows in the original, and whether the original's labels come from the language table
  (they should: the label hashes are not in the package).
