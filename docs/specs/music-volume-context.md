# Independent menu and game music volume

The recorded PC Audio page exposes Menu Music Volume and Game Music Volume. This repair supplies two independent
controls through the existing music mixer track. It does not define Interactive Music, EA Trax enable or output mode.

## Compatibility and precedence

`music_volume` remains a legacy common baseline. A legacy-only configuration or alias applies its value to both
contexts, including zero. With no stored music keys both contexts retain the existing 60 percent default.
`menu_music_volume` and `game_music_volume` override the corresponding baseline within the same settings layer.
Normal settings-layer priority still wins: a higher-priority legacy common value overrides lower-priority explicit
context values; a higher-priority context value overrides only that context. An unspecified context inherits normally.
Existing ExtraOptions music disable remains a higher-priority common mute when its layer is applied.

Changing either menu slider writes only its own context key. Config writes preserve unknown/unrelated keys. Alias
snapshots include both resolved context values and the legacy baseline so current profiles round-trip without
combining the independent levels. Old aliases containing only `music_volume` continue to set both contexts.
The legacy console setter sets both live contexts; the two new setters affect only their named context.

## Playback context

Active driving and the same paused gameplay scene use Game Music Volume. The menu scene, garage, and explicit EA
Trax menu preview use Menu Music Volume. Pursuit interactive music and licensed driving songs share the game gain.
Pausing a driving scene does not select the menu gain. Closing the jukebox restores the current scene context.
A scene transition changes the active gain on its next audio update. Changes to either slider take effect live;
changing gain never reloads a song or restarts pursuit music. The existing master gain still multiplies music.
Settings storage remains independent of the current playback context. A muted context cannot overwrite the other.

## Validation scope

Check legacy/default migration, same-layer and cross-layer precedence, config and alias round-trips, independent
slider changes and live mixer output across menu/game/paused/preview contexts. Synthetic mixer checks do not certify
reference loudness, physical controller behavior or original-game mix parity.
