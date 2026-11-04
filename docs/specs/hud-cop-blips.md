# Police icons on the minimap

How the original game draws the police on the in-game HUD minimap: which cars and the helicopter get an icon, when,
in which colour, and where. The minimap itself (the picture, the placement rule, the turning) is in
[hud-minimap.md](hud-minimap.md). The icon images and their names are in [../formats/minimap.md](../formats/minimap.md).

- **Source:** `speed.exe` v1.3 (`Need For Speed Most Wanted Black Edition`), read in Ghidra and in the disassembly for
  behaviour only; the CC0 decomp ([dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw)) for the names of the pursuit's
  engagement flag and in-pursuit state. The installed `HUD_SingleRace.fng` for the objects. No code was copied.
  Provenance: [provenance/hud-minimap.md](../provenance/hud-minimap.md).
- **Evidence tags** as in [docs/README.md](../README.md#evidence-tags). **[binary]** = read in the executable;
  **[inferred]** = a reading that a capture has not confirmed; **[package]** = the installed HUD package; **[user]** =
  reported by the user from play; **[capture]** = seen in a stock-game capture (a recording of the original game's
  pursuit state and the icons' hidden flags, about 14.5 minutes from heat 1 to heat 5).
- **Correction (2026-11-02).** An earlier version of this page described the world map's code (the pause map's Pursuit
  view, section 6) as the HUD minimap's. The HUD minimap has its own update, described in sections 1 to 5.

## 1. The objects

- Eight car icons `MMICON_COPCAR_0` .. `_7` (`Minimap_Icon_Car.tga`, 14 x 16: an arrow), and the helicopter's group
  `HELICOPTER_ICON_GROUP` (the helicopter picture, 28 x 28, and its blades, 26 x 26, with a looping 300 ms spin
  script). A translucent line-of-sight cone `HELICOPTER_LINE_OF_SIGHT` (64 x 64, alpha 160) sits beside them.
  **[package]** All of these are in one top-level icon group of the package with the other map icons.
- There is no roadblock, spike-strip or last-known-place icon on the HUD minimap, and no separate wreck texture.
  **[binary]** (The rewrite's roadblock icon is its own, see section 7.)

## 2. Which police get an icon

Each HUD update the minimap runs one police pass. **[binary]**

1. **Pursuit gate.** If the player is not in a pursuit, or the pursuit has been evaded (bailed), every police icon and
   the helicopter are hidden and nothing else happens. So no police icon shows in free roam, not even for a patrol car
   in view, and every icon goes on the frame the player gets away. (A debug switch that shows cops outside a pursuit
   is off in the shipped game.)
2. The pass walks the active police vehicles. A wrecked car is still active until it is removed from the world, so a
   wreck stays in the pass.
3. **A car** gets an icon only if it has come within the pursuit level's full-engagement radius of the player during
   this pursuit. The pursuit marks a car so on every update while it is within that radius (3D distance; 150 m when
   the level gives none, 50 m at heats 1 to 7 in the stock data) and the pursuit is not in cool-down. The mark is
   cleared only when the car is reset for a new life (respawned or reused), not when it wrecks, flees or leaves the
   pursuit. **[binary]** for the gate, **[inferred]** from the decomp for when the mark is set and cleared,
   **[capture]** for the radius: with these rules the number of shown car icons matched the capture in 98 % of the
   samples (30 m: 84 %, 75 m: 90 %); a chase car stayed hidden until it came within about 50 m. Cars that never came
   that close (patrols, new searchers spawned during cool-down) are not shown.
4. **A roadblock** marks its own cars within a fixed **300 m** of the player, whatever the level's radius and in
   cool-down too. The value is read from the decomp; **[capture]**: roadblocks stood 204 to 256 m from the player when
   they appeared, and their cars showed from the first sample (a 150 m or 250 m reach would have hidden them).
5. **The helicopter** is shown whenever it is an active vehicle of the pursuit and the gate is open. Neither the
   engagement mark nor its sight of the player gates it. It does not use a car icon and never flashes. **[binary]**
   **[capture]**: shown in every sample of its two flights, chasing and leaving.
6. At most eight cars are shown. In the original every active non-helicopter car takes one of the eight slots, shown
   or not, and the pass stops after the eighth. **[binary]**

## 3. Colour

The car's colour is set on every update. **[binary]**

- A car that is in the pursuit (chasing, or manning a deployed roadblock) flashes its light bar from a counter that
  steps once per update and runs through nine states: three blue (`#0000FF`), one light grey (`#CCCCCC`), three dark
  red (`#A00000`), two light grey.
- The update runs once per **rendered frame**, not per simulation step. **[capture]**: the counter moved 7 steps (mod 9)
  per 50 ms sample in 85 % of the samples and stood still while the game was paused; a 60 Hz simulation step would
  have moved it 3. So the cycle lasts nine frames: 150 ms at 60 frames per second.
- A shown car that is not in the pursuit any more is **steady light grey `#CCCCCC`**, never white. That is a wrecked
  car (it leaves the pursuit when it wrecks). **[inferred]** from the in-pursuit state's writers.
- **[decomp]** The icon pass tests two things per car: one decides whether the car is drawn at all (a car failing it is
  hidden), the other whether its bar flashes. **[observed]** In the original a car sent away in the cool-down is not
  drawn as a grey car, and in a capture the cars near the player in the cool-down are pursuit members or roadblock cars.
  The remake hides a car sent away (`ai/minimap.rs`).
- The colour's alpha is replaced by the placement fade (section 4).

## 4. Placement

All police icons use the same rule as the other blips ([hud-minimap.md](hud-minimap.md) section 7): within 0.06
picture widths of the player (about 400 m on the city map) an icon sits where the vehicle is; farther away it is held on
the rim of the disc, opaque up to 0.125 picture widths, then fading linearly to invisible at 0.23. It is never hidden
for distance. The icon points along the vehicle's heading relative to the map. **[binary]**

## 5. The helicopter's sight cone

The cone is placed with the helicopter, at the same place and heading. It plays its tracking script while the
helicopter has seen the player within the last quarter second, otherwise its idle script; the script is set only
when it changes. With no helicopter placed the group and the cone are hidden. **[binary]** **[capture]**: the cone was
shown in every sample in which the helicopter's icon was, and hidden with it. The cone's two scripts are a single colour
key each **[package]**.

## 6. The world map's Pursuit view

Opening the world map (the pause map) during a pursuit selects its Pursuit view, otherwise the view the player last
used. Only the Pursuit view lists police: the engaged cars and the helicopter, and the standing roadblocks, each with
a filter toggle. Its car icons flash on their own counter (no in-pursuit test); its helicopter cone pulses. **[binary]**

## 7. The rewrite

- **Feed:** `TrafficWorld::map_blips` (`crates/nfsmw/src/scenes/world/ai/minimap.rs`). While the world has a pursuit
  (it drops it on the frame the player gets away or is busted), it lists the helicopter while it flies and every police
  car with the engagement mark (`AiCar::was_engaged`), wrecks included. `active` is the car's in-pursuit state
  (`AiCar::is_pursuing_on_map`: chasing and not sent away, or a roadblock car; never a wreck). With no pursuit it lists
  no police. The standing roadblocks are listed too, as the rewrite's own icon.
- **Mark:** `TrafficWorld::update_budget` marks the chase cops within the level's engagement radius outside cool-down,
  and the roadblock cars within 300 m at any time (section 2 item 4). Only the chase cops count for the backup budget.
- **Colour:** `crates/nfsmw/src/hud/cop_blip.rs`: steady `#CCCCCC` when not active, else the nine-step light bar,
  one step per look tick of 1/60 s (`hud::TICK_SECONDS`): the original's per-frame step at 60 frames per second. The
  world map's moving cars use the same cycle. The Custom theme's map keeps its own calmer 0.1 s step.
- **Helicopter icon:** the layout (`hud/elements.rs`) treats the helicopter group as a minimap root, so its picture and
  blades show with it; the blip binding hides and places the group itself. The feed lists a second blip at the same
  place and heading for the sight cone (`HELICOPTER_CONE`, style `HelicopterCone` in `hud/blip_styles.rs`): active while
  the helicopter's last sight test saw the player (its grace included), which plays the cone's tracking script, else its
  idle one; the cone keeps the package's alpha 160 under the placement's fade. The minimap shows the helicopter whenever
  it flies in the pursuit. The Custom theme's map does too (`ui/theme/custom/map/marks.rs`;
  [custom-hud/fair-knowledge.md](custom-hud/fair-knowledge.md) rule 1).
- **Deviations, kept on purpose or not done yet:**
  - The light bar steps at a fixed 60 Hz. The original steps once per rendered frame, so its flash is faster on a faster
    display; the rewrite does not tie it to the frame rate.
  - A car icon is drawn only inside the disc (`Visibility::InZone`), at the user's request; the original pins engaged cars
    to the rim. **[user]**
  - Slots go only to drawn icons; the original's eight-car cut-off (which can also drop the helicopter) is not copied.
  - The cone's "seen within a quarter second" is the helicopter's own sight result with its grace.
- **Tests:** `scenes::world::ai::pursuit::tests::minimap` (the pursuit gate, the engagement mark, the roadblock's
  300 m reach, wrecks and fleeing cops steady, nothing after the pursuit), `pursuit::tests::air` (the helicopter's blip
  and its cone go with the pursuit), `hud::blips_tests` (steady grey, the helicopter group with its blades placed and
  rim-pinned, the cone on the helicopter with its scripts), `hud::cop_blip_tests`, and the installed
  `hud::pursuit_install_tests` (the real package's helicopter picture and blades show, the cone shows with them and
  switches scripts).

## 8. Open questions

1. How long a wreck stays before it is removed (the unspawn time of a wreck out of view).
2. Not in the capture: the colour and the on-screen place of each icon (only the hidden flags and the flash counter were
   recorded), the cone's two colours, a wreck's and a fleeing cop's icon, and more than eight cars at once.
3. In the capture every police icon was also hidden, with the flash counter standing still, for 0.2 to 1 s at a time
   (about 25 times, at all heats). The minimap was not updated then (the HUD hidden by a camera change or a menu, not
   read further); it does not change the rules above.
