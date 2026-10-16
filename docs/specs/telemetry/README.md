# Telemetry server (milestone 13)

Design for an optional local telemetry server: a fast, read-only REST snapshot, live streams for dashboards and
overlays, session recording, and opt-in control endpoints. Designed now, not built. Written 2026-10-16. The
same design is meant to be extended later for LAN and online multiplayer.

Read in this order:

1. [features.md](features.md): scope, the priority legend (P0, P1, P2), the feature catalogue and the summary.
2. [channels.md](channels.md): every data channel with its kind (snapshot, stream or event), rate, unit, real
   source symbol and build status.
3. [transport-and-privacy.md](transport-and-privacy.md): threads, the lock-free hand-off, endpoints and formats,
   the 0.2 ms game-thread budget, privacy defaults and origin checks.
4. [control.md](control.md): the opt-in write endpoints, the shared command table, and how it ties to the mod
   API (milestone 12).
5. [multiplayer-reuse.md](multiplayer-reuse.md): what carries over to LAN and online play, and what must be
   decided now so that it does.

Related: [roadmap](../../roadmap.md) (milestones 12, 13 and 9 to 11), [LAN server launcher](../lan-server.md)
(the original `server.dll` lobby, a different protocol), [architecture](../../architecture.md) (crate layout).

Rules that apply here: the game data sources are named from the code as it is today. A channel that has no
source yet is marked not built, not invented. Engine-generic transport goes in `libs/` with no NFS:MW names;
game channels and the command table go in `crates/nfsmw`. Status of every item is in the catalogue.
