# Provenance: cop damage and the cops-wrecked counter (E8b)

What was checked in the install for the cop damage model and the pursuit board's counter, and what was not.
The rules and numbers are in [ai-pursuit-cop-cars.md](../specs/ai-pursuit-cop-cars.md) section 6 and 2.2; the HUD
objects are in [hud-pursuit.md](../specs/hud-pursuit.md).

## Sources

- `speed.exe` (install `D:\Need For Speed Most Wanted Black Edition`), Ghidra image base `0x400000`.
- `HUD_SingleRace.fng` dumped with `nfsmw dump-screen HUD_SingleRace.fng` from `GLOBAL/InGameB.bun` **[verified]**.

## Binary evidence

| Address | String | Use found |
|---|---|---|
| `0x008addb8` | `DamageCopCar` | class name, referenced from data at `0x00888360` **[binary]** |
| `0x00891120` | `cops_destroyed_in_pursuit` | referenced from `FUN_005a3c30`, `FUN_006013f0`, `FUN_00418f30`, `FUN_0052e750` **[binary]** |
| `0x00892ef0` | `cops_destroyed` | referenced from `FUN_005a3c30`, `FUN_0043e8d0`, `FUN_0052e750` **[binary]** |
| `0x00891150` | `cops_in_pursuit` | referenced from `FUN_004190c0` **[binary]** |
| `0x0089d688` | `cops_damaged_in_pursuit` | the post-pursuit stats table, next to `cost_to_state_in_pursuit` **[binary]** |
| `0x008a1bac` | `COPS_TAKENOUT_ICON` | referenced from `FUN_00595af0` **[binary]** |
| `0x00892c00` | `AIActionTooDamaged` | the retirement action named by the spec **[binary]** |

The `cops_*` names belong to the pursuit's statistics (the same table as `bounty_in_pursuit` and `cost_to_state_in_pursuit`),
which is where the destroyed count is summarised. The HUD counter is driven by the object below, not by these strings.

## HUD object (verified in the package)

The dump of `HUD_SingleRace.fng` has a group with the children `Cops_Icon.tga`, `Cops_TakenOut_Icon.tga` and
`Cops_Damaged_Icon.tga`, and three string objects with placeholder `99`, `99`, `999`.

- `Cops_TakenOut` hashes to `0xa16f9f1e` with `fe_hash_upper` (checked against the dump's object name hash). It is the
  string beside `Cops_TakenOut_Icon` (right-justified, `FONT_MW_BODY`). **[verified]**
- The group's own name hash is `0xe0b1430b`. None of the candidate names tried (`CopsGroup`, `Cops_Group`, `CopsStats`,
  `Cops_Panel` and about 300 variants) hashed to it, so the name stays unknown; the group is bound by hash. **[verified hash, name unknown]**
- The group's fade-in scripts start on the messages `0x3c97521a` and `0x5c930796`, the same messages as the pursuit board
  (`hud-pursuit.md`). The rewrite holds the group at full alpha during a chase, as it does the board. **[verified dump, inferred behaviour]**

## Damage constants: not re-read this session

The per-car `HIT_POINTS`, `HP_THRESHOLD`, `SUPPRESS_DIST`, the flipped timer of 6.0 s and the shock constants are the values
the spec cites from the decompiled sources (`ai-pursuit-cop-cars.md` section 6, marked [verified] there). This session did not
decompile `DamageCopCar::OnImpact` or `DamageVehicle::OnCollision` to re-read them: the functions are unnamed (`FUN_*`) and the
pcode of the candidate functions is over 200 kB. Open for a later pass: confirm the `HitPointScale` table, the zone rule, and
the flipped test's ground normal. The `RepPointsForDestroying` table and the kill pop-up are not reversed.
