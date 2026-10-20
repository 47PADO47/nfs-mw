# UI themes: the Remastered screens and HUD pieces

What the Remastered look draws beyond the vertical icon menus ([ui-themes-remastered-menus.md](ui-themes-remastered-menus.md)):
the dialogs, the key list, the EA Trax playlist, the credits, the Blacklist and a rival's bio, and the free-roam HUD pieces
(the minimap with its heat ring, the radio card, the pursuit timer). Companion to [ui-themes.md](ui-themes.md) section 10.

- **Sources read:** this repository only (`crates/nfsmw/src/frontend/`, `crates/nfsmw/src/ui/theme/remastered/`,
  `crates/nfsmw/src/hud/`, `libs/blackbox-ui`) and reference pictures supplied by the project owner (a minimap with a heat
  ring and a Blacklist screen). No game data beyond the install's own textures, and no decompiled code.
- **Status:** built. Checked with offscreen screenshots (`nfsmw play --drive --ui-style remastered --screenshot out.png`,
  `nfsmw view-screen <screen>.fng --ui-style remastered --screenshot out.png`).

## 1. The screens, as menu lists

Every screen below publishes a `MenuList` through `ScreenLogic::list` and is drawn by `VerticalMenu` (the data is the same
cell, [ui-themes-remastered-menus.md](ui-themes-remastered-menus.md) section 3). `MenuStyle` says which page it is.

| Screen (slot) | Style | What is drawn |
|---|---|---|
| Question and notice dialogs (`Dialog`) | `Dialog` | A dark panel over a scrim, the title in the accent colour, the message wrapped under it, the answers as slanted bars. The geometry (`menu/dialog.rs`) is shared with the hit test. |
| Key list and remapping (`ControlsList`) | `Sub` with `MenuValue::Keys` | A row per action with two key caps, one per column; an outline in the accent colour marks the column that has the focus. A column waiting for a key reads as the prompt. |
| EA Trax playlist (`Jukebox`) | `Sub` with `MenuValue::Choice` | The songs as rows with their mode; the order of play is the line beside the title. |
| Credits (`Credits`) | `Credits` | The roll in the middle of the screen, scrolling up, fading at both ends; a line of capitals is a heading. |
| Blacklist (`Blacklist`, `SafeHouseRivalChallenge.fng`) | `Blacklist` | See section 2. |
| Rival bio (`Blacklist`, `SafeHouseRivalBio.fng`) | `Rival` | See section 2. |

## 2. The Blacklist and a rival's bio

`ChallengeLogic` and `BioLogic` (`frontend/career_screens/blacklist.rs`) publish one `MenuEntry` per rival with a
`MenuValue::Rival(RivalInfo)`: the rank, the portrait's texture key (`RIVAL_NN` from `GLOBAL/RIVALS.BIN`, [career-regions.md](career-regions.md)
section 8), the alias, car and strength from the game's text table, the bounty and the status.

- **Rows.** Fifteen rows, rival 15 at the top, each with rank, picture, name, bounty and status. The selected row is on the
  menu's slanted bar; a rival not reached yet is dimmed, with a dark picture. Status is `Complete` (every boss race won),
  `InProgress` (the rival the career is at, or one with its gate open) or `Locked` (`status_of`). The count of rivals beaten
  ("5/15") is at the right of the title.
- **Card.** The selected rival's picture, name, alias, bounty and status at the right of the screen. A window narrower than
  about 4:3 has no room for it (`card_area`) and shows the rows only.
- **Input.** Up and Down walk the rivals (the logic asks for `rotate_pad`, so they arrive as Left and Right, the stepper the
  package has), the pointer selects a row, and Accept or a click opens the bio of the selected rival (`Args::rival`). Back
  leaves for the safe house menu. The boss race is not built, so the screen has no Challenge action; under Original the
  challenge button still answers with its alert.
- **Bio page.** A large picture with a frame, the name, alias, car and strength, and the bio wrapped to the room left
  (`dialog::wrap_text`). Back and Accept leave, as under Original.
- **Hints.** `prompts::hint_set`: Select, View Rival and Back on the list; Back on the bio.

## 3. The minimap

