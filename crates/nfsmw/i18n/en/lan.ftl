### lan
### The words LAN play shares between its screens, the session HUD, the session world and the console. The original has
### no LAN screens; where it has the word (the common buttons, the race types, the speed unit) the code shows the
### game's own label instead.

## Modes. Circuit, sprint, drag, speedtrap and lap knockout are the game's race types.

lan-mode-free-roam = Free Roam
lan-mode-pursuit-coop = Pursuit Co-op
lan-mode-cops-vs-racers = Cops vs Racers
lan-mode-tag = Tag
# A mod's own mode, by its number.
lan-mode-custom = Mod mode { $id }

## Where the player is with the network (the Join page's status line, `net.status`).

lan-status-idle = not in a session
lan-status-scanning = looking for LAN games
lan-status-connecting = connecting
lan-status-in-lobby = in the lobby
lan-status-playing = playing
lan-status-spectating = spectating
# $why: why the session ended (lan-end-*), or the system's own words for a failure.
lan-status-disconnected = disconnected: { $why }

## Why a session ended, as a sentence fragment.

lan-end-left = you left
# $why: one of lan-reject-*.
lan-end-rejected = the server refused the seat ({ $why })
# $why: one of lan-link-*.
lan-end-link = the connection ended: { $why }
lan-end-resume-expired = the connection was lost and could not be resumed
lan-end-password-required = the game needs a password
lan-end-password-rejected = wrong password
lan-end-schema-mismatch = the server runs another game build
lan-end-protocol = the server broke the protocol

## Why a server refused the seat.

lan-reject-server-full = the server is full
lan-reject-protocol-mismatch = another protocol version
lan-reject-data-mismatch = other game data
lan-reject-mod-mismatch = other mods
lan-reject-banned = banned
lan-reject-in-progress = a race is running
lan-reject-loadout-rejected = the car was refused
lan-reject-rate-limited = too many tries
lan-reject-server-shutting-down = the server is shutting down
lan-reject-sim-build-mismatch = another game build

## Why the connection closed. $code is the protocol's reason number.

lan-link-remote = closed by the other side (reason { $code })
lan-link-local = closed here (reason { $code })
lan-link-timeout = timed out
lan-link-overflow = the other side stopped acknowledging
lan-link-shutdown = shut down
# $why: one of lan-handshake-*.
lan-link-handshake = handshake failed: { $why }
# $why: one of lan-packet-*.
lan-handshake-packet = malformed handshake packet: { $why }
lan-handshake-noise = handshake failed (key or version mismatch)
lan-handshake-random = the system random source failed
lan-handshake-config = unusable handshake configuration
lan-handshake-no-answer = no answer
lan-handshake-full = host full
lan-handshake-protocol = protocol version not supported by the host
lan-handshake-shutting-down = host shutting down
lan-handshake-rejected = refused by the host (reason { $code })
lan-packet-bad-length = wrong length
lan-packet-bad-type = wrong packet type
lan-packet-bad-version = unknown format
lan-packet-bad-padding = padding not zero
lan-packet-bad-field = bad length field

## Why the server removed the player.

lan-kick-kicked = you were kicked
lan-kick-banned = you were banned for { $minutes } min
lan-kick-violations = removed for invalid messages
lan-kick-idle = removed for being idle
lan-kick-voted = voted out

## A server's line in the Join page's list: the mode and the head count, then the round trip and the flags.

lan-server-unknown-mode = Unknown mode
lan-server-players = { $mode }  { $players }/{ $max }
lan-server-locked = locked
lan-server-racing = racing
lan-server-other-version = other version
# A round trip in milliseconds.
lan-ping = { $ms } ms
# A player the lobby has no row for, by seat number.
lan-slot = slot { $slot }
lan-someone = someone

## The server's notices. $who is a player's name; $place a place as format-ordinal writes it.

lan-notice-join = { $who } joined
lan-notice-leave = { $who } left
lan-notice-kick = { $who } was kicked
lan-notice-mute = { $who } was muted
lan-notice-unmute = { $who } can chat again
lan-notice-vote-passed = the vote passed
lan-notice-vote-failed = the vote failed
lan-notice-vote-started = a vote started
lan-notice-finished = { $who } finished
lan-notice-finished-place = { $who } finished { $place }

