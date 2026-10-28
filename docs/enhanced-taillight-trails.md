# Enhanced NOS taillight trails

This optional presentation effect replaces the historical brake-flare copies with
short connected ribbons. F8 > Effects offers a master switch, an enhanced switch,
length, exposure, width, brightness, glow and tint. Disabling enhancement retains the original PC
path; disabling the master switch hides both styles. Existing lamp surfaces and
the centre brake light are unchanged.

All five sliders default to 100%. Length sets a 6 metre spatial cap; exposure sets a 120 ms window.
Each is multiplied independently by its percentage. A bounded Hermite curve
reconstructs the lamp path and is sampled at uniform distances, with at most
65 points. This retains the arc through a corner instead of a short straight smear. Width is a 0.10 metre
width reference, with a 0.165 metre soft halo and a 0.045 metre core half-width.
Brightness controls emission independently of coverage; the core emits above
display white to feed the existing bloom pass. Glow controls halo coverage without
changing the global bloom setting. At zero glow, the core can still produce
optical bloom when the global bloom effect is enabled. Settings persist globally
across car changes and F8 snapshots, independently of style/quality presets.
Source defaults retain original trails; a preview build can enable enhancement
in its own settings profile.

The existing physics feed still determines actual NOS burning. Enhanced history
records interpolated displayed poses once per world update. Each rear lamp traces
its own historical positions, including the car's rotation. Time and distance
clipping interpolate boundary positions rather than selecting a fixed sample count.
The continuous head-to-tail UV coordinate keeps the shader fade from restarting
at each segment. Shared ribbon edges prevent holes between segments.

Presentation time is scaled with Speedbreaker and freezes with the world. Releasing
NOS keeps the emitted head at its historical position and removes the oldest
part of the path while fading it over 120 ms of presentation time. Invalid
poses, large jumps and long update gaps clear history; existing car replacement,
respawn and hidden-car resets clear it too. History is bounded at 128 poses.
This does not change physics frequency or gameplay.

Installed marker directions, broken-light states, the ordinary rear-light intensity
and camera visibility still gate the effect. The centre brake marker is excluded.
No new textures, particles, post-processing pass or persistent scene lights are
required. Geometry uses the existing additive, depth-tested streak pass.

The appearance is a new artistic effect, not a claim of original-game parity.
It uses the existing in-project streak shader. Low presentation rates still limit
path detail; interpolation cannot recover motion absent from the source samples.
Visual quality and GPU cost require driving comparisons, including sharp turns,
bright and dark scenes, damaged lamps, Speedbreaker and different frame rates.

Original trail behaviour and provenance: [nitrous-light-trails.md](specs/nitrous-light-trails.md).

The ribbon starts at a narrow lamp aperture and widens over its first 0.35 metres.
Triangles inside the current car collision box are removed with interpolated UVs;
ordinary scene depth testing still hides other obstacles. This owner exclusion is
an approximation of the body, not a per-pixel mesh silhouette.

The native speed blur now reads player speed and actual NOS burning directly,
with its own presentation-time smoothing. Traffic/police presence no longer gates
the blur. The existing motion blur switch and strength still apply.

## NOS lamp appearance

Enhanced trails can use a thin ribbon or a short installed-texture flare head
feeding the ribbon. Flare heads reuse at most two installed rear profiles, three
small samples per lamp. This follows the authored flare artwork, not the exact
lens mesh silhouette. No original-game texture is shipped with this feature.

NOS lamp glow selects the existing left/right lamp ON textures as a cosmetic
state. The centre slot still requires real braking. Broken lamps retain their
individual damage state. Enhanced lamp materials remain governed by the existing
Enhanced vehicle lamps option. Ordinary actor lamps do not receive NOS state.
The cached mesh variants deduplicate equivalent ON states; nothing is uploaded
per frame to animate the lamp switch.

Speed response smoothly scales emission and modestly adjusts trail length,
with bounded gain from 0.4 to 1.4 even at extreme speeds/settings. At zero response
the strength is constant. Softness redistributes energy between a wider halo and
a narrower core. Shimmer adds travelling centimetre-scale variation away from the
root, using world presentation time so pause and Speedbreaker remain coherent.
Zero shimmer removes it. The lamp attachment point is never displaced.
## Stationary preview

F8 > Effects > Enhanced NOS taillight trails offers a session-only preview.
It holds world simulation while synthetic presentation history follows a selected
speed (10–400 km/h) and corner bend. Speed affects the same exposure, length and
lamp response used during driving. Both directions of bend are available.

The newest sample stays attached to each displayed lamp. Shimmer advances from
wall time while previewing, even though gameplay is held. No NOS, braking or
vehicle motion is requested. Disable preview or close F8 to clear the synthetic
history and resume the previous simulation state. Preview controls are excluded
from settings, startup files and F8 snapshots; appearance edits remain saveable.

## Appearance recipes and lens dimensions

Enhanced MW, Long Exposure and Pursuit buttons apply complete appearance recipes,
retaining the selected tint and trail master switch. Manual edits remain editable
and saved as concrete values. Lamp bloom controls the installed-texture flare head;
trail bloom controls the ribbon halo. NOS lamp glow controls the lamp ON state.
Thin ribbon intentionally excludes the textured head.

Follow lamp dimensions measures nearby vertices of named BRAKELIGHT solids during
car loading. Bounded width/height influence the head and ribbon width. Unrecognized
or combined body/lamp meshes use a compact fallback. This is a dimension estimate,
not lens segmentation or exact round/multiple-lamp silhouette reconstruction.

Boost engagement adds a bounded 160 ms emission pulse. Release progressively
removes oldest history over 120 ms, alongside opacity fading. The newest released
sample stays at its historical position. Shimmer uses lower-frequency, smaller
position offsets and restrained brightness modulation. F8 preview can cycle boost
on/off without consuming NOS. No additional render pass or per-frame mesh upload
is introduced. Road spill and an under-car trail remain future experiments.
