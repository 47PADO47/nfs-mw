# Quick Play brief

- **Spec:** [specs/quick-play.md](../specs/quick-play.md). The earlier record of the screen's objects and the unlock sentences is in
  [career-parity-menus.md](career-parity-menus.md) (sections "The Quick Play brief" and "Unlock sentences").
- **Method:** Ghidra on `/speed.exe` (project `nfsmw`), 2026-10-20, plus `nfsmw dump-screen Quick_Race_Brief.fng` and a dump of
  the installed `GLOBAL/FE_ATTRIB.bin` collections. Function names are Ghidra's (`FUN_` plus the address).
- **Implemented:** `crates/nfsmw/src/frontend/quick_play.rs` and `quick_play/`.

## What each address gave

| Address | Function | What it gave |
|---|---|---|
| `0x00581790` | `FUN_00581790` | The manufacturer name of a front-end car: it reads the byte at offset `0x40` of the car's `frontend` collection and returns the name for 1 BMW, 2 FORD, 3 SUBARU, 4 PORSCHE, 5 AUDI, 6 MAZDA, 7 GENERAL_MOTORS, 8 DODGE, 9 TOYOTA, 10 MITSUBISHI, 11 MCLAREN, 12 MERCEDES, 13 NISSAN, 14 LOTUS, 15 LAMBORGHINI, 16 RENAULT, 17 LEXUS, 18 PONTIAC, 19 CHEVROLET, 20 VAUXHALL, 21 ASTONMARTIN, 22 VOLKSWAGEN, 23 FIAT, 24 CADILLAC, 25 CORVETTE (anything else: the empty string). The install's collections agree: field `0xBAF4AB4C` is at layout offset `0x40`, `cobaltss` holds 19, `a4_32` 5, `imprezzawrx` 3, `rx8speed` 6. This is the manufacturer order `career-car-lot.md` left open. |
| `0x00591330` | `FUN_00591330` | The manufacturer logo: `CARSELECT_MANUFACTURER_%s` with that name, hashed (`FUN_005af1c0`), kept when the texture exists (`FUN_00503400`), else `GENERIC_LOGO_128`. An empty manufacturer name goes to the generic logo at once. |
| `0x00591240` | `FUN_00591240` | The model-name picture: `SECONDARY_LOGO_%s` with the collection's name (the string field at offset `0x18`, field `0x9CA1C8F9`, `cobaltss`), else `GENERIC_LOGO_256`; also generic when the manufacturer name is empty. |
| `0x005AF1C0` | `FUN_005af1c0` | The label and texture hash: start -1, `h = h * 33 + upper(c)`. It upper-cases, so the rewrite's case-sensitive `label_hash` needs the upper-case label key (`BLACKLIST_RIVAL_15_AKA`, not the exe's lower-case format string). |
| `0x007A4680`, `0x007C2BA0`, `0x007B9930` | car card setups | Set the two logo images (names `0x3E01AD1D` manufacturer, `0xB05DD708` model) with `FUN_00515B90(image, texture hash)`, the texture swap used here. |
| `0x007AADE0` | `FUN_007aade0` | The event briefing's fill (a sibling screen with the same object names): the length is a float of the race record (`FUN_005FB850`, offset `0x1C`, else the attribute `0x7C11C52E`, `RaceLength`), printed with the format `%$0.1f %s`; the unit word and the divisor are metric `0x8569A26A` "KMs" and 1000.0 when the options byte at `+0x3B` of the player options is 1, else `0x867DCFD9` "Miles" and 1609.34. The track name is the label `TRACKNAME_<id>` (`FUN_0056E560`: the event id, dots as underscores) when it exists, else the id text. The race kind is read from the `EventIconType` text against a table of 11 names (`FUN_005FAA20`, table at `0x008F5CD8`) giving 0 Sprint, 1 Circuit, 2 Drag, 3 Knockout, 4 Tollbooth, 5 Speedtrap. |
| `0x0056E090` | `FUN_0056e090` | The race kind's icon texture: 0 `0x2521E5EB` (`MODE_ICON_SPRINT`), 1 `0xE9638D3E`, 2 `0xAAAB31E9`, 3 `0x3A015595`, 4 and 10 `0x1A091045`, 5 and 9 `0x66C9A7B6`. The four named icons hash to the names in the task (`MODE_ICON_CIRCUIT`, `_DRAG`, `_LAP_KNOCKOUT`). |
| `0x0056E010` | `FUN_0056e010` | The race kind's name label: 0 `0xB94FD70E` "Sprint", 1 `0x034FA2C1` "Circuit", 2 `0x6F547E4C` "Drag", 3 `0x4930F5FC` "Lap Knockout", 4 `0xA15E4505` "Tollbooth", 5 `0xEE1EDC76` "Speedtrap". The Quick Race menu builder (`0x007AA900`) pairs the same icon and label for its mode icons. |
| `0x007A3C00` | `FUN_007a3c00` | The unlock sentence of a stable car by its handle: `0x2CF370F0` 100% complete, `0x03A94520` career won, `0x2CF385B2` challenge series, `0xCB6AAF2F` event 68, and the rival handles `0x13624E..0x136253` and `0x965F..0x9666` the sentence `0x4EF2A115`. |
| `0x007A3C70` | `FUN_007a3c70` | The Blacklist member of a rival handle: `0x965F..0x9666` are members 2 to 9, `0x13624E` is 10, `0x13624F..0x136253` are 11 to 15; any other handle -1. |
| `0x007ADA50` | `FUN_007ada50` | The car lot's (`Car_Select.fng`) card fill: it shows the unlock text object `0x2D25B2C4` with the sentence for a locked stable car, running the show script `0x1CA7C0` (and `0x16A259` to hide) on the object `0xD0F7C7CC`. The brief's unlock group `0xEDCE4` has the same two scripts. |
| `0x007A2EFD` | (inside an undefined function) | The only code that names `Quick_Race_Brief.fng` (a push of the screen name, then a jump to the screen push routine `0x00525940`). It sits next to the title setter `FUN_007a2de0`. The brief's own class was not found from here. |

