# The original game's race netcode (behaviour notes)

What the stock PC `speed.exe` (1.3) shows about how a networked race was synchronised: the message catalogue, the
host and client state machines, the per-frame session update, the bit stream writer, the timing check and the
developer direct-connect settings. This is a **reference**, not something to implement: [ADR 0006](../decisions/0006-multiplayer-netcode.md)
section 15 keeps nothing of the original protocol, and [ADR 0009](../decisions/0009-multiplayer-netcode-revisions.md)
says what these notes changed in our design. The Ghidra trail is in [provenance](../provenance/original-race-netcode.md).
Evidence tags as in [reversing.md](../reversing.md).

It complements [LAN server](lan-server.md), which covers the lobby (`server.dll`, the HTTP peer lookup) and the
peer UDP channel (`COb`, `PNG`, `DSC`, `MIC`). The race messages below are a layer above that channel; their payload
is most likely what travels inside `MIC` **[unconfirmed]**.

## 1. Message catalogue

The executable keeps a table of message names next to the state names. Nothing in the code refers to it, so it is
a debug name table left in the release build **[verified]** (no references, no pointer to any of the strings).
The names follow one scheme **[decomp]**: `MSG_<delivery>_<direction>_<name>`.

| Part | Values | Meaning |
|---|---|---|
| Delivery | `R`, `U` | Reliable, unreliable |
| Direction | `SC`, `CS`, `BI` | Server to client, client to server, both ways |

| Message | Delivery, direction | Reading **[unconfirmed]** unless marked |
|---|---|---|
| `WELCOME` | R, SC | The host accepts a client |
| `CARDESCRIPTION` | R, BI | Each machine describes its car (model and customisation) |
| `DATACRC` | R, BI | A checksum of game data, so that machines with different data do not race |
| `RESTARTLOAD` | R, SC | The host restarts the load (race restart) |
| `SSTATECHANGE` | R, SC | The host's state changed (section 2) |
| `CSTATECHANGE` | R, CS | A client's state changed |
| `CLIENTLEFT` | R, SC | Another client left |
| `SERVERQUIT`, `CLIENTQUIT` | R, SC / CS | Orderly shutdown |
| `DRIVERFINISH` | R, BI | A driver crossed the finish |
| `STARTRACESYNC` | U, BI | Start-of-race synchronisation |
| `SYNCANIMATION` | U, SC | Keeps an animation (the intro or the countdown) in step |
| `SCORE` | U, BI | Race standing |
| `CLOCKSYNC` | U, CS | A clock sample from a client |
| `CARSPAM` | U, BI | Car state, sent repeatedly by **every** machine |
| `CARSPAMCLOCKSYNC` | U, SC | Car state with a clock sample attached |

**What the catalogue says about the model** **[unconfirmed]**: car state flows both ways (`BI_CARSPAM`), so each
machine simulates and owns its own car and broadcasts it. That is option C in ADR 0006 ("P2P with each peer owning
its car"), with a host that owns the session and the clock. Our design keeps the server authoritative over every car
(ADR 0006, authority option 1), mainly for contacts between players and for cheating; the original's choice fits a
4-player race without traffic sync.

## 2. State machines

Names from the same table **[verified]** (strings), order from their layout and from the update code **[decomp]**:

- **Host:** `INITIAL` → `WELCOME` → `WAITJOINS` → `LOADING` → `WAITLOADS` → `NIS` → `READY` → `RACING`.
- **Client:** `INITIAL` → `WAITING` → `CONNECTED` → `DESCRIBED` → `LOADING` → `NIS` → `READY` → `RACING`.

`NIS` is the in-game cinematic (the race intro). The host waits twice: for joins, then for every client's load
(`WAITLOADS`), before all machines play the intro and start together. A client is `DESCRIBED` once its car
description has been exchanged.

## 3. The per-frame session update

One function runs every frame from the online tick, after the lobby callbacks **[decomp]**. The session object has
a small integer state (values 1 to 6 seen); a reset routine puts it to 1, then 2.

- **Clock slew** (racing state, clients only) **[decomp]**: the client keeps an offset to the host clock (in
  milliseconds **[unconfirmed]**). Each frame it compares the offset with its target. If they differ by more than 100,
  it jumps. Otherwise it moves toward the target by at most 3 per frame when ahead, and 1 per frame when behind
  (differences of -1 to 3 are taken whole). So small errors are slewed, large ones stepped, and the clock is slower to
  catch up than to fall back.
- **Timers** **[decomp]**, on a counter that ticks at 4,000 per second (it is scaled by 0.00025 to seconds):
  - every 4 s (host only): sends a short status message (an 8-bit field and a time value);
  - every 20 s: sends an 8-bit and a 32-bit field taken from the local player's record (a checksum or a counter
    **[unconfirmed]**), and flags the player if the 32-bit value changed against a stored copy;
  - every 2 s: the timing check of section 5.
