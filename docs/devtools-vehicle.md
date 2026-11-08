# Developer tools: the vehicle panel, the rider's gear and the controller

Part of the developer tools ([development.md](development.md), the F12 console, [the F6 garage](garage-overlay.md), [the
graphics lab](graphics-lab.md)). This page describes three things added to them: **one list of what a vehicle can change**, read
from the vehicle's own data and used by a panel, by the console and by the garage's Parts tab; **the rider's gear**, changeable
live; and **a controller way into every panel**.

## The vehicle options (`garage::options`)

The tools used to list customization from fixed tables: the Parts tab looped over the five model categories with the labels
"Body kit", "Spoiler", "Hood", "Roof scoop" and "Rims", so a bike showed its exhaust as a *Spoiler*, its brake discs as a *Body kit*,
and a Hood, a Roof scoop and a Window tint that it does not have. `VehicleOptions` ([ADR 0014](decisions/0014-vehicle-components.md),
[RIDE 6 bike upgrades](modding/ride6-bike-upgrades.md)) replaces that with the **slots of the vehicle on show**:

| Source | Slots | Name, group, choices |
|---|---|---|
| The vehicle's **components** (`[[slot]]` of a mod car: a RIDE 6 bike) | each slot of its definition, in its order | the slot's id as the label (`brake_discs`: Brake discs), its `group` (`looks`, `performance`, any a mod writes), the components it accepts with their own names, prices (when the data has one) and rules |
| The garage's **model categories** | body kit, spoiler, hood, roof scoop, rims, paint, window tint, rim paint, vinyl / livery, for a category the car's catalogue has choices in | the garage's words, group `parts` or `appearance`; a category a component slot maps onto (`exhaust` is kept as a spoiler, `rims`, `brake_discs` as a body kit, `bar_pad` as a hood: `nfsmw-data` `components::select`) takes the slot's name, group, prices and rules |
| The garage's **performance levels** | a ladder for each category the car's upgrade catalogue has stages in | group `performance`; a bike's levels are named by its ladder components (`Engine kit`), a car's are Stage 1, 2, 3 |

A car of the 2005 game and a car of the 2012 mod have no components: their categories *are* their components, and an empty category
(a car with no hood choices) is not listed. A vehicle with components lists them first, then the categories nothing maps onto.
There is one reader (`garage/options/build.rs`) and one writer (`change.rs`), so the panel, the console and the garage cannot disagree.

### Where a choice is kept

| Store | What it holds | Saved |
|---|---|---|
| A model category | the hash of the choice in `customization::Selection::parts` (a component choice is the hash of the car and the component, `select::choice_hash`) | with the garage's own selection (`my-cars.toml`, the save) |
| A performance level | `PerformancePackage::levels` | with the save, as for every car |
| **The session** (new) | `customization::Selection::slots`: slot id to component id, the empty id meaning "emptied on purpose", for a component slot **no category or level holds** | not saved: the original save and `my-cars.toml` hold hashes only |

The third store is what lets *any* slot of a vehicle's components be changed, not only the seven the garage knows: a mod that adds a
`mirrors` slot to a bike gets it in the panel and the console with no change to the game, and `select::picks` (which resolves the
components the car draws) reads it, so the part appears or goes at once. A slot's `param` effects reach the physics only for the
slots the garage keeps (the performance ladders): a session slot changes what is drawn.

A change is **checked** before it is applied: the vehicle's rules (a component that needs another slot's tier, an empty slot that
cannot be empty) and the catalogue's (rim paint needs aftermarket rims; fitting other rims takes the rim paint off) are run on the
result, and when they do not hold nothing changes and the reason is the message. A blocked choice is shown with its reason.

## The vehicle panel (F11)

**F11**, the devtools menu or `vehicle panel` opens it. It works in the world (the car you drive: the car is dressed again where it
stands, speed kept, like an F6 change) and in the car viewer (`view-car`, the asset viewer: the car on the floor is loaded again).
It lists, by group:

