# Optional high-quality smoke

Written before implementation, 2026-10-08. This is an original visual option,
not a reproduction of the original game's particle assets or algorithms.

Standard remains the default: 35 particles per second per wheel at full existing
physics smoke intensity, 512 live particles, the existing analytic mask.
High uses 70 per second, at most 1536 live particles, smaller varied puffs and
rotating, evolving procedural density. Lifetime stays 1.4-2 seconds. Both modes
use fixed-step emission and far-to-near billboard sorting. Changing quality
clears smoke and fractional emitters while retaining marks.

High samples the completed opaque scene depth. It reconstructs the scene point
with inverse view-projection and fades smoke alpha over the first 0.30 m of
separation along the camera ray. Reverse-Z scene depth greater than particle
depth means the particle is occluded; empty depth means no intersection fade.
The particle pass reads depth after the scene pass ends, without binding that
texture as an attachment. Captures use their own depth target. This smooths
visible car, road and wall intersections but does not make particles collide
with geometry or simulate volumetric smoke.

Engine inputs stay generic: procedural particle detail, per-particle age/seed,
and a world-unit soft intersection distance. Game quality configuration remains
in the application layer. Select through settings, CLI and F12 console, and the
Video options in the main and pause menus now merged into main. The menu saves
the selection through the existing config writer. A new driving scene inherits
the current quality; changing it while paused refreshes the visual buffers
without advancing the frozen vehicle simulation.

Reference for the soft-particle concept only: NVIDIA GPU Gems 3, chapter 23,
section 23.4, https://developer.nvidia.com/gpugems/gpugems3/part-iv-image-effects/chapter-23-high-speed-screen-particles.
No shader listing or other source code is copied. The density function and
reverse-Z reconstruction are written for this renderer.
