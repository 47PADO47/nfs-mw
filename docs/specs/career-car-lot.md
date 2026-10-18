# Career car lot and Car Select: rules, prices, availability

What the original lets the player buy, sell and drive in a career: the price and unlock data of each car, the checks on a
purchase and on a sale, the garage limits, and which preset cars unlock when. The flow into the car lot is in
[career-flow.md](career-flow.md); the stable that stores the cars is in [career-save.md](../formats/career-save.md)
section 4.

- **Sources:** `speed.exe` (Black Edition PC v1.3) through the Ghidra MCP and disassembly; `GLOBAL/FE_ATTRIB.bin` and its
  untouched 2005 backup `GLOBAL/fe_attrib.bak` (class `frontend`), read with the repo's `blackbox-attrib` reader; the
  language table. Addresses are in the [provenance file](../provenance/career-flow.md).
- **Evidence tags:** **[exe]** read in the exe, not tested in game. **[verified]** read from the install files.
  **[unconfirmed]** a guess.
- **The installed `FE_ATTRIB.bin` is modified** (a mod tool rewrote it: 446 `frontend` collections against 411 in the backup,
  and extra cars). The table in section 2 is from the **backup**, which is the 2005 data. The added cars are listed in 2.2.

## 1. Where the data comes from **[exe + verified]**

Each stable car record (24 bytes of data, `career-save.md` 4.1) points at a `frontend` collection (record +4). The fields of
that collection the code reads, by their offset in the collection's layout:

| Layout offset | Type | Use |
|---|---|---|
| 0x38 | u32 | class: 0 North America, 1 Europe, 2 Japan, 3 the root "vehicles" class (traffic, cops, specials) |
| 0x4C | i32 | **price** in dollars |
| 0x58 | bool | **for sale** |
| 0x59 | u8 | **unlock rank**: the car is available while the player's stage is at most this value |
| 0x40 | u32 | manufacturer index (1 BMW, 2 Ford, 3 Subaru, 4 Porsche, 5 Audi, 6 Mazda, 7 GM, 8 Dodge, 9 Toyota, 10 Mitsubishi, 11 Mercedes, 12 ... ) **[unconfirmed: order]** |

(The field keys are hashes: price `0xE61673E9`, unlock rank `0x60F99F57`, for sale `0xFBCF1A48`, class `0x0DE009BE`.)

When a car record is created, its flag word is built from this: the class sets the high half
(0x10000/0x20000/0x40000/0x80000); the low half gets 0x20 ("not for sale") when the class is 3 or the for-sale field is
false, else 0x1 ("in the car lot") **[exe]**.

**Stage** is the settings byte of [career-save.md](../formats/career-save.md) 3.2: 16 in the prologue, then the rank of the
rival being challenged, falling to 1. A car with unlock rank `r` is **available when `r >= stage`**. So a rank 16 car is
available from the start; a rank 14 car after the player has beaten rival 15 (stage 14); a rank 2 car when only rivals 2 and
1 remain **[exe]**. A locked car is still listed (greyed) in the career car lot **[exe]**.

## 2. The 2005 catalogue **[verified]**

Cars with a price and the for-sale flag set in `fe_attrib.bak`: 32 cars. `unlock` = unlock rank, `class` as above.

| Car (collection) | Price | Unlock | Class |
|---|---:|---:|---|
| cobaltss | 26,000 | 16 | NA |
| is300 | 27,000 | 16 | JP |
| punto | 27,000 | 16 | EU |
| gti | 35,000 | 16 | EU |
| a3_20t | 32,000 | 14 | EU |
| tt | 35,000 | 14 | EU |
| eclipsegt | 30,000 | 13 | JP |
| a4_32 | 35,000 | 13 | EU |
| supra | 40,000 | 12 | JP |
| clio | 40,000 | 12 | EU |
| rx8 | 32,000 | 11 | JP |
| cts | 32,000 | 11 | NA |
| mustanggt | 36,000 | 11 | NA |
| lancerevo8 | 36,000 | 10 | JP |
| sl500 | 75,000 | 10 | EU |
| gto | 35,000 | 9 | NA |
| monaro | 35,000 | 9 | EU |
| caymans | 60,000 | 9 | EU |
| rx7 | 31,000 | 8 | JP |
| imprezzawrx | 42,000 | 8 | JP |
| elise | 48,000 | 7 | EU |
| clk500 | 75,000 | 7 | EU |
| 997s | 75,000 | 6 | EU |
| db9 | 90,000 | 6 | EU |
| viper | 98,000 | 5 | NA |
| gallardo | 120,000 | 5 | EU |
| corvette_car | 80,000 | 4 | NA |
| 911turbo | 105,000 | 4 | EU |
| murcielago | 265,000 | 3 | EU |
| fordgt | 270,000 | 3 | NA |
| carrera_gt | 280,000 | 2 | EU |
| slr | 300,000 | 2 | EU |

