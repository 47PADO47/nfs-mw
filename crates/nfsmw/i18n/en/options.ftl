### options
### The rows of the option screens the original has no label for (`frontend/options*.rs`, `input_options.rs`,
### `hd_options.rs`, `mod_options.rs`), and the values several rows share. On, Off and Auto are the original's own labels.
### The Video page's rows are in `options-graphics.ftl`.

## Values several rows share

options-off = Off
options-low = Low
options-medium = Medium
options-high = High
options-very-high = Very High
options-ultra = Ultra
options-maximum = Maximum
options-balanced = Balanced
options-standard = Standard
options-custom = Custom
options-original = Original
options-original-pc = Original PC
options-enhanced = Enhanced
options-remastered = Remastered
# A row whose setting the running renderer cannot do.
options-unavailable = Unavailable

## Audio

# The speech volume row: the police dispatch's radio lines (the setting `speech_volume`).
options-police-dispatch-volume = Police Dispatch

## Gameplay

options-hud-layout = HUD Layout
options-hud-layout-centered = Centered
options-hud-style = HUD style
options-hud-theme = HUD theme
options-speed-units = Speed units
options-speed-units-metric = Metric (km/h)
options-speed-units-imperial = Imperial (mph)
options-minimap = Minimap
options-minimap-fixed = Fixed
options-minimap-rotating = Rotating
options-turn-signals = Turn Signals
options-cop-health-bars = Cop Health Bars
# The row that picks the game's language; each language is listed by its own name (`languages.ftl`).
options-language = Language

## Player

options-camera-chase = Chase
options-camera-dynamic-fov = Dynamic FOV
options-camera-drift = Drift
options-camera-pursuit = Pursuit

## Controls

options-deadzone-mode = Deadzone Mode
options-deadzone-rescaled = Rescaled
options-deadzone-cutoff = Cutoff
options-steering-deadzone = Steering Deadzone
options-camera-deadzone = Camera Deadzone
options-trigger-deadzone = Trigger Deadzone
options-steering-sensitivity = Steering Sensitivity
options-camera-sensitivity = Camera Sensitivity
options-mouse-sensitivity = Mouse Sensitivity
options-invert-camera-y = Invert Camera Y

## Discord

options-discord-presence = Discord Rich Presence
options-discord-time-played = Show Time Played

## Mods page

# The UI theme row, and what it shows while no mod theme is chosen.
options-mods-ui-theme = UI theme
options-mods-built-in-style = Built-in style
# What a mod's button row shows where a toggle shows its value.
options-mods-press = Press
# A mod's switch when the next start changes what this run did. $id is the mod's id.
options-mods-restart-to-unload = { $id } (restart to unload)
options-mods-restart-to-load = { $id } (restart to load)

## EA Trax jukebox: where a song plays

options-jukebox-menu = Menu
options-jukebox-in-game = In game
options-jukebox-both = Both
