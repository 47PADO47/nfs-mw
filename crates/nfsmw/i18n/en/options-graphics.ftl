### options-graphics
### The Video page's rows (`frontend/options_rows.rs`, `graphics_options*.rs`, `renderer_options.rs`, `post_options.rs`,
### `options_effective.rs`). Names that are not language (FSR 1, DLSS, Vulkan, FXAA, Black Box) stay in the code.

## Display and frame rate

options-window-mode = Window Mode
options-window-windowed = Windowed
options-window-borderless = Borderless
options-window-exclusive = Exclusive
options-frame-limit = Frame Limit
options-frame-limit-unlocked = Unlocked
# $fps is the frame limit (30, 60, 144).
options-frame-limit-fps = { $fps } FPS
options-performance-overlay = Performance Overlay
options-metrics-basic = Basic
options-metrics-advanced = Advanced
# The row that opens the F8 graphics lab, and what it shows.
options-advanced-graphics = Advanced Graphics
options-open-f8 = Open F8

## Renderer

options-renderer = Renderer (After Restart)
options-graphics-api = Graphics API (After Restart)
options-ray-tracing = Ray Tracing
options-frame-generation = Frame Generation

## Resolution and upscaling

options-render-scale = Render Scale
options-upscaler = Upscaler
options-upscaler-bilinear = Bilinear
options-sharpening = Sharpening
options-upscale-quality = Upscale Quality
# The temporal upscalers' quality modes.
options-upscale-native = Native
options-upscale-quality-mode = Quality
options-upscale-balanced = Balanced
options-upscale-performance = Performance
options-upscale-ultra-performance = Ultra Performance

## Post-processing

options-anti-aliasing = Anti-Aliasing
options-motion-blur = Motion Blur (High Speed)
options-motion-blur-strength = Motion Blur Strength
options-tonemap = SDR Tone Mapping
options-bloom = Bloom

## Style and quality

options-hdr = HDR Output (Auto / On / Off)
options-visual-style = Visual Style
options-style-original-pc = Original PC (incomplete)
options-overall-quality = Overall Quality
options-graphics-preset = Graphics Preset
options-visual-treatment = Visual Treatment (Experimental)
options-treatment-subtle = Subtle
options-treatment-reference = Reference (mod)
# A treatment strength set from the console or the config file. $percent is a whole number.
options-treatment-custom = Custom ({ $percent }%)

## Detail

# $level is the world detail (Low, Maximum).
options-world-detail-value = { $level } (Textures: next world load)
options-car-geometry-low = Low (adaptive)
options-car-geometry-high = High (adaptive)
options-car-geometry-highest = Always highest
options-traffic-range-preset = Traffic Range Preset
options-traffic-reference = Reference
options-traffic-extended = Extended
options-filtering-trilinear = Trilinear
options-filtering-anisotropic-8x = Anisotropic 8x
options-filtering-anisotropic-16x = Anisotropic 16x
# A filtering change the running renderer applies only after a restart. $filtering is the mode.
options-filtering-restart = { $filtering } (Restart)

## Shadows and ambient occlusion

options-shadow-quality = Shadow Quality
options-sun-shadows = Enhanced Sun Shadows
options-sun-shadow-resolution = Sun Shadow Base Resolution
options-sun-shadow-distance = Sun Shadow Distance
# $metres is a whole number of metres.
options-shadow-distance = { $metres } m
options-shadow-distance-far = { $metres } m (+ distant coverage)
options-sun-shadow-casters = Sun Shadow Caster Detail
options-shadow-casters-all = All scenery
# $radius is in metres, with one decimal.
options-shadow-casters-radius = Scenery radius ≥ { $radius } m
options-fe-shadows = Front-End Shadows
options-ambient-occlusion = Ambient Occlusion
options-ao-intensity = AO Intensity
options-ao-comparison = AO Comparison
options-ao-comparison-split = Split Screen

## Reflections and car shading

options-reflection-quality = Reflection Quality
options-car-shading = Car Shading
options-car-shading-simple = Simple
options-car-shading-shadowless = Original PC (shadowless)
# An original car material asked of a renderer that draws it with its own shader.
options-car-shading-fallback = Custom shader fallback
options-car-reflections = Car Reflections
options-reflections-static-sky = Static Sky
options-road-reflections-world = World
options-road-reflections-vehicles = World + vehicles
options-road-reflections-blurred = World + vehicles + blur
options-cube-size = Car Cubemap Resolution (Per Face)
options-pow2-cube = Power-of-Two Cube Size
options-vehicle-brightness = Vehicle Reflection Brightness
options-reflection-detail = Reflection Detail
options-reflection-detail-capture = Capture
options-reflection-detail-view = Main View
options-reflection-content = Reflection Content
options-reflection-content-restored = Restored
options-reflection-distance = Reflection Distance
options-mirror-size = Mirror Image Height
options-mirror-tint = Mirror Tint
options-mirror-tint-stock = Stock

## Vehicle effects

options-tire-smoke = Tire Smoke
options-smoke-quality = Smoke Quality
options-skid-marks = Skid Marks
options-collision-sparks = Collision Sparks
options-sparks-restored = Restored (Experimental)
options-speed-trails = Speed Trails (Experimental)
options-speed-trail-style = Speed Trail Style
options-speed-trails-streaks = Speed Streaks
options-speed-trails-car = Car Only
options-exhaust-flames = Exhaust Flames
options-custom-fonts = Custom Fonts
options-hd-fonts = HD Fonts
options-hd-cursor = HD Cursor
options-hd-movies = HD Movies
