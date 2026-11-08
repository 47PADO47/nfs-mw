# Colour grading with the 2012 cubes: how it was checked

What was run, on what, and what it showed. The design is [../specs/colour-grading.md](../specs/colour-grading.md), the data
[../formats/chameleon-environment-grading.md](../formats/chameleon-environment-grading.md). Nothing from either game is in the
repository: the screenshots are in `C:\r6work\grading-shots` (not committed).

## Setup

- Native renderer, Vulkan, release build, offscreen (`view-world --screenshot`, the window hidden and unfocused), `--muted`,
  1280 x 720, the same spot of the city for every picture (`--at 1945,1850`, a free camera over the harbour road), no traffic,
  the profile `C:\r6work\profile-grading` (an isolated `BLACKBOX_PROFILE_DIR`, the 2012 mod with `trees = false`, `traffic = false`).
- The 2012 mod converted the cubes in the game's first run (`files/GRADING/MW2012/`: `profile.toml` and eleven `.LUT` files, 135,256 bytes;
  `cargo test -p nfsmw-2012-content -- grading --ignored` with `MW2012_GAME_DIR` checks the same output).
- Grading off: no `option_grading_variant` in `[mod_values.nfsmw]` (the option's own value, Off). On: `option_grading_variant = "mw2012"`,
  `grading_strength = 1`. The hour with `--exec "environment hour H"` (clouds `environment clouds 0.2`), rain with `--exec "environment rain on"`
  (full rain, 14 s before the capture so the rain and the wet road build up).
- Script: `C:\r6work\grading-work\shoot.sh <name> <hour> <on|off> <rain 0|1> <strength>`; the matrix is `matrix.sh` and `wetmatrix.sh` in the
  same folder; `sheets.py` makes the side by side pictures.

## Pairs (grading off | on)

`C:\r6work\grading-shots\pairs\<name>.png`, each with the mean absolute difference of the two frames in grey levels (0 to 255) as measured by
`sheets.py`. Two runs of the same frame with grading off differ by 0.4 on average (clouds and the sky's noise), so a difference below
about 1 is none.

| Pair | Hour, weather | Cubes in force | Mean difference |
|---|---|---|---|
| `dry-02-night` | 2:00, dry | night (the identity) | 0.05 |
| `dry-04-predawn` | 4:00, dry | before dawn `0x124514` | 7.69 |
| `dry-0530-dawn` | 5:30, dry | dawn `0xF0864` | 7.79 |
| `dry-08-morning` | 8:00, dry | morning `0x1244FC` | 9.38 |
| `dry-12-noon` | 12:00, dry | noon `0x4E220` | 9.61 |
| `dry-1550-blend` | 15:30, dry | noon to evening, half way (two cubes blended on the GPU) | 10.03 |
| `dry-19-evening` | 19:00, dry | evening `0xF0862` | 12.61 |

**The wet pairs are not valid and are to be shot again.** `wet-02-night`, `wet-0530-dawn`, `wet-08-morning`, `wet-12-noon` and
`wet-19-evening` were taken with `always_rain = true` in the config, which the game does not read from the file (`rain_amount` and
`always_rain` are session settings), so there was no rain and the grade was the dry one: their on-minus-off colour shifts are
identical to the dry pairs'. The dev command `environment rain on` was added afterwards for this (`scenes/world/environment.rs`); it
and the re-shoot were **not built or run** (a stop order came first). `wetmatrix.sh` is the script for the five wet pairs.


The grade is gentle by design: the 2012 cubes lift the mids (the noon cube turns the grey 136 into 165, 173, 173), cool the shadows and
warm the highlights; the night cube is the identity, so a dry night does not change (0.05, which is the run's own noise) and that is the
neutral-cube guarantee seen in a real frame. The wet pairs differ from the dry ones by the wet cubes (cooler, see the format doc).

## Strength

Same noon scene, grading off and `grading_strength` 0.25, 0.5, 0.75, 1 (`pairs/strength-sheet.png`). The mean difference from the
ungraded frame is 2.41, 4.85, 7.20 and 9.61: linear in the strength, as the shader's blend is.

## Cost of the pass

**Not measured.** The test `cost_of_the_pass_at_1080p` (`libs/blackbox-gpu-passes/src/colour_cube_tests.rs`, ignored) times 300 draws
in one submission at 1920 x 1080 from an `Rgba16Float` image into `Rgba8Unorm` for one cube of 16, one of 32 and two cubes of 16, and
prints the milliseconds per draw. Run it on an idle machine:
`cargo test --release -p blackbox-gpu-passes cost -- --ignored --nocapture`. The end-to-end figure is
`nfsmw view-world --bench-seconds 20 --offscreen --no-vsync --muted --screenshot-size 1920x1080` with and without the grading chosen
(the machine was shared and loaded while the screenshots ran, so no number is given).

## Not checked

Another GPU, DX12 or GL, an HDR display, a window on screen, the game with real traffic and a car on the road, the rear-view mirror's
picture, and the Bevy renderer (it does not draw the grade).
