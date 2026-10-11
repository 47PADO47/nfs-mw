# Window modes

The main-menu and pause-menu Video options include Window Mode. Left/right
cycles Windowed, Borderless and Exclusive; leaving the settings screen saves
the selection through the existing per-user config writer. Monitor selection
and explicit resolutions remain available through config, CLI and the console.

The application supports `windowed` (default), `borderless` (the monitor's desktop size) and
`exclusive` (a video mode advertised by the selected monitor). This is a new application feature;
it does not claim to reproduce the original game's window-management code. It uses the existing
[Bevy window API](https://docs.rs/bevy_window/0.20.0/bevy_window/enum.WindowMode.html) and
[winit fullscreen support](https://docs.rs/winit/0.30.13/winit/window/struct.Window.html#method.set_fullscreen),
both already permissive dependencies of this project.

## Starting a window

All viewer and movie commands share these options:

```sh
nfsmw view-world --drive --window-mode borderless --monitor primary
nfsmw view-car BMWM3GTR --window-mode exclusive --resolution 1920x1080 --monitor 0
nfsmw view-car BMWM3GTR --window-mode windowed --resolution 1280x720
```

Each setting resolves independently: command line, environment, per-user config file, defaults.
Invalid environment/config values log a warning and fall through; invalid CLI values are rejected.

| CLI | Environment | TOML key | Default |
|---|---|---|---|
| `--window-mode MODE` | `NFSMW_WINDOW_MODE` | `window_mode = "borderless"` | `windowed` |
| `--monitor MONITOR` | `NFSMW_MONITOR` | `monitor = "primary"` (or `monitor = 0`) | `current` |
| `--resolution SIZE` | `NFSMW_RESOLUTION` | `resolution = "1920x1080"` | `native` |

The monitor is `current`, `primary` or a zero-based index in the window backend's enumeration.
An unavailable index falls back to the primary monitor (or the first available monitor).
`current` follows the window's actual monitor. Monitor indices may change after display reconnection.

`resolution` is `native` or `WIDTHxHEIGHT`, with both dimensions between 160 and 16384 physical
pixels. In windowed mode, `native` starts at 1280x720; an oversized request is fitted to the desktop
with a DPI-scaled margin for decorations. In exclusive mode, `native` preserves the desktop
resolution and nearest advertised refresh rate. A specific size selects the highest advertised
refresh rate at that size. Borderless always uses the desktop size.

## Switching during a run

Alt+Enter switches between windowed and the last successfully selected fullscreen mode
(borderless initially). Either Alt key works. The shortcut is inactive while the developer console
is open or the window lacks focus. F12 opens the console:

```text
window_mode borderless
window_mode exclusive
window_mode windowed
monitor 0
resolution 1280 720
resolution native
set resolution 1920x1080
get window_mode
window
monitors
```

`window` reports the actual Bevy window mode, physical/logical size, DPI scale, position, focus and
visibility. `monitors` lists the backend's indices, desktop sizes, scales, refresh rates and counts
of video modes. `get` reports requested preferences; mode fallback updates the mode preference.
As with other console settings, changes last for the run and do not write the config file.

The last windowed physical size and desktop position are retained across fullscreen switches.
Late resize events during the first three restoration frames cannot replace that placement.
Normal moves/resizes then become the next remembered placement. A position on a disconnected
display is recentered. Explicit monitor changes center a windowed window on the new display.
Changing `resolution` while fullscreen also updates the requested size for the next windowed mode.
The OS's DPI scale stays active, and the render bridge resizes the surface to physical pixels.
Losing focus releases mouse capture through the existing cursor system; click to capture again.

## Fallbacks and limits

An exclusive size absent from the selected monitor's video modes falls back to borderless and
logs a warning. Window backends outside Windows, X11 and macOS also use borderless: in particular,
winit's Wayland backend advertises video modes but ignores exclusive fullscreen requests.
The application validates advertised modes; a driver/OS failure while applying an advertised
mode remains a limitation of winit's window API. On macOS, exclusive mode also restricts task
switching; borderless is the usual desktop choice. Linux and macOS behavior is unit-tested at the
selection layer but needs live platform testing.

`--screenshot` always keeps its window hidden, offscreen and unfocused at the fixed capture size
and scale of 1. Window-mode, monitor and resolution CLI/environment/config preferences cannot
change that. Console attempts to change them report an error; the screenshot still completes.
Read-only `window` remains available for confirming the capture state.

## Validation

Unit tests cover parsing/layer precedence, monitor index selection, advertised video modes,
invalid-mode fallback, DPI-aware fitting, position restoration after intermediate resize events,
Alt+Enter/focus/console gating and screenshot isolation. Run `cargo test -p nfsmw app::window`,
`cargo test -p nfsmw settings` and `cargo test -p nfsmw devtools::console`.

For an interactive check, start windowed at 1280x720, move/resize it, toggle Alt+Enter twice and
confirm size/position restoration. Use F12 to switch all three modes and an unsupported exclusive
resolution (for example a size absent from `monitors`' display modes). Alt+Tab out and back, then
resize the window and confirm rendering and UI still work. Check `window` and the runtime log;
test a second monitor/DPI scale when available. Finally request borderless with `--screenshot`
and `--exec "window"`: the report must remain windowed, hidden and at the fixed screenshot size.
