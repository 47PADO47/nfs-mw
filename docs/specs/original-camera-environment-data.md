# Original gameplay camera environment inputs

## Installed data contract

Camera collision and terrain movement require existing game attributes alongside
the per-car camera profiles. Read these through the normal inherited AttribSys
collection API; do not infer camera eligibility from prop size or substitute an
audio surface category for terrain camera parameters.

`smackable.CAMERA_AVOIDABLE` is a boolean controlling whether an object is eligible
for the camera's object-avoidance query. Preserve it in each loaded prop's dynamics
data independently of collision sound, mass, breakability and respawn settings.
A missing field retains the reader's false fallback. Eligibility alone does not
define the obstacle shape, query mask, contact tolerance or camera correction;
those belong to the camera obstruction pipeline.

`simsurface.CAMERA_NOISE` is a float array consumed by the native terrain-camera
path. Preserve its first two values in order and make them available by the
contact surface name hash, independently of grip and audio data. Unknown hashes
use the installed `unknown` surface's values. If no usable collection exists,
use a zero pair. The data reader does not scale, interpolate or integrate the
values; interpretation and retained timing belong to the native follow pipeline.
The pair order is amplitude then frequency, multiplying the corresponding
native terrain-noise speed-curve outputs. Each absent array item independently
uses zero; a one-item array does not repeat its first value into the second slot.

## Verification

Ignored installed-data tests compare every usable prop dynamics record's flag
and every surface's pair with the resolved source attributes. Synthetic fallback
tests cover unknown and absent surface records. These checks establish the data
feed, not a match of the full resulting movement or camera collision response.

## Runtime producers

Read player and AI camera geometry from their physical rigid bodies, not the
drawable model origin. Convert simulation vectors `(right, up, forward)` to
render-world vectors `(forward, -right, up)`. Avoidable-body matrices retain
local forward, right and up axes, with the physical body center as translation;
half extents use that same axis order. Use the rigid body's own radius for the
under-vehicle predicate, independently of padded collision-shape broad phases.

The player anchor excludes lateral center-of-gravity offset and adds the other
two rotated components to physical body position. For a render interpolation
query, transport that anchor by the displayed car pose's change from the current
physics model pose. Do not add an artificial front-bumper distance.

Existing resident AI vehicles and their trailers expose physical current poses.
Eligible props must have an existing registered rigid body, be collidable and
visible, and not be sinking. Their installed camera flag gates eligibility.
The host exports AI actors in resident vector order and props by stable ID;
matching the original game's global registration-list order remains unverified.
Static world collision remains a separate source.

Terrain noise uses the current contacted surface; engine overrev is RPM above
redline minus 500; brake is true for a positive latest physical-step brake or
handbrake control. The current free-roam
host supplies no drag-race flag. Helicopter distances use the current full 3D
physical position, converted to the same render coordinates as the player.
Native retained collision timers and scalar speed-derivative acceleration are
owned by the camera timeline, not fabricated from model interpolation deltas.

Camera impact events retain each physics tick's world, ground and object contact
sources independently of the audio queue's render-time drain. Feed the incoming
relative velocity's closing normal component, clamped at zero, to the camera
timers. Tangential sliding speed and response delta-v do not contribute to that
value. The consumer identifies physics steps and processes each at most once;
waiting for a car or using the free camera must not replay retained contacts.

Gear-change input compares the transmission's integer gear with the retained
previous value. Its native IDs are reverse0, neutral1, first2, then increasing
forward gear IDs; the HUD subtracts one, while this input retains the raw IDs.
Every change qualifies, including reverse/neutral transitions;
do not restrict it to upshifts. Seed the previous value when attaching a car so
its initial gear does not create a transition. Transient pulse gating and
duration belong to the camera consumer, independently of this raw input.
