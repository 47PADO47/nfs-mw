# Racer names: which opponents have a fixed name, and where it comes from

How the original names the computer-driven racers of a race: the names a career race fixes (the prologue's
opponents, the Blacklist rivals) and the drawn names of every other racer. The opponents' cars are in
[ai-racers.md](ai-racers.md) section 2; the drawn pool is in [../provenance/race-names.md](../provenance/race-names.md).
Evidence tags as in [docs/README.md](../README.md#evidence-tags). Sources: `speed.exe` v1.3 in Ghidra (read only,
[provenance](../provenance/race-names.md)) **[exe]**; `GLOBAL/gameplay.bin`, `GLOBAL/attributes.bin`,
`GLOBAL/GlobalB.lzc` and `LANGUAGES/English.bin` read with the repository's readers and a throwaway script **[verified]**.

## 0. Summary

1. When a racer is added to a race, the game picks its name in three steps, first match wins **[exe]**:
   (a) a **boss race** of a career names it `BLACKLIST_RIVAL_<bin>_LEADERBOARD` (section 2);
   (b) otherwise, in a career, the racer's character `RacerName` attribute, a language label (section 3);
   (c) otherwise a random `RACERNAME_000`..`RACERNAME_149`, never one another racer of the race holds.
   Steps (a) and (b) need the play mode "career" (`2`, section 1); quick race (`0`) and online (`1`) go straight to (c).
2. The prologue's opponents are named by (b): Razor, Ronnie, Bull and Rog are fixed; the two other racers of the last
   prologue circuit are drawn (section 4). The Blacklist rivals are named by (a) (section 5).
3. The cars are fixed by the character's `PresetRide` (section 4 and 5); nothing about the car is random for them.
4. The earlier rewrite drew **every** opponent's name from the pool (also in a career), and left `RacerName` unread
   (`docs/formats/gameplay-races.md` called it empty). That is the defect this page fixes (section 6).

## 1. When a name is picked **[exe]**

The routine that adds a racer to the race status (`GRaceStatus`, the Lua `AddRacer` binding) copies the driver in,
numbers the racer and then names it. The race status holds the play mode at `+0x1964` (0 quick race, 1 online, 2
career), the current race at `+0x1968` and the current bin at `+0x196C`
([formats/career-state.md](../formats/career-state.md)). Both fixed-name steps test `mode == 2` first, so they never apply
to a quick race or a custom race. The name is stored as a pointer to the text (the racer record, `+0x08`).

## 2. Step (a): a boss race names its racer after the bin **[exe]**

In a career, when the current race's node has `BossRace` set (attribute `0xFF5EE5D6`) and a current bin exists, the
racer's name is the language text of `BLACKLIST_RIVAL_<NN>_LEADERBOARD`, `NN` the current bin as two digits. The
text is the rival's alias in capitals (section 5). It does not look at the racer's character: every racer added to a
boss race gets it (the boss races have one opponent). The bin is the current bin (`+0x196C`, loaded from the stage
byte of the profile); a boss race is only reached from its own bin, so it is the race's bin
(`race_bin_<NN>/...`) **[unconfirmed]** for a replay of an earlier rival.

## 3. Step (b): the character's `RacerName` **[exe + verified]**

A character (class `gameplay`, collections under `character`) has a text attribute `RacerName`
(`0xBEAB64C5`, `vlt_hash("RacerName")`). When it is not empty, its text is hashed as a language label (`bStringHash`)
and the racer takes the label's text. An empty text, or a missing label, falls through to the random draw. Of the
314 characters, 5 set it, all of them prologue characters, and all five hold a `BLACKLIST_RIVAL_<NN>_LEADERBOARD`
label of the same family as step (a). The game's text for each is in the table of section 4.

## 4. The prologue races **[verified]**

None of the prologue races is a boss race (`BossRace` false), so step (b) decides. `Skill` is the character's
`SkillLevel`. The car is the preset ride's car type (`PresetRides` of `GlobalB.lzc`, [formats/cardata.md](../formats/cardata.md));
the preset also lists the parts, which the rewrite does not apply to a racer's car.

