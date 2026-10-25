# Custom HUD: the notification system

One manager decides which messages the player sees, in which order, and for how long. It is event-driven: a message
comes from a `HudEvent` (an edge in the game state), never from polling a condition and redisplaying it. Built in PR 4:
the generic queue `libs/blackbox-ui/src/notify.rs`, the NFS:MW catalogue `crates/nfsmw/src/hud/notify_catalog.rs`,
and the shared edge detector `crates/nfsmw/src/scenes/world/feed/`. Drawn in the `Notifications` slot (17) by PR 8.
Index: [README.md](README.md).

## 1. Where events come from

The mod feeds already diff the frame snapshots (`modding/pursuit_feed.rs`, `modding/races_feed.rs`). PR 4 moves that
diffing into one `scenes/world/feed/` that both the mods and the HUD consume: one detector, two readers. A `HudEvent`
is raised once per edge.

`HudEvent` (PR 4): pursuit phase changed, heat up, roadblock reported, helicopter tracking, contact lost, cooldown
started, escaped, busted, checkpoint passed, missed checkpoint or wrong way, lap, new position, near miss (from
`TrafficEvent::NearMiss`, throttled), speed trap, bounty milestone, tyre punctured, heavy damage, vehicle disabled,
nitrous on, event discovered, part unlocked.

## 2. A notification's fields

Each catalogue entry defines:

| Field | Meaning |
|---|---|
| id | the catalogue id (`NotifyId`), unique |
| category | `Pursuit`, `Race`, `Vehicle`, `Navigation`, `Progress` |
| priority | one of the five below |
| text | a Fluent message id in `hud.ftl`, with arguments |
| secondary text | optional second message id (a distance, a name, a count) |
| icon | optional Custom icon id |
| duration | seconds on screen before it expires |
| dismissal | `Timed` (expires), `WhileHolds` (stays while its condition holds, then expires), `Manual` (until dismissed or it expires) |
| audio cue | optional HUD cue ([feedback.md](feedback.md)) |
| haptic | optional rumble pattern ([feedback.md](feedback.md)) |
| dedupe | the key that makes two pushes "the same", the cooldown window, and the policy |

## 3. Priorities

| Priority | Level | Example | Deferred while intense |
|---|---|---|---|
| Critical | 1 | a confirmed imminent road hazard: roadblock ahead | no |
| High | 2 | a critical mechanical warning: tyre blown, vehicle disabled; heat up | no |
| Medium | 3 | missed checkpoint, wrong way, contact lost, cooldown | no |
| Low | 4 | event discovered | yes |
| Background | 5 | a newly unlocked part | yes |

## 4. The queue (`NotifyQueue<K>`)

Engine-generic: no game names, no Bevy, a pure state machine stepped with a time delta, so it is tested without the
game. `K` is the dedupe key type.

- **Capacity.** At most **3** visible at once. Others wait in a pending list (at most 8; when full, the oldest
  Background, then Low, item is dropped).
- **Order.** Visible items are ordered by priority, then by arrival. A Critical item is always the top slot.
- **Preemption.** A **Critical** or **High** item that arrives while 3 are visible takes the place of the
  lowest-priority visible item that is below it. The displaced item returns to pending with its remaining time if more
  than 1 s is left; otherwise it is dropped. Medium, Low and Background never displace a visible item; they wait.
- **Deferral.** While the game is intense (a pursuit phase is `Active`, or a race is `Racing`), Low and Background items
  stay pending and are not shown. When the intensity ends they are released in order, if they have not aged out:
  a pending Low item is dropped after 20 s, a Background item after 60 s.
- **Dedupe.** A push whose key matches a **visible** item refreshes that item instead of adding one: its remaining time
  goes back to the full duration and its secondary text is replaced, but its entry animation is **not** restarted. A
  push whose key matches an item that left within its cooldown window is ignored. A push whose key matches a pending
  item replaces that item's text and keeps its place.
- **Expiry.** Each visible item counts down its duration on the queue's clock. `WhileHolds` items expire only once their
  condition is cleared (the caller clears the key) and then after a short tail (1 s).
- **Dismiss.** `dismiss_top()` removes the top-most dismissible visible item. A Critical item is not dismissible while
  its condition holds.
