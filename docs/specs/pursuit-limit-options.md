# Optional police limits

These are remake enhancements. Reference pursuit behavior remains the default.
The F8 Traffic panel groups them under **Police limits (enhancement)**.

- `enhanced`: off by default; controls whether either limit override takes effect.
- `max_active_cops`: 8 to 64, initially 16 for the optional mode. Applies to all
  active police cars: patrols, pursuing cars, roadblocks and support together.
- `heat_cap`: zero preserves event/career limits; 1 to 10 overrides their maximum.
  The authored final pursuit remains fixed at six. Special heat rows are exposed
  as an explicit enhancement without claiming they are ordinary career tiers.

The active-car override does not alter the wave's composition or the finite
evasion/backup budget. Increasing it permits the existing heat row and support
requests to use available room. It does not promise 64 active cops at every heat.
Disabling the override restores the configured reference/INI cap, and restoring
event heat bounds clamps an active pursuit without resetting its statistics.

Settings apply through the existing traffic-settings propagation path and use
the `[pursuit_options]` TOML table. They participate in F8 Save for startup,
named presets, A/B restoration and Traffic Reset section. Old files default to
disabled; unknown fields survive a write. Invalid field types do not partly apply.

The development Heat buttons remain explicit forced starts rather than saved
heat-progression overrides. Changing only the active-car limit preserves their
forced range. An enabled, nonzero heat cap takes precedence over a development
start; the final pursuit's fixed heat six takes precedence over both. Leaving
an event clears the development override for the restored event/career bounds.
No original-game installation files are changed.
