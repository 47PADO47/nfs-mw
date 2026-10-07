# Scenery visibility (exclude flags)

- **Sources read:** dbalatoni13/nfsmw (CC0, decompiled): `src/Speed/Indep/Src/World/Scenery.cpp`
  (`ScenerySectionHeader` culling, `SceneryOverrideInfo::AssignOverrides`) and
  `src/Speed/Indep/Src/Ecstasy/Ecstasy.cpp` (`SetupSceneryCullInfo`). Read for understanding; no code copied.
- **Data inputs:** `SceneryInstance::ExcludeFlags` (u32 at 0x18 of each 64-byte instance; see
  [../formats/maps.md](../formats/maps.md)); later, `SceneryOverrideInfos` (`0x00034108`) in `L2RA.BUN`.

## Behaviour

Every view (the player's camera, the rear-view mirror, reflection and environment-map renders) has a set
of **view flags**. A scenery instance is drawn in a view only if its **exclude bits** and the view flags
have nothing in common.

The exclude bits are the instance's low 8 flag bits with bits `0x20` and `0x40` **inverted**. So for
those two bits the instance flag means "also draw me in this kind of view", and for every other bit it
means "never draw me in this kind of view":

```
exclude_bits = (instance.ExcludeFlags & 0xFF) XOR 0x60
visible      = (exclude_bits AND view_flags) == 0
```

View flags known so far:

| Flag | Set for | Effect on instances |
|---|---|---|
| `0x10` | every view | instances with `0x10` are never drawn as static scenery: race barriers (`XO_TrackBarrierPlayer_1`, 6,922 copies), animated props drawn by the animation system (cranes, the airliner) |
| `0x02` | the player views (view ids 1 and 2) | instances with `0x02` are hidden from the player |
| `0x20` | the rear-view mirror (view id 3) | instances *without* `0x20` are hidden from the mirror |
| `0x40` | reflection / special render modes (`0x1800` modes) | instances *without* `0x40` are hidden there |
| `0x100` | environment-map views (ids 16–21) | |
| `0x01`, `0x04` | some view modes; race play mode 1 | |

So the normal player view uses `0x10 | 0x02`, and an instance is visible there unless it has bit `0x10`
or bit `0x02` set.

**Scenery overrides.** When a race or event starts, `SceneryOverrideInfo` records replace the low 16 bits
of chosen instances' flags (the high 16 bits are kept). That is how race barriers are switched on (their
`0x10` bit is cleared). One more detail: if an instance has bit `0x800000` and the override flips bit
`0x400`, the instance is mirrored along its local x axis. Not implemented yet.

Other bits seen in the culling code: `0x80` and `0x1000100` choose which LOD slot is used in special
modes; `0x2000000` biases the LOD choice. Bits 16–31 are kept through overrides.

## Constants

None beyond the flag values above, which are part of the data format.

## How to check it

- In free roam, no chevron race barriers should be visible, and animated cranes should not appear
  twice. Both hold with the `0x12` mask in `nfsmw view-world`.
- Start a race in the original game and compare which barriers appear with the override records.
