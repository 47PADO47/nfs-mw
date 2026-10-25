# Joining a LAN game from Discord

Status: built 2026-10-25 (`feat/discord-join`), not yet tried with two Discord accounts. The presence is
`crates/nfsmw/src/discord/`, the game's side `crates/nfsmw/src/net/discord.rs` and `NetSession::lan_game` /
`NetSession::join_from_discord` (`net/session.rs`). The multiplayer it joins is [game wiring](game-wiring.md); the
settings are in [front end](frontend.md).

## 1. What a friend sees

While the player is seated in a LAN game, Discord shows "In a LAN game" with a party (`2 of 8`). When the room is open, the
party carries a join secret: friends see **Ask to Join**, and the player can **Invite** a friend from Discord. A friend who
accepts gets a **Join** button; pressing it makes their Discord hand the secret to their running game, which joins the server.

The game is not started by Discord: the friend must have it running (Discord launches a game from a URI scheme the game does not
register). The secret is the server's address, `ip:port`.

## 2. The party

| Part | Value |
|---|---|
| Party id | `nfsmw:<ip:port>` of the server |
| Size | seated players and the room's `max_players` |
| Join secret | `ip:port`, only when the room is **open**: this PC hosts it, it has no password, and a seat is free |
| Address | the `net_public_address` setting when set, else this PC's LAN address (looked up by routing a UDP socket toward a documentation address; nothing is sent; refreshed every 10 s), with the listen server's port |

A player who only joined a game shows the party and the server's address but no secret: the host offers the game, and a guest's
LAN address would not reach it. Over the internet the host sets `net_public_address` to a public or VPN address and forwards the
game port (online play is [milestone 11](online.md)).

`discord_join` (default on, dimmed while `discord` is off) switches the party and the secret off; the status line stays.

## 3. Taking a join

`DiscordJoins::take` drains the secrets friends' Discords sent. A secret is **data from another player**: it is accepted only
if it parses as a socket address (`join_address`): no host name is looked up and nothing else is acted on. The game then
`NetSession::join`s it with no password; the LAN menu follows the session's status and opens the lobby. An invite is ignored
(and logged) while the player hosts or is seated in a game, or while `discord` or `discord_join` is off.

## 4. Join requests

Discord sends **Ask to Join** as a request from a user id. The presence worker answers it from what the presence shows: with an
invite when the party offers a secret, else it declines. The room's state is the game's last report (one frame old); the
server still refuses a full or locked room. A request is answered once and never kept for a later presence.

## 5. The Discord side

`link.rs` uses the `discord-presence` crate (the `discord-rich-presence` crate cannot read events). The crate runs the pipe on its
own thread: the link queues the activity (Discord's rate limit is 15 s between updates), subscribes to `ACTIVITY_JOIN` and
`ACTIVITY_JOIN_REQUEST` after connecting, and the callbacks only pass what they get on. Closing the link ends the activity (the
old `clear_activity` call is gone). The crate's own log lines are off in the default filter. The worker keeps its own pace for
reconnects (every 15 s while Discord is not running).

## 6. Not built

- Starting the game from Discord's Join button (a registered URI scheme and its launch arguments).
- Spectating from Discord (`secrets.spectate`).
- Names or a relay: the secret is a raw address, so friends outside the LAN need the host's port forwarded or a VPN.
- A prompt before accepting an Ask to Join: an open room accepts every request. A password makes a room closed.
