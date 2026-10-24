### options-hud
### The HUD options page (`frontend/hud_options.rs`): one row per `[custom_hud]` key, its values, and the line under
### the page. On and Off are the original's own labels. Speed units are the Gameplay page's row (`options.ftl`).

## Rows

options-hud-preset = Preset
options-hud-scale = HUD scale
options-hud-opacity = HUD opacity
options-hud-text-size = Text size
options-hud-map-mode = Map mode
options-hud-map-opacity = Map opacity
options-hud-map-zoom = Map zoom
options-hud-map-orientation = Map orientation
options-hud-advanced-telemetry = Advanced telemetry
options-hud-world-markers = World markers
options-hud-race-info = Race information
options-hud-pursuit-info = Pursuit information
options-hud-free-roam-extras = Free-roam extras
options-hud-notifications = Notifications
options-hud-high-contrast = High contrast
options-hud-palette = Colour palette
options-hud-motion-reduction = Reduce motion
options-hud-camera-shake = Camera shake
options-hud-fov-intensity = Speed FOV intensity
options-hud-haptics = Controller vibration
options-hud-volume = HUD volume

## Values

options-hud-preset-minimal = Minimal
options-hud-preset-balanced = Balanced
options-hud-preset-advanced = Advanced
options-hud-map-compact = Compact
options-hud-map-centered = Centered
# The map's orientation: the `minimap` setting's fixed and rotating views.
options-hud-map-north-up = North up
options-hud-map-heading-up = Heading up
options-hud-notifications-all = All
options-hud-notifications-important = Important
options-hud-palette-default = Default
options-hud-palette-deuteranopia = Deuteranopia
options-hud-palette-tritanopia = Tritanopia

## The line under the page

# Under the Custom style the line sums the options up: the map, then how much information, then the accessibility
# options that are on, joined by commas, as "Centered map, balanced information, high contrast".
options-hud-preview-no-map = No map
options-hud-preview-compact-map = Compact map
options-hud-preview-centered-map = Centered map
options-hud-preview-expanded-map = Expanded map
options-hud-preview-minimal = minimal information
options-hud-preview-balanced = balanced information
options-hud-preview-advanced = advanced information
# The six options a preset covers were changed by hand.
options-hud-preview-custom = custom information
options-hud-preview-high-contrast = high contrast
options-hud-preview-reduced-motion = reduced motion
# Under another style, where most rows wait for the Custom style (the camera, vibration, volume and map orientation rows
# apply to every style). Short: the Remastered page shows it beside its title.
options-hud-note-other-style = Mostly for the Custom style
