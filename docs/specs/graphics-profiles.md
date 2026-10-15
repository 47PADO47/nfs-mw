# Graphics profiles and shared controls

## Scope

One persistent request drives menu, F8 and renderer. Visual style selects a
baseline; quality changes rendering cost without changing colour treatment.
The existing Black Box renderer is not synonymous with original-game graphics.
Original PC remains explicitly incomplete. This work organizes existing effects;
it does not implement the missing original shader families.

The reference target may use the owner's modded PC installation for effects
and compatibility improvements that can be reproduced with stock game assets.
Separate verified stock behavior, asset-independent restored behavior and
mod-asset-dependent appearances in the coverage ledger. A mod pack is not a
runtime dependency. Reference captures/configuration/hashes stay private.

## State and persistence

Settings owns Appearance (base style, quality and stable lighting options).
Existing grading, reflections, AO and post fields remain canonical. F8 holds
only transient UI/diagnostic state and a working view during an egui pass.
It reads the shared request before preview and commits edits afterward; the
renderer consumes the shared request, never a second independent lab profile.
Normal launches preserve legacy preferences. Missing appearance data means
Custom and existing lighting defaults, with no automatic art change.

Styles: Original PC (incomplete), Enhanced and Custom. Original selects the
available original midday treatment, authored shadow overlays and original-style
car reflections; custom dynamic shadows/AO/clearcoat/ambient additions and
modern tone mapping/bloom are disabled. Missing original effects remain gaps,
not inventions. Enhanced retains the current art baseline: ACES, subtle Visual
Treatment, glossy cars and modern lighting. Custom preserves manual choices.
A modified baseline displays Custom while retaining its base for quality changes.

Quality Low/Balanced/High owns map size/range, filtering cost, reflection budget
and AO budget. It does not change tint, treatment strength, AA or tone mapping.
Style selection is explicit. Environment time/rain, audio, camera, frame cap,
resolution and controls are outside style/quality ownership.

A versioned appearance table persists stable lighting options. Freeze time and
surface debug are session diagnostics and are not serialized. Malformed or
future schemas produce a diagnostic and preserve legacy settings. Writes keep
unrelated config keys. F8 Save profile writes the owned appearance/post/quality
request only; environment overrides are excluded. Revert restores the startup
request, A/B swaps owned graphics, and section reset affects only its domain.

## UI and coverage

The Video menu offers visual style and quality; advanced effect tuning lives in
F8 tabs for Look, Lighting, Materials, Effects, Environment and Diagnostics.
Changes preview immediately. Unsupported features retain the request and expose
the backend's effective result/reason. F8 is resizable and owns input like F6.
Neither interface silently resets the other. Original shadow overlays are
original art, not a complete original dynamic-shadow implementation.

Maintain a typed feature inventory with origin, parity coverage, current method
and remaining work. Render the same inventory in Diagnostics and document it.
Status is independent of availability: implemented does not mean original parity.
No new original behavior is claimed without a spec and matched evidence.

## Validation

Cover legacy migration, config round trips/unknown keys, malformed and future
schemas, style/quality independence, custom overrides, gameplay isolation,
menu-to-F8-to-renderer synchronization, A/B/revert, save/reload and capabilities.
Keep inactive effects neutral. Test live resize, paused world changes and both
native graphics APIs. Per-frame synchronization must not reset clocks or rebuild
unchanged renderer resources. Existing high-FPS camera/clock checks remain valid.
