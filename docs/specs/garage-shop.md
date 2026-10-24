# Garage shopping and customization records

## Sources and evidence

**[decomp]** Read-only behavioral reference:
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0:
`CustomizeManager.cpp`, `CarCustomize.cpp`, `FECustomize.hpp`, `UnlockSystem.cpp`,
`FEReflected.hpp`, `VehicleDB.hpp/.cpp`, `PhysicsUpgrades.hpp`,
`PhysicsTunings.h`, generated `frontend.h`, `PhotoFinish.cpp` and `CarInfo.cpp`.
This document states facts and behavior, not a translation of reference code.
The reference contains incomplete methods. Price/refund behavior was therefore
checked against the original PC 1.3 binary's cart-total, checkout and trade-in
routines. Its SHA-256 is
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
That establishes static code behavior, not a live menu comparison.

## Session and purchase

Opening customization copies the selected vehicle's customization record into
a preview record. The installed record stays unchanged while browsing.
The preview is the installed record with the shopping cart's choices applied.
Removing a cart item restores the installed choice for that slot.
Discarding the session retains the installed vehicle, cash and markers.

The cart holds one item per model/material slot and one per performance
category. Replacing a pending item retains the original installed trade-in,
rather than trading in the previous unpurchased preview. Checkout installs only
active items. My Cars uses customization records without spending career cash.
Career checkout spends cash, or consumes the corresponding marker in the backroom.

Performance has no trade-in. Eligible model parts have a displayed trade-in
of one quarter of the installed part's purchase price, truncated to an integer.
The original binary applies the fraction once in both cart total and checkout.
Per-item delta is new price minus that credit. A negative delta awards cash;
the signed cart total can be negative. Returning to zero-price stock can
therefore refund an eligible installed aftermarket model part.
Paint, vinyl recolors and HUD recolors have special pricing/trade-in rules.
Auxiliary vinyl and HUD colors have no separate purchase cost.
The second number on the right door is not billed separately.

The original trade-in predicate excludes every performance package and visual
slots 76–83, 91, 99–115, 123 and 131–135. All five exposed model categories
are eligible; paint, tint, vinyls, decals, numbers and gauge/color slots are not.

## Installed pricing data

Inspect `frontend` collections whose parent is `carparts`.
The collection name itself is not the category identity:

| Attribute | Hash | Value |
|---|---|---|
| `feCarPartName` | `0x3b874697` | Signed 32-bit unlockable entity ID |
| `feCarPartInfo` | `0x1002ea65` | Array of 12-byte level records |

Each little-endian level record is `Level: i32` at byte 0, `Rep: f32` at byte 4,
and `Cost: f32` at byte 8. Select the record whose Level equals the requested
level. Cost is truncated to an integer. Unknown/missing prices must remain
distinguishable from an authored zero-cost record in the remake.

| Category | Entity ID |
|---|---:|
| Tires, brakes, chassis, transmission, engine, induction, NOS | 4, 5, 6, 7, 8, 9, 10 |
| Body kit, spoiler, rims, hood, roof scoop | 11, 12, 13, 14, 15 |
| Custom HUD, window tint | 17, 18 |
| Body paint and rim paint | 23 |
| Vinyl | 40 |
| Door numbers | 43 |
| Front/rear window decal | 44 |
| Left/right door decal | 46 |
| Left/right quarter decal | 48 |

The price level for a visual part is its installed upgrade level. Performance
uses a global level: global maximum minus this car's maximum plus local level.
Global maxima are 4 for engine/transmission/brakes and 3 for the other categories.
Level zero is original equipment and needs special handling, rather than
applying the performance offset to a stock restoration.

## Unlocks and backroom

The saved unlock table has 57 records, each 8 bytes:
career unlocked level/new level/times seen at bytes 0/1/2; quick-race unlocked
level/new level/times seen at bytes 4/5/6; bytes 3 and 7 are padding.
Fields are signed bytes. Category IDs above index this table.
Career parts are available when their requested level is at most the category's
career unlocked level. Level-zero visual parts are always available.
Career completion, prior profile completion and the debug unlock override can
open otherwise locked parts. My Cars uses the quick-race unlock fields.

Ordinary backroom part unlocks permit one level beyond the career unlock.
The performance screen has a separate Junkman option: its category must accept
Junkman and have the corresponding unused marker. It does not require the
highest ordinary stage to be installed or unlocked. Nitrous and induction
retain their installed kit requirements. Checkout consumes that marker instead
of charging cash. The generic unlock helper's level-7 rule does not replace
the performance screen's own Junkman entry path.

