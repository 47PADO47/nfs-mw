# 0013 — The Custom HUD 2026: a from-scratch rewrite of the `custom` UI style

- **Status:** accepted (2026-10-24). Specified in PR 1; built by the PR stack below. Nothing of the new look is built yet.
- **Spec:** [specs/custom-hud/](../specs/custom-hud/README.md) holds the visual tokens, the typography, the layouts, the map,
  the notification rules, the settings table and the list of what is not shown. This ADR records the decisions only.
- **Builds on:** [ADR 0008](0008-ui-themes.md) (one component system, slots with an Original floor, one presenter) and
  [ADR 0012](0012-localization.md) (every player-visible string goes through the Fluent catalogue).

## Question

`UiStyle` (`crates/nfsmw/src/settings/ui_style.rs`) has three looks. **Original** runs the game's FEng packages.
**Remastered** has its own components for the cluster, the minimap, the pursuit timer, the police radar, the radio card,
the vertical menus and the full-screen world map (`crates/nfsmw/src/ui/theme/remastered/`). **Custom** covers little: a
bottom-right drive card (`ui/theme/custom/`: speedometer, tachometer, gear, meters), the radio card and the world map's
legend. Every other Custom slot (minimap, pursuit, radar, race information, menus) falls back to Original.

A design brief ("Dynamic HUD Remaster", the owner's) asks for a modern 2026 driving HUD: a horizontal telemetry cluster,
a centred translucent road map, contextual free-roam, race and pursuit layouts, a notification manager, world markers,
accessibility settings and its own menus, showing only real gameplay data. How should Custom grow into that without
disturbing Remastered or Original, without a second draw path, and without inventing data?

## Decision

1. **Remastered stays exactly as it is. Custom is rewritten from scratch.** Custom gets its own components for the HUD
   **and** the menus. It borrows Remastered's design ideas (italic condensed numerals, dark translucent plates,
   segmented bars, vertical menus) but shares no component code with it; `remastered/` is a design reference only.
   The old Custom drive card (`speedometer.rs`, `tachometer.rs`, `gear.rs`, `meters.rs`, `drive_layout.rs`,
   `drive_tests.rs`) is deleted in PR 5 and replaced in PR 6.
2. **Fonts come from the operating system, with no new dependency and nothing bundled.** A finder tries known paths per
   platform (Bahnschrift on Windows, DejaVu Sans Condensed, Liberation Sans Narrow or Noto Sans on Linux, DIN Condensed
   or Avenir Next Condensed on macOS). Bahnschrift's variable axes are set through `ab_glyph`'s `VariableFont`, which
   the existing `ab_glyph` 0.2.32 already provides. The file is read at run time from the player's machine, like the
   game install, so nothing is redistributed. When no face is found, the HUD falls back to the raster font the rewrite
   already embeds (egui's Ubuntu Light from `epaint_default_fonts`) with a narrower `Look`, and logs it once.
3. **The centred map is the default, and on.** `[hud] map_mode` defaults to `centered`. The map is a player-centred
   translucent overlay in the middle of the view, not a corner minimap. Its orientation reuses the existing `minimap`
   setting (`MinimapMode`), so there is one orientation switch.
4. **New HUD slots, appended to the catalogue as ABI.** `ElementId` numbers are mod ABI ([ADR 0008](0008-ui-themes.md),
   `bb.ui`): never renumbered, never reused. The free block 13 to 19 takes:

   | Slot | Id | Original adapter |
   |---|---|---|
   | `RaceStatus` | 13 | the HUD package's race place and lap objects, drawn today by `emit_unclaimed` |
   | `Leaderboard` | 14 | the package's `LeaderBoardGroup` (`hud/leaderboard.rs`) |
   | `BountyRow` | 15 | the package's bounty row (`hud/elements.rs`, `BOUNTY_ROW`) |
   | `Banner` | 16 | empty (new-only slot) |
   | `Notifications` | 17 | empty (new-only slot) |
   | `WorldMarkers` | 18 | empty (new-only slot) |
   | `VehicleCondition` | 19 | empty (new-only slot) |

   Original and Remastered draw exactly what they draw today, node for node; `compose_tests` guards this. The centred
   map fills the existing `Minimap` slot (4).
5. **The pursuit phase is explicit.** `PursuitPhase { None, Active, Search, Cooldown, Escaped, Busted }` is derived in
   `scenes/world/ai/` from the pursuit's own state (`is_pursuing`, `cooldown_status`, the sighting, `end_reason`, the
   bust flow), never from the heat value or from a cop being near. Whether the simulation separates Search from Cooldown
   is pinned in PR 3b against `libs/blackbox-pursuit/src/cooldown.rs`; if it does not, Search is dropped and the spec
   says so.
