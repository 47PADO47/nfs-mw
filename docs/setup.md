# First-time setup

`nfsmw setup` is a short questionnaire for new users. It finds the game, asks two questions and saves the
answers to the per-user config file, so you don't have to edit a file by hand.

```sh
./target/release/nfsmw setup
```

On Windows: `target\release\nfsmw.exe setup`.

## What it asks

1. **Where the game is installed.** Type the folder that contains `speed.exe`, for example
   `/home/you/Games/Need For Speed Most Wanted Black Edition`. `~/` is expanded to your home folder, and
   quotes around a dragged-in path are removed. The folder is checked with the same test as
   `check-install`: if required files are missing, setup says which ones and asks again. If an install is
   already found (from `--game-dir`, `$NFSMW_GAME_DIR`, `.env` or an earlier config), that folder is
   offered in brackets; press Enter to accept it.
2. **Window mode.** Choose one of:
   - `1` windowed (the default),
   - `2` borderless (a window that covers the desktop without changing the display mode),
   - `3` fullscreen (the game's exclusive mode; on Wayland it falls back to borderless, see
     [window modes](window-modes.md)).

   You can change the window mode later with Alt+Enter in the game, or with `--window-mode` on the command line.

Press Enter at any question to accept the default shown in brackets.

## What it saves

Setup writes two keys to the config file and keeps every other key that is already there:

```toml
game_dir = "/home/you/Games/Need For Speed Most Wanted Black Edition"
window_mode = "windowed"
```

The config file is:

| Platform | Path |
|---|---|
| Linux | `~/.config/nfsmw/config.toml` |
| Windows | `%APPDATA%\nfsmw\config\config.toml` |

`nfsmw check-install` prints the path it used, and says `found via` the config file when the install comes
from there.

## Running it again

Run `setup` again at any time to pick another folder or window mode. The answers replace the saved values.
If the folder given to setup has no usable install, nothing is saved.

Setup needs a terminal to answer in. If it stops with "no answer", run it from an interactive terminal, not
from a script or a launcher that has no input.

## Skipping setup

Any of these sets the install folder without setup. The first match wins:

1. `--game-dir PATH` on the command line,
2. the `NFSMW_GAME_DIR` environment variable,
3. a `.env` file with `NFSMW_GAME_DIR=...` next to the program or in the working folder,
4. `game_dir` in the config file,
5. the retail registry key (Windows only).

Window mode can be set with `window_mode` in the config file, the `NFSMW_WINDOW_MODE` environment variable
or `--window-mode`. See [window modes](window-modes.md) for the full list of settings.

## Troubleshooting

- **"that folder is missing: ..."**: the folder doesn't contain a complete install. Check that it is the folder
  with `speed.exe` in it and that the game is fully installed.
- **The game says it cannot find the install**: the config file wasn't written, or another setting points
  elsewhere. Run `nfsmw check-install` to see which config file and folder it uses.
- **Build fails with `libudev` or `alsa` not found** (Linux): install the development packages, for example
  `libudev-dev` and `libasound2-dev` on Debian and Ubuntu based systems, then build again.
