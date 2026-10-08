# Front-end menus (screens, icon menus, option widgets, flow)

What the game code around the FEng menu packages does: which screen logic runs a package, how the icon menus
and the option screens drive the objects of their package, and how the screens follow one another. The packages
themselves are described in [formats/frontend.md](../formats/frontend.md), how they run in
[feng-runtime.md](feng-runtime.md), and how pad input reaches them in [feng-input.md](feng-input.md).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled), under
  `src/Speed/Indep/Src/Frontend/`: `FEPackageData.cpp` (screen factory), `FEManager.cpp`,
  `MenuScreens/Common/{FEMenuScreen,feIconScrollerMenu,feUIWidgetMenu,feWidget,Slider}.cpp`,
  `MenuScreens/Safehouse/{uiMain.cpp,options/uiOptionsMain.cpp,options/uiOptionsScreen.cpp,
  options/uiOptionWidgets.cpp}`, `MenuScreens/InGame/uiPause.cpp`,
  `MenuScreens/Loading/{FEBootFlowManager,FEMovieScreen,FESplashScreen}.cpp`, `FEngHashes/*`. Read for
  understanding; no code copied. Provenance: [provenance/frontend-menus.md](../provenance/frontend-menus.md).
- **Evidence tags** as in the [docs README](../README.md#evidence-tags). **[decomp]** = read in the decompiled
  code, **[verified]** = checked against the packages of the install (object names hash to the ids below),
  **[ours]** = a decision of this rewrite where the original is not reproduced.

## 1. Screens

The game keeps a stack of loaded packages ("screens"). Each screen has a logic class chosen by the package
file name (`ScreenFactoryData`); the engine passes it every message a package sends to the game
(`NotificationMessage(message, object, controller mask, package)`) and a tick message every frame. Package commands
from responses or from code are **switch** (unload the top screen, load another), **push** (load over it, the
lower screen loses pad control) and **pop**.

| Package | Logic (original class) | Used for |
|---|---|---|
| `MainMenu.fng` | `UIMain` (icon menu) | the main menu |
| `MainMenu_Sub.fng` | `UIOptionsMain` (icon menu) when the game mode is options | the option categories |
| `Options.fng`, `OptionsPCDisplay.fng`, `Pause_Options.fng` | `UIOptionsScreen` (widget menu) | the rows of one category |
| `Pause_Main.fng` | `PauseMenu` (icon menu), or `UIOptionsMain` with the options flag | the pause menu |
| `LS_EALogo.fng`, `LS_PSA.fng` | `MovieScreen` | the boot movies |
| `MW_LS_Splash.fng`, `WS_MW_LS_Splash.fng` | `SplashScreen` | the title screen |

The screens that ship with the install and are not covered here (career, quick race, customisation, online,
dialogs) are listed by `nfsmw list-screens`.

**Leaving a screen [decomp, verified].** A button press does not act at once: the screen stores the message
and the pressed object, then posts `LEAVE_SCREEN` (`0x587C018B`) to its own package. The package responds with
the leave scripts of its objects (about 600 ticks) and, from a script event, sends `EXIT_COMPLETE`
(`0xE1FDE1D1`) to the game. Only then does the screen act on what was pressed (set a category, queue the next
package). `INIT_COMPLETE` (`0x35F8620B`) arrives the same way when a screen has finished appearing.

**Last button [decomp].** The game remembers per package which button was current when the screen was left and
selects it again when the screen returns (`FEngGetLastButton`); the icon menus store the option index.

## 2. Icon menus (`MainMenu`, `MainMenu_Sub`, `Pause_Main`)

A row of icons that scrolls under a fixed cursor. The package has up to 10 or 11 image buttons named
`OPTION_1` .. `OPTION_n` (the buttons are `AffectAllScripts | IsButton | DontNavigate`; their positions in the file
are placeholders), a string `ICON_TITLE` (hash `0x5E7B09C9`) with its shadow `0x0DFB7A2E`, the region
`ICON_SCROLL_REGION` and the master `OPTION_MASTER`.

**Setup [decomp].** Each option has a texture, a name label and a description label. The screen puts two *book
ends* (an empty option with the texture `END_OF_SCROLLER`) before the first and two after the last option, so
slots 1..n hold `2 + options + 2` icons. Option `i` is given the image `OPTION_i`, the texture of the option (the
image's texture is swapped, `FEngSetTextureHash`), and an x offset: the offsets add up as
`offset(i+1) = offset(i) + width(i) + spacing`, spacing -5, widths from the package (64). The first real option
is selected. The centre `(cx, cy)` is the centre of `ICON_SCROLL_REGION`, which is then hidden.

**Every tick [decomp].** The scroll value moves towards `-offset(selected)` over 0.2 s with a cubic ease. For
each icon, with `x = cx + scroll + offset`:

```
scale = 0 outside [cx - W/2, cx + W/2]   (W = 350, the scroller width)
      = 1 within 1.5 of cx
      = linear to 0 at the edges:  left (x - (cx - W/2)) / (W/2), right ((cx + W/2) - x) / (W/2)
x'    = x + width * (1 - scale)^3  if x < cx,  x - width * (1 - scale)^3  otherwise    (pulled towards the centre)
the icon is hidden when x' lies outside the region, else centred at (x', cy) with size = original * scale,
its top edge moved so it scales about its middle
colour = idle colour * scale + fade colour * (1 - scale)      (white 0xFFFFFFFF -> 0x00FFFFFF; the pause
                                                              menu uses 0xFFFFAE40 -> 0x00FFAE40)
```

**Messages [decomp].** `PAD_LEFT` / `PAD_RIGHT` select the previous / next option that is not greyed out
(no wrap), then refresh the header: the label of the option is set on `ICON_TITLE` and its shadow, and at the first
or last option the package gets `END_PAD_LEFT` (`0xD7118934`) / `END_PAD_RIGHT` (`0xB9B17747`) (the arrows
fade). `BUTTON_PRESSED` on the selected icon stores the press and posts `LEAVE_SCREEN`. `PAD_BACK` stores the
press and the screen leaves with the back result. On `EXIT_COMPLETE` the stored press is applied to the option
(`React`) and the screen is switched.

### 2.1 Main menu entries [decomp]

`MainMenu.fng` options, in order, as `(name label, icon texture)`: Career (`0x5815A2B5`, `0x03704F3D`),
Challenge Series (`0xCC8CB746`, `0x9A962438`), Quick Race (`0x54020A7A`, `0x4E6FBB02`), My Cars
(`0x1AFD5BE6`, `0xB0C46023`), Profile Manager (memory card builds only), Options (`0x19A8C0AF`,
`0x3058FE37`). The title is the label `0xB24AAE58` on `TITLE_GROUP` (`0xB71B576D`). A `Quit` button (the
object `0x104f9d`, label `0xC51FBF74`, keyboard hint `Q`) lives in the package; the game answers it with the
message `0x62799A4C` **[verified, name unknown]**.

Selecting Options sets the game mode to options and switches to `MainMenu_Sub.fng`; `PAD_BACK` there returns to
`MainMenu.fng`.

### 2.2 Option categories [decomp]

`MainMenu_Sub.fng` as `UIOptionsMain`: options Audio (`0xE76CD783`, icon `0xF37AF144`), Video (`0x8A006328`),
Gameplay (`0x4DF98FB2`), Player (`0xD708EFEF`), Controller (`0xA04A7B26`), then EA Trax and Credits when not
opened from the pause menu. The title is `options` (`0x4ECA678C`). Pressing one sets the category and switches to
`Options.fng` (in game: `Pause_Options.fng`); `PAD_BACK` returns to `MainMenu.fng` (in game: `Pause_Main.fng`).

### 2.3 Pause menu [decomp]

`PAD_START` (and `PAD_BACK`) while driving pause the game and show `Pause_Main.fng`. Its options for a free-roam
session: Resume Free Roam (label `0x01BD185C`, icon `0x12BB5EA2`), Quit (label `0x3C14C420`, icon `0x4C9E34E6`),
Tuning (when available) and Options (label `0x2B5A03A8`, icon `0x520DE4E3`). The header `0x863404B5` shows
`0x6C839FBE`. `PAD_START` / `PAD_BACK` resume; Resume and Quit act on `EXIT_COMPLETE` (resume = unpause, quit =
back to `MainMenu.fng`); Options switches to the option categories with the pause flag, whose Back returns to
the pause menu and whose Start resumes.

## 3. Option screens (widget menus) [decomp, verified]

`Options.fng` and `Pause_Options.fng` have numbered rows. Row `n` is the objects `OPTION_NAME_n` (a string and a
button), `OPTION_DATA_n` (a string), `LEFT_ARROW_n` and `RIGHT_ARROW_n` (images), and for sliders `BASE_SLIDER_n`
and `FILLBAR_SLIDER_n` (images), with `TITLE_MASTER` and `DATA_MASTER` fixing the row's title and data columns. The
screen shows at most 9 rows (10 from the pause menu). Rows are added in order; a row is a *toggle* (a title and a
data string that cycles through values) or a *slider*.

- **Selection.** `PAD_UP` / `PAD_DOWN` select the previous / next enabled row with wrap-around; the selected row's
  objects play `HIGHLIGHT` (`0x249DB7B7`), the previous row's `UNHIGHLIGHT` (`0x7AB5521A`); disabled rows play
  `DISABLE`. When there are more rows than fit the list scrolls and the cursor object plays `POSn` for the row
  slot. `PAD_LEFT` / `PAD_RIGHT` / `BUTTON_PRESSED` act on the selected row.
- **Toggle row.** Left or right flips or cycles the value; the title and the data string are language labels
  (`FEngSetLanguageHash`). Booleans show `On` (`0x417B2604`) / `Off` (`0x70DFE5C2`).
- **Slider row.** A value in 0..1 moves by 0.1 per press (`min`, `max`, `increment`). The fill bar width is
  `range * (value - min) / (max - min)` (range = the width of the slider base, 164), and its right UV edge is
  set to the same width; the data string shows the integer value. The value is applied at once (audio volumes
  update the mixer on every press).
- **Back.** `PAD_BACK` asks to confirm when something changed (a two-button dialog; not reproduced here),
  otherwise posts `LEAVE_SCREEN`; on `EXIT_COMPLETE` the options are marked dirty (saved later) and the
  previous screen is shown. `PAD_BUTTON5` offers the defaults.
- **Header.** `HEADERTEXT` (`0x42ADB44C`) gets the category title label (Audio `0x3932C2E4`, Video `0x48478029`,
  Gameplay `0x01CCE8C2`; from the pause menu `0xB1426DFA`, `0xD94EA03F`, `0x3936D9F8`).

The original audio rows are: Sound Effects Volume (`0xFD487543`), Car Volume (`0x218E4B08`), Speech Volume
(`0x9E5FB82A`), Menu Music Volume (`0x418E681D`), Game Music Volume (`0xDF21EAC2`), Interactive Music
(`0xA3DBB390`), EA Trax mode (`0xDCFB6B36`) and, in the front end, Audio Mode (`0x2881AB87`); video has
Widescreen (`0xD3588630`); gameplay has Autosave, Game Moment Camera (`0xF26A5CBF`), Car Damage (`0x1582ADFF`),
Rearview Mirror (`0x85A6CE05`), Units (`0x01E19173`; Imperial `0xFBD74FC5`, Metric `0xAF70E736`) and the map
settings; the player screen has Gauges (`0xAC148579`) and more.

### 3.1 What this rewrite shows [ours]

The original settings are not the settings of this rewrite, so the rows are mapped, keeping the original
screens, labels and behaviour where a label exists:

| Category | Row | Label | Setting |
|---|---|---|---|
| Audio | Master Volume | `0x26782C3E` | `master_volume` (slider, 0 to 100 in steps of 10) |
| Audio | Sound Effects Volume | `0xFD487543` | `sfx_volume` |
| Audio | Engine Volume | `0xA2B1F888` | `engine_volume` |
| Audio | Menu Music Volume | `0x418E681D` | `music_volume` |
| Video | Vsync | `0x6CEB9CB6` | `vsync` (On / Off) |
| Video | Frame Limit | none in the language table, plain text | `max_fps` (Unlocked, 30, 60, 120, 144, 240) |
| Video | Performance Overlay | none, plain text | `show_metrics` (Off, Basic, Advanced) |
| Gameplay | Gauges | `0xAC148579` | `hud` (On / Off) |

Changes apply at once and are written to the config file when the screen is left.

## 4. Game flow

**Boot [decomp].** `BootFlowManager` shows, in order: `LS_EAlogo.fng` (movie `ealogo`, cannot be skipped),
`LS_PSA.fng` (movie `psa`, accept or start skips), `MW_LS_AttractFMV.fng` (movie `attract_movie`, skippable; shown
when the splash times out, not in the normal order), `MW_LS_Splash.fng` (the title screen), then `MainMenu.fng`.
A movie screen shows its movie as an FEng movie object (the object whose name hash is `0x58BCF5B6` /
`0x72CF9F38`), tells the game `0xC3960EB9` when it ends, and the game switches to the next boot screen. The
splash accepts `PAD_ACCEPT` / `PAD_START` once `INIT_COMPLETE` arrived, and starts the attract movie after 30 s
without input. The memory-card screens between splash and main menu (`MC_Main_GC.fng`) do not apply on the PC.

**Free roam [decomp, ours].** The career starts in free roam in the original; this rewrite has no career, so
Career and Quick Race both start free roam with the stock car.

## How to check it

1. `nfsmw view-screen MainMenu.fng --screenshot out.png`, then with `--ui-script "wait 1;right;right;wait 0.5"`: the icons
   scroll, the title follows the selection.
2. `nfsmw view-screen Options.fng` for the rows (the screen needs its category: `--category audio`).
3. Compare with the running original at the same window size (not done): icon sizes and positions after a scroll,
   row positions of the option screens.