Two more priced entries are not for sale: `m3_gtre46` (the starting BMW M3 GTR, price field 500,000, unlock 15) and `camaro`
(price field 5, a placeholder). The unlock ranks run from 16 down to 2; nothing unlocks at rank 1.

### 2.1 Reading it

- 4 cars are available at the very start (rank 16), all between 26,000 and 35,000.
- Prices rise from about 30,000 at ranks 14 to 9 to 265,000 to 300,000 at ranks 3 and 2. The first purchases come from the
  prologue and first-race cash; the extra-options mod notes 30,000 from the first three races **[community]**.
- Prices shown by the UI come from the same field (accessor `0x00581730`).

### 2.2 Cars added by this install's data **[verified]**

Not in the 2005 data, priced in the installed file: `skylinezt` 150,000 (rank 5), `trueno` and `levin` and `corollae88`
15,000 (rank 16), `truenoid` 45,000, `fxxevo` 450,000 (rank 3), `sf90` 5,000,000 (rank 2). They are add-on cars and show that
the car lot is entirely data driven: a new car needs a `frontend` collection and a vehicle collection and nothing else.

## 3. Rules of a purchase **[exe + verified labels]**

The Car Select screen in the car lot (context bits 0 and 15) handles the accept button. In order:

1. **Price check**: price greater than cash (settings +0xC) -> dialog "Not enough cash to buy this car!"; no purchase.
2. **Garage limit**: the number of owned career cars (records with the owned flag, a class bit and not marked 0x40) is
   10 or more -> "There's no more room in your Safe House. Sell a previously bought car to make space for a new one."
   The limit 10 is a literal in the screen code. The stable itself refuses at 25 (`career-save.md` 4).
3. **Stage 16** (prologue): a plain "Are you sure you wish to buy this car?" confirmation.
   **Stage 15 or lower**: a three-button dialog "Do you want to buy this car and drive it right away or have it delivered to
   your Safe House?" with **Drive Now**, **Deliver** and **Cancel**.
4. On **Deliver**: the lot record is copied into a free stable slot with the owned flags, a free customization record and a
   free career record (index stored at +0x11); the price is taken from cash (cash cannot go below 0: the subtraction
   saturates). The current car is **not** changed.
5. On **Drive Now**: the same, and the new record's handle becomes the settings' current car.

The copy keeps the car's class bits and builds the part list from the preset's default parts. A fresh career record has
impound strike limit 3 and everything else 0 (`career-save.md` 4.3).

## 4. Rules of a sale **[exe + verified labels]**

In the **safe house** Car Select (context bit 0, no bit 15) and only when stage is 15 or lower:

1. The selected car must have a career record; else nothing happens.
2. If the car's impound state is not 0: "This car is impounded, you cannot sell it."
3. If the player has one or fewer cars not in the impound state: "You cannot sell the only car left in your safe house."
4. Otherwise a confirmation shows the sale price: **half of the car's price field**. On yes, cash gains that half
   (integer shift right by one), the car is removed from the stable (the routine at `0x00590D20`) and the list is rebuilt.
   What happens to the current car when the sold car was the current one is **[unconfirmed]**: the screen re-reads the
   current car after the sale.

The sale price ignores any parts bought for the car and the car's condition. The price field of the starting M3 GTR is
500,000, so it would sell for 250,000 **[exe + verified data]**.

## 5. Cars chosen in the safe house

The safe house Car Select lists the owned career cars; the accept button sets the current car handle in the settings. A car
in the impound state is shown but may not be selectable **[unconfirmed]**.

## 6. Preset cars and when they unlock **[exe + verified]**

The "preset ride" records (flag 0x8) are the Blacklist rivals' cars and some bonus cars. They are in the stable from the
start and appear in the quick-race lists once available. Availability uses the **stage** byte:

- The rival cars have handles that are the name hashes of the presets **`BL2` to `BL15`** (the Blacklist ranks). Rival `n`'s
  car is available when the stage is **below `n`**, i.e. after rival `n` has been beaten (`BL15` once stage <= 14, `BL2` once
  stage <= 1). Rival 1's car is not in this list.
- Bonus presets behind settings flag bits ([career-save.md](../formats/career-save.md) 3.3): bit 14 unlocks `BONUS_C6R`
  (`0x2CF370F0`) and `E3_DEMO_BMW` (`0x03A94520`), bit 15 `BONUS_SL65` (`0xCB6AAF2F`), bit 16 `BONUS_GT2` (`0x2CF385B2`),
  bit 18 an unnamed preset (`0x34498EB2`). The locked-car messages say how: event 68 of the Challenge Series (SL65), every
  Challenge Series event (GT2), winning the career (E3 BMW), 100 percent game complete (C6R)
  ([career-text.md](career-text.md) 5.3).
