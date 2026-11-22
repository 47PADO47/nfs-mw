# Moving lamp shadows

## Native lighting audit

Player headlights use up to two intact head markers. Traffic and police headlights use the same
damage masks and select up to eight nearby cars within 150 m (default four). Their beams already
light roads, walls and car paint. Player beams and the first four traffic beams precede fixtures
for raster shadows. The remaining beams compete for spare slots. Wet glints use the same visibility
as diffuse light. The default pool admits 128 lights and eight 1024-square shadow maps within 80 m.

Player brake and running lights already emit a 5 m red spill from intact tail markers. AI rear
lamps have artwork but no equivalent surface spill. Flashing police bars emit up to three 14 m
colour cones per car using the COPLIGHT marker clock, for the nearest four active cars. Both
surface spill types previously bypassed shadow maps.

## Controls and cost

Enable the desired spill in F8 > Lighting > Local lights (prototype), then enable **Moving spill
shadows**. The console equivalent is `set moving_spill_shadows on`. The switch defaults off.
Up to two rear or bar cones per frame take available shadow slots after priority headlights
and before fixtures. One slot is reserved for an active bar when both types are present. Moving vehicles and resident scenery use
the existing caster feed. Budget overflow still produces unshadowed spill.

The upper cost estimate is two moving-light depth passes plus filtered comparisons on lit pixels.
They can displace two fixture shadows. Map storage stays within the configured pool. No ray queries
are added. Wide cones spread their depth resolution across more road than a headlight beam.

Sources are the project's local-light emitter, shadow-feed and renderer code and the
[local lamp shadow audit](local-lamp-shadows.md). No external implementation was ported.

## Installed-game measurements

On 2026-11-20 the native release build ran on a Vulkan RTX 3070 with the installed city,
rain at 75%, experimental night, eight 1024-square local maps and RT sun and reflection
options requested. The parked BMW held the brake near (2363, 3350). Matched 1920×1080
captures preserved the red spill on open road. A second capture pair spawned a COPMIDSIZE
during a heat-one pursuit. Enabling the setting darkened the wet pavement under the police
car while its roof flashes and forward beams remained visible. The captures do not test
interior receivers or every vehicle mesh.

In one running 3840×2160 scene the console toggled the setting off, on and off again.
Three-second traces gave these medians from delayed GPU timestamps and local-shadow counters.

| Moving spill shadows | GPU samples | Base render (ms) | Selected lights | Local caster draws |
|---|---:|---:|---:|---:|
| Off | 58 | 13.125 | 4 | 166 |
| On | 135 | 13.227 | 4 | 272 |
| Off again | 146 | 13.206 | 4 | 165 |

The enabled median was 0.02–0.10 ms above the two disabled medians. That is whole-render
timing for this view, not an isolated pass measurement. The roughly 106 additional caster
draws confirm that the two rear maps actually rendered. Across four separate 12-second
benchmarks, median frame time varied between 17.10 and 19.91 ms while other worktrees
compiled. This does not establish a general frame-rate cost for pursuits or moving traffic.