## Rules the rewrite took without an exe source

- **The pool rule** (`rank >= stage - 2`, bins likewise): from the help text `0x7C3E459E` only. The text says the selection
  is "based on your Blacklist position" and "includes all cars and tracks for the two upcoming Blacklist positions". Cars of the
  catalogue are available while `rank >= stage` (`career-car-lot.md`, **[exe]**), so the two upcoming positions are the ranks
  `stage - 1` and `stage - 2`. **[ours]**.
- **Races outside the Blacklist bins** are never picked **[ours]**.
- **The unlock sentence for a catalogue car** is member `rank + 1`: a rank-`r` car comes with the stage `r`, reached by beating
  member `r + 1`. The exe's sentences are for the stable's preset cars **[ours]**.
- **The folder of a front-end car** from its collection name **[ours]**.

## Dead ends

- **The brief's controller (`UIQRBrief`).** The class name string `0x008A0834` is referenced only from an unwind record
  (`0x00871722`), so the constructor, the vtable and the pick code were not found by name. The Randomize message `0xC519BFC4`
  is `PAD_BUTTON5`; its compares at `0x007AB7D4` (the track select's handler), `0x007BB390` and `0x007BFEE9` (the car select's)
  belong to other screens. No caller of a random function was followed. The object names the brief shares with the event
  briefing (`0xB5154998`..`0xB515499C`) mean different things on the two screens (the briefing's `0xB5154998` is the
  difficulty word), so only the package's own dump was used to name them.
- **The pool rule itself** (above): not found.
- **Which cars have no physics type.** `trueno`, `levin` and `corollae88` of this install's catalogue have no `pvehicle`
  collection of that name; not investigated.
