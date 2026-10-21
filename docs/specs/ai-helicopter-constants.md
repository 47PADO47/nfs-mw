# The police helicopter: constants checked in speed.exe

Split from [ai-helicopter.md](ai-helicopter.md) to keep that file short. Tags as there. Method and addresses:
[../provenance/ai-helicopter-tuning.md](../provenance/ai-helicopter-tuning.md).

## Verified in `speed.exe` (Ghidra, PC v1.3)

The binary has no symbols; these were found from the strings and their references, and the names are ours.

| Claim | Where | Evidence |
|---|---|---|
| Spawn 250 m behind the target | `SpawnPursuitHelicopter` at `0x004269a0` | pushes `0x437A0000` = 250.0f into the nav advance |
| Height +20 m | same function | adds the float at `0x00890560` = 20.0 to the spawn height |
| Nav points backward | same function | multiplies the target direction by `0x00890DA8` = -1.0 |
| One helicopter at a time, a free one is reused | `0x00426610` (an inactive `copheli` in the vehicle pool) | filters the pool by the `copheli` type name, returns the first not spawned |
| Cop manager asks for a helicopter | `0x0043E8D0` (the cop manager's update) | compares the cop type's hash with `copheli` (`0x008915DC`) and calls `0x004269A0` |

Pass 2 (below, "Constants in speed.exe") checked the constants against the float pool. The function holding
`DoDriving` and the helicopter's `Update` is still not located, so the rules of sections 4, 5 and 6 are not yet tied to
code addresses.

## Constants in speed.exe (pass 2)

Source: the float pool of `speed.exe` at `0x00890500` to `0x00890E8F` (PC v1.3), decoded as IEEE singles. The heli
constants sit in one cluster at `0x00890828` to `0x00890868` (values below). Use sites are not found, so the value is
**[decomp]** and the link to the helicopter is **[unconfirmed]** (cluster only). Method and addresses are in
[provenance/ai-helicopter-tuning.md](../provenance/ai-helicopter-tuning.md).

| Constant | Value | Pool address | Meaning | Port |
|---|---|---|---|---|
| Work lead | 0.45 s | 0x890840 | `work` and `dest` lead (section 4) | `control.rs` `WORK_LEAD` |
| Look-ahead lead | 0.65 s | 0x890844 | the `ahead` point of `CheckHeliSheet` | `control.rs` `ahead_point` |
| Sheet steer gains | 0.3, 0.4 | 0x890848, 0x89084C (also 0x8905D0) | sliding around a roof and a wall | `control.rs` `ahead_roof`, `lead_roof` |
| Camera avoid | lead 0.2 s, sphere 7 m, kick 1.2 | 0x8905CC, 0x8905F0 (7), 0x890860 (1.2) | `AvoidCamera` | `helicopter/camera.rs` |
| Response ratio | 2 | 0x890628 | desired accel = 2 x velocity error | `flight.rs` |
| Accel limit | `min(80, 0.6 v + 30)` | 0x89062C (0.6), 0x890620 (80), 0x890820 (30) | section 5 | `flight.rs` |
| Closing / brake | closing > 3; speed cut 0.4; brake at 15 m | 0x8905DC, 0x890690 (3); 0x8905D0 (0.4); 0x890668 (15) | section 4 | `control.rs` |
| Braking divisor | 1.55 | not in 0x890500-0x890E8F | section 4 | `control.rs` (no PC source) |
| Vertical | climb x5 below 7 m; descend x3 above -5 m; add 2 x target vy | 0x8905E0 (5), 0x8905F0 (7), 0x8905F4 (3), 0x890DB8 (-5) | section 4 | `control.rs` `vertical` |
| Follow lead | 30 + 0.4 v, cap 45 | 0x89052C (30), 0x89055C / 0x890658 (45) | section 6.1 | `behaviour.rs` `follow` |
| Follow speed | 50 / 30 / 20 m/s at 300 / 15 / 3 m | 0x890558 (50), 0x890544 (300), 0x890678 (10), 0x89067C (20) | section 6.1 | `behaviour.rs` `follow` |
| Follow heights | +6, +12 m; 2 m (skid); 55 m split | 0x8905A8 (6), 0x890590 (12), 0x890628 (2), 0x890584 (55) | section 6.1 | `behaviour.rs` `follow` |
| Skid | height 13 m; approach 0.23 s; window 5 m; timer 8 s; cooldown -5 s; side 6 m | 0x890648, 0x89064C, 0x890654, 0x89059C (8), 0x8905A8 (6) | section 6.1 | `behaviour.rs` |
| Strike | 4 m; 0.4 m height; speed 100 m/s | 0x89065C (4), 0x8905D0 (0.4), 0x890530 / 0x890664 (100) | section 6.1 | `behaviour.rs` |
| Search | radius 1000 m; arrival 30 m; speed 70 m/s; +5 m | 0x890538 (1000), 0x890820 (30), 0x89057C (70) | section 6.1 | `behaviour.rs` |
| Exit | height 25 m; seek up at 15 m; 200 m dive | 0x890DC0 (25), 0x890DAC (15), 0x8908BC (200) | section 6.2 | `behaviour.rs` `Exit` |
| Sight | grace 150 m, 20 m | 0x89076C (150), 0x890560 (20, also spawn) | section 3 | `sight.rs` |
| Spawn | 250 m back, +20 m, backward (-1) | 0x890560 (20, verified in code), 0x890DA8 (-1, verified), 0x890854 (250) | section 2.2 | `mod.rs` `spawn_distance` |
| Think period | 0.125 s | 0x8905C8, 0x890830 | section 1 | `mod.rs` `THINK_PERIOD` |
| Yaw gain / lean | 8; lean 9/80 and 12/80; clamp 0.1 | 0x8905E4 (8), 0x890598 (9), 0x890590 (12), 0x8905C4 (0.1) | section 5 | `flight.rs` |
| Smoothing | 1/8 new, 7/8 old | 0x8905F0 (7), 0x8908C8 (0.125) | section 5 | `flight.rs` |

**Not found** in the pool read (0x00890500 to 0x00890E8F): 1.55 (braking), 0.035 (pitch gain), 0.092 (strike lead),
1.3 (yaw clamp), 1.8 and 4.8 (skid lift), 1.9 (strike across), 0.7, 1.14, 35 (skid window), 75 (visual sphere), 85 (exit
seek), 600 (despawn). They may be immediate operands in code. Their values are the spec's, not confirmed in `speed.exe`.

**Port gaps** (the sheet steering was closed in pass 3, see Rewrite status): `AvoidCamera` is ported (the world gives the
helicopter the game camera each step). Still open: the search arrival rule (section 6.1, a new point within 30 m; the port takes the angle from the caller).