The marker kind differs from the unlock entity: brakes 1, engine 2, NOS 3,
induction 4, chassis 5, tires 6, transmission 7, body kit 8, hood 9, spoiler 10,
rims 11, roof scoop 12, HUD 13, vinyl 14, decals 15 and paint 16.
Ordinary category markers use parameter zero and require owned state 1;
consumption clears the kind and state to zero, retaining the parameter.
Tint, rim paint and numbers have no separate marker.

**[static executable, 2026-10-24]** Counting and redemption use different
parameter matching for vinyls. Counting vinyl kind 14 with requested parameter
zero counts all owned vinyl markers, including nonzero parameters. Other
counts require an exact parameter match. Redemption always requires exact
kind, parameter and owned state 1; it removes only the first matching record.
The original vinyl slot checkout requests kind 14 and parameter zero. It does
not choose a parameter from the selected vinyl family. A nonzero-parameter
vinyl marker can therefore contribute to the displayed count without being
consumed by that zero-parameter request. Preserve the records rather than
silently redeeming a different parameter.

### My Cars Junkman availability

The original PC performance screen exposes an additional Junkman option when
the car/category can install it and either of these conditions holds:

- Career flags contain Burger King reward bit 13 (`0x2000`) and the screen is
  outside career mode.
- The profile has completed a career at least once and the vehicle is not the
  hero `BMWM3GTRE46` type.

The first alternative does not have the second alternative's explicit hero-car
exclusion. Category compatibility still applies to both alternatives.
The second alternative also applies in ordinary career customization; it is
not limited to My Cars. That extra option has zero purchase price and does not
require a backroom marker when bought in the ordinary shop.
The reward flag is separate from the `19.8.69` event bit (`0x8000`).
The reference photo-finish screen sets the Burger King reward for event
`19.8.31`. That reward does not advance ordinary quick-race unlock levels.
These facts were checked against the original PC performance screen routine;
the photo-finish award trigger is reference-source evidence.

### Unlock progression data

On entering a career bin, its `unlocks` collection-key list (attribute
`0xbaf89280`) names gameplay upgrade awards. Each award has `UpgradePartID`
text (`0x3c2fdaab`) and an integer `UpgradeLevel`. Exact part texts map to the
table above: `brakes`, `chassis`, `engine`, `induction`, `nos`, `tires`,
`transmission`, `bodykit`, `hood`, `hud`, `numbers`, `paint`, `rims`,
`roofscoop`, `spoiler`, `tint`, `vinyls`.
The `decals` text maps level 1 to entity 44, level 2 to entity 46 and level 3
to entity 48. The level is retained after that mapping.
An award clamps its level to at least zero and writes the career level and
new-part level, plus the corresponding quick-race fields. Progression must
apply these installed awards rather than inventing rank thresholds.

## Native per-car customization record

The 408-byte record has 139 little-endian 16-bit installed part indices at
byte 0. `0xffff` denotes absent/unset. These values are part-database indices,
not hashes. The index is the local position in a pack plus the lengths of all
previously loaded packs. Preserve pack order when resolving modified installs.

| Byte offset | Field |
|---|---|
| `0x118` | Seven signed 32-bit performance stages: tires, brakes, chassis, transmission, engine, induction, NOS |
| `0x134` | Signed 32-bit Junkman bit mask in the same order |
| `0x138` | Three tuning profiles, each seven float32 values and 28 bytes |
| `0x18c` | Signed 32-bit active tuning profile |
| `0x190` | Preset identity |
| `0x194` | One-byte customization handle |

Tuning order is steering, handling, brakes, ride height, aerodynamics, NOS,
induction. Preserve all padding, unknown slots and untouched records.
Store extensions such as RGB colors and future mod-only parts separately;
do not reinterpret native indices as portable mod identities.

## Full parity acceptance boundary

Required garage coverage includes main-menu My Cars and career shop entry,
per-car records, all compatible body kits/spoilers/hoods/scoops/rims and sizes,
paint finishes/colors, rim paint, tint, vinyl groups/recolors, six decal zones
and authored positions, paired door numbers, custom HUD colors, all seven
performance categories, Junkman, three tuning profiles, cart preview/removal,
purchase/cancel, cash and unlock checks, backroom markers, heat changes and
save/reload. Controller navigation must cover every offered control.

Installed-data and synthetic checks do not establish original screen, camera,
animation, price, unlock, heat or rendering parity. Record remaining gaps
explicitly until original runtime comparisons cover them.