6. **Fair knowledge.** The HUD shows only what the player could know. A cop is drawn on the map, or as an edge arrow,
   only while it is sighted (or the helicopter has the player in sight). Otherwise a last-known marker fades at the
   pursuit's last-seen position. A roadblock is announced only when dispatch reports it or it is within sight.
7. **Engine-generic pieces go to `libs/blackbox-ui`.** A generic priority and dedupe queue (`NotifyQueue<K>`, 5
   priorities, 3 visible, preemption, deferral, dedupe that refreshes) and a stroked `Polyline` component (segments as
   rotated quads, joins, a clip with a soft edge). The NFS:MW catalogue of notifications, the map data and the mode
   logic stay in `crates/nfsmw`. Neither piece names the game; `blackbox-ui` stays Bevy-free.
8. **One rich feed for Custom.** A `CustomFeedCell` holds the frame's `HudState` (behind an `Arc`), the notification
   view, the `[hud]` settings and the elapsed time. Remastered keeps reading `DriveFeedCell`, unchanged.
9. **A `[hud]` settings table** (PR 2) carries the presets (Minimal, Balanced, Advanced), scale, opacity, text size, the
   map, the information toggles, the notification filter, accessibility (high contrast, colour-vision palettes, motion
   reduction), camera shake and FOV multipliers, haptics and a HUD volume. It goes through the existing layered settings
   and the per-key TOML merge, so an old config without the table loads with the defaults.
10. **Never invent data.** A metric the game does not compute is hidden, not faked. Optional data is `Option` and hides
    its widget when absent. The list of what is not shown, and why, is part of the spec.

## The PR stack

Each PR is a stacked branch of small Conventional Commits, files under 500 lines, guard clauses instead of `else`
chains. Each PR updates its section of the spec.

| PR | Title | What it adds |
|---|---|---|
| 1 | `docs(ui): spec the Custom 2026 HUD and fix stale Remastered docs` | this ADR, the spec, stale doc fixes |
| 2 | `feat(settings): a [hud] table for the Custom HUD` | `settings/hud.rs`, presets, console, merge |
| 3a | `feat(hud): telemetry, race, damage and camera data for the HUD` | `HudState` split, new fields, rival gap |
| 3b | `feat(ai): an explicit pursuit phase and what the player knows` | `PursuitPhase`, `PoliceIntel`, `next_turn` |
| 4 | `feat(hud): HUD events and the notification queue` | one feed detector, `HudEvent`, `NotifyQueue`, catalogue |
| 5 | `feat(ui): Custom theme foundation: OS fonts, tokens, layout, slots` | `system_fonts.rs`, tokens, layout, slots 13 to 19 |
| 6 | `feat(ui): the Custom horizontal telemetry cluster` | the cluster |
| 7a | `feat(ui): a stroked polyline in blackbox-ui` | `Polyline` |
| 7b | `feat(ui): the Custom centred translucent map` | the map, its modes and input actions |
| 8 | `feat(ui): Custom free-roam, race and pursuit layouts` | the mode resolver and the three layouts |
| 9 | `feat(ui): Custom world-space markers` | projected markers and edge arrows |
| 10 | `feat(ui): Custom feedback, HUD audio and haptics` | feedback chips, the `Hud` volume group, rumble |
| 11a | `feat(frontend): Custom menus: main, categories, pause, options, dialogs` | Custom menus, the HUD options page |
| 11b | `feat(frontend): Custom controls, jukebox, credits, blacklist, LAN and world map` | the remaining screens |
| 12 | `feat(devtools): deterministic HUD scenarios and a screenshot matrix` | `--hud-scenario`, `tools/hud_shots` |

