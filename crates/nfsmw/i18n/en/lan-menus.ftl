### lan-menus
### The LAN menu's pages: Home, Host, Join, the lobby and its player menu, the session settings and the results. The
### rows the game has its own label for (the common buttons, ready, the password, the laps) are not here.

## Page titles.

lan-title-home = LAN Game
lan-title-host = Host a LAN Game
lan-title-join = Join a LAN Game
lan-title-settings = Session Settings
lan-title-results = Race Results

## Home.

lan-home-subtitle = Playing as { $name }
lan-home-host = Host a Game
lan-home-join = Join a Game
lan-home-name = Your name

## The Host page and the session settings. The field rows are lan-field-* (lan.ftl).

lan-host-subtitle = Port { $port }  as { $name }
lan-track-city = Whole city
# An empty value: no track for the mode, no password.
lan-none = none
lan-host-heat = Up to heat { $heat }
lan-host-apply = Apply
lan-host-start = Start Hosting

## The Join page. $status is one of lan-status-* (lan.ftl).

lan-browser-found = { $n ->
    [0] No games found yet  ({ $status })
    [one] { $n } game found  ({ $status })
   *[other] { $n } games found  ({ $status })
}
lan-browser-refresh = Refresh
lan-browser-type-address = type an address
lan-browser-direct = Direct connect
lan-browser-other-version = { $name } runs another version of the game
lan-browser-needs-password = this game needs a password

## The lobby.

# The status line: the mode, then the head count over the limit.
lan-lobby-players = { $mode }  { $players }/{ $max } players
lan-countdown = Race starts in { $seconds }
lan-lobby-car = Car
lan-lobby-role = Role
lan-lobby-say = Say
lan-lobby-quick = Quick message
lan-lobby-vote-restart = Vote to restart
lan-lobby-settings = Session settings
lan-lobby-hold = Everyone wait
lan-lobby-start = Start race
lan-lobby-leave = Leave
lan-lobby-voting-kick = Vote: kick { $name }
lan-lobby-voting-restart = Vote: restart
lan-lobby-voting-mode = Vote: mode { $mode }
lan-lobby-voting-track = Vote: track { $track }
lan-lobby-voting-skip-results = Vote: skip results

## The roster: the badges after a player's name and the player's state.

lan-badge-host = {"["}host]
lan-badge-admin = {"["}admin]
lan-badge-muted = {"["}muted]
lan-badge-you = (you)
lan-player-watching = watching
lan-player-ready = ready
lan-player-not-ready = not ready
lan-role-racer = Racer
lan-role-cop = Cop
lan-role-spectator = Spectator

## The session's phase in the lobby's status line (lobby, loading and waiting are the game's labels).

lan-phase-intro = Intro
lan-phase-countdown = Countdown
lan-phase-running = Running
lan-phase-results = Results

## One player's menu.

lan-player-whisper = Whisper
lan-player-mute-local = Mute for me
lan-player-unmute-local = Unmute for me
lan-player-vote-kick = Vote to kick
lan-player-kick = Kick
lan-player-mute-session = Mute in session
lan-player-unmute-session = Unmute in session
lan-player-ban = { $minutes ->
    [one] Ban for { $minutes } minute
   *[other] Ban for { $minutes } minutes
}

## The footer: what a key does, and the names of key groups (single keys keep their own names).

lan-hint-select-or-type = Select / type
lan-hint-select-or-say = Select / say
lan-hint-change = Change
lan-hint-move = Move
lan-hint-stop-typing = Stop typing
lan-key-arrows = Arrows
lan-key-left-right = Left/Right
