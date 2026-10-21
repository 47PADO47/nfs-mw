# Quick Play

Quick Play is the first icon of the Quick Race menu ("Quick Play", `MODE_ICON_QUICK_SELECT`). It opens a brief
(`Quick_Race_Brief.fng`, title "quick play brief") with a random car and a random race, Randomize to pick again and Accept
to drive. The Quick Race menu's help text is label `0x7C3E459E`: "This special Quick Race game mode gives you a random car,
track and race type. The selection is based on your Blacklist position and includes all cars and tracks for the two upcoming
Blacklist positions."

- **Code:** `crates/nfsmw/src/frontend/quick_play.rs` (the screen), `quick_play/pool.rs` (the pool, the pick),
  `quick_play/text.rs` (what the screen writes), `quick_play/fill.rs` (the package's objects).
- **Where the exe was read:** [provenance/quick-play.md](../provenance/quick-play.md).
- **Evidence tags:** **[exe]** read in the exe. **[verified]** read from the install's files or seen on the screen.
  **[ours]** the rewrite's choice where the exe is unclear; marked in the code too.

## 1. Behaviour

1. Opening the screen picks a car and a race and shows them. The pick is repeated on every Randomize.
2. **Randomize** is the game message `0xC519BFC4`, which is `PAD_BUTTON5`: the key `1` presses pad buttons 1 and 5 (the pad
   mask of `Action::MenuButton1`), and the package also routes a release of its Randomize button to the same message
   **[verified]**. It picks another pair than the one shown (the same pair only when the pool has one).
3. **Accept** (`PAD_ACCEPT`) plays the package's leave script (`EVENT_HANDLER` / `FORWARD`), and when the script reports
   `EXIT_COMPLETE` the screen asks the front end to start the race: `Command::StartQuickPlay { race, car }`, the race's
   gameplay collection name and the car's folder under `CARS/`. The race runs with its own laps and opponents, not the
   Custom Race options (the brief has no options **[exe]**: the package's Difficulty, Catch Up and Traffic column is hidden).
4. **Back** (`PAD_BACK`) returns to the Quick Race menu (`MainMenu_Sub.fng`, `Sub::QuickRace`).
5. The screen names the car as the menu backdrop's look (`ScreenLogic::look`, `Look::Car`), so the 3D backdrop shows the
   picked car.
6. The footer shows Back (Esc), Accept (Enter) and Randomize (key 1), in that order **[verified]** (the reference screenshot).
7. The cars and the races are read in the background when the front end starts. Until both are there the screen has no pick
   and tries again every frame.

## 2. The pool

The pool is every car of the front-end catalogue and every race of the quick race board (`quick_race.rs`), limited by the
career's stage. The stage is the loaded profile's Blacklist stage (`Career::stage`, 16 for the prologue, falling to 1), and 16
when no profile is loaded.

| | In the pool when | Tag |
|---|---|---|
| a car | its unlock rank is at least `stage - 2` | **[ours]**, from the help text |
| a race | its Blacklist bin (the first number of its event id, `10.2.1` is bin 10) is at least `stage - 2` | **[ours]** |

The rule reads the help text: a car of rank `r` is available while `r >= stage` (`career-car-lot.md` section 1 **[exe]**), and
the text adds "the two upcoming Blacklist positions", the ranks `stage - 1` and `stage - 2`, whose cars are not the player's
yet. The exe's own selection was not found (provenance, dead ends), so the arithmetic is ours. A bin follows the same rule:
bin 15 opens at stage 15, as the rap sheet's ladder does. At the prologue (stage 16) the pool has the cars of rank 16, 15 and
14 and the races of bins 15 and 14.

A car the player lacks is still driven. The pool reaches past the stage so that the next cars can be tried, and the unlock
line (section 6) says how to get them **[ours]**.

**The cars.** A catalogue collection is a pool car when it is for sale, has a price above 0, is of one of the three car
classes (not the root "vehicles" class) and has a folder under `CARS/`. The price test leaves out the manufacturer and class
collections of the same file, which have the for-sale flag by inheritance **[verified]**. The folder is found from the
collection's name (`pool::folder_for`): the name itself, the name without underscores, the name up to its first underscore
(`carrera_gt` is `CARRERAGT`, `a4_32` is `A4`). **[ours]**: the exe takes the folder from the car's record, which the
catalogue does not link. In the installed (modified) catalogue 37 cars have a folder; `trueno`, `levin` and `corollae88`
have no physics type under that name and so no stat bars.

**The races.** The quick race board: the races the world runs, except the pursuit challenges. Only a race with a Blacklist bin
from 1 to 15 is in the pool. The prologue (bin 16), the collectors (21), the online events (99) and the challenge series'
tollbooths have no Blacklist position to measure and are never picked **[ours]**.

## 3. The pick

`pool::pick` takes the offered car indices and the offered race indices and draws one of each with a small xorshift64*
generator (`pool::Rng`), seeded from the clock when the screen opens (the seed is a parameter, so tests replay a sequence). A
redraw replaces a pair equal to the last one. There is no generator in the dependencies and none is needed.

## 4. The car

| What | Source | Tag |
|---|---|---|
| manufacturer index | the catalogue collection's field `0xBAF4AB4C` (layout offset `0x40`), a byte | **[exe]** |
| manufacturer names, index 1 to 25 | BMW, Ford, Subaru, Porsche, Audi, Mazda, General Motors, Dodge, Toyota, Mitsubishi, McLaren, Mercedes, Nissan, Lotus, Lamborghini, Renault, Lexus, Pontiac, Chevrolet, Vauxhall, Aston Martin, Volkswagen, Fiat, Cadillac, Corvette | **[exe]**, **[verified]** against 6 catalogue cars |
| manufacturer logo | texture `CARSELECT_MANUFACTURER_<NAME>` (the names with an underscore for General Motors: `GENERAL_MOTORS`); `GENERIC_LOGO_128` when the car has no manufacturer or the texture is missing | **[exe]** |
| model-name picture | texture `SECONDARY_LOGO_<collection name>` (the name as stored, `cobaltss`); `GENERIC_LOGO_256` when the car has no manufacturer or the texture is missing | **[exe]** |
| stat bars | `nfsmw_career::carlot::stats::bar_fractions` of the car among all pool cars, as the car lot shows them | **[ours]**, `career-car-lot.md` open question 1 |

This answers the manufacturer index order that `career-car-lot.md` left open (open question 1, last bullet): the order above
is the exe's switch, and the installed collections agree (`cobaltss` 19 Chevrolet, `a4_32` 5 Audi, `imprezzawrx` 3 Subaru,
`rx8speed` 6 Mazda). Index 26 and above, and 0, have no name; the Ferrari collection is among those.

The bars use the same three LED strips as `Car_Select.fng`, the same geometry (80 / 40 / 120 pixel first strip over a 160
pixel strip, 32 pixels per uv unit); only the GUIDs differ (section 6). The `LED_DELTA` ticks, which compare with another car,
are hidden.

## 5. The race lines

| Line | What it shows | Tag |
|---|---|---|
| mode icon | by race type: Sprint `0x2521E5EB` (`MODE_ICON_SPRINT`), Circuit `0xE9638D3E`, Drag `0xAAAB31E9`, Lap Knockout `0x3A015595`, Tollbooth `0x1A091045`, Speedtrap `0x66C9A7B6` | **[exe]**, the four named ones **[verified]** by hashing the names |
| mode text | the type's label: `Sprint` `0xB94FD70E`, `Circuit` `0x034FA2C1`, `Drag` `0x6F547E4C`, `Lap Knockout` `0x4930F5FC`, `Tollbooth` `0xA15E4505`, `Speedtrap` `0xEE1EDC76` | **[exe]** for the sibling event briefing; **[ours]** that this brief uses the same labels |
| track name | the label `TRACKNAME_<event id with dots as underscores>` (`TRACKNAME_10_2_1`); the event id when the table has no such label (38 of the 212 races of this install) | **[exe]** |
| `Length:` | the route length divided by 1000 with the unit word `KMs` (label `0x8569A26A`) in metric units, or by 1609.34 with `Miles` (`0x867DCFD9`) otherwise, printed `%.1f` and a space and the word | **[exe]** |
| `Laps:` | the race's `NumLaps`, at least 1 | **[ours]** |
| `Opponents:` | the number of opponents the race lists (`Opponents`), the number it starts with | **[ours]**, `gameplay-races.md` section 5 |

The length is the race's stored `RaceLength` (field `0x7C11C52E`, a float in metres). When it is 0 the length of the route
through the start, the checkpoints and the finish is used. The unit follows the Speed units setting (default metric): a
metric player sees `6.9 KMs` where an imperial one sees `4.3 Miles` for the same route.

## 6. The unlock line

A car the profile lacks (`rank < stage`) shows a sentence in the line above the footer; a car it has hides the line (the
group's own scripts `0x1CA7C0` show and `0x16A259` hide, as `Car_Select` runs them for a locked car).

| Car | Sentence | Tag |
|---|---|---|
| handle `0x2CF370F0` | label `0xBD8BAC94`, "Unlock this car by getting 100% game complete." | **[exe]** |
| handle `0x03A94520` | `0xBD8BAC93`, "... by winning the career mode." | **[exe]** |
| handle `0x2CF385B2` | `0xBD8BAC92`, "... by completing every event in the Challenge Series." | **[exe]** |
| handle `0xCB6AAF2F` | `0xBD8BAC91`, "... by completing event number 68 in the Challenge Series." | **[exe]** |
| handles `0x965F..0x9666` (members 2 to 9) and `0x13624E..0x136253` (10 to 15) | `0x4EF2A115`, "Unlock this car by beating %s, Blacklist member %d." with `%s` the alias `BLACKLIST_RIVAL_NN_AKA` and `%d` the member | **[exe]** |
| any other car with `rank < stage` | the same sentence for member `rank + 1` (rank 14 is opened by beating member 15); no line for a rank past member 15 | **[ours]** |

The handles are those of the stable's preset cars; a catalogue car's handle is the hash of its name, so only the last row
applies to the catalogue today. The alias label is the exe's lower-case key hashed upper-cased; `label_hash` is case
sensitive, so the table is asked for `BLACKLIST_RIVAL_15_AKA`.

## 7. The objects of the package

From `nfsmw dump-screen Quick_Race_Brief.fng`. Name hashes are what `Cx::object` takes.

| Object | GUID | Name hash | What the screen does |
|---|---|---|---|
| mode icon | `0xF5E9E` | `0x2521E5EB` | swaps the texture (section 5) |
| mode text group (text and shadow) | `0xF5E07` | `0xB5154998` | writes the mode text on both strings |
| track name | `0xE9728` | `0xB5154999` | text |
| length value | `0xE9729` | `0xB515499A` | text |
| laps value | `0xE972A` | `0xB515499B` | text |
| opponents value | `0xE972B` | `0xB515499C` | text |
| manufacturer logo | `0xEDDCD` | `0x3E01AD1D` | swaps the texture (section 4) |
| model-name picture | `0xEDDDD` | `0xB05DD708` | swaps the texture (section 4) |
| TOP SPEED bar | group `0xF5C77`, strips `0xF5C78`, `0xF5C79`, tick `0xF5C7A` | `0x4037F0E4` | strip widths and uv |
| ACCELERATION bar | group `0xF5C7E`, strips `0xF5C7F`, `0xF5C80`, tick `0xF5C81` | `0x46BCC4AA` | strip widths and uv |
| HANDLING bar | group `0xF5C85`, strips `0xF5C86`, `0xF5C87`, tick `0xF5C88` | `0xD108E905` | strip widths and uv |
| unlock line | group `0xEDCE4` (`0xFE8FDBF7`), string `0xF0F0D` (`0xA63265D0`) | | scripts show or hide; text |
| option column (hidden) | `0xEDEA1`, `0xEDEAD`, `0xEDEA4`, `0xEDEB3`, `0xE972C`, `0xE96F9`, `0xE96DF` | | hidden: Difficulty, Catch Up, Traffic, their values and the `Track:` caption |
| footer buttons | Randomize `0xFE5C0`, Accept `0xFE5BB`, Back `0xFE5B6` | | the package's own; the footer hints draw over them |

The package's responses: Accept `0x406415E3`, Back `0x911AB364`, Randomize `0xC519BFC4` (section 1).

## 8. Open

- The exe's own pool rule and its picker: the brief's controller was not found (provenance, dead ends). The pool rule is ours.
- Whether the exe lets the player drive a car that is not yet unlocked in Quick Play. The help text suggests it (the two
  upcoming positions are in the pool), so the rewrite does.
- The length unit of the reference screenshot (`6.9 Miles`): the exe's switch is the Speed units setting, which the rewrite
  follows. If the original prints the same number in miles, its stored length is not metres; not checked.
- Cars with no physics type under their front-end name (`trueno`, `levin`, `corollae88` here) show empty bars.