- Other bonus presets (ten hashes, the Collector's Edition presets `CE_ELISE`, `CE_SL500`, `CE_SUPRA`, `CE_CAMARO`,
  `CE_CORVETTE`, `CE_997S`, `CE_SL65`, `CE_GT2`, `CE_C6R` and one unresolved) are available only when the install holds the file
  `GLOBAL/INGAMEC.BUN` (a probe at start-up).
- If the profile's "career completed at least once" byte is set, or the debug flag is set, everything is available in the
  non-career modes.
- Ordinary cars (the table in section 2) use the unlock-rank rule of section 1 in the **career** lot; in quick race the same
  car uses the stage rule and the "completed" byte, and cars in a short list of vehicle-type ids are always available
  **[exe, list not interpreted]**.

## 7. My Cars (quick race customs)

The `My Cars` garage lists the records with the quick-race flag 0x4 only. A player can keep **20** of them (a literal in
the screen code); career cars are not editable there. The count of such cars is a helper of the stable **[exe]**.

## 8. Rewrite

Built in `crates/nfsmw-career::carlot`: the catalogue and stage-based availability (`catalogue`, sections 1, 2 and 6), the
purchase and sale checks (`purchase`, `sale`, sections 3 and 4), and the garage limits as `Limits::SPEC`. The car
catalogue is read by `nfsmw-data::career_cars`. The choices and gaps are in
[provenance/career-core.md](../provenance/career-core.md). The purchase and sale flows that call these checks are not
wired yet. The limits (10 purchased cars, 25 records, 20 customs) are values in `Limits`, not screen literals.

## Open questions