- every slot as a **stepper** `◀ choice ▶  n/last`, `n` being the number `vehicle set` takes (0 is stock): click the left or right half,
  or Left/Right while it has the focus (a controller's D-pad), Page Up / Page Down for ten at a time, or Enter for the next one; a `List`
  button lists every choice for the mouse, with a search box when the list is long (a vehicle's rims or vinyls can be hundreds);
- for a group whose slots can all be emptied, an **on** checkbox (see Groups); for a group with changes, a **stock** button;
- the **body colour** of a car that can be repainted, and **Reset to stock**;
- **Rider gear**, below.

The panel shows what the data lacks under the list ("Not in this vehicle's data: ...").

## Console

| Command | What it does |
|---|---|
| `vehicle` or `vehicle slots` | the slots with numbered choices, the installed one starred, prices and blocked reasons; `vehicle set` takes those numbers |
| `vehicle set <slot> <choice>` | fits a choice at once. The choice is a number (0 is stock), a component id or a part's hash as listed, `stock`, `none` (empties a slot that can be), or a part of its name when only one fits |
| `vehicle reset` | every slot to stock, and the paint to the vehicle's own |
| `vehicle groups` | the groups with their slots, how many are changed, whether they can be switched, and what the data lacks |
| `vehicle group <name> on\|off\|stock` | switches a group of optional slots, or puts the group back to stock |
| `vehicle color <RRGGBB\|stock>` | the body colour of a vehicle that can be repainted |
| `vehicle panel` | opens the panel |
| `gear [list]`, `gear set <part> <choice>`, `gear stock` | the rider's gear, below |
| `devtools [menu\|vehicle\|console\|lab\|extra\|garage\|close\|press <button>]` | opens a panel without its key (`--exec "devtools vehicle"` in a screenshot run), closes them all, or presses a controller button once (`up down left right a b lb rb`) |

`vehicle slots` for a RIDE 6 bike lists `exhaust`, `rims`, `brake_discs` (and `bar_pad` on the bikes that have one) under `looks` with the
components of that bike, and `perf_engine`, `perf_chassis`, `perf_brakes` under `performance`; for a car of the 2005 game it lists
`body`, `spoiler`, `hood`, `roof`, `rims`, `paint`, `tint`, `rimpaint`, `vinyl` (the words the `parts` command and `view-car --part` use,
`Category::key`) and `engine`, `transmission`, `induction`, `nitrous`, `chassis`, `tires`, `brakes`.

### Groups on and off; the Panigale's road equipment

A **group** is the set of slots a vehicle's data lists under one `group` name. A group is **switchable** when every one of its slots is a
component slot that can be emptied (`optional = true`, or no default): `off` empties them, `on` puts back the default component (or the
first one when there is no default), `stock` puts back the stock choice. This is data-driven: the tools name no group.

A bike's **road equipment** (side mirrors, the homologation plate holder; 159 of the 293 bikes have some) is the optional slot
`road_equipment`, group `road`, that the importer writes ([modding/ride6-road-equipment.md](modding/ride6-road-equipment.md)):
`vehicle group road off` takes the mirrors and the plate holder off and `on` puts them back, and the panel's checkbox does the same. A bike
imported without the shop's options, or with neither part, has no such slot: `vehicle groups` then says so and the panel shows it under the list.
What the importer writes (the lights are still not offered):

- The exporter writes the parts `mirrors` and `homologation` (`parts.json`); a lights preset is a field (`neutral`). The importer kept
  them in the body solid, as it keeps a stock exhaust in the body of a bike with no exhaust slot.
- The importer gives each of them a solid of its own, as it does the exhaust and the bar pad (`<ID>_KIT00_MIRRORS`,
  `<ID>_KIT00_HOMOLOGATION`), binds them as parts (`role = "body"`) and writes one optional slot and one component showing them:

  ```toml
  [[slot]]
  id = "road_equipment"
  group = "road"
  accepts = ["r6:road.on"]
  default = "r6:road.on"
  optional = true
  ```

  with `r6:road.on` a component whose `part` effect shows `mirrors` and `homologation`. Nothing else changes: `vehicle group road off` and
  the panel's checkbox then take the road equipment off and on. (The slot's id has no garage category, so it is kept for the session.)
- The lights are a preset of the RIDE 6 vehicle, not a mesh; they would need their own component with a `behaviour` effect the game maps
  to the lamp setup. Not done.

The same mechanism was checked with a real bike's own data by writing an optional slot over a bike's stock bar pad in a test profile
(below): the pad goes and comes back with the group.

## The rider's gear

The rider's helmet, suit, gloves, boots and body are the garage's Visual pages (`garage/rider_gear`,
[ride6-rider-gear.md](specs/ride6-rider-gear.md)): a mod lists them in `RIDER/GEAR/gear.json`. The tools change the same thing the same way:
`gear list` prints each part's numbered entries (1 is the rider's own gear, as in `--part rider-helmet=N`) with the worn one starred,
`gear set helmet 3` (or an id, or a part of a name, or `stock`) wears it, `gear stock` takes it all off. A choice is **kept in the
profile's `rider-gear.toml`** (`rider-gear-guest.toml` for the guest) exactly like a choice made in the garage, and the rider in the
world reads it at once (the rider's gear generation moves). The panel's **Rider gear** section has a stepper for each part with something
to choose and a *Rider's own gear* button. Gear is the player's, not the bike's: it can be set with any vehicle on show and is worn on the
next bike. The `rider_gear` setting off keeps the choice and shows the rider's own gear (the commands say so). The catalogue is read when a bike's
rider loads or the garage opens; before that `gear list` says no gear is available.

## The controller

Every developer panel (the console, the graphics lab, the extra options, the garage, the vehicle panel, the devtools menu) can be walked
and closed with a gamepad. The panels are egui windows; egui moves a keyboard focus between widgets with the arrow keys and Tab and
presses the focused one with Enter, so the controller *is* those keys (`devtools/pad_nav`).

| Button | In a developer panel |
|---|---|
| **L3 + R3** (both stick clicks) | open the devtools menu; with anything open, close it all |
| D-pad, left stick | move the focus (it repeats while held: after 0.4 s, 8 a second); on a slider or a stepper, left and right change the value, and held for 1.2 s they jump ten entries at a time (a list of 300 helmets) |
| **A** | press the focused widget (the stepper takes the next choice) |
| **B** | close the open popup or leave the text field, else close the panel (back to the menu, when the menu opened it) |
| **LB / RB** | the previous / next widget (Shift+Tab / Tab) |
| Start, Escape | step back as B does: close the panel (they never quit the game while a panel is up) |

With nothing focused, the first D-pad or stick move gives the focus to the first (down, right) or the last (up, left) widget, and the
menu and the panel focus their first line themselves when a controller is connected. Long lists scroll to follow the focus. A line of
buttons is shown at the bottom of the screen over any panel while a controller is connected.

**The game does not read the controller while a panel is up.** The menu and the vehicle panel set `UiFocus` and free the mouse
(`devtools/panels.rs`), as the console, the lab, F5 and F6 do; `UiFocus` silences every action but Cancel, Console and Garage
(`input/state.rs`), so the car neither accelerates nor steers, the HUD buttons do nothing, and the front end's menus do not move. The
panel closes, the mouse capture comes back. A normal shop transaction (`garage.menu`) owns its input and is never touched by the tools.

The chord is read from the controller directly, not through the action bindings, so it works whatever is rebound, and it is listed in
`nfsmw keys` ("Other"). Two clicks of the sticks are bound in the game too (L3 is the nitrous boost of the free camera and the ABS level of a
bike, R3 the next song): the first click of the chord can step the ABS level or the song once. Nothing else moves, because the chord
opens the menu in the same frame and the actions go quiet.

The **devtools menu** lists the vehicle panel, the console, the graphics lab, the extra options, the garage (only where the scene has
one), the metrics overlay (cycles off, basic, advanced), the performance trace (start or stop recording; F9 does too) and, in a
`--hud-scenario` run, the scenario's name. Each panel keeps its own key (F12, F8, F5, F6, F11) and its own state; opening one from the menu
or from a key closes the others, so there is one panel at a time.

## Where the code is

| What | Where |
|---|---|
| The options (view model, edits, groups, text) | `crates/nfsmw/src/garage/options/` |
| The session store, the pick of a slot | `nfsmw-data` `car/customization.rs` (`Selection::slots`), `car/mods/components/select.rs` |
| The scenes' side | `viewer/mod.rs` (`vehicle_options`, `change_vehicle`), `scenes/world/garage/options.rs`, `scenes/car/vehicle.rs` |
| The panel, the gear | `crates/nfsmw/src/devtools/vehicle/` |
| The menu, the open panels, the keys | `devtools/menu.rs`, `devtools/panels.rs` |
| The controller | `devtools/pad_nav/` (`state.rs` is the logic as plain data, `system.rs` reads the pads) |
| The commands | `devtools/console/dev_parse.rs`, `dev_cmd.rs` |

## Verifying it offscreen

A test profile with a few real bikes (`BLACKBOX_PROFILE_DIR=<profile>`, `docs/modding/drive-script.md`), then:

```sh
nfsmw view-world --drive DUCATI_PANIGALE-V4-S-2025 --camera-mode hood --no-traffic --muted --wait-for-load \
  --exec "vehicle set exhaust 2" --exec "devtools vehicle" --screenshot panel.png --screenshot-delay 6
nfsmw view-car DUCATI_PANIGALE-V4-S-2025 --yaw 90 --muted --exec "vehicle set rims 1" --exec "devtools vehicle" --screenshot viewer.png
nfsmw view-world --drive KTM450 --muted --wait-for-load --exec "devtools" --exec "devtools press down" --screenshot menu.png
```

The unit tests are `garage/options/tests.rs` (the view model on component and category fixtures), `devtools/panels_tests.rs` (opening,
closing, focus), `devtools/pad_nav/tests.rs` (the navigation), `devtools/console/dev_tests.rs` and `dev_parse.rs` (the commands),
`devtools/vehicle/gear_tests.rs` (the gear) and `nfsmw-data` `components/tests.rs` (the session slots).

## Limits

- A session slot is not saved, and its `param` effects do not reach the physics (only the garage's performance ladders do).
- The panel does not buy: it is a developer tool, like F6, and never touches the career's cash, markers or car records.
- The Performance tab of the F6 garage still lists the seven categories with the Junkman checkboxes; the panel and the console list only
  the ladders a vehicle has, named by its components. The Appearance tab keeps its category selectors.
- The car viewer has no physics, so it lists no performance levels.
- The pad cannot type: the console's line needs a keyboard (D-pad up and down still walk its history, A runs the line).
