### menus

## Menu entries and headings the original has no name for (frontend/icon_menu/vertical.rs, frontend/widget_menu.rs).

# The option category with the sensitivity and camera rows, beside the key list's Controls.
menus-control-options = Control Options
# The pause menu's entry that ends a co-op session.
menus-end-coop = End Co-op
# The option category of the installed mods' settings.
menus-mods = Mods
# The option category of the Custom HUD's options.
menus-hud = HUD
# The safe house's entry for one Blacklist rival's stage; $stage is its number, 15 down to 1.
menus-blacklist-stage = Blacklist { $stage }

## The Remastered main menu (ui/theme/remastered/menu/mod.rs): the tagline's two lines, shown in capitals.

menus-tagline-first = Race your way
menus-tagline-second = to the top

## The EA Trax jukebox (frontend/jukebox.rs).

menus-jukebox-no-songs = No EA Trax songs in this install
# The Remastered list's line about the playlist.
menus-jukebox-order-random = Random order
menus-jukebox-order-sequential = Sequential order
# The same while Re-Order is on; $order is one of the two lines above.
menus-jukebox-reordering = { $order }, re-order on

## Outside the game's window (discord/presence.rs, app/pacing.rs, frontend/flow_coop.rs).

# The line Discord shows under the application's name.
menus-presence-playing = Playing NFS: Most Wanted
# The line while the player is seated in a LAN game.
menus-presence-lan = In a LAN game
# The window's title, refreshed each second: $scene is the scene's name, $renderer the graphics backend, $fps the frames
# of the last second, and $status the scene's own status (empty, or starting with " - ").
menus-window-title = { $scene } - { $renderer } - { $fps } fps{ $status }
# Player 1's name in a co-op session when no career profile is loaded.
menus-player-fallback = Player
