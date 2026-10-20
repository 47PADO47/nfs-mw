# Paint cooling: how fitting visual parts lowers a career car's heat

What the original does with the `fecooling` collection of the attribute database, where it applies, and what the
screens show. The heat values themselves, their growth and the other factors (evade 0.95, race win 0.94, milestone
0.93, paid fine 1.0) are in [ai-pursuit-heat.md](ai-pursuit-heat.md) section 3 and [career-economy-flows.md](career-economy-flows.md)
section 4. The impound side of the car select is in [busted-aftermath.md](busted-aftermath.md).

- **Evidence tags:** **[verified]** read in the install data in this pass; **[decomp]** read in `speed.exe` (Ghidra, read
  only) or in the public CC0 reconstruction `dbalatoni13/nfsmw` (understanding only, nothing copied; the reconstruction
  is a decompilation of the same game, so it is tagged **[community]** where `speed.exe` was not re-read);
  **[unconfirmed]** a guess or a rule whose use is not proven.

## 1. Summary

1. `fecooling` is a class with one collection of eleven floats, all below 1: the factors for a new body kit, decal,
   hood, numbers, paint, rim, rim paint, roof scoop, spoiler, vinyl and window tint. **[verified]**
2. A factor is applied to the heat of the car's **career record** when a part of that kind is fitted in the
   customization garage of a career: `heat = heat * factor * extra`, where `extra` is 1.0 in the ordinary shop and 0.75
   in the backroom shop. **[community]**
3. It is **not** applied at the start of a chase, per second, or by the pursuit. The earlier plan of this repository
   (a per-car cooling read at the chase start) has no support in the data or the reconstruction; the only writers of
   the record heat are the ones in [career-safehouse-2.md](career-safehouse-2.md) section 8.3. **[community]**
4. The car select shows the heat as a meter: the multiplier icon `HEAT_X<n>` and ten LED segments. The customization
   screens show the same meter with a preview of the heat after the parts in the cart. **[community]**

## 2. The table **[verified]**

Read from the install (`GLOBAL/*` attribute database, class `fecooling`, collection `default`), 2026-10-20:

| Field | Factor | Slot in the customization (community) |
|---|---|---|
| `NewBodyKit` | 0.69 | body kit; also the `LEFT_DOOR_LNUMBER` decal slot (the reconstruction notes this as a likely bug) |
| `NewDecal` | 0.98 | the decal slots (windows, doors, quarters) |
| `NewHood` | 0.90 | hood |
| `NewNumbers` | 0.97 | numbers (no slot reaches it in the code read) |
| `NewPaint` | 0.70 | base paint |
| `NewRim` | 0.95 | front wheel |
| `NewRimPaint` | 0.97 | rim paint |
| `NewRoofScoop` | 0.97 | roof |
| `NewSpoiler` | 0.90 | spoiler |
| `NewVinyl` | 0.75 | vinyl layer 0 |
| `NewWindowTint` | 0.99 | window tint |

The installed test `the_installed_cooling_table_reads_eleven_factors` (`nfsmw-data`, `cooling.rs`) asserts these values.

## 3. When it applies **[community]**

`UpdateHeatOnVehicle(part, record)` runs for a part that is not a performance package, in career mode only. The slot of
the part picks the factor (table above). `extra` is 0.75 when the shop is the backroom, else 1.0. Performance packages
(engine, brakes...) never cool.

- **Preview.** The garage shows the heat the cart would leave: it copies the record heat to a scratch record and applies
  the factor of every active cart item whose slot differs from the part being looked at, plus the part itself when it
  differs from the installed one. The real record is untouched until the purchase.
- **Purchase.** The factor of each part in the cart multiplies the real record heat.
- Heat is never raised by a part.

**[unconfirmed]** Whether the purchase applies the factor once per cart item at the checkout or when the item is
fitted in the preview, and whether re-buying the stock part counts. The reconstruction ties the real record to the
checkout; this repository follows that.

## 4. The heat meter **[community]**

The meter is `CustomizeMeter` initialised with minimum 1.0, maximum 5.0. Both the current and the preview heat are
clamped to 1..=5, so a car with no heat reads as heat 1.

- **Multiplier icon.** `HEAT_X<n>` with `n` the whole part of the preview heat (1 to 5).
- **Segments.** Ten LEDs `HEAT_BASE_LED_1` to `_10`, each a tenth of a heat level above the preview's whole part.
  Segment `i` (0-based) is *off* when `bottom + 0.1 i + 0.0005 >= current`, *cool* when that is below the current heat
  but `>= preview`, and *on* otherwise. The scripts are `OFF`, `COOL` and `ON` (hashes `0xccfa`, `0x13ff6c`, `0x63c`,
  checked against the install package); `COOL` marks the segments a part about to be bought would remove. **[verified]**
  for the object and script names (package dump), **[community]** for the rule.
- The meter group is `0xf2492598` of `Car_Select.fng`; it is hidden for a car with no career record and in the car lot.

## 5. Rewrite

- **Table:** `nfsmw-data/src/cooling.rs` reads the eleven floats. The career crate holds a plain
  `economy::cooling::CoolingTable` (no install types) with `CareerRecord::cool_for_part`, `preview_heat` and the meter
  rule `HeatMeter`.
- **Where it applies:** the rewrite's garage fits a whole selection at once (`scenes/world/garage.rs`
  `commit_garage_selection`). In career mode the parts that differ from the applied selection (body kit, spoiler, hood,
  roof scoop, rims, and the paint colour) multiply the current car's heat (`scenes/world/career/cooling.rs`). The
  rewrite's garage offers no decals, vinyls, numbers, rim paint or window tint, so those factors are read but unused.
  There is no backroom shop and no price, so `extra` is always 1.0. A car swap in the garage is not a purchase.
- **Persistence:** the record heat is the one saved with the profile (`stable` career record `+0xC`); the cooled value
  is what the next chase starts from (`spawn_heat`, floor 1).
- **Shown:** the career car select draws the meter for the shown car (`frontend/career_screens/car_select.rs`). The
  customization garage of the rewrite is an overlay window with no heat display; the preview is not drawn there.

## 6. Open questions

- The `NewNumbers` and `NewRimPaint` slots exist in the table but the garage code read only reaches the others.
- Whether the backroom shop's 0.75 is applied on top of the table value as the reconstruction does (`factor * extra`).
- The meter's behaviour above heat 5 (clamped here, as the reconstruction clamps).
