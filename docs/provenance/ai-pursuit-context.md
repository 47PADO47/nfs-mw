# Pursuit context integration

- Date: 2026-10-23.
- Spec: [ai-pursuit-context.md](../specs/ai-pursuit-context.md), complementing
  [ai-pursuit-heat.md](../specs/ai-pursuit-heat.md) sections 1 and 3.
- Reference: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
  decompiled. `AIPursuit.cpp` constructor and growth update, `GRaceStatus.h`
  heat/bin accessors, `AIVehicle.cpp` heat table selection, generated `gameplay`
  declarations. Read for facts only.
- Implementation: authored Rust data readers and event-lifetime integration;
  no reference implementation copied or ported.

The runtime selects the race table for non-pursuit races before construction and restores the
open-world table when leaving the race. Wave, backup, growth and rep data use
the selected table. Heat bounds distinguish career events, quick races,
challenges and the final pursuit. Challenge limits come from the event's actual
bin fields; special heat levels do not identify an event by themselves.

Automatic pursuit starts obey context bounds. Development console heat starts
can raise the ordinary cap. The final pursuit fixes heat at six and cannot end
through ordinary cooldown. Growth adds a bucket multiplier only with a loaded
career profile; challenges use the last bucket.

Synthetic checks cover event maximum precedence, absent `MaxRaceHeatLevel`,
career/quick-race/challenge/final bounds, profile-dependent growth and explicit
development overrides. Installed checks inspect all ten rows of both tables and
keep wave size distinct from engagement budget. These are code and data checks,
not matched original/remake gameplay acceptance. No new original-game recording
was used. The unavailable PC `MaxRaceHeatLevel` field retains the bin maximum;
its getter implementation remains an evidence gap.

The generated `pursuitlevels` declarations identify `CollapseInnerRadius` as
`Int32`, unlike the floating-point outer radius. The reader preserves this
distinction; installed tests check the selected collection, field and decoded
type for every heat row before testing the resulting runtime tables.
