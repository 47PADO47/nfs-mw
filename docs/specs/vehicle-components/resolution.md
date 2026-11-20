# Resolution: how effects compose

Part of [vehicle components](README.md). Implemented in `libs/blackbox-components` (`resolve.rs`, `validate.rs`,
`state.rs`, `view.rs`). Resolution is a pure function:

```
resolve(vehicle, catalog, selection) -> Result<ResolvedVehicle, Vec<Problem>>
```

It first validates the selection and returns the problems if there are any; the game never builds from an invalid one.

## Order

1. **Picks in effect.** For each slot, in the vehicle's order: the selection's pick, or the default. A slot is **active**
   when all its `when` rules hold on the earlier slots (an inactive earlier slot counts as empty).
2. **Fold.** For each active slot with a component, in slot order, and for each of its effects in listed order, apply the
   effect to the result. Nothing else orders anything: no priorities, no load order between components.

Slot order is therefore the one tie-breaker, and it is data the vehicle's author controls.

## Parts

`parts = (vehicle.parts ∪ every shown part) ∖ every hidden part`.

- A part **hidden by any component is hidden**, even if another component shows it. The resolver records a
  `Note::PartHidden { part, hidden_by }` so a mod author finds the clash.
- **Rule of thumb for authors:** a part that can change is *shown by the component that owns it*, and is not in the vehicle's
  fixed `parts`. The stock exhaust component shows `muffler.stock`; the race exhaust shows `muffler.race`. Swapping needs no
  `hide` at all, and a vehicle that offers the slot with a different default needs no edit to the component. `hide` is for
  the cross-slot case: a wide body kit hides the stock spoiler's part.
- A part id the game has no solid for is the game's problem to report at load ([manifest.md](manifest.md#parts)); the
  resolver does not know what exists.

## Surfaces

`surface` effects merge per `target`, field by field: a `texture`, a `finish`, and each named `colours` channel replace what
earlier effects set; a field an effect leaves out keeps the earlier value. So paint (`finish`, `base` colour) and a vinyl
(`texture`, channel colours) on different targets never interact, and two components on one target layer in slot order.
A colour that names an input takes the pick's value, or the input's default.

## Parameters

Each key collects a **modifier**: the last `set`, the sum of `add`s, the product of `mul`s, the lowest `min` and the highest
`max`. The game applies it to its own base value:

```
value = clamp_min_max( ( set_or_base + add ) * mul )          # the cap (min) is applied after the floor (max)
```

| Op | Collected as | Stacking |
|---|---|---|
| `set` | the value of the last setter | an override. Two setters of different values: the later slot wins, `Note::SetConflict` |
| `add` | sum | independent, order does not matter (floats are summed in slot order, so the result is bit-stable) |
| `mul` | product | independent. Two `mul 1.1` give 1.21: percentages **compound**. Authors who want "+10% total" use `add` on a fraction |
| `max` | the highest | a floor: "grip never below 0.5" |
| `min` | the lowest | a cap: "top speed at most 300" |

**Why this order** (set, add, mul, limits): an override (a tier that *is* the stage 2 value) is the base the other things
act on; flat changes ("−1.5 kg") apply before percentages ("+4% power") so a percentage scales the flat gain too; limits
come last so no stack escapes a hard range. It is the usual modifier-stack order in games, and it matches how the three
games' own data already works (the 2005 Junkman modifiers scale an already resolved package; RIDE 6's tuning rows are
additive or multiplicative on a resolved field).

A **number** is `f64` in the library; the game casts to its own precision. A `set` of a flag, text or list is read with
`flag(key)`, `set_value(key)` or `apply_list(key, base)`; a list is replaced by a `set` and then scaled element-wise by
`add`/`mul` (a torque curve, gear ratios). Mixed kinds (`mul` on a text) are refused by the definition check.

A key nothing touches returns the game's base unchanged. The library never sees the base: it returns modifiers.

## Behaviours

A `behaviour` effect writes `domain -> name`; the **later slot wins** and a differing pair of writers gives
`Note::BehaviourConflict`. The library does not know what a name does. The game owns the map from
`(vehicle, domain, name)` to one of:

- **a handler selector.** `handling.model = "drift"` makes the game pick its drift handling code; `damage.profile =
  "armoured"` picks a damage table; `exhaust.sound = "race"` picks a sound set. The game registers the names it supports
  per domain; a name it does not know is reported when the catalog loads (`Catalog::used()` lists them all).
