# Menu platform scenery light

Status: implemented for the front-end platform stages; checked offscreen against stock and modded PC captures of
the main menu. The safe house tone and the other platforms still need matched captures.

## Original behaviour

The front end does not draw its platform files as pre-lit geometry. Each shading group keeps the world effect its
effect id selects ([world material routing](world-material-routing.md)): ordinary scenery uses the World family
and normal-mapped scenery uses the normal-map family. In both families the vertex colour is the ambient term:

`RGB = 2 * T.rgb * (A * V.rgb + D * L)`

`A` is the time-of-day ambient colour times its intensity (4 on PC), `D` the diffuse colour and `L` the Lambert term
of the sun. The platform files author small vertex colours (about 20/255 on the crib's walls and floor), so drawing
them as `2 * T * V` alone leaves the room nearly black. **[verified: installed stock and modded crib data; matched
stock capture brightness]**

The platform object builds its draw masks by solid-name suffix: `CAST_SHADOW_MAP`, `CURRGEN` and `NEXTGEN`
**[exe]**. It posts message `0xA05A328E` to the car select screen when the selected car changes, which runs that
screen's loading LED row **[exe]**.

## Light source

The world shaders read the shared time-of-day lighting object ([time of day](time-of-day.md)). In game state 3,
which is the front end **[mod source cross-check]**, the sun's phase limit is 0.35. The remake places the front-end
sun at that limit with the lighting object's default orbit offsets. It uses the clear-sky preset pair (no weather
runs behind the menus) and the same caps as the world: diffuse at most 1, ambient intensity 4, specular power 11.
The light stays in the world frame. The original turns only the platform's solids (`menu-backdrop-looks.md`, "The room's turn"), not the lighting object.

Not established:

- The phase the front end actually holds. The integrator may still move it inside `[0.35, 0.65]` while the menus run.
- The front end's cloud blend.
- Whether any platform uses a separate light.

These are **[ours]** until a live read of the lighting object in the front end confirms them. Missing or invalid
presets leave the scenery on the renderer's fixed terms rather than invented colours.

## Remake rules

- Room groups use the world routing and normal materials. Groups missing normal-map inputs fall back to the World
  family.
- Shadow casters stay separate inputs; they are not drawn twice.
- The menu frame carries the scenery light: its direction and surface light. The car keeps its own front-end rig.
- The renderer's original-world-materials setting selects the authored arithmetic. With it off, the remake's
  approximate world lighting applies, as in gameplay.
- No global brightness factor, grade or emitter compensates for the light.

Authored platform emitters run as particles (`menu-backdrop-looks.md` section 4.5). Animated light textures are separate work.