- **Filter.** The `[custom_hud] notifications` setting filters at push time: `all` passes everything; `important` passes
  Critical, High and Medium; `off` passes Critical only. A critical warning is never filtered out: it is the safety
  floor.

The API, as a sketch (exact signatures in PR 4):

```text
NotifyQueue::new(capacity = 3)
push(item, now) -> Pushed { Shown | Refreshed | Pending | Ignored }
tick(dt, intense)
clear(key)          // the condition of a WhileHolds item no longer holds
dismiss_top()
visible() -> &[Visible<K>]   // item, age, remaining, entering (for the animation)
```

## 5. The catalogue (examples)

Durations in seconds; window is the dedupe cooldown.

| Id | Priority | Text | Secondary | Duration | Dismissal | Dedupe key, window, policy |
|---|---|---|---|---|---|---|
| `RoadblockAhead` | Critical | `Roadblock ahead` | distance | 4 | `WhileHolds` (until passed) | site, 15 s, refresh |
| `TyreBlown` | High | `Tyre blown` | which wheel | 5 | `Timed` | wheel, 30 s, refresh |
| `TyrePunctured` | High | `Tyre punctured` | wheel, seconds to blow-out | 5 | `Timed` | wheel, 30 s, refresh |
| `VehicleDisabled` | High | `Vehicle disabled` | | 5 | `WhileHolds` | one, 10 s, refresh |
| `HeatUp` | High | `Heat level {n}` | | 3 | `Timed` | one, 5 s, refresh (newer level wins) |
| `HelicopterTracking` | High | `Helicopter overhead` | | 4 | `Timed` | one, 30 s, ignore |
| `ContactLost` | Medium | `Visual contact lost` | | 3 | `Timed` | one, 10 s, refresh |
| `CooldownStarted` | Medium | `Cooldown` | | 3 | `Timed` | one, 10 s, ignore |
| `MissedCheckpoint` | Medium | `Missed checkpoint` | direction to it | 4 | `WhileHolds` (until recovered) | checkpoint, 10 s, refresh |
| `WrongWay` | Medium | `Wrong way` | | 3 | `WhileHolds` | one, 5 s, refresh |
| `FinalLap` | Medium | `Final lap` | | 3 | `Timed` | race, none |
| `EventDiscovered` | Low | `{event} discovered` | type, distance | 5 | `Timed` | event, forever, ignore |
| `PartUnlocked` | Background | `{part} unlocked` | | 5 | `Manual` | part, forever, ignore |

Checkpoint confirmation, position changes, near misses, speed traps, bounty deltas and nitrous activation are **not**
queue items: they are short feedback in the strip, the cluster or the feedback lane ([feedback.md](feedback.md)), so
they never compete with warnings.

## 6. Presentation (PR 8)

- Left-middle zone, up to three plates stacked downwards ([layout.md](layout.md)).
- A plate: icon, text in the label key size 14, secondary line size 11, sentence case.
- Critical: solid `critical` plate with dark text and its icon, top slot, at least 0.8 alpha whatever `opacity` is.
  High: `surface_raised` plate with a 3-unit `caution` bar on the left. Medium and below: `surface` plate.
- Entering slides in from the left over `slide` (200 ms); leaving fades over `fade_out`. A refreshed item does not
  replay its entry. Motion reduction: fades only.
- The egui mod message card (`hud/message.rs`, `bb.hud`) becomes a Medium queue item with the mod's text when the style
  is Custom.

## 7. Input

`DismissNotification` (PR 4, config name `dismiss_notification`): keyboard X, no pad default (every pad button is taken
in the driving layout); remappable through `[bindings]`. It calls `dismiss_top()`. **[ours]**

## 8. Tests (PR 4)

- Priority order and the Critical top slot.
- Preemption: Critical and High displace the lowest visible item below them; the displaced item returns to pending or
  is dropped by its remaining time; Medium never displaces.
- Dedupe: a repeat refreshes the timer without restarting the entry; a repeat in the cooldown window after expiry is
  ignored.
- Deferral: Low and Background held while intense, released after, dropped when too old.
- Expiry and `WhileHolds` with `clear`; dismiss; the `notifications` filter's safety floor.
- Event-driven: an identical state over N frames yields one notification.
