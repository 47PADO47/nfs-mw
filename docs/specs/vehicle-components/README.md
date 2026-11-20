# Vehicle components

Specs for [ADR 0014](../../decisions/0014-vehicle-components.md): a vehicle is a **base vehicle plus installed
components**. A component changes how the vehicle looks, what its numbers are and how it behaves; each base vehicle says
which slots it offers and which components each slot accepts. The same model serves the 2005 game's garage, the 2012
content mod and the RIDE 6 bikes mod.

Status: **designed, with a prototype**. The pure model and resolver are built and tested as
[`libs/blackbox-components`](../../../libs/blackbox-components/src/lib.rs) (synthetic fixtures for the three games, no game
data). Nothing in the game uses it yet; the staged plan is in [the migration plan](../../plans/vehicle-components.md).
These specs are the author's own design (no decompiled source); the facts about the existing code and games they rest on are
cited where they are used.

| Page | Covers |
|---|---|
| [data-model.md](data-model.md) | vehicle, slot, offer, component, rules, inputs, effects, selection, and the invariants |
| [resolution.md](resolution.md) | how effects compose and in what order, conflicts and notes, behaviours, repair, pricing and unlocks, determinism |
| [manifest.md](manifest.md) | the TOML a mod writes (`components/*.toml`, `[[slot]]`, `[parts]`), extending stock cars, and what happens to `[customization]`, `[stats]`, `[wheels]`, `[bike]` |
| [runtime.md](runtime.md) | the crate API, what the game binds, saves and networking, the mod SDK, the screens |
| [examples.md](examples.md) | the 2005 parts, the 2012 tiers and the RIDE 6 bikes, worked end to end |

## Words used

| Word | Means |
|---|---|
| **Vehicle** (`VehicleDef`) | a base vehicle: the model parts it is built with and its slots. A car, a bike, anything the game drives |
| **Slot** | a place where one component can be installed: `engine`, `exhaust`, `paint`, `chain_colour` |
| **Component** | one installable thing: `engine.stage2`, `exhaust.race`, `paint.metallic`. Defined once in a **catalog**, accepted by any number of slots of any number of vehicles |
| **Offer** | a slot's entry for a component: the component plus what only that vehicle changes (price, extra rules) |
| **Effect** | what a component does: `part`, `surface`, `param` or `behaviour` |
| **Selection** | what the player installed: the picks that differ from stock, by slot. The empty selection is the stock vehicle |
| **Pick** | one slot's content: a component (or nothing) and the values the player gave (a colour, a text) |
| **Resolved vehicle** | the result of `resolve(vehicle, catalog, selection)`: what the game builds |
| **Stock** | every slot at its default component. The default is what a new vehicle ships with |
| **Adapter** | game code that produces vehicles and catalogs from the game's own data (the 2005 parts database) and binds a resolved vehicle to the game's solids, physics and handlers |

## Why one model

The three games already describe customization as lists of named things that change named fields. The 2005 game links an
attribute collection per upgrade level; RIDE 6's `TuningComponentData_*` rows name a member path (`FrontSuspension.SpringStrength`),
a modifier type (multiplier, additive, curve) and a range ([ride6-physics-elements.md](../../formats/ride6-physics-elements.md));
the 2012 game keeps `PowerTrain`, `Nitrous`, `Body`, `Chassis` and `Tyre` upgrade lists per car
([chameleon-customization.md](../../formats/chameleon-customization.md)). What differs is which things a vehicle offers
and what they touch, so the model keeps *what is offered* as data per vehicle and *what a thing does* as a short list of
four effect kinds.