`hud/roots.rs` now splits the minimap's package pieces, and the `Minimap` slot claims the package's heat group and the two heat
arcs that ring the map (`0xC46A80A9`, `0xB0F63386`), so a theme that draws its own minimap hides them (Original draws them
as before). The Remastered `Minimap` (`ui/theme/remastered/minimap/`) reads the HUD tree from the frame cell:

- the window is where the backing disc is (the map group scrolls under it as the car moves), and the direction of north is the
  turn of the map group (`Window::of`);
- the game's own map pieces, multiplied by a dark tint (`MINIMAP_MAP`), over a dark disc; the player's arrow in the accent colour
  (`Tinted::recoloured`); the blips stay the game's own and draw over it (the Blips slot is Original);
- a dark bezel with a gold hairline just inside it; the north marker, a small dark badge with an N on the bezel at the
  direction of north;
- the heat ring: ten segments outside the bezel and touching it, as many lit (accent colour) as whole heat levels reached (`lit_segments`); a
  flame (the package's own `Heat_Icon`) and the multiplier "x5" at the top left (`multiplier`, 1 to 10);

The minimap has one ring, for the heat. The chase's own state (busted, escaping, cool-down) is the pursuit panel's (section 4).
The ring is the generic `blackbox_ui::Ring` (an arc as a row of short quads); `DriveFeed::heat` carries the car's heat.

## 4. The radio card and the pursuit timer

- **Radio card** (`remastered/radio.rs`). The song on the air on a dark plate that fades to clear at its right, a gold bar down its
  left edge, the title in bold capitals, the artist and album, and the time played in the accent colour. It sits where the HUD puts
  the card (`custom::radio_placement`) and has the Custom card's timing (up for six seconds, fading over the last). A string wider
  than the plate is cut with "..." (`fitted`), because the renderer does not clip text.
- **Pursuit panel** (`remastered/pursuit.rs`, the `PursuitMeter` slot): a right trapezoid (the generic `blackbox_ui::Trapezoid`,
  its left edge slanting in towards the top) at the top right with the chase's time, the minutes and seconds in the accent colour.
  Under it the cop cars the chase has **called** (`HudState::cops_engaged`), how many are **damaged** and how many are
  **wrecked** (`PursuitHud::cops_destroyed`). Then the bars of the chase's end, shown only while they apply: the signed meter
  (`PursuitHud::busted`: above zero the red **BUSTED** bar, below zero the accent **ESCAPING** bar, hidden at rest) and the
  **COOLDOWN** bar once the player has got away (`cooldown_shown`, filled by `evade`). A bar has its share in percent beside its
  label, and the plate's side bar turns red while the player is being busted.
- **Police detector** (`remastered/radar.rs`, the new `PoliceRadar` slot, number 12). It takes the detector's package objects
  (`hud::roots::police_radar`: its group with the laser, the arrow and the two LED rows, and the backing in either layout), so the
  Original look draws them as before. The Remastered look is a slim dark pill at the top centre with a row of fourteen dashes
  each side and a small dial with the detector's 45 degree cone. With a cop in range the dashes light outward from the dial in
  the original's sweep (it starts at 30 percent and grows over a period of 1.5 times the range, so a near cop sweeps faster; the
  look keeps no state, it takes the phase from the clock), the arrow (the package's own) turns to the cop, and the dashes are
  red when the cop is under a third of the range away. With no cop the pill is dimmed. It hides while a chase runs and is back
  during the cool-down, as the original's detector (`hud-pursuit-state.md`).

## 5. Not done, and open

- The busted overlay (the police video camera frame), the rear-view mirror and the police radar are drawings of devices in the
  game's world and stay Original. The blips stay the game's own icons.
- The track select, the car lot, the rap sheet, the memory card and the SMS screens are still Original.
- The Blacklist has no Challenge action until race mode exists.
- Checked on one aspect (16:9) by screenshot, with the Bevy palette. The Remastered palette changes the colours only.

## 6. Tests

`remastered/menu/blacklist.rs` (rows, the hit test, the card, the bio), `remastered/minimap/` (the window, the heat segments, the
evade arc, the tint, the fade), `remastered/radio.rs`, `remastered/pursuit.rs`, `remastered/radar.rs`, `frontend/career_screens/blacklist_tests.rs`
(`status_of`), `libs/blackbox-ui` (`Ring`, `Trapezoid`, `Tokens`).
