# Extra options and screenshot tools

F5 opens Extra options. F6 remains the garage; F8 retains graphics and traffic density.
F5 / Escape closes the panel. Opening another developer panel closes F5.

## Camera

- Auto-center, delay and return speed affect the two remake chase cameras.
- Distance, height, vertical FOV and speed zoom are independent preferences.
- Original camera modes retain their installed behavior. They are labelled separately
  from the remake chase cameras; original extra views remain directly selectable.
- Freecam detaches from the current view and preserves its FOV. Returning through F5
  or the normal freecam shortcut keeps the car where it was.
- Home centers the remake chase and releases a camera bookmark. F5 offers alternate keys.
- F4 hides the complete composed UI. F5 restores it. An alternate key is selectable.
- Pause stops simulation while freecam and the remake chase orbit remain usable.
- A camera bookmark saves position, orientation and FOV in world or car-relative space.
  A car-relative view follows the car's position and heading.
- Normal F6 car swaps keep the driving camera following the new car. They never pin a view.
- With an explicitly selected bookmark or freecam, F6 can keep the new car at the last
  supported road position for screenshot comparisons. Bookmarks and freecam stay selected;
  Home returns to the driving camera.

## World and testing

Physics update rate defaults to **60 Hz (original)**. **144 Hz (comparison)** is
an optional offline comparison mode that changes numerical handling. Rendering
FPS has its own independent limit; neither physics choice caps rendering FPS.
The choice saves as `physics_rate_hz = 60` or `144` in `config.toml` and in alias
settings. CLI `--physics-rate-hz 60|144` and environment `NFSMW_PHYSICS_RATE_HZ`
can override it. Multiplayer always uses 60 Hz regardless of the local choice.

Traffic off removes ambient civilians immediately and suppresses their automatic spawns.
It retains police and race opponents and does not rewrite F8 density. Police off ends
pursuits without awarding an escape and bars patrol/reinforcement spawning.

Heat can be started/set and locked from 1 to 10. Unlimited NOS fills an installed system;
it does not add an upgrade. Unlimited Speedbreaker keeps its normal activation rules.
Refill, repair and recover-to-road are explicit actions.

Gameplay-changing controls are available in local single-player career and free roam.
Override settings remain session-only, while normal career progress and rewards still save.
Multiplayer and co-op testing overrides are rejected. Teleport is barred during a race.

Teleport accepts map coordinates, a GPS destination, or a saved location/heading. It uses
normal road placement and stops the car. Return restores the previous teleport location.
Weather/time controls share the F8 environment state. Clear restores the environment
that existed before the F5 override and removes testing overrides.

## Persistence

Camera preferences, locations and bookmarks merge into `[extra_options]` in the active
per-user `config.toml`. Unrelated settings survive. Malformed files are left unchanged
and an error is shown. Isolated builds honor `BLACKBOX_PROFILE_DIR`.
Reload preferences reads the file explicitly; there is no file watcher.

Testing overrides, pause, freecam, hidden UI and selected bookmarks are session-only.
Named testing profiles are stored in `testing-presets.toml` beside the config. Saving
one does not make it active on startup. Profiles cover traffic, police, heat, charge
and pause controls. Weather uses the separate F8 environment controls.

## F6 presets

A searchable visible list loads a saved car setup with one click through normal garage
validation. Update selected replaces that setup; Save as creates or replaces the entered
name. Management includes rename, delete and refresh. Rename refuses an existing name.
Existing `garage-presets.toml` files remain compatible. Car cards align model/manufacturer
text at the same left inset. Camera settings belong to F5.

The console shares this state: `extra camera` / `extra testing` lists values;
`extra camera auto_center off` saves a preference, while
`extra testing traffic_off on` applies a session override. Unknown keys and
invalid values are rejected. `extra clear` restores the normal testing state.