1. The manufacturer index order and the part-info array of the `frontend` record. The stat bars are answered in part (pass 5, 2026-10-18); the lit-segment rule is still open.
   - [exe][verified] Three bar groups in `Car_Select.fng`: TOP SPEED = group `0xF1737`, ACCELERATION = `0xF172F`, HANDLING = `0xF173F`. Each is a Multi-Textured Image of `LED_main.tga` strips (160 px, uv 0 to 5.0) plus one `LED_DELTA` tick. The shorter first strips (80 / 40 / 120 px) are fixed in the .fng.
   - [exe][verified] Update messages: `0x26C9925F` (TOP SPEED, script `0x5079C8F8`), `0x0B9A1B40` (ACCELERATION), `0x06FE3A05` (HANDLING). Screen start `0x97BDB465` plays `0x033113AC` on all three. Labels: `0x89B304` "TOPSPEED", `0x89B310` "ACCELERATION", `0x89B2F8` "HANDLING".
   - [exe] Two different bar classes exist. **Correction to pass 4:** the 10-segment builder `0x007AFF10` (`HEAT_BASE_LED_%d`, lo 1.0, hi 5.0) is called only for `HEAT_METER` objects (from `0x007B0C30`, `0x007B12B0`, `0x007BB3E0`, and `0x007B4D70` at +0x88). The car-lot bars are not built by it. The `[1, 5]` range in the spec does not apply to them.
   - [exe] Car-lot bars are `PREVIEWBAR` objects: vtable `0x0089B098`, child names `PREVIEWBAR_%s` (`0x0057F440`). Their method at vtable slot +0x28 (`0x0056CE30`) forwards to slots +0x0C and +0x2C. The refresh `0x007B8F10` runs from `0x007BE3F0`, whose object (vtable `0x008B7F24`) holds the bars at +0x1D4, +0x218 and +0x25C.
   - [exe] Bar object to label (see 2.1 below for the axis): `0x007B4D70` sets +0xD8 = ACCELERATION, +0x11C = TOPSPEED, +0x160 = HANDLING, each with width 160.0. In the `0x007B8F10` refresh, +0x25C is labelled TOPSPEED. +0x1D4 and +0x218 are ACCELERATION and HANDLING in an order not yet decoded. Which screen owns the `0x007B4D70` object is [unconfirmed].
   - [unresolved] The lit-segment rule: the code that turns a value into lit LED strips is not in the exe. The value messages are FEng-table driven. The next step is the `Car_Select.fng` script dump (`nfsmw dump-screen`) for the `LED_main` uv scale.
   - [unresolved] The name of flag hash `0xCDC136E8` (seven-bit mask read by `0x0067C570`) and of the part hash set in `0x00678CE0` (`0xF1F5FBC7`, `0xC92A0142`, `0x07A7A3E5`, `0xAFA210F0`, `0xBD38D1CA`, `0x36350867`, `0xB1669F64`, `0xAACBE2E7`).
   - [exe] Manufacturer index order: not touched in pass 5; still open.

   **Pass 5 (2026-10-18): per-car stat source and blend.**
   - [exe] Two getters `0x00679660` (lo) and `0x006796C0` (hi). Each takes the car and an out vec3 (x, y, z). Both find the car's key (`0x00452430`; alt key `0x004546A0` when flag bit 0 is set at +0x10), then walk the global list at `0x0092C4E8`. A node with key at +0x2C gives lo = +0x14, +0x18, +0x1C and hi = +0x20, +0x24, +0x28. No node: return false. So the per-car base term **is** in the list; it applies to all three axes, not only x.
   - [exe] The list is built by `0x00683EB0` over every `pvehicle` record (class `0x4A97EC8F`). Per car, `0x00678CE0` simulates the car from its parts and returns three floats: time to 44.703 m/s (100 mph), a top-speed value, and a handling value (a lerp of attribute range `0xAACBE2E7` [0..1] by the y-weight column). Population min and max normalize each to [0, 1]. Mapping into the node: lo.x = norm(top speed), lo.y = norm(handling), lo.z = 1 - norm(time to 100 mph). So a stock car gets a non-zero base unless it is the population minimum.
   - [exe] hi is taken from a second pass over the cars that pass `0x0067F950`; that routine writes each category's max (`0x00672E80`) into the car through `0x0067F800`, then re-simulates. Interpretation: hi = all categories at max, lo = as-is. The `0x0067F800` side effect is not decoded.
   - [exe] Blend `0x0067C570(car, out)`. Per axis j in x, y, z: `raw_j = sum_i w_ji p_i / sum_i w_ji` over seven categories, with `p_i = installed_i / max_i` (`0x00677F80`), and `w` the rows at `0x008F8928 + 12 i`. Mask `m_j = sum_{i in M} w_ji / sum_i w_ji`, `M` = bits of hash `0xCDC136E8`. Then `hi'_j = hi_j + (1 - hi_j) m_j`, `lo'_j = lo_j + (hi'_j - lo_j) 0.33 m_j`, `out_j = lo'_j + (hi'_j - lo'_j) min(1, (1 + 0.33 m_j) raw_j)`. Skipped sums under 1e-6 are not used. Output in [0, 1].
   - [exe] Consequence: a car with no upgrades (all `p_i = 0`) shows `lo'`, not zero. The bar is never empty for a stock car. The `[1, 5]` mapping is not in this path.
   - [exe] Getter `0x007AF220(this, mode)` returns out.x (mode 0), out.y (1), out.z (2), and 0.0 otherwise. Its car comes from `*(this + 4)` (the selected car handle). So mode 0 is x (TOP SPEED), 1 is y (HANDLING), 2 is z (ACCELERATION).
   - [exe] Axis map from the `0x00678CE0` outputs: x = top speed, y = handling, z = 1 - time to 100 mph. Together with the `0x007AF220` modes: TOPSPEED = x, HANDLING = y, ACCELERATION = z.
   - [exe][unresolved] `0x007B8F10` builds two blends per bar: one for the selected car (`DAT_009B9EC4`) and one for the player's car (`DAT_009B9EC8`). Which one drives the fill and which the `LED_DELTA` tick is not decided from the decompile (the float arguments are lost). The per-bar value passes to FEng through the PREVIEWBAR method, so the arguments need the disassembly.
   - [exe] Category index (exe index to category). The switch in `0x007B8F10` maps the upgrade id in `[this + 0x148]` (`0x201` to `0x207`) to index: `0x205` = 0 (tires), `0x206` = 1 (brakes), `0x203` = 2 (chassis), `0x202` = 3 (transmission), `0x201` = 4 (engine), `0x207` = 5 (induction), `0x204` = 6 (nitrous). Index 5 = induction is [exe]: `0x007AFAE0` tests category 5 on the car, and only id `0x207` uses that test. Names for the other ids follow the repo order and are [unconfirmed]. The category strings are in the exe at `0x008A0D08` to `0x008A0D40`.
   - **Status: UNRESOLVED (partial).** Answered: the per-car lo and hi source, the blend, the axis map, the category index. Open: the lit rule (FEng side), the fill-versus-delta choice, why lo and hi would differ (see the `0x0067F800` side effect), the hash `0xCDC136E8`.

2. Whether the car lot lists locked cars with a lock icon or hides them in the retail UI.
3. Where the per-car "tab" filter of the Car Select screen (six tabs) maps to class bits.
4. How the default stable decides the flag combinations seen in saves (`career-save.md` open question 3).
