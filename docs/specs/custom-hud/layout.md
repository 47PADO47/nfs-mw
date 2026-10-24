# Custom HUD: layout, anchors and slots

Where every Custom element sits on any window shape, and which element slot it fills. Built in PR 5
(`crates/nfsmw/src/ui/theme/custom/layout.rs`, the new `ElementId`s in `ui/theme/element.rs`). Index:
[README.md](README.md).

## 1. The content box

The Custom HUD is laid out in a **content box**, not across the whole window.

- Let `a` = window width / height. The box is as tall as the window and `min(a, 16/9)` times as wide, centred
  horizontally. On 16:9 and narrower windows the box is the whole window; on 21:9 and 32:9 it is the central 16:9 part.
- The HUD is not stretched across an ultrawide window: the cluster, the race strip and the pursuit panel stay within
  reach of the eye. The **cluster still anchors to the box edge**, the bottom-right corner of the 16:9 box, not to the
  window's corner. **[ours]**
- World markers and their edge arrows are the exception: they use the whole window, because they point at the world
  ([world-markers.md](world-markers.md)).
- Below 16:9 (4:3, 5:4) the elements keep their anchors and shrink: the effective scale is
  `scale × clamp(a / (16/9), 0.8, 1.0)`, so a 4:3 window draws at 0.8 of `[hud] scale`.
- The existing `hud_layout` setting (`pc`, `classic`, `xbox360`, [hud-viewport.md](../hud-viewport.md)) places the
  Original and Remastered HUDs. Custom ignores it: the content box and `[hud] scale` replace it.

Units are the presenter's HUD units: the box is 480 units high, so a 16:9 box is about 853 units wide.

## 2. The safe area

Every anchor is inset from the box edge by the **safe inset**: 3.5% of the box height (about 17 units) on all four
sides, at every scale. Elements never draw in the inset, except the map's soft edge and world-marker edge arrows. The
inset is fixed, not a setting, in this stack. **[ours]**

## 3. Anchors and zones

Each element names an anchor and an offset in HUD units; `layout.rs` turns them into a rectangle from the box, the
inset and the effective scale. The scale is applied about the anchor, so a larger HUD grows inwards and never leaves
the safe area.

| Anchor | Zone contents | Notes |
|---|---|---|
| Top-left | race intro card, radio card, discovery chip | the radio card keeps its current top-left place in the box |
| Top-centre | race status strip; under it the turn-cue chip (Compact map) and the banner | the banner (countdown, escape, busted cards) sits in the upper third, never over the road ahead of the car |
| Top-right | pursuit panel and bounty row; the race leaderboard | in a race with a pursuit the pursuit panel stacks under the leaderboard |
| Left-middle | the notification stack (up to 3), growing downwards from 36% of the box height | left of the road, clear of the cluster |
| Bottom-left | vehicle condition chip | |
| Bottom-centre | nothing persistent | the car occupies it in the chase views |
| Bottom-right | the telemetry cluster, with the feedback lane just above it | inspired by Remastered's cluster corner |
| Centre | the map overlay (Centered and Expanded modes) | translucent, clipped to a soft ellipse ([map.md](map.md)) |

Zone rules:

- A zone holds its elements in a column; elements never overlap across zones. When `text_size` or `scale` makes a
  column too tall, the lowest-priority element of the zone hides first (for top-right: the leaderboard before the
  pursuit panel).
- Nothing persistent sits in the central third of the box above the horizon except the translucent map.
- The busted overlay keeps its own full-screen layer (`BustedOverlay`, slot 11); Custom does not claim it.

## 4. Draw order

From back to front: the map overlay, world markers and edge arrows, the cluster and the other HUD zones, the banner,
the notification stack. A critical notification draws above everything in the HUD layer. The menus and the busted
overlay keep their own layers ([ui-themes-compose.md](../ui-themes-compose.md)).

## 5. The slot catalogue for Custom

`ElementId` numbers are mod ABI ([ui-themes.md](../ui-themes.md) section 6): appended, never renumbered. PR 5 appends
seven HUD slots in the free block between `PoliceRadar` (12) and `MainMenu` (20):

| Slot | Id | Original adapter | Custom component (PR) |
|---|---|---|---|
| `RaceStatus` | 13 | the HUD package's race place and lap objects, drawn today by `emit_unclaimed` | race strip (8) |
| `Leaderboard` | 14 | the package's `LeaderBoardGroup` (`hud/leaderboard.rs`) | compact leaderboard (8) |
| `BountyRow` | 15 | the package's bounty row (`hud/elements.rs`, `BOUNTY_ROW`) | bounty in the pursuit panel or free-roam extras (8) |
| `Banner` | 16 | empty (new-only) | intro, countdown, escape and busted cards (8) |
| `Notifications` | 17 | empty (new-only) | the notification stack (4, drawn in 8) |
| `WorldMarkers` | 18 | empty (new-only) | world markers and edge arrows (9) |
| `VehicleCondition` | 19 | empty (new-only) | the condition chip (8) |

Moving the race, leaderboard and bounty objects from "unclaimed" into slots must not change what Original draws: the
nodes are the same and keep their package order. `compose_tests` is extended in PR 5 to check, for Original and
Remastered, that the composed HUD is node for node what it was before the new slots.

The existing slots Custom claims:

| Slot | Id | Custom component | PR |
|---|---|---|---|
| `Tachometer` | 1 | rev strip of the cluster | 6 |
| `Speedometer` | 2 | speed digits, unit and speed scale | 6 |
| `GearIndicator` | 3 | `Empty` (the gear box is part of the speedometer row) | 6 |
| `Minimap` | 4 | the centred map in its current mode | 7b |
| `Blips` | 5 | `Empty` (the map draws its own marks) | 7b |
| `PursuitMeter` | 6 | the pursuit panel | 8 |
| `Nitrous` | 7 | nitrous bar | 6 |
| `Turbo` | 8 | boost bar | 6 |
| `Speedbreaker` | 9 | speedbreaker bar | 6 |
| `RadioCard` | 10 | the radio card, restyled | 11b |
| `PoliceRadar` | 12 | `Empty` in Custom; threat direction comes from the map and edge arrows under fair knowledge | 8 |
| menus 20 to 32, 40 | | the Custom menus ([menus.md](menus.md)) | 11a, 11b |

Between PR 5 (which deletes the old drive card) and PR 6, Custom draws the Original pieces for slots 1 to 9. This is
accepted inside the stack, which merges as a whole.

## 6. The feed

Custom components read one shared cell, `CustomFeedCell` (PR 5), filled once per frame in `hud/plugin.rs::present`:
the frame's `HudState` behind an `Arc` (a clone is a reference count), the notification view, the `[hud]` settings,
the elapsed time and the screen. Remastered keeps reading `DriveFeedCell`, which does not change.
`hud_component()` keeps its signature for mod token themes (`ui/theme/modded`) and returns the new components as they
land.

## 7. Tests (PR 5)

- The content box and the cluster rectangle at 4:3, 16:9, 21:9 and 32:9: the box width, the cluster's right edge on
  the box edge (not the window edge) on the two ultrawide shapes, the 0.8 scale floor at 4:3.
- The safe inset at three scales; no anchor rectangle leaves the box minus the inset at `scale` 1.5 and `text_size` 1.5.
- The new `ElementId` numbers (13 to 19) are pinned by the existing numbering test.
- `compose_tests`: Original and Remastered unchanged.
