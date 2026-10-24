# Custom HUD 2026

The design of the rewritten `custom` UI style: a modern driving HUD and menus written from scratch, with a horizontal
telemetry cluster, a centred translucent road map, contextual free-roam, race and pursuit layouts, a notification
manager, world markers and accessibility settings, showing only real gameplay data. Written 2026-10-24 (PR 1); each later
PR of the stack updates its section. The Original and Remastered styles are not changed by any of it.

The decision and its alternatives are [ADR 0013](../../decisions/0013-custom-hud.md). It builds on the theme framework of
[ADR 0008](../../decisions/0008-ui-themes.md) and [ui-themes.md](../ui-themes.md) (slots, the Original floor, one
presenter) and on [ADR 0012](../../decisions/0012-localization.md) (every visible string from the Fluent catalogue).

## Pages

Read in this order:

| Page | What it covers |
|---|---|
| [visual-identity.md](visual-identity.md) | Design rules, the colour tokens with the brief's hex values, high contrast, the colour-vision palettes, opacity, scale, transition durations and motion reduction |
| [typography.md](typography.md) | The three font roles, the OS font search order per platform (Bahnschrift's axes, the Linux and macOS faces), why OS fonts, the Ubuntu Light fallback, tabular numerals |
| [layout.md](layout.md) | The 16:9 content box on ultrawide windows, the safe area, the anchors and zones, the cluster bottom-right, the new slots 13 to 19, the feed |
| [telemetry.md](telemetry.md) | The horizontal cluster: speed, speed scale and its spring, revs and gear, the shift cue, nitrous, speedbreaker, boost, the advanced row |
| [map.md](map.md) | Map modes Off, Compact, Centered (default) and Expanded, orientation from `minimap`, layers, road simplification and caching, the `Polyline` component, input actions and default keys |
| [modes.md](modes.md) | The mode resolver, free roam, the presets per mode, the vehicle condition chip |
| [race.md](race.md) | Intro, countdown, the status strip, the leaderboard, the rival gap and its sign convention, results |
| [pursuit.md](pursuit.md) | `PursuitPhase` and its transitions, the pursuit panel per phase, police awareness, directional threats |
| [fair-knowledge.md](fair-knowledge.md) | What the HUD may reveal: sightings, radio reports, last-known positions, roadblocks; timings it may claim |
| [notifications.md](notifications.md) | Event sources, a notification's fields, the five priorities, the queue (capacity 3, preemption, deferral, dedupe that refreshes), the catalogue |
| [world-markers.md](world-markers.md) | Projection before shake, marker kinds, edge arrows, styling, the cap, occlusion |
| [feedback.md](feedback.md) | Near-miss, speed-trap and bounty feedback, the `hud_volume` group, police dispatch as `speech_volume`, cues, `GamepadRumbleRequest` |
| [settings.md](settings.md) | The `[custom_hud]` table: every key, type, range and default; the Minimal, Balanced and Advanced presets; the user-facing table is [hud-settings.md](../../hud-settings.md) |
| [menus.md](menus.md) | The Custom menus, the list-look predicate, the screens, the HUD options page |
| [dev-scenarios.md](dev-scenarios.md) | `--hud-scenario` fixtures, the screenshot matrix, the runtime checks and the measurements owed |
| [not-shown.md](not-shown.md) | Drafting, drift scoring, temperatures, cooling damage, search-area shape, split-screen styles and the rest, with the reason |

## The PR stack

| PR | Builds | Spec pages |
|---|---|---|
| 1 | this spec, ADR 0013, stale doc fixes | all |
| 2 | the `[custom_hud]` settings table | [settings.md](settings.md) |
| 3a | telemetry, race, damage and camera data in `HudState` | [telemetry.md](telemetry.md), [race.md](race.md), [modes.md](modes.md) |
| 3b | `PursuitPhase`, `PoliceIntel`, `next_turn` | [pursuit.md](pursuit.md), [fair-knowledge.md](fair-knowledge.md) |
| 4 | the shared feed detector, `HudEvent`, `NotifyQueue`, the catalogue | [notifications.md](notifications.md) |
| 5 | OS fonts, tokens, layout, slots 13 to 19, `CustomFeedCell` | [typography.md](typography.md), [visual-identity.md](visual-identity.md), [layout.md](layout.md) |
| 6 | the telemetry cluster | [telemetry.md](telemetry.md) |
| 7a, 7b | `Polyline`; the centred map and its actions | [map.md](map.md) |
| 8 | the free-roam, race and pursuit layouts, the condition chip | [modes.md](modes.md), [race.md](race.md), [pursuit.md](pursuit.md) |
| 9 | world markers | [world-markers.md](world-markers.md) |
| 10 | feedback, HUD audio, haptics | [feedback.md](feedback.md) |
| 11a, 11b | the Custom menus | [menus.md](menus.md) |
| 12 | scenarios and the screenshot matrix | [dev-scenarios.md](dev-scenarios.md) |

## Evidence tags

As in [docs/README.md](../../README.md#evidence-tags), plus the tag the split-screen HUD spec uses:

- **[ours]**: a design choice of this rewrite, following the owner's brief. Every design statement in this spec is
  **[ours]** unless tagged otherwise; the tag is written out where a specific number or rule is a starting point that
  may be tuned.
- **[verified]**: checked on a real file; here, Bahnschrift's variable axes read from the development machine's
  `bahnschrift.ttf` (Windows 10).
- **[unconfirmed]**: to be pinned by the PR named next to it, then rewritten here.
- Facts about the rewrite's own code name the file they come from and carry no tag.

Nothing in this spec comes from `speed.exe` or the decompilation. Performance numbers are **to be measured**; none is
claimed before it is ([dev-scenarios.md](dev-scenarios.md) section 4).

## Rules carried over from the brief

- Show only real gameplay data; hide what the game does not compute ([not-shown.md](not-shown.md)).
- Keep the road view clear.
- Be event-driven: no polling and redisplaying the same message.
- Persist settings through the existing settings system; apply them live.
- Test every piece of logic that can be tested without rendering.
