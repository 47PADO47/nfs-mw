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
- Bonus presets behind settings flag bits ([career-save.md](../formats/career-save.md) 3.3): bit 14 unlocks `0x2CF370F0`
  and `0x03A94520` (the E3 demo BMW), bit 15 `0xCB6AAF2F`, bit 16 `0x2CF385B2`, bit 18 `0x34498EB2`.
- Other bonus presets (ten hashes, from the Black Edition set) are available only when the install holds the file
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

Needed for the rewrite: a car catalogue with price, unlock rank, class and for-sale flag per car (available from
`FE_ATTRIB.bin` through `blackbox-attrib`), the purchase/sale checks of sections 3 and 4, and the stage-based availability
of section 6. The limits (10 purchased cars, 25 records, 20 customs) can be configuration values.

## Open questions

1. The manufacturer index order and the use of the other `frontend` fields (the seven float stat bars, the part-info
   array).
2. Whether the car lot lists locked cars with a lock icon or hides them in the retail UI.
3. Where the per-car "tab" filter of the Car Select screen (six tabs) maps to class bits.
4. How the default stable decides the flag combinations seen in saves (`career-save.md` open question 3).
