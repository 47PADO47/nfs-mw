# game-install

Find, validate and read a user's own install of a (Windows-era) game, for reimplementations that load the
original data at runtime. Game-agnostic: a `GameSpec` supplies the environment variable, registry values,
required files and known executable hashes. Lookup order: explicit path, environment variable, `.env`,
per-user config file, Windows registry. `GameDir` gives case-insensitive access so the same code works on
Linux.

For an isolated test profile, set the process-local `BLACKBOX_PROFILE_DIR` to an
absolute directory. Config then uses `<root>/<app_dir>/config/config.toml` and
logs use `<root>/<app_dir>/logs` on every platform. This overrides OS known-folder
discovery without changing the registry or the normal user profile. Existing
installation lookup precedence is unchanged. Unset the variable to restore the
normal paths. A relative or empty override resolves neither path, so it cannot
silently write into the normal profile. These functions do not create folders.

License: MIT OR Apache-2.0.
