# Performance packages

Sources read: the original PC 1.3 executable, inspected locally with Ghidra;
and [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0 reference
declarations `PhysicsUpgrades.hpp`, generated `pvehicle`, `junkman`, `nos`
and `induction` class layouts, and `MWAttribUserTypes.hpp`.
The reference's upgrade implementation is incomplete. The behavior below is
derived from the shipped executable, not copied from reference code.

Evidence status: **[decomp]** static analysis on 2026-10-13. This describes
package resolution; it does not establish complete driving-model parity or
claim an observed original-game driving test.

## Categories and limits

The seven independent categories are tires, brakes, chassis, transmission,
engine, induction and nitrous, in that order. A package stores an integer level
per category and a separate Junkman bit per category.

Each vehicle supplies its own maximum through the corresponding
`*_upgrades` field. The number of linked attribute collections is not the
maximum level. Zero means no normal package upgrades for that category.
Negative levels and levels above the vehicle's maximum should be rejected
by the public selection API; level zero selects original equipment.

## Resolving a level

Read category references from the original vehicle. Level zero uses reference
zero. A positive level requires at least two endpoint references and a positive
maximum. Let `t = level / maximum`, using floating-point division.

When the reference array contains exactly three entries, use entry one as
the lower installed-package endpoint and entry two as the upper endpoint.
Otherwise use entries zero and one. The three-entry case still uses
`level / maximum`; it does not subtract one from either term.
This supports categories whose original equipment is absent, such as nitrous.

At the maximum level, use the upper collection directly. At an intermediate
level, inherit the lower collection and combine the upper collection's fields
with matching lower fields. Supported numeric types are float, signed integer
and an axle pair consisting of front and rear floats. For a component `x`,
the resolved value is `lower * (1 - t) + upper * t`. Integer results truncate
toward zero. Axle-pair front and rear components interpolate independently.

For numeric arrays the output length is the larger endpoint length. An absent
element contributes zero; this matters when a transmission gains a gear.
The installed attribute definitions determine which fields and types exist.
Do not invent multipliers per level or substitute a global upgrade maximum.

## Junkman

The vehicle's `junkman` reference selects modifier data. Each category has its
own `*_package` modifier array. A modifier record contains a 32-bit class key,
a 32-bit definition key, and a float scale, in that order (12 bytes total).
The category identifies the active collection to modify; the inspected
modifier application reads the definition key and scale from each record.
It does not use the class key to redirect a modifier to another category.

A category is eligible when its modifier array exists with the expected
modifier type and contains at least one entry. There is no general requirement
that the normal upgrade maximum be positive or that the highest normal stage
be installed. In addition:

- Induction requires positive low boost or high boost in the active induction
  collection. Original naturally aspirated equipment is ineligible until an
  induction package supplies boost.
- Nitrous requires positive capacity and positive torque boost in the active
  nitrous collection. An absent original nitrous kit is ineligible.

Apply modifiers after ordinary package resolution. Multiply every element of
the named numeric field by the record's scale, including both axle components.
Integer scaled values truncate toward zero. Applying an already-installed
Junkman modifier is idempotent: resolve from original data, then scale once.
Changing an ordinary level preserves its Junkman selection and reapplies it
to the newly resolved collection when eligible. Removing Junkman restores
the ordinary level without the scale; restoring stock keeps Junkman only when
the stock category is eligible.

## Bounds of this specification

This specifies package data only. Driving additionally depends on transmission,
engine/induction/nitrous state machines, tire forces, suspension, aero, contact,
input shaping, assists, collision and tuning. Their individual specifications
and comparisons remain necessary before claiming complete original behavior.
Frontend prices, unlocks, heat changes and sound/exhaust upgrade presentation
are separate consumers. Dynamic attribute allocation and node-cache details
need not be reproduced when an immutable resolver gives equivalent values.