PR 12 depends only on PRs 3a, 3b and 4 and may land right after PR 5 if the widget PRs need screenshots earlier. Inside
the stack Custom shows the Original pieces between PR 5 and PR 6; the stack merges as a whole.

## Alternatives considered

- **Extend the existing Custom drive card.** Its fixed FEng positions, single feed (`DriveFeedCell`) and corner layout
  do not carry a contextual HUD, a centred map or menus. Growing it would keep a layout the brief rejects.
- **Make the new look a fourth `UiStyle` and keep the old Custom.** Two token-driven looks with overlapping purpose,
  more combinations to test, and a settings value nobody would pick. The owner chose to replace Custom.
- **Share components with Remastered.** It would couple two looks the owner wants independent: any change to the new
  HUD could move Remastered. Remastered stays untouched; its code is read for ideas only.
- **Bundle a font** (an OFL face such as Barlow Condensed or Saira). It adds a binary asset to a repository whose
  `.gitignore` is a whitelist and whose leak check refuses binaries, and a licence notice to carry. OS fonts give a
  condensed face on all three platforms with no file in the repository. A new font crate (`fontdb`, `font-kit`) was also
  rejected: a short list of known paths is enough, and `ab_glyph` already reads TrueType, collections and variable axes.
- **A corner minimap as the default.** The brief asks for a centred overlay as the signature feature and the owner chose
  Centered as the default. Off, Compact and Expanded remain one key away.
- **Infer the pursuit phase from the heat or the cop count.** The brief forbids it, and it would flicker: heat stays
  high while the player hides, and a parked cop nearby is not a pursuit.
- **A notification queue inside `crates/nfsmw`.** Priorities, preemption and dedupe are engine-generic and need no game
  name; in `libs/blackbox-ui` they are testable without the game and reusable.
- **A second edge detector for the HUD.** The mod feeds (`modding/pursuit_feed.rs`, `races_feed.rs`) already diff the
  frame snapshots. PR 4 moves that diffing into one `scenes/world/feed/` that mods and the HUD both consume.

## Consequences

- **ABI.** Slots 13 to 19 become mod ABI. A `bb.ui` mod can override them; a slot outside the catalogue is still refused.
- **Original and Remastered are frozen.** The `compose_tests` node-for-node checks and a screenshot diff guard them.
  Remastered gains no feature from this work.
- **Fonts differ per machine.** The Custom HUD looks slightly different on each platform, and on a machine with none of
  the listed faces it uses the fallback. Layout must not depend on exact glyph widths beyond the tabular numerals rule.
- **New settings surface.** The `[hud]` table, seven new input actions (map mode, expand, zoom in, zoom out,
  orientation, toggle HUD, dismiss notification) and a `Hud` audio group. Defaults are chosen so an old config gets
  Balanced with the centred map.
- **Costs.** About fifteen PRs; the map needs road simplification and a per-frame polyline build, whose frame cost is
  to be measured (PR 7b) and recorded in the spec. Split-screen keeps its own HUD path (it bypasses themes today); Custom
  in split screen is a follow-up.
- **Not shown.** Drafting, drift scoring, engine, brake and tyre temperatures, cooling damage and a search-area shape are
  not in the simulation, so the HUD has no widget for them. The spec lists them.
