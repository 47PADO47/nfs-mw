//! The options the viewers and the game share, and how they become settings.

use std::path::PathBuf;

use blackbox_gfx::GraphicsApi;
use clap::Args;

use super::hud_demo::{parse_hud_demo, parse_hud_scenario};
use super::parse::parse_screenshot_size;
use crate::app::pacing::MaxFps;
use crate::devtools::{ShowMetrics, ShowReadout};
use crate::settings::{Monitor, Resolution, WindowMode};

/// Options shared by the viewers.
#[derive(Args, Clone)]
pub struct ViewArgs {
    /// Graphics backend: auto, vulkan, dx12 or gl [env NFSMW_BACKEND; default auto].
    #[arg(long)]
    pub backend: Option<GraphicsApi>,
    /// Which renderer draws: blackbox (the native one) or bevy; it applies on the next start, and a build or PC
    /// without the bevy renderer uses blackbox [env NFSMW_RENDERER; config `renderer`; default blackbox].
    #[arg(long, value_name = "blackbox|bevy")]
    pub renderer: Option<crate::settings::RendererKind>,
    /// Disable vsync [env NFSMW_VSYNC=off; config `vsync = false`].
    #[arg(long)]
    pub no_vsync: bool,
    /// NVIDIA Reflex: off, on or boost; available with native Streamline Vulkan.
    #[arg(long)]
    pub reflex: Option<crate::settings::ReflexMode>,
    /// DLSS frame generation: off, 2x, 3x, 4x, 5x or 6x (hardware-dependent).
    #[arg(long)]
    pub frame_generation: Option<crate::settings::FrameGeneration>,
    /// Frame-rate cap: a number such as 60, or `unlocked` (vsync still applies unless --no-vsync)
    /// [env NFSMW_MAX_FPS; default unlocked].
    #[arg(long, value_name = "FPS|unlocked")]
    pub max_fps: Option<MaxFps>,
    /// Performance overlay: off, basic or advanced [env NFSMW_SHOW_METRICS; default off].
    #[arg(long, value_name = "off|basic|advanced")]
    pub show_metrics: Option<ShowMetrics>,
    /// Window mode: windowed, borderless or exclusive [env NFSMW_WINDOW_MODE; default windowed].
    #[arg(long, value_name = "MODE")]
    pub window_mode: Option<WindowMode>,
    /// Monitor: current, primary or a zero-based index [env NFSMW_MONITOR; default current].
    #[arg(long, value_name = "MONITOR")]
    pub monitor: Option<Monitor>,
    /// Physical resolution: WIDTHxHEIGHT or native [env NFSMW_RESOLUTION; default native].
    /// Borderless uses the desktop size; screenshots keep their fixed size.
    #[arg(long, value_name = "WIDTHxHEIGHT|native")]
    pub resolution: Option<Resolution>,
    /// The scene's debug readout, bottom left: off, minimal (one line next to the original HUD) or full
    /// [env NFSMW_SHOW_READOUT; config `show_readout`; default minimal].
    #[arg(long, value_name = "off|minimal|full")]
    pub show_readout: Option<ShowReadout>,
    /// Render one frame to this PNG file and exit instead of opening an interactive window.
    #[arg(long, value_name = "FILE.png", group = "capture_target")]
    pub screenshot: Option<PathBuf>,
    /// Physical render target for a screenshot or hidden benchmark; independent of window preferences.
    #[arg(long, hide = true, requires = "capture_target", value_name = "WIDTHxHEIGHT", value_parser = parse_screenshot_size)]
    pub screenshot_size: Option<[u32; 2]>,
    /// For --screenshot: seconds to wait after the scene is ready before the first capture [default 0].
    #[arg(long, requires = "screenshot", value_name = "SECS")]
    pub screenshot_delay: Option<f32>,
    /// For --screenshot: how many captures to take; more than one numbers the files (out-1.png, out-2.png, ...) [default 1].
    #[arg(long, requires = "screenshot", value_name = "N")]
    pub screenshot_count: Option<u32>,
    /// For --screenshot with --screenshot-count: seconds between captures [default 1].
    #[arg(long, requires = "screenshot", value_name = "SECS")]
    pub screenshot_interval: Option<f32>,
    /// Measure frame times for this many seconds once the scene has loaded, print p50, p95, p99 and the worst
    /// frame with the renderer and the CPU and memory use, and exit. Add --no-vsync for meaningful numbers.
    #[arg(long, value_name = "SECONDS", conflicts_with = "screenshot", group = "capture_target")]
    pub bench_seconds: Option<f32>,
    /// With --bench-seconds: keep the window hidden, unfocused and off-screen, as a screenshot run does, so a measurement
    /// never takes focus from the desktop.
    #[arg(long, requires = "bench_seconds")]
    pub offscreen: bool,
    /// Run a console command once the window is up (repeatable), e.g. --exec "fps 60" --exec "car PORSCHE911". A line
    /// `wait <seconds>` holds the lines after it that long; `wait ready` holds them until the world has loaded and the
    /// driven car is on the road. Each line and its reply are printed.
    #[arg(long, value_name = "COMMAND")]
    pub exec: Vec<String>,
    /// Start with the developer console open.
    #[arg(long, hide = true)]
    pub open_console: bool,
    /// Start with the runtime garage overlay open (world and driving scenes).
    #[arg(long)]
    pub open_garage: bool,
    /// With --open-garage: open the vehicle panel (F11) on this page instead of the F6 car browser. The pages are performance,
    /// tuning, appearance and parts (what the F6 garage's tabs were).
    #[arg(long, value_enum, requires = "open_garage")]
    pub garage_tab: Option<crate::garage::OptionsTab>,
    /// Play no sound (no output device is opened).
    #[arg(long)]
    pub no_sound: bool,
    /// Run the audio through a silent output: no device opens, the audio trace still works [env NFSMW_AUDIO_OUTPUT=silent].
    #[arg(long, conflicts_with = "no_sound")]
    pub silent_audio: bool,
    /// Master volume, 0 to 100 [env NFSMW_MASTER_VOLUME; config `master_volume`; default 80].
    #[arg(long, value_name = "0-100")]
    pub volume: Option<crate::settings::Percent>,
    /// Mute all audio: everything keeps playing (the radio and the songs too), silently.
    #[arg(long, conflicts_with = "no_sound")]
    pub muted: bool,
    /// Show the in-game HUD in any viewer (while driving it is on unless the `hud` setting is off)
    /// [env NFSMW_HUD; config `hud`].
    #[arg(long)]
    pub hud: bool,
    /// Hide the in-game HUD even while driving.
    #[arg(long, conflicts_with = "hud")]
    pub no_hud: bool,
    /// HUD placement: pc, classic (centered) or xbox360 [env NFSMW_HUD_LAYOUT; default pc].
    #[arg(long, value_name = "pc|classic|xbox360")]
    pub hud_layout: Option<crate::settings::HudLayout>,
    /// The look of all HUD and menu UI: original, custom or remastered [env NFSMW_UI_STYLE; config `ui_style`; default original].
    #[arg(long, value_name = "original|custom|remastered")]
    pub ui_style: Option<crate::settings::UiStyle>,
    /// The language of the menus and the HUD, a tag such as en, de or pt-BR; en-XA is the pseudo-locale
    /// [env NFSMW_LANGUAGE; config `language`; default the installer's language, else the system's].
    #[arg(long, value_name = "tag")]
    pub language: Option<crate::settings::LanguageTag>,
    /// The second world to play instead of the city, by id, for a world a mod provides (`rockport` is the city)
    /// [env NFSMW_WORLD; config `world`; default the city].
    #[arg(long, value_name = "id")]
    pub world: Option<crate::settings::world_id::WorldId>,
    /// Legacy radio switch: ea_trax is --ui-style original, custom is --ui-style custom. --ui-style wins if both are set [env NFSMW_RADIO_HUD].
    #[arg(long, value_name = "ea_trax|custom")]
    pub radio_hud: Option<crate::settings::RadioHudStyle>,
    /// Enable tire smoke [env NFSMW_TIRE_SMOKE; config `tire_smoke`; default on].
    #[arg(long, conflicts_with = "no_tire_smoke")]
    pub tire_smoke: bool,
    /// Disable tire smoke.
    #[arg(long)]
    pub no_tire_smoke: bool,
    /// A named set of graphics settings: custom (none; the default), low, medium, high or ultra (high plus taa and
    /// ray tracing, which only the bevy renderer has). Any setting given on its own, in any layer, wins over
    /// the preset [env NFSMW_GRAPHICS_PRESET; config `graphics_preset`].
    #[arg(long, value_name = "custom|low|medium|high|ultra")]
    pub graphics_preset: Option<crate::settings::GraphicsPreset>,
    /// Car materials: enhanced (legacy alias: glossy), original, original-unshadowed or simple
    /// [env NFSMW_CAR_SHADING; config `car_shading`; default glossy].
    #[arg(long, value_name = "enhanced|simple|glossy|original|original-unshadowed")]
    pub car_shading: Option<crate::settings::CarShading>,
    /// Local car reflections [env NFSMW_REFLECTIONS; config `reflections`; default medium].
    #[arg(long, value_name = "off|low|medium|high")]
    pub reflections: Option<crate::settings::ReflectionQuality>,
    /// Surface filtering: trilinear, 8x or 16x [env NFSMW_TEXTURE_FILTERING; config texture_filtering; default 8x].
    #[arg(long)]
    pub texture_filtering: Option<crate::settings::TextureFiltering>,
    /// Car cube face size: auto, a percentage of the quality's face (25% to 400%) or pixels [env NFSMW_REFLECTION_CUBE_SIZE;
    /// config `reflection_cube_size`; default 100%].
    #[arg(long, value_name = "auto|25%-400%|px")]
    pub reflection_cube_size: Option<crate::settings::ReflectionSize>,
    /// Rear-view mirror picture size, the same forms as the cube face; the width keeps the bezel's shape
    /// [env NFSMW_REFLECTION_MIRROR_SIZE; config `reflection_mirror_size`; default auto].
    #[arg(long, value_name = "auto|25%-400%|px")]
    pub reflection_mirror_size: Option<crate::settings::ReflectionSize>,
    /// Round car cube faces down to a power of two [env NFSMW_REFLECTION_POW2; config `reflection_pow2`; default off].
    #[arg(long, conflicts_with = "no_reflection_pow2")]
    pub reflection_pow2: bool,
    /// Do not round car cube faces to a power of two.
    #[arg(long)]
    pub no_reflection_pow2: bool,
    /// Mirror tint: off, on (the stock tint), #rrggbb or r,g,b [env NFSMW_MIRROR_TINT; config `mirror_tint`; default off].
    #[arg(long, value_name = "off|on|#rrggbb")]
    pub mirror_tint: Option<crate::settings::MirrorTint>,
    /// Reach of the car cube and the mirror: unlimited, or original (100 m and 192 m) [env NFSMW_REFLECTION_DISTANCE;
    /// config `reflection_distance`; default unlimited].
    #[arg(long, value_name = "unlimited|original")]
    pub reflection_distance: Option<blackbox_gfx::reflection::distance::ReflectionDistance>,
    /// Scenery detail of the car cube: its own face (capture) or the main view's rule (view) [env NFSMW_REFLECTION_LOD;
    /// config `reflection_lod`; default capture].
    #[arg(long, value_name = "capture|view")]
    pub reflection_lod: Option<blackbox_gfx::reflection::lod::ReflectionLod>,
    /// What the car cube adds: original, or restored (traffic cars) [env NFSMW_REFLECTION_CONTENT; config
    /// `reflection_content`; default original].
    #[arg(long, value_name = "original|restored")]
    pub reflection_content: Option<blackbox_gfx::reflection::content::ReflectionContent>,
    /// Environment reflection of the vehicles, a percentage from 0 to 300 [env NFSMW_VEHICLE_REFLECTION_BRIGHTNESS;
    /// config `vehicle_reflection_brightness`; default 100%].
    #[arg(long, value_name = "0%-300%")]
    pub vehicle_reflection_brightness: Option<blackbox_gfx::reflection::brightness::VehicleBrightness>,
    /// Smoke presentation: standard or high [env NFSMW_SMOKE_QUALITY; config `smoke_quality`; default standard].
    #[arg(long, value_name = "standard|high")]
    pub smoke_quality: Option<crate::settings::SmokeQuality>,
    /// Draw the 3D scene at this percent of the output size per axis, 50 to 200 [env NFSMW_RENDER_SCALE;
    /// config `render_scale`; default 100]. Below 100 the scene is scaled up with --upscaler; the HUD stays sharp.
    #[arg(long, value_name = "50-200")]
    pub render_scale: Option<crate::settings::RenderScale>,
    /// How the scene is scaled up: fsr1 (FidelityFX Super Resolution 1), bilinear, or off (the render scale is
    /// ignored); fsr3 is temporal and runs on both renderers, fsr4 and dlss only on the bevy renderer; a renderer
    /// without the one asked for runs the next best [env NFSMW_UPSCALER; config `upscaler`; default fsr1].
    #[arg(long, value_name = "off|bilinear|fsr1|fsr3|fsr4|dlss")]
    pub upscaler: Option<crate::settings::UpscaleMode>,
    /// FSR 1 sharpening, 0 (off) to 100 [env NFSMW_UPSCALE_SHARPNESS; config `upscale_sharpness`; default 80].
    #[arg(long, value_name = "0-100")]
    pub upscale_sharpness: Option<crate::settings::Percent>,
    /// How far a temporal upscaler (fsr3, fsr4, dlss) renders below the output: auto, native, quality (67 %),
    /// balanced, performance or ultra_performance [env NFSMW_UPSCALE_QUALITY; config `upscale_quality`;
    /// default quality].
    #[arg(long, value_name = "auto|native|quality|balanced|performance|ultra_performance")]
    pub upscale_quality: Option<crate::settings::UpscaleQuality>,
    /// Ray-traced lighting: off, low, medium or high; only the bevy renderer on a GPU with ray queries has it,
    /// and switching it on or off needs a restart [env NFSMW_RAY_TRACING; config `ray_tracing`; default off].
    #[arg(long, value_name = "off|low|medium|high")]
    pub ray_tracing: Option<crate::settings::RayTracingLevel>,
    /// Turn the radio off [env NFSMW_RADIO=off; config `radio = false`; default on].
    #[arg(long, conflicts_with = "radio")]
    pub no_radio: bool,
    /// Turn the radio on (it is on unless the config file or environment turn it off).
    #[arg(long)]
    pub radio: bool,
    /// Active vehicle budget, including player, police and trailers; 0 turns new traffic off
    /// [env NFSMW_TRAFFIC; config `traffic`; default 20].
    #[arg(long, value_name = "CARS")]
    pub traffic: Option<u32>,
    /// Turn traffic off (the same as `--traffic 0`).
    #[arg(long, conflicts_with = "traffic")]
    pub no_traffic: bool,
    /// Share of the traffic that are patrol cops, 0 to 100 percent
    /// [env NFSMW_COP_SHARE; config `cop_share`; default 5, one in twenty].
    #[arg(long, value_name = "0-100")]
    pub cop_share: Option<crate::settings::Percent>,
    /// Make the traffic stop at red lights at junctions (a rewrite extension: the original has no lights)
    /// [env NFSMW_TRAFFIC_LIGHTS; config `traffic_lights`; default on].
    #[arg(long, conflicts_with = "no_traffic_lights")]
    pub traffic_lights: bool,
    /// Traffic ignores junctions, as in the original game.
    #[arg(long)]
    pub no_traffic_lights: bool,
    /// Draw a health bar above each cop car that chases you (a rewrite extension: the original shows none)
    /// [env NFSMW_COP_HEALTH_BARS; config `cop_health_bars`; default off].
    #[arg(long, conflicts_with = "no_cop_health_bars")]
    pub cop_health_bars: bool,
    /// Draw no cop health bars.
    #[arg(long)]
    pub no_cop_health_bars: bool,
    /// Draw the high-speed radial blur [env NFSMW_MOTION_BLUR; config `motion_blur`; default on].
    #[arg(long, conflicts_with = "no_motion_blur")]
    pub motion_blur: bool,
    /// Draw no high-speed radial blur.
    #[arg(long)]
    pub no_motion_blur: bool,
    /// The high-speed radial blur's strength, 0 to 2; 1 is the curve as it is
    /// [env NFSMW_MOTION_BLUR_STRENGTH; config `motion_blur_strength`; default 1].
    #[arg(long, value_name = "0-2")]
    pub motion_blur_strength: Option<f32>,
    /// How far the colour grading of a mod's variant goes from the frame's own colour, 0 to 1
    /// [env NFSMW_GRADING_STRENGTH; config `grading_strength`; default 1].
    #[arg(long, value_name = "0-1")]
    pub grading_strength: Option<f32>,
    #[command(flatten)]
    pub hd_content: crate::settings::hd_content::HdContentArgs,
    #[command(flatten)]
    pub custom_fonts: crate::settings::custom_fonts::CustomFontsArgs,
    #[command(flatten)]
    pub fe_shadows: crate::settings::fe_shadows::FeShadowsArgs,
    #[command(flatten)]
    pub net: crate::settings::net::NetArgs,
    /// Enable skid marks [env NFSMW_SKID_MARKS; config `skid_marks`; default on].
    #[arg(long, conflicts_with = "no_skid_marks")]
    pub skid_marks: bool,
    /// Disable skid marks.
    #[arg(long)]
    pub no_skid_marks: bool,
    /// Enable optional impact/scrape sparks [env NFSMW_COLLISION_SPARKS; default off].
    #[arg(long, conflicts_with = "no_collision_sparks")]
    pub collision_sparks: bool,
    /// Select original PC particles or the experimental restored streaks.
    #[arg(long)]
    pub spark_style: Option<crate::settings::SparkStyle>,
    /// Disable impact/scrape sparks.
    #[arg(long)]
    pub no_collision_sparks: bool,
    /// Enable optional high-speed wind trails [env NFSMW_SPEED_TRAILS; default off].
    #[arg(long, conflicts_with = "no_speed_trails")]
    pub speed_trails: bool,
    /// Disable high-speed wind trails.
    #[arg(long)]
    pub no_speed_trails: bool,
    /// Enable the tail-pipe flames [env NFSMW_EXHAUST_FLAMES; config `exhaust_flames`; default on].
    #[arg(long, conflicts_with = "no_exhaust_flames")]
    pub exhaust_flames: bool,
    /// Disable the tail-pipe flames (nothing of them is loaded).
    #[arg(long)]
    pub no_exhaust_flames: bool,
    /// Show the rear-view mirror in the HUD [env NFSMW_REAR_VIEW_MIRROR; config `rear_view_mirror`; default on].
    #[arg(long, conflicts_with = "no_rear_view_mirror")]
    pub rear_view_mirror: bool,
    /// Hide the rear-view mirror in the HUD.
    #[arg(long)]
    pub no_rear_view_mirror: bool,
    /// Show the game in Discord Rich Presence [env NFSMW_DISCORD; config `discord`; default on].
    #[arg(long, conflicts_with = "no_discord")]
    pub discord: bool,
    /// Hide the game from Discord Rich Presence.
    #[arg(long)]
    pub no_discord: bool,
    /// Show the elapsed play time in Discord Rich Presence [env NFSMW_DISCORD_TIME_PLAYED; config `discord_time_played`; default on].
    #[arg(long, conflicts_with = "no_discord_time_played")]
    pub discord_time_played: bool,
    /// Hide the elapsed play time from Discord Rich Presence.
    #[arg(long)]
    pub no_discord_time_played: bool,
    /// Let Discord friends join your LAN game from your presence [env NFSMW_DISCORD_JOIN; config `discord_join`; default on].
    #[arg(long, conflicts_with = "no_discord_join")]
    pub discord_join: bool,
    /// Do not let Discord friends join your LAN game from your presence.
    #[arg(long)]
    pub no_discord_join: bool,
    /// Tone-mapping curve: off or aces [env NFSMW_POST_TONEMAP; config `post_tonemap`; default off].
    #[arg(long, value_name = "off|aces")]
    pub post_tonemap: Option<crate::settings::PostTonemap>,
    /// Bloom around bright areas: off, low, medium or high [env NFSMW_POST_BLOOM; config `post_bloom`; default off].
    #[arg(long, value_name = "off|low|medium|high")]
    pub post_bloom: Option<crate::settings::PostBloom>,
    /// Anti-aliasing of the 3D scene: off, fxaa, spatial smaa or Bevy-only taa
    /// [env NFSMW_POST_AA; config `post_aa`; default off].
    #[arg(long, value_name = "off|fxaa|smaa|taa")]
    pub post_aa: Option<crate::settings::PostAa>,
    /// Midday visual treatment: off, low (subtle), high (stock), or reference (supplied mod override).
    #[arg(long, value_name = "off|low|high|reference")]
    pub visual_treatment: Option<crate::settings::VisualTreatment>,
    /// Dynamic soft sun shadows (Bevy) [env NFSMW_SHADOWS; config `shadows`; default off].
    #[arg(long, value_name = "off|low|medium|high")]
    pub shadows: Option<crate::settings::ShadowQuality>,
    /// Experimental ambient occlusion [env NFSMW_AO; config `ao`; default off].
    #[arg(long, value_name = "off|low|medium|high")]
    pub ao: Option<crate::settings::AoQuality>,
    /// AO strength 0 to 100 [env NFSMW_AO_INTENSITY; default 75].
    #[arg(long, value_name = "0-100")]
    pub ao_intensity: Option<crate::settings::Percent>,
    /// Compare AO in the same frame: left off, right on [env NFSMW_AO_COMPARE].
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    pub ao_compare: Option<bool>,
    /// Use the depth-prefiltered AO horizon sampler [env NFSMW_AO_DEPTH_MIPS].
    #[arg(long, value_parser = crate::settings::parse_bool)]
    pub ao_depth_mips: Option<bool>,
    /// AO estimator [env NFSMW_AO_METHOD; default gtao].
    #[arg(long, value_name = "gtao|ssao|xegtao")]
    pub ao_method: Option<crate::settings::AoMethod>,
    /// Who changes gear: automatic or manual (Q/E, the bumpers or the wheel paddles shift)
    /// [env NFSMW_TRANSMISSION; config `transmission`; default automatic].
    #[arg(long, value_name = "automatic|manual")]
    pub transmission: Option<crate::settings::Transmission>,
    /// Real-time physics cadence; rendering FPS is independent. 144 changes numerical handling
    /// [env NFSMW_PHYSICS_RATE_HZ; config `physics_rate_hz`; default 60].
    #[arg(long, value_name = "60|144", value_parser = crate::settings::parse_physics_rate)]
    pub physics_rate_hz: Option<u32>,
    /// Which camera gameplay uses: chase, dynamic-fov, bumper, hood, close, far, superfar, drift or pursuit [env NFSMW_CAMERA_MODE; config `camera_mode`; default chase].
    #[arg(long, value_name = "chase|dynamic-fov|bumper|hood|close|far|superfar|drift|pursuit")]
    pub camera_mode: Option<crate::settings::CameraMode>,
    /// Numbers for a HUD that has no car behind it: "speed_kmh,rpm,max_rpm,gear[,nos_percent[,boost_psi[,speedbreaker_percent[,speedbreaker_active[,pursuit_seconds[,evade_percent]]]]]]" (for
    /// reference screenshots; a nitrous or boost value shows that gauge).
    #[arg(long, hide = true, value_name = "SPEED,RPM,MAX_RPM,GEAR[,NOS[,PSI[,BREAKER[,ACTIVE[,PURSUIT[,EVADE[,BUSTED[,RADAR]]]]]]]]", value_parser = parse_hud_demo, allow_hyphen_values = true)]
    pub hud_demo: Option<crate::hud::HudState>,
    /// A named, deterministic HUD state for checking the HUD by eye and the screenshot matrix (`tools/hud_shots.ps1`):
    /// a development fixture drawn over the scene with a `SCENARIO` label, never game data
    /// (`docs/specs/custom-hud-scenarios.md`).
    #[arg(long, hide = true, value_name = "NAME", value_parser = parse_hud_scenario, conflicts_with = "hud_demo")]
    pub hud_scenario: Option<crate::devtools::hud_scenarios::HudScenario>,
    /// Load no mods, native ones included: the same as `mods = off` (ADR 0005 section 6) [env NFSMW_MODS].
    #[arg(long)]
    pub no_mods: bool,
    /// A developer run: mods load from copies in the cache, so a rebuilt library can replace the original.
    #[arg(long)]
    pub mod_dev: bool,
    /// A second folder of mods to load, besides the mods folder in the config directory.
    #[arg(long, value_name = "PATH")]
    pub mods_dir: Option<PathBuf>,
    /// Seed the random choices single player takes from the clock (opponent names, grid slot, radio lines, police
    /// speech, playlist, quick picks), for repeatable runs; it sets no multiplayer session rule [env NFSMW_SEED].
    #[arg(long, value_name = "U64")]
    pub seed: Option<u64>,
    /// Force every mechanics-changing setting to the original game's behaviour for this run, whatever the config
    /// file, the mod-style ini files and the saved options say; nothing is saved [env NFSMW_STOCK_RULES=1].
    #[arg(long)]
    pub stock_rules: bool,
}