lan-votes = { $n ->
    [one] { $n } vote
   *[other] { $n } votes
}

## The chat and notice feed. $text is the player's own line.

lan-chat-all = { $name }: { $text }
lan-chat-team = { $name } (team): { $text }
lan-chat-whisper = { $name } (whisper): { $text }
lan-feed-session-over = Session over: { $why }
lan-feed-connection-lost = Connection lost, trying to resume
lan-feed-connection-resumed = Connection resumed
lan-feed-won = { $name } won ({ $place } of { $racers })
lan-feed-knocked-out = { $name } is knocked out (lap { $lap })
# $unit: the game's speed unit label.
lan-feed-speed-trap = { $name }: speed trap { $trap }, { $speed } { $unit }
lan-feed-tag = { $name } is it
lan-feed-penalty = { $name }: +{ $seconds } s for the reset

## The preset quick lines, in their protocol order.

lan-quick-police-behind = Police behind me!
lan-quick-roadblock-ahead = Roadblock ahead!
lan-quick-wait-for-me = Wait for me
lan-quick-go-go-go = Go go go!
lan-quick-nice-one = Nice one
lan-quick-sorry = Sorry!
lan-quick-good-race = Good race
lan-quick-ready-when-you-are = Ready when you are

## A race result row: the best lap and the flags, between the separator.

lan-result-best = best { $time }
lan-result-dnf = DNF
lan-result-busted = busted
lan-result-knocked-out = knocked out
lan-result-left = left
lan-result-separator = {", "}

## The session HUD: the running vote and the spectator's panel.

# $what: one of lan-hud-vote-kick and the others below; $votes: lan-votes.
lan-hud-vote = Vote: { $what }  { $votes }
lan-hud-vote-kick = kick { $name }
lan-hud-vote-restart = restart
lan-hud-vote-change-mode = change the mode
lan-hud-vote-change-track = change the track
lan-hud-vote-skip-results = skip the results
lan-hud-vote-left = { $seconds } s left
lan-hud-vote-key = ({ $key } to vote)
lan-hud-free-camera = Free camera
lan-hud-following = Following { $name }
lan-hud-spectating = Spectating

## The session world's message line.

lan-net-race-start = { $racers ->
    [one] GO! { $racers } racer
   *[other] GO! { $racers } racers
}
lan-net-race-finish = Car { $car } finished in place { $place }
lan-net-pursuit-start = Pursuit! Heat { $heat }
lan-net-pursuit-busted = Pursuit over: Busted
lan-net-pursuit-evaded = Pursuit over: Evaded
lan-net-pursuit-abandoned = Pursuit over: Abandoned
lan-net-player-left = Player { $player } left
lan-net-race-over = { $finishers ->
    [one] Race over: { $finishers } finisher
   *[other] Race over: { $finishers } finishers
}

## Session settings: why a change was refused, and each field's name (also the Host page's rows).

lan-settings-invalid = { $field } is not valid with these settings
lan-settings-locked = { $field } cannot change during a race
lan-settings-below-count = { $field } is below the players already in
lan-settings-too-large = the settings are too large
lan-settings-not-permitted = only the host or an admin may change the settings
lan-settings-no-password = a password is required but none is set
lan-settings-unknown-key = unknown setting
lan-field-name = Room name
lan-field-max-players = Player limit
lan-field-max-spectators = Spectator limit
lan-field-mode = Mode
lan-field-track = Track
lan-field-laps = Laps
lan-field-traffic-density = Traffic density
lan-field-cop-level-cap = Police
lan-field-pursuits = Pursuits
lan-field-collision = Collision
lan-field-catch-up = Catch-up
lan-field-nitrous = Nitrous
lan-field-damage = Damage
lan-field-allowed-cars = Allowed cars
lan-field-join-in-progress = Join in progress
lan-field-password-required = Password required
lan-field-public = Public
lan-field-sim-build-match = Same game build

## Names, rooms and addresses.

# The room's name when none is typed. $player is the player's name.
lan-default-room = { $player }'s game
lan-room-name-bad = not a room name (1 to 32 characters, no control characters)
lan-name-bad = not a player name (1 to { $max } characters, no control characters)
lan-address-bad = "{ $address }" is not an address (an IP, optionally with :port)
lan-unavailable = LAN play is not available in this run