| Event | Race collection | Opponent | `RacerName` label | Name shown | Preset ride | Car type | Skill |
|---|---|---|---|---|---|---|---|
| 16.1.0 | `16_1_0_partial_dday` | `razor` | `BLACKLIST_RIVAL_01_LEADERBOARD` | RAZOR | `RAZORMUSTANG` | `MUSTANGGT` | 20 |
| 16.2.2 | `16_2_2_sprint` | `ronnie_mccrae` | `BLACKLIST_RIVAL_03_LEADERBOARD` | RONNIE | `DDAYSUPRA` | `SUPRA` | 0 |
| 16.2.3 | `16_2_3_sprint` | `toru_sato` | `BLACKLIST_RIVAL_02_LEADERBOARD` | BULL | `BL2` | `SLR` | 0 |
| 16.1.1 | `16_1_1_circuit` | `opponent` | `BLACKLIST_RIVAL_16_LEADERBOARD` | ROG | `OPM_MUSTANG_BOSS` | `GTO` | 0 |
| 16.1.1 | `16_1_1_circuit` | `opponent2` | none | drawn | `OPM_WRX_VERSION2` | `IMPREZAWRX` | 0 |
| 16.1.1 | `16_1_1_circuit` | `opponent3` | none | drawn | `OPM_RX8_Version3` | `RX8` | 0 |
| 16.2.1 | `16_2_1_d_day_intro` | `opponent` | `BLACKLIST_RIVAL_01_LEADERBOARD` | RAZOR | `RAZORMUSTANG` | `MUSTANGGT` | 20 |
| 16.8.1 | `16_nis_rog_razor` | `rog`, `razor` | none | not raced | `OPM_MUSTANG_BOSS`, `RAZORMUSTANG` | | 100 |

16.8.1 is a cutscene shell with no racing. The name shown is the label's text in `English.bin`.

## 5. The 15 Blacklist rivals **[verified]**

Every boss race (37 of them with an opponent, plus the final pursuit) has the rival alone as its opponent. The
rival's character holds the preset ride, no `RacerName` (step (a) names it). The leaderboard text is the alias
([career-text.md](career-text.md) section 2) in capitals, with a space kept for `BIG LOU`.

| Bin | Name shown | Boss collection | Preset ride | Car type | Skill | Boss races |
|---|---|---|---|---|---|---|
| 15 | SONNY | `tony_ho` | `BL15` | `GTI` | 0 | 2 |
| 14 | TAZ | `vince_kilic` | `BL14` | `IS300` | 15 | 2 |
| 13 | VIC | `victor_vasquez` | `BL13` | `SUPRA` | 20 | 2 |
| 12 | IZZY | `isabel_diaz` | `BL12` | `RX8` | 25 | 2 |
| 11 | BIG LOU | `lou_park` | `BL11` | `ECLIPSEGT` | 30 | 2 |
| 10 | BARON | `karl_smit` | `BL10` | `CAYMANS` | 40 | 2 |
| 9 | EARL | `pierre_dupont` | `BL9` | `LANCEREVO8` | 45 | 2 |
| 8 | JEWELS | `jade_barrett` | `BL8` | `MUSTANGGT` | 50 | 2 |
| 7 | KAZE | `kira_nazakato` | `BL7` | `CLK500` | 55 | 2 |
| 6 | MING | `hector_domingo` | `BL6` | `GALLARDO` | 60 | 2 |
| 5 | WEBSTER | `wes_allen` | `BL5` | `CORVETTE` | 65 | 3 |
| 4 | JV | `joe_vega` | `BL4` | `VIPER` | 75 | 3 |
| 3 | RONNIE | `ronnie_mccrea` | `BL3` | `DB9` | 80 | 3 |
| 2 | BULL | `toro_sato` | `BL2` | `SLR` | 80 | 3 |
| 1 | RAZOR | `razor_callahan` | `E3_DEMO_BMW` | `BMWM3GTRE46` | 80 | 5 |

The preset rides `BL2` to `BL15` are in the car tables; bin 1 has no `BL1`, its rival drives the E3 demo preset. The
boss races of a bin sometimes carry another bin's number in their name (`race_bin_11/12_1_2_r_circuit`); the folder
decides the bin. Bin 16 has no rival, its text (`BLACKLIST_RIVAL_16_LEADERBOARD`, `ROG`) is used only by the
prologue circuit of section 4.

## 6. What the rewrite does

- `Character.racer_name` reads `RacerName` (`nfsmw-data`, `gameplay/params.rs`).
- `race/setup.rs` (`opponents_of`) gives each opponent a `name_label`: for a career race that is a boss race, the
  leaderboard label of the race's bin (`names::bin_of` of the race folder, `names::fixed_label`); otherwise the
  character's `RacerName`; for a quick or custom race, none.
- `race/names.rs` (`name_opponents`, `name_racers`) turns a label into the language text; every racer without one, or
  whose label has no text, draws from the pool as before, avoiding the player's alias and the fixed names. The
  leaderboard of the HUD, the result cards and the results report read the names from the race plan, so they follow.
- The player keeps the profile's alias (the original lets a human racer keep its own name **[unconfirmed]**).
- The cars were already fixed by the preset ride (`CarTables::preset`); only the name was random.

Open questions: the exact meaning of the current bin for a replayed rival race (section 2); how the original keeps the
player's own alias in a boss race, where step (a) would name the player's racer too (not read) **[unconfirmed]**; the preset's
parts and paint for a racer's car (the rewrite shows the stock car of the type).
