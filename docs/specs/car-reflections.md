# Car environment reflections

Sources: the retail PC v1.3 `IDI_CAR_FX` effect, already described in
[car-assembly.md section 8](car-assembly.md#8-the-car-shader), and
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0 decompiled
reference, revision `80993f35224c61d2f1b135beba76c6f8234c7940`:
`CarRender.cpp` (`UpdateEnvironmentMapCameras`), `eEnvMap.hpp` and
`EcstasyData.hpp`. Reference code stays outside the repository. This specification
describes behavior; the implementation is new Rust and WGSL, not translated code.

## Established behavior

- **[verified PC shader]** Car light materials control reflection strength with
  their environment minimum, range and facing power. The sampled environment
  contributes `environment * baked_occlusion * strength * diffuse_alpha * 0.5`.
  The reflection direction mirrors the view direction about the surface normal.
- **[decomp, shared game code]** One environment map has six cameras and six
  views: front, right, back, left, up and down. It follows the player's geometry
  anchor, falling back to the player's rigid body or a front-end car.
- **[decomp]** The geometry anchor is raised by 0.95 world units for the
  environment origin. The viewer position receives a separate 0.6-unit vertical
  offset. The rigid-body fallback compensates the anchor offset first. The exact
  PC `UpdateCameras` placement calculation is not recovered here.
- **[verified strings]** PC graphics options distinguish enabling car environment
  maps from updating their contents. No static car cube map was found in the base
  install. A generated sky alone is therefore not an implementation of the
  original scenery reflections.

## Capture contract

The game supplies a world-space origin and scenery from the loaded surroundings,
including objects behind the gameplay camera. Camera-frustum culling of the
player view must not determine the reflected content. The player car is omitted
to avoid seeing the inside of its own body. Menus, HUD, screen effects and AO are
not part of this capture.

The renderer produces square 90-degree perspective views with a common origin,
in cube addressing order +X, -X, +Y, -Y, +Z, -Z. The game uses Z-up coordinates;
reflection sampling stays in game coordinates, matching the existing glossy
shader. Each face is drawn straight into its cube face, the store pass's vertical
flip folded into the face's view. A refresh is spread over a few frames: High
draws two faces a frame (a full cube every three frames), Medium and Low three,
the cached mode all six; the first capture, a settings change, a clock jump or
30 m of travel draws all six at once. Irradiance is projected once all six faces
are fresh. No GPU-to-CPU readback is needed during play. Each face has its own
camera uniform and instance buffer, so a capture uses one queue submission
without one face overwriting another's camera. Those buffers and render targets
are reused between captures.

Captures preserve the original gamma-space scene colour. Bloom, tone mapping,
TAA, FSR and the player-view render scale do not alter a reflection capture.
Existing light-material ramps and shader reflection strength remain unchanged.

## Choices and limits

Capture sizes, update cadence, near plane and distance limits are rewrite tuning,
not claimed original PC constants. Quality offers an off mode retaining the old
generated sky and progressively larger captures. A scene/camera cut refreshes
when the car origin changes substantially. Disabling capture releases its render
resources and restores the previous static environment. The scenery is limited
by what is already streamed.

`reflections` defaults to medium: low uses 64-pixel faces at up to 15 captures per
second, medium uses 128 at up to 30, high uses 256 with two faces refreshed every
frame. Each face uses its
own scenery frustum/LOD selection at its capture resolution rather than drawing
the entire surroundings six times. The same object may use different meshes in
the player view and capture. Simple car shading skips captures. Video options,
the `reflections` console command, `--reflections` and `NFSMW_REFLECTIONS` provide the same quality/on-off
comparison. No mod assets are needed or distributed.
The high setting updates once per rendered frame, including above 60 FPS; it does
not inherit an original-game frame-rate limit. Low and medium use elapsed seconds
and refresh early after a significant move. Bevy capture implementation is left
to its backend maintainer; unsupported backends keep their static environment and
hide the capture setting without overwriting the stored preference. Streaming or
a change to the scenery visibility set invalidates the capture even when the game
clock is paused, so loading screens cannot leave a rate-limited capture empty.

This pass covers car surfaces using the car shader, including their window
materials. Road and water reflections need distinct original shader contracts;
they are not enabled by painting the whole world with the car reflection shader.

## Validation

Synthetic scenes must verify face orientation, reflection of off-camera geometry,
exclusion of the player car and HUD, colour preservation, moving origins, quality
changes and disabling/re-enabling. Installed-game captures must show building and
road detail on the car at different locations using base-game assets. Automated
checks do not establish full visual parity with every original material or scene.