- **a named parameter set.** `tyre.compound = "slick"` selects the vehicle's own table row for slick tyres (the 2012
  tyre list, RIDE 6's `Tire/GripCurve` rows keyed by compound). The numbers live in the vehicle's data
  ([manifest.md](manifest.md#behaviour-tables)), so no number about physics is in a component and the library names no
  physics.

A component usually pairs the two: a `behaviour` for what the game maps and `param`s for the portable numbers.
Behaviour choices are **not** stacked: one name per domain.

## Conflicts and notes

The resolver never fails on a conflict of the kinds above; the rule decides and a `Note` says so
(`SetConflict`, `PartHidden`, `BehaviourConflict`). Notes are for authors (logged at load of a test selection, shown by a
validator tool), not for players. What the library **does** refuse is a *selection* that breaks a rule, listed next.

## Validation

`validate(vehicle, catalog, selection) -> Vec<Problem>`:

| Problem | Cause |
|---|---|
| `UnknownSlot` | the vehicle has no such slot |
| `InactiveSlot` | the slot is inactive (its `when` fails) but the selection sets it |
| `NotAccepted` | the slot does not list the component, or the catalog lacks it |
| `EmptyNotAllowed` | an empty pick for a slot that cannot be empty |
| `ConditionFailed` | a rule of the component or the offer does not hold; carries the rule, so a screen can say which |
| `BadInput` | an input the slot lacks, of the wrong kind, or out of range |

`Locked` is reported only by `locked()` ([below](#pricing-and-unlocks)).

## Repair

`normalize(vehicle, catalog, selection) -> Selection` returns the nearest valid selection: every pick that causes a problem
goes back to its default, repeatedly, then the canonical form. It terminates (each pass removes at least one non-default
pick) and always succeeds, because a default has no rules. A screen calls it after each player change: choosing a different
engine resets the turbo that needed the old one, and emptying the chain removes the chain colour with it. A screen should
not offer an option whose state is `Blocked`, so `normalize` normally resets *other* slots, never the player's click.

## Pricing and unlocks

The library holds the hooks and none of the economy.

- A component has a `price` and an optional `unlock` name; an offer may override the price for its vehicle.
- `quote(vehicle, catalog, from, to) -> Quote { install: [Line], remove: [Line] }` lists, for every active slot whose
  component differs, what is installed and what is taken off, with prices. The **game** decides what a removed part
  refunds (the 2005 shop gives a quarter of the purchase price for eligible model parts,
  [garage-shop.md](../garage-shop.md)), what a free vinyl colour costs, and whether a career or My Cars pays.
- `locked(vehicle, catalog, from, to, gate) -> Vec<Problem::Locked>` lists the *newly installed* components whose `unlock`
  the `Gate` refuses (`Gate` is a one-method trait; the career, the shop's heat gates or a test implement it).
- **Resolving never asks the gate.** A car in a save keeps the parts it was bought with even if the unlock rules change; only
  a change made now is gated. The network checks structure only (below), not the other player's unlocks.
- `slot_views` marks each option `Selected`, `Available`, `Blocked(rules)` or `Locked(unlock)` (rules first, then unlocks),
  with the price, for the screens.

## Determinism

Everything that decides a result is ordered data: `BTreeMap`/`BTreeSet`, vectors in declaration order, slots in vehicle
order. There is no hashing of values, no random numbers, no clock, no thread. Floats follow one fixed order of operations
(IEEE-754 basic operations, no fused multiply-add, no fast-math), so two machines with the same definition and selection
resolve to bit-identical numbers; tests resolve repeatedly and compare. The network does not rely on that alone: it sends
selections (ids and bounded inputs), never resolved numbers, and compares a definition digest first
([runtime.md](runtime.md#networking)).

## Not in the model

Choices that are not components stay outside, on purpose: the **licence plate text** and **design** (kept beside the save,
[car-customization.md](../../modding/car-customization.md#where-the-text-and-the-design-are-kept)) are an input of a slot in
the new model but are *stored* by the same side stores for now; **tuning sliders** (suspension, gearing, aero tuning
in the 2005 Tuning screen) are continuous player settings, not discrete parts, and stay in `TuningSelection`. A component can
unlock one (`aero.tuning_unlocked`), as a fitted spoiler does today.
