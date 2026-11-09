# The shop on the components model: what a vehicle's shop lists, prices, refuses and saves

How the auto shop, the performance shop, the Junkman and My Cars' customization screens read a vehicle that has **components**
([ADR 0014](../decisions/0014-vehicle-components.md), `libs/blackbox-components`, [the bikes' components](../modding/ride6-bike-upgrades.md)),
and how a car of the original game stays exactly as it was. The reader of the data is `garage::options::VehicleOptions`
([devtools-vehicle.md](../devtools-vehicle.md#the-vehicle-options-garageoptions)); the shop screens consume it and add nothing to its
panel. Tags: **[ours]** a choice of this port, **[verified]** run offscreen or in a unit test, **[unverified]** not run.

## 1. Which vehicle uses it

`garage::options::has_components(car)`: the car's mod published components with at least one `[[slot]]` (a RIDE 6 bike). Every other
vehicle (all 2005 cars, the 2012 mod's cars, a mod car with no `[[slot]]`) has no components and keeps **the shop's fixed categories**:
the nine model categories (Body kit, Spoiler, Hood, Roof scoop, Rims, paint, tint, rim paint, vinyl) and the seven performance
categories, an empty one **dimmed, not dropped**, in the original's order and words. Those categories are also what `VehicleOptions`
lists for such a car (`options/build.rs`, `category_slot` and `level_slot`), so the fallback is one model, and a test ties the two
(`the_fixed_categories_are_what_the_vehicle_options_list_for_a_car_without_components`). The golden lists of a plain car are
`a_car_of_the_original_game_keeps_the_fixed_categories_greyed_where_it_has_none` (`frontend/garage_ui/choices/slots_tests.rs`).

## 2. What the screens list **[verified]**

For a vehicle with components `frontend/garage_ui/choices/slots.rs` builds the sections from `VehicleOptions::listed`:

| Section | Slots in it | Entry opens |
|---|---|---|
| **Parts** | a slot kept in a model category (body kit, spoiler, hood, roof scoop, rims: a bike's brake discs, exhaust, bar pad, rims), and a component slot no category holds whose `group` is not `performance`, `appearance` or `visual` (`looks`, `road`, `parts`...) | the choices of the slot (`Page::Choices`); the rims no longer go through the brand pages |
| **Performance** | a slot kept as a level (a bike's engine, chassis and brake ladders), and a component slot of group `performance` | the choices of the slot, in the original's performance package screen |
| **Visual** | the pages every vehicle has (Paint, Vinyls, rider gear, Rim Paint, Window Tint, Decals, Numbers, Plate, Custom Gauges) and a component slot of group `appearance` or `visual` | as before |

The entry's text is the **slot's own name** (`Exhaust`, `Brake discs`, `Engine`: the slot id humanized, `options/names.rs`), the page's title
and the cart line say the same (`Page::component_title`, `Row::label`), and the choices carry the **components' names** (`Stock exhaust`,
`Akrapovic Completo Standard`). A slot with a choice besides its stock one only is listed: **an empty slot is not listed**, and a section
with no slot is not offered on the main page. The Visual pages a bike has nothing behind (vinyls, rim paint, window tint, decals, numbers,
custom gauges) are left out the same way (`choices.rs`, `visual`), where a car keeps them. The rows underneath are the ones the shop
always had: `Row::Part(category)` and `Row::Performance(category)` for a slot a category or a level keeps, and `Row::Slot(position)` for a
slot only the components know; so saves, the cart, the original screens' pictures and the three UI styles (Original, Custom, Remastered
draw the same `Page`s) read the same rows.

The **Junkman** (the backroom) lists the slots a marker buys (the model categories and the levels, as `Row::Junkman` for a level) and none
of the slots only the components know: those and the rider's gear have no marker.

## 3. Rules and the refusal **[verified]**

A choice is made through `VehicleOptions::apply` (`garage/menu_slots.rs`), which checks the vehicle's rules on the **result**: a
component or offer that requires or excludes another slot's component or tier, the catalogue's own (rim paint needs aftermarket rims;
fitting other rims takes the rim paint off), an empty slot that cannot be empty. When a rule does not hold nothing changes, and:

- the choice is listed but **dimmed** (`Item::enabled` is false) so Accept does nothing on it;
- the shop's message line shows the data's reason while the choice is highlighted: `Loud exhaust cannot be fitted: needs another
  perf_engine` (`Item::blocked`, drawn where the shop's other refusals are, `0xa6298e25`);
- taking away what another choice needs (the engine stage the loud exhaust requires) is refused the same way.

The F6 garage and the vehicle panel use the same function, so the shop, the console (`vehicle set`) and the panel cannot disagree.

A choice can also **bring others with it** (an offer's `bundle`, [data model](vehicle-components/data-model.md#offer)): choosing `Brakes level 2`
(a level) fits the brake discs of the bike's kit 2 (a part), and going down to level 1 swaps them for kit 1's and to the original
equipment takes them off. It is part of the same `VehicleOptions::apply` (`garage/options/bundle.rs`, after the edit and before the
rules are checked), so the shop, the panel, `vehicle set`, the performance shortcuts (Maximum and Stock) and Reset all do it, and the
cart shows the part as a line of its own. The link is the vehicle's data and any slot can have it (a level, a model category or a
slot only the components know); the RIDE 6 importer writes it for the brake ladder and the disc kit of the same level. Choosing the
discs by themselves does not change the level.

## 4. Prices and unlocks **[ours]**

| Slot kept as | Price | Trade-in (non-backroom) | Unlock |
|---|---|---|---|
| a model category | the component's `price` when the data gives one (an offer's price wins over the component's), else the first tier of the category's table, as every mod car's choice always was | a quarter of the replaced component's price when it had one, else the table's quarter | the career's table for the category's entity at level 1, as before |
| a level | the ladder component's `price` for the level fitted, else the stage's price in the table | none, as before | the career's table, as before |
| a component slot no category holds | the component's `price`, else the first tier of the **body kit** table | a quarter of the replaced component | the body kit entity at level 1 |
| the rider's gear | the part's default price unless owned ([the gear spec](ride6-rider-gear.md)) | none | none |

The RIDE 6 mod writes no price (the data has none), so a bike's parts cost the table prices today; a mod that writes `price` in its
`components/*.toml` gets its own. Going back to the stock component, or emptying a slot, costs nothing. The backroom pays with markers
and keeps the table's numbers. The code is `garage/pricing/components.rs` (`adjust`, `session`, `gear`) over `Pricing::items`, which
adds a line for each changed component slot and for each changed piece of gear.

## 5. What is saved and what it does

| What | Kept in | Reaches |
|---|---|---|
| a pick in a slot a model category or a level keeps | the hash in `Selection::parts` (`my-cars.toml`, `garage.toml`) or the level in the package (the save), as before | the model and `car_with_customization` |
| a pick in a component slot no category holds (`Selection::slots`) | `presets::Stored::slots`, a table `slot id = "component id"` (the empty id: emptied on purpose), written **only when not empty** | `components::select::picks`, then the model's parts and the physics |
| the rider's gear | the profile's `rider-gear.toml`, not the car | the rider |

**Both `my-cars.toml` and the career's garage extension (`garage.toml`)** carry the table, since both are `presets::encode`. A pick the
mod no longer has (`components::of(folder)` lacks the slot or the component) is dropped on reading with a log line: the stock part is
worn, as every saved choice of a mod car does. The **original save's** customization record cannot name a mod car's parts, so
`records::write` stores the stock part for a hash the installed database does not list (`native_choice`, mod cars only) and the extension
keeps the real choice; its fingerprint is the record's, so the pair stay together.

**Physics.** `PhysicsData::car_with_customization(type, package, &Selection)` resolves the whole selection (`select::resolve`) and applies the
`param` effects through `components::keys` (`bike.engine.peak_torque_nm`, `bike.chassis.mass_kg`, `bike.brakes.front_force_n`,
`bike.brakes.rear_force_n`, limited to 0.5 to 2 times the base), whichever slot holds the component: an exhaust that adds torque, a
disc that adds brake force. `car_with_performance` is the same with no parts. The world's `apply_garage_selection` rebuilds the physics
when a part of a car with components changes; a car of the original game never does (its parts are in the model only). The shop's
rating bars still read the 2005 tables and do not move for a bike (the estimate of `ride6-bike-upgrades.md`).

## 6. Where the code is

| Piece | File |
|---|---|
| the options' shop helpers (`of_draft`, `of_selection`, `listed`, `ShopSection`, `has_components`) | `crates/nfsmw/src/garage/options/shop.rs` |
| the rows' lists, choice and refusal | `crates/nfsmw/src/garage/menu_slots.rs`, `menu_rows.rs` |
| the sections' entries | `crates/nfsmw/src/frontend/garage_ui/choices/slots.rs` |
| the cart's lines | `.../garage_ui/table/cart/{rows,names}.rs` |
| prices | `crates/nfsmw/src/garage/pricing/{components,items}.rs` |
| saving | `crates/nfsmw/src/garage/presets.rs`, `scenes/world/garage/records.rs` |
| physics | `crates/nfsmw-data/src/car/physics/data.rs`, `car/mods/components.rs` |
| the test bike | `crates/nfsmw/src/garage/menu_fixture.rs` |

## 7. Checked offscreen, and not done

**Checked** (release build, muted, offscreen; the commands are in the work notes of the change, `C:\r6work\shop-shots`): a KTM450 as the career's
current car in a career auto shop: the Parts section (`Exhaust`, `Rims`, and the brake discs and bar pad where the bike has them), the exhaust
list with the components' names (`Akrapovic S K4MET6 ...`), the bike's own rims (9 choices, which the brand pages never reached), the
Performance section (`Engine`, `Chassis`, `Brakes` only), the engine page with the component's name (`Engine kit level 2`) and no 2005
package text, the Visual section without the pages a bike has nothing behind, the rider gear page and its cart line. **A purchase**: an
exhaust in the cart (cost 1,000), the checkout, cash 300,000 to 299,000 in the saved profile, and the choice in `garage.toml`
(`[[parts]] slot = 1`, the Spoiler category's hash of the car and the component) beside the unchanged native record. A **gear purchase**: Rider Body
500 in the cart, cash 299,500, `rider-gear.toml` with `owned = ["body:suit001_female"]`. A 2005 car (Cobalt SS) in the same shop shows the fixed
categories (`Body Kits`, 7 performance categories, `Pro` 14,200, the 2005 package texts) as the integrated build shows them. The three UI
styles draw the garage through the same original screens (Custom and Remastered restyle only the HUD and the vertical menus), so the pages
are the same in all three.

**Not done**
- A career car that is a bike: the career's car lot sells the 2005 cars, so a bike reaches a career shop only if a profile's current car
  is one (the shots used a profile whose current car record was edited to the bike's key). Nothing sells a bike.
- A **rule's refusal on real data**: the RIDE 6 components carry no `when` rules, so the dimmed choice and its message are covered by unit
  tests on authored data (`menu_slots_tests.rs`, `slots_tests.rs`), not seen on a real bike.
- The shop's rating bars and the performance package texts of a bike (`PD_*`) still come from the 2005 tables; the component's name is
  shown in their place.
- The component's `unlock` names have no provider yet ([ADR 0014](../decisions/0014-vehicle-components.md), open question 12).
- Gear has no unlock tier and no trade-in.
