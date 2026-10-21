# Prop lifetime evidence

The private PC image uses the executable identity recorded in
[ground-contact response](ground-contact-response.md). Inspected entry points:
`0x66AF60` (scenery offscreen countdown), `0x677830` (simulation start/hide),
`0x669F50` (dropout start), `0x66A0B0` (sinking tick), `0x670A30` (rest policy),
`0x66A170` (off-world clock), `0x66B0D0` (simulation end), and `0x67BE90`
(placed-model destructor). `RESPAWN_TIME` has no immediate key reference in the
checked text section. The CC0 reference independently establishes detached-part
time accumulation and parent/child model release. These are isolated routine and
source findings, not a live route comparison. Frustum visibility and downward
world-face queries are rewrite adapters with explicit remaining limits.

Six countdown cases were executed in isolated x86 memory. With allowance 1.0,
unseen 0.75 leaves 0.25, visible 0.25 reloads 1.0, and unseen 0.3 from 0.25
expires and clamps to zero. Exact equality, an already-zero timer and a negative
timer do not expire. The boundary regression preserves this PC distinction from
the separate detached-part accumulator.

Streaming follow-up (2026-10-21), same hashed PC executable: section destruction
`0x67BF80` records simulated time and up to 2048 scenery override bits before
deleting models. Section initialization `0x6828D0` reuses those bits unless the
strict `unload_time + 180 < current_time` test expires them. The 180-second value
is a directly read float; this is unrelated to `RESPAWN_TIME`. The race-status
`ActivelyRacing` byte bypasses expiry: setter `0x604050` writes that field and
Lua registration `0x61E750` binds it as `SetActivelyRacing`. Constructor
`0x67BC20` takes the saved hidden flag: it sets the
override bit instead of running `RestoreScene` (`0x6778F0`). Thus reload must
not re-run event sequences or recreate detached pieces. `0x6828D0` restores a
section's pursuit-breaker icons only when no hidden bits remain. Explicit global
restoration (`World_RestoreProps`, `0x74D320`) resets models, explosions, icons,
animations and the section cache (`ResetPropTimers`, `0x672200`). These findings
are static observations, not an original-game drive-around timing measurement.

Resident follow-up (2026-10-21): the scenery `IModel::OnProcessFrame` adapter at
`0x66AFE0` invokes the model's offscreen routine, then calls `ReleaseModel` on
expiry. This confirms resident model release can restore a placement; section
persistence is not an exemption from authored resident cleanup. Scenery
`SpawnModel` (`0x677540`) constructs the placeable-scenery base (`0x6722C0`),
whose simulation-end callback (`0x66B0D0`) zeroes the countdown. Vehicle parts
instead retain `DrawVehicle::Part::mOffScreenTask` and accumulate invisible time
after rest, independently confirmed by its CC0 source. These distinct classes
must not be selected merely by membership in the rewrite's fragment map.
Scenery `ESpawnFragment` no-physics creation starts no body simulation; the model
and its parent association remain. Tests cover both kinds after rest, including
the separate no-physics path. The visible dropout gate remains an explicit
rewrite adapter; its pending-cleanup regression fixes the adapter's own task
deactivation, not a claim about original visibility policy.

The CC0 `SimModel.cpp::Model::HideModel` ends drawing and simulation without
releasing children or the model. PC `Smackable::OnDetached` at `0x67B760`, reached
through its attachable vtable, clears the detached model reference and kills its
physical simulation unless already killed. This establishes why a hidden parent
does not keep colliding or expire into its intact placement. The fragment
callback at `0x63B320` invokes `IModel::HideModel` for `HideParent`; it is not just
a drawing flag. The rewrite now shares this transition between both hide paths
while retaining explicit model release as a separate operation.