- **Load timeout** **[decomp]**: in the loading state, a client gives up after 140 s and the host after 120 s. A
  second check in the same state fires after 125 (units not identified).
- **Below the world** **[decomp]**: a racing car whose height drops under -3.0 flags its player and drops the
  connection of the others with reason codes 3 and 11 (reading of the arguments **[unconfirmed]**).

## 4. The bit stream

Messages are written with a bit writer over a 1,024-byte buffer **[decomp]**: a cursor counts bits, values are written
most significant bit first with an explicit bit count (8 and 32 seen), and a value that starts inside a byte is split
across bytes. The same layout as the bit writer of our protocol ([protocol](multiplayer/protocol.md)); nothing of the
original's code is used, and bit packing is general practice.

## 5. The timing check and cheat counters

Every 2 s the game compares how far two of its clocks moved (both on the 4 kHz counter). If they disagree by more than
0.5 s over the window, it counts a strike against the player **[decomp]**. Strikes are kept per player as 16-bit
counters indexed by a reason code (1 for timing, 4 and 5 seen elsewhere), saturating at `0xFE01` **[decomp]**. Settings
named `MIN_CHEAT_THRESHOLD`, `RESPONSE_GRAPH` and `RESPONSE_REMAP<n>` are read from a lobby-provided config into a
response table **[decomp]**; what the response is was not traced.

Our design does this on the server instead: the server owns the clock and every car, so a speed hack has nothing to
act on ([security-gameplay](multiplayer/security-gameplay.md)). The idea kept is the per-reason strike counter for logs.

## 6. Developer direct connect

A config parser reads keys that set up a session without the lobby **[decomp]**:

| Key | Effect |
|---|---|
| `ISSERVER` | This machine hosts |
| `NUMCLIENTCARS` | Number of clients the host waits for |
| `SERVERIP`, `SERVERPORT` | Where a client connects |
| `CLIENTIP0` to `CLIENTIP2` | The clients a host adds (3 slots, so 4 players) |
| `AICONTROL` | Cars driven by the AI (a test aid) |
| `NEWCARSMOOTHING` | Stored, never read (an abandoned smoothing switch) |
| `ONLINEENABLED`, `ONLINEDEBUGDRAW`, `NetworkDebug` | Feature and debug switches |

The host names itself `TheHost` and the clients `Client<n>`, then waits until the expected number of clients has
joined **[decomp]**. The online code reserves a 215,040-byte (`0x34800`) buffer **[decomp]**.

## 7. What our design takes from this

Ideas only, in our words ([ADR 0009](../decisions/0009-multiplayer-netcode-revisions.md)):

- An explicit session state machine with a load barrier, an intro and a synchronised start (section 2).
- A load timeout (default 120 s) after which a stuck client is dropped instead of holding everyone.
- Clock correction that slews small errors and steps large ones (ADR 0006 section 11 already had this).
- A data check on join (our `data_id`) and a car description message (our loadout).
- Per-reason strike counters for the server log.

## Open questions

- The `CARSPAM` payload, rate and quantisation (the sender was not found; the names have no references).
- Whether race messages travel inside the `MIC` datagrams of [LAN server](lan-server.md).
- The units of the clock offset and of the 125 threshold.
