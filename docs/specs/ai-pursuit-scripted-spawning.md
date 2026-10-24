# Scripted cop spawning and lane occupancy

Sources: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
decompiled; `AI/Activities/AICopManager.cpp` `TrySpawnCop`, `UpdateSpawnRequests`,
`SpawnPursuitIVehicle`; `AI/Common/AISpawnManager.cpp` `CheckSpawnPosition`.
Read for facts only; no code copied. Complements [ai-pursuit-cops.md](ai-pursuit-cops.md).

A scripted request remains queued until its model and placement are available.
An `InPursuit` request creates or joins the local player's pursuit; absence of a
pursuit is not a reason to delete it. A separate roadblock role creates a
stationary cop. Failed placements retain the request. Requests cancelled when
the chase ends must not restart that finished chase. **[decomp]**

The remake loads assets through its scene, so a ready chase request first asks
that scene to construct the pursuit. Readiness requires spawning enabled, a
loaded requested model, no overlap, a valid road cursor and supported physical
placement at its authored location. A preflight uses the same chassis placement
query without allocating a render key or advancing the spawn random stream. The queued cop
is then placed through the normal service pass. The scene starts only one chase
and does not require speeding, sight or a 911 call for this explicit request.
An invalid/off-world ground placement still remains pending; the original's
simple-physics fallback is not claimed here. **[ours]**

Ordinary patrol spawning treats a moving traffic car's occupied lane and its
destination lane as occupied until the change completes. With matching travel
node, either lane requires fifteen metres; a different lane requires five.
Wrecked traffic cars always require fifteen metres and non-traffic drivers
require forty. Lane ids and the travel node are separate tests. **[decomp]**

The pursuit seek-walk placement reverses the road cursor at the end of its walk.
The disc fallback instead initializes its road cursor along the target's planar
travel direction, using body forward below one metre per second. It does not
point every spawn directly at the target. Support and authored placements keep
their existing explicit facing rules. **[decomp]**

The opening script uses its existing `cop_spawn_1` and `cop_spawn_2` markers.
`CopsOn` remains active across preparation frames until `NoNewPursuitsOrCops`;
that action cancels unplaced requests. See [story-flow.md](story-flow.md) section
2.4 for the marker and script evidence. The inferred PC active cop budget and
undocumented infraction rules remain separate research questions.
