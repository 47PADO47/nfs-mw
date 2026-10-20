# Pursuit hiding spots

Sources: the CC0 reconstruction of the perpetrator and pursuit update code (read for behaviour only); the installed
`TrackPathZones` of `L2RA.BUN`; the authored pursuit HUD package. Evidence tags: **[decomp]** read in the public
reconstruction, **[verified]** checked against the install by a test, **[inferred]** concluded from other evidence,
**[open]** not established.

## 1. Where the spots come from

A hiding spot is a track path zone of type 8 (HIDDEN). The track's metadata holds 34 of them [verified], each a
polygon (4 to 7 points) on the map plane with a non-zero `Elevation` and `Data[0] = 0`. They are tiny (7 to 43 m
across) and sit on or beside roads: under overpasses, in car parks, behind buildings, in tunnel mouths. They load once
with the track, not per streaming section, so the whole track is always known.

`Elevation` is the **physics height of the ground in metres** [verified]. Where a road passes through a zone's
middle (about half of the 34), the road surface differs from the elevation by a median of under half a metre
(`blackbox-roads` installed test `real_install_hidden_zone_elevations_are_ground_heights`). The unit is therefore the
same as the car's physics height, with no scale.

`Data[0]` names who the spot hides the player from: the hash of `"Car"` hides from ground cars only, `"Heli"` from the
helicopter only, anything else from both [decomp]. Every installed spot is "anything else" [verified].

## 2. The test of "inside"

Each perpetrator update, for the player's car [decomp]:

- the point tested is the body position with its **feet** found by subtracting the body's half height (the body
  origin is its middle), in the map frame `(z, -x)`;
- a zone holds the player when the point is inside its polygon **and** (`Elevation` is exactly 0, or the feet are
  within 1.25 m of `Elevation`); a zero elevation means "any height" (no installed spot has one).

## 3. The latch

The spot does not hide the player the instant it is entered [decomp]. The perpetrator keeps a latch time, a timer and
a "was in a spot last update" flag:

1. Each update the two hidden flags (cars, helicopters) start false.
2. Zones are visited in file order. On the first update inside any zone the latch is set once: **0.05 s** when the
   player is not currently seen, **99 999 s** when it is. "Not seen" means the pursuit's evade level is at least 0.05
   (the cool-down meter has started), or there is no pursuit at all. On later updates inside, a latch above 999 keeps
   the timer at zero; otherwise the timer grows by the frame time. The frame time counts **once** per update however
   many zones overlap (the remaining zones add zero).
3. When the timer is **greater than** the latch, the zone sets the flags by its scope: cars only, helicopters only,
   or both.
4. With no zone holding the player, the was-in flag and the timer clear.

The consequence for play: a spot entered while a cop still has the player in sight never hides it, however long it
stays. The player has to leave, lose the cops, and go in again with the evade meter already running [decomp].

The HUD's "how hidden" value is `min(1, timer / latch)` once the timer is above 0.07 s, and zero before that
[decomp]. A seen entry therefore reads zero for ever, and an unseen one reads one after about 0.07 s, since the quick
latch is only 0.05 s.

## 4. What the flags do

- **Evade progress.** While the player is hidden (from cars **or** helicopters) and no cop has it in sight (the
  pursuit's 7 s window has passed), the hidden-zone time grows by `dt * HiddenZoneTimeMultiplier` of the heat level
  (4 to 8 at heat 1 to 7, 2 or 4 at heat 8 to 10; 3 with no pursuit row). It resets to zero the moment the pursuit
  is in sight. The evade meter reads `(time since any cop saw the player + hidden-zone time) / required time`
  [decomp, matches `ai-pursuit.md` section 4.3].
- **Cop sight.** A cop that fails to see a hidden player remembers that it failed, and does not test again until the
  player is no longer hidden [decomp, `ai-pursuit.md` section 4.1]. The helicopter's own sight is latched the same way
  by the helicopter flag.
- **Roadblock sight.** A roadblock cop's 60 m awareness does not count a hidden player [decomp].

## 5. Rewrite status

Implemented in `libs/blackbox-pursuit/src/hiding.rs` (the latch, scopes, entry rule, height test and HUD fill) and
`crates/nfsmw/src/scenes/world/ai/police/hiding.rs` (zone discovery from the track's zones, the player's feet from its
half height, the evade input, the per-cop sight latch and the `pursuit hiding` console listing).

- **Previously** the world used the bare zone test as the hidden flag: no 0.05 s latch, no seen-on-entry rule, the body
  height instead of the feet, and cars only for the evade multiplier. These are corrected.
- **HUD.** The cool-down bar holds a "HIDING" text group (`0x60b45523`) and a car icon (`0x38074901`) at zero alpha.
  Each answers two messages of the package with scripts: `0x4027A94E` plays a fade-in (script 1350158584, the icon
  then pulses) and `0x451CD24C` a fade-out (script 53547948) **[verified]**. The rewrite plays those scripts when the
  latch's fill appears and when it goes (`hud/hiding_mark.rs`); that the original raises them from the perpetrator's
  hidden fill is **[inferred]**, as its feeder (`TimeUntilHidden`) is not in the public sources. The orange bar
  `0x5c2a4f20` beside them answers no message and stays at rest. The mark sits under the cool-down group, so it shows
  only while that bar does.
- **[open]** Whether the original raises the mark for the quick (0.05 s) latch, since the fill reaches one within a few
  frames; here the mark shows for as long as the latch holds.

## 6. Checks

Unit tests of the latch (quick, seen entry, scope, overlap, fill, height, evade entry rule) and of the HUD mark
(synthetic package). Installed tests: the elevations against the road surface; the HUD mark fading in and out on the
real package; a parked player in the installed spot under the overpass at physics (-2194, 2730) escapes a heat 1
chase that no cop sees after 14.6 s, against 25.0 s on the open road, and the HUD frame raises the mark; the console
report names the spot and "PLAYER INSIDE". A playable check: `pursuit hiding`
lists the spots nearest to the player with their heights and says whether the player is inside one; driving into a
listed spot with a running chase and no cop in sight sets the `hidden` flags after 0.05 s and makes the evade meter
run `HiddenZoneTimeMultiplier` times faster.
