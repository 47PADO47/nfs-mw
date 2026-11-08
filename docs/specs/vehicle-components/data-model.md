# Data model

Part of [vehicle components](README.md). Types are those of `libs/blackbox-components` (`model.rs`, `effect.rs`,
`selection.rs`). Every id is a string; nothing is an enum of a game's categories.

## Ids

`SlotId`, `ComponentId`, `PartId`, surface targets, parameter keys and behaviour domains are strings. The library never
interprets them: the game maps them. Convention for readability and for mods that share a catalog:

- lower case, dotted paths: `exhaust.race`, `tyre.compound`, `front.spring_n_per_m`;
- a mod's own ids are prefixed with its id when they could clash (`ride6:exhaust.race`); the stock game's adapter uses the
  prefix `nfsmw:`;
- an id is 1 to 64 bytes of `[a-z0-9_.:-]`. The mod loader enforces this; the library accepts any string up to the
  128 bytes the wire form allows.

Ids are what saves and the network carry, so **an id never changes meaning**; renaming one loses the choices saved under it
(the current system has the same property: a choice is told apart by a hash of the folder, the kind and the name,
[car-customization.md](../../modding/car-customization.md)).

## Vehicle

```
VehicleDef { id, parts: {PartId}, slot: [Slot] }
```

`parts` are the model parts drawn before any component: the **fixed** body. A part that can change is shown by the
component installed in its slot, not listed here (see [resolution](resolution.md#parts)). `slot` order matters: it is the
order of screens and of effect composition.

## Slot

```
Slot { id, group, accepts: [Offer], default?, optional, when: [Condition], inputs: [InputDef] }
```

| Field | Rule |
|---|---|
| `id` | unique in the vehicle |
| `group` | the section a screen lists it under (`performance`, `looks`, `chain`); free text, empty allowed |
| `accepts` | the components the slot takes, in screen order. At least one. No component twice |
| `default` | the stock component; must be accepted and **must have no rules** (no `when` on the component or the offer). `None`: the slot is empty on a stock vehicle |
| `optional` | the slot may be emptied. A slot with no default is always optional |
| `when` | the slot exists only while these hold. They may name **earlier slots only**, so activity is settled in one pass and cannot be circular. An inactive slot installs nothing, shows nothing, and counts as empty for every rule |
| `inputs` | what the player gives when picking here (below) |

*Per-vehicle availability is exactly this list.* A KTM 450 declares `bar_pad` and no `chain`; a Panigale V4 S declares
`chain` and `chain_colour` and no `suspension`. A selection naming a slot the vehicle lacks is a `Problem::UnknownSlot`
(refused, not ignored), so a network peer or a save cannot install a part the bike has no place for.

## Component

```
Component { id, name, tier, effects: [Effect], when: [Condition], price, unlock? }
```

| Field | Meaning |
|---|---|
| `name` | what screens call it: a translation key ([ADR 0012](../../decisions/0012-localization.md)) or a plain name; empty shows the id |
| `tier` | position in an upgrade ladder; read by `Condition::Tier` and by screens (bars, "stage 2"). 0 for stock and for things that are not tiers |
| `effects` | see below, applied in order |
| `when` | rules that hold wherever it is installed ("this exhaust needs a race engine") |
| `price` | currency units, 0 free; what a unit is, and trade-in, are the game's |
| `unlock` | the name of an unlock the game checks ([resolution](resolution.md#pricing-and-unlocks)); `None`: always available |

A **catalog** (`Catalog`) maps id to component and is shared: a mod's catalog is loaded once, every vehicle of the game and of
other mods may accept its components. A duplicate id is an error (a mod cannot replace another mod's component).

## Offer

```
Offer { component, price?, when: [Condition] }
```

A slot's entry for a component. `price` overrides the component's for this vehicle (a Ducati's exhaust costs more than a
KTM's); `when` adds rules for this vehicle only ("only with the long swingarm"). In TOML an offer is a bare id or a table.

## Rules (`Condition`)

All of a component's, offer's and slot's rules must hold (AND). Each looks at one other slot:

| Rule | Holds when |
|---|---|
| `is { slot, any_of }` | the slot holds one of these components |
| `is_not { slot, any_of }` | it holds none of them (an empty or inactive slot passes) |
| `filled { slot }` / `empty { slot }` | it holds something / nothing |
| `tier { slot, at_least }` | its component's tier is at least this (an empty slot is tier 0) |

Dependencies ("only available with X") are `is`/`tier`/`filled`; exclusions are `is_not`/`empty`. Rules are evaluated on
the whole selection, so an exclusion declared on one side already forbids the pair; declaring it on both sides only gives
the screen a reason on both. A rule that fails on a pick makes that pick invalid; [repair](resolution.md#repair) resets it.

## Inputs

A slot can ask the player for values when a component is picked there:

| Definition | Value | Check |
|---|---|---|
| `rgb { key, default }` | `[r, g, b]` sRGB | none |
| `num { key, default, min, max }` | a number | finite, in range |
| `text { key, default, max_len }` | a string | at most `max_len` characters |

An input left out of a pick takes its default. Effects reference an `rgb` input by key as a colour source. Inputs carry the
free-form parts of the 2005 shop (custom RGB body colour, the three vinyl channel colours, rim colour), RIDE 6's chain
colour, and a licence plate text, without a component per colour.

## Effects

Four kinds, listed by a component and applied in order.

| Effect | Fields | Does |
|---|---|---|
| `part` | `show: [PartId]`, `hide: [PartId]` | adds model parts to the drawn set and removes others. A swap is one effect that does both |
| `surface` | `target`, `texture?`, `finish?`, `colours: {channel: colour}` | changes how a named surface (a material, a paint area, a decal layer) looks. A colour is `[r,g,b]` or the key of an `rgb` input |
| `param` | `key`, `op`, `value` | changes a number the game reads, or sets a flag, text or list. `op` is `set`, `add`, `mul`, `min` or `max` |
| `behaviour` | `domain`, `select` | picks one named behaviour of a domain: `tyre.compound = "slick"`, `handling.model = "drift"`, `exhaust.sound = "race"` |

These cover the three kinds of change the brief asks for:

- **Visual:** `part` (add, hide, swap meshes), `surface` (materials, colours, textures, decals as a surface layer).
- **Performance:** `param` (stat and physics-parameter deltas and overrides: mass, power, grip, drag, brakes, suspension).
- **Behaviour:** `behaviour` (a selector the game maps to a different handler or a named parameter set), plus `param`
  flags for switches (`drift_assist`, `damage.resistant`).

`param` keys name what the game binds. By convention they mirror the manifest path of the number they change
(`stats.mass_kg`, `bike.front.spring_n_per_m`), so a mod author reads the key off the vehicle's own manifest
([manifest.md](manifest.md#parameter-keys)); the game checks every key against the keys it can bind when the catalog loads.

## Selection

```
Selection { slot -> Pick },   Pick { component?: ComponentId, inputs: {key: Input} }
```

Sparse: a slot that is not in the selection holds its default. `Pick.component = None` empties an optional slot. The
**canonical form** drops a pick equal to the slot's default and every input equal to its default, so two selections of one
vehicle are the same car exactly when their canonical forms are equal. Saves and the network store canonical selections.

## Invariants (checked by `VehicleDef::check`)

1. Slot ids are unique and non-empty; every slot accepts at least one component; no component twice.
2. Every accepted component, and every id named by a rule, is in the catalog; every rule names a slot of the vehicle.
3. A slot's `when` names only earlier slots.
4. A default is accepted and has no rules; a slot with no default is optional.
5. Inputs have valid ranges and defaults; every colour that names an input names an `rgb` input of the slot, for every
   component the slot accepts.
6. `add`, `mul`, `min`, `max` take a finite number; all values are finite.
7. The stock selection validates.

From 4 and 7 follows what repair relies on: **the stock vehicle is always installable and resetting any pick to its default
is always valid**.
