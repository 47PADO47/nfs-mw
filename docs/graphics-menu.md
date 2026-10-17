# Video options

The Video list groups display and frame pacing first, followed by the graphics
preset, resolution and upscaling, scene shading and post effects, vehicle effects,
and the performance overlay. Renderer and graphics API choices explicitly say
they apply after restarting. Unsupported renderer features remain hidden.

Controls stay selectable when their parent is off, but are dimmed and ignore
adjustment. Smoke Quality requires Tire Smoke. Render Scale requires spatial
upscaling; Sharpening (Off, Medium, High) requires FSR 1 or FSR 3. Temporal upscalers own resolution and
anti-aliasing, so they enable Upscale Quality and disable those manual controls.
The active renderer's resolved settings determine availability: a saved DLSS
preference falling back to FSR 1 permits manual anti-aliasing and sharpness.
Its resolution still comes from the saved temporal quality mode, so Render
Scale remains disabled. Values are retained when their parent is disabled.
