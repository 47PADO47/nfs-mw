# game-install

Find, validate and read a user's own install of a (Windows-era) game, for reimplementations that load the
original data at runtime. Game-agnostic: a `GameSpec` supplies the environment variable, registry values,
required files and known executable hashes. Lookup order: explicit path, environment variable, `.env`,
per-user config file, Windows registry. `GameDir` gives case-insensitive access so the same code works on
Linux.

License: MIT OR Apache-2.0.
