# Context-specific minimap modes

The recorded modded-PC Gameplay menu offers Free Roam Map Mode and Race Map Mode. Existing original research in
[hud-minimap](hud-minimap.md), section 6, defines Fixed, Rotating and Off, with fresh defaults Fixed in free roam
and Rotating in races. This change exposes the existing minimap effects; it adds no new map rendering mode.

## Layer precedence and compatibility

Config keys `free_roam_map_mode` and `race_map_mode` independently override the legacy `minimap` key in the same
layer. Missing or invalid context values inherit that layer's legacy key, then lower layers. A higher-priority
legacy key overrides lower-priority context keys, preserving existing higher-layer `minimap` and `NFSMW_MINIMAP` behavior. An unrelated
higher layer must not erase lower context values. Defaults apply only after all layers merge. A legacy-only config
therefore keeps the same mode in both contexts. A fresh profile uses the documented per-context defaults.
Resolved Settings materializes both choices so a full alias save and reload preserves the resolved state exactly.

The old global HUD Map Orientation row, console `set minimap`, and session-only orientation key remain global:
they set both contextual modes together. Gameplay rows edit only their own context and persist only that key.
The global `minimap` field remains the legacy/global control value, not a summary of two differing selections.

## Runtime selection

Single-player HUD telemetry selects Race while drag telemetry, a race object, race stage or race-specific map source exists;
otherwise it selects Free Roam. A normal free-roam pursuit still uses Free Roam. This covers countdown, ordinary,
drag and pursuit-race map sources without relying on whether the event tiles have finished loading.
The contextual mode controls Original/Remastered minimap visibility and orientation through the shared state.
Custom HUD retains its independent map-layout/visibility policy and obtains orientation from that shared state,
as before. Its expanded-map and orientation shortcuts retain their existing scope. Split-screen's north-up shared
map remains its separate existing layout behavior; this change does not redefine that interface.

## Acceptance

Verify both rows in main and pause menus, both directions, independent persistence, global-control behavior,
free-roam/race transitions, Off and return to free roam. Unit/runtime-state checks do not certify recorded menu
appearance, physical controller behavior, or exact reference defaults in mod-altered profiles.
