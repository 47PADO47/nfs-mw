# Game-world environment audit

Scope: native game-world rendering at upstream base `e7d201e1`. This is a code
and installed-data audit, not a matched gameplay comparison.

## Fog and haze

World lighting supplies atmospheric scattering from installed sky and haze
colours, scalar parameters, sun direction and cloud state. Original world
material families consume it; some unfogged material families intentionally do
not. The effect and soft-particle shaders instead apply linear distance fog.
The resulting full-scene appearance, especially particles and shelter, has not
been compared with a matched original frame. Do not infer a missing fog volume
from this difference without a reference location and draw evidence.

## Local world particles

The inspected PC stream has 290 emitter-library chunks with 1,535 group
placements. Of these, 23 placements have 42 associated trigger records. The
group keys and textures are read from the install. The prior world streamer
did not consume these libraries. The initial implementation handles ordinary
untriggered resident placements; its limits and remaining trigger behavior are
in [world-emitter-placements](specs/world-emitter-placements.md).
The installed group names identify 577 ordinary and thick fog placements, 223
chimney and smokestack placements, and eight small steam placements. Other
groups include leaves, birds and fountains. Some group counts include triggered
placements; the reader alone does not prove their visual appearance. Gameplay
verification is still required.

## World mesh animation

The track contains eleven entity trees, eleven instance records and thirteen
animation-bank objects. Named channels include the airliner, blimp, cargo ship,
sailboat and seven tower cranes. The world now connects the ordinary looping
entities to their animation channels and meshes. The donut sign and windmill
blades use a separate continuous rotation rule, also implemented. Triggered
and other animation modes remain outside this initial implementation. Texture
animation and wind sway are separate existing systems.

Do not mark any of these as visually complete without installed gameplay checks
and comparisons to the corresponding original scene.
