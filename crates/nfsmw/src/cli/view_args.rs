//! The options the viewers and the game share, and how they become settings.

use std::path::PathBuf;

use blackbox_gfx::GraphicsApi;
use clap::Args;

use super::hud_demo::parse_hud_demo;
use super::parse::parse_screenshot_size;
use crate::app::pacing::MaxFps;
use crate::devtools::{ShowMetrics, ShowReadout};
use crate::settings::{Monitor, Partial, Resolution, WindowMode};

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
    #[arg(long, value_name = "FILE.png")]
    pub screenshot: Option<PathBuf>,
    /// Physical render target for a screenshot (default 1280x720); independent of window preferences.
    #[arg(long, hide = true, requires = "screenshot", value_name = "WIDTHxHEIGHT", value_parser = parse_screenshot_size)]
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
    #[arg(long, value_name = "SECONDS", conflicts_with = "screenshot")]
    pub bench_seconds: Option<f32>,
    /// Run a console command once the window is up (repeatable), e.g. --exec "fps 60" --exec "car PORSCHE911".
    #[arg(long, value_name = "COMMAND")]
    pub exec: Vec<String>,
    /// Start with the developer console open.
    #[arg(long, hide = true)]
    pub open_console: bool,
    /// Play no sound (no output device is opened).
    #[arg(long)]
    pub no_sound: bool,
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
    /// How the radio announces songs on the HUD: ea_trax (the original chyron) or custom [env NFSMW_RADIO_HUD; default ea_trax].
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
    /// Car shading: glossy (three lights, sun highlight, reflections) or simple (one light; the cheap path)
    /// [env NFSMW_CAR_SHADING; config `car_shading`; default glossy].
    #[arg(long, value_name = "simple|glossy")]
    pub car_shading: Option<crate::settings::CarShading>,
    /// Local car reflections [env NFSMW_REFLECTIONS; config `reflections`; default medium].
    #[arg(long, value_name = "off|low|medium|high")]
    pub reflections: Option<crate::settings::ReflectionQuality>,
    /// Smoke presentation: standard or high [env NFSMW_SMOKE_QUALITY; config `smoke_quality`; default standard].
    #[arg(long, value_name = "standard|high")]
    pub smoke_quality: Option<crate::settings::SmokeQuality>,
    /// Draw the 3D scene at this percent of the output size per axis, 50 to 200 [env NFSMW_RENDER_SCALE;
    /// config `render_scale`; default 100]. Below 100 the scene is scaled up with --upscaler; the HUD stays sharp.
    #[arg(long, value_name = "50-200")]
    pub render_scale: Option<crate::settings::RenderScale>,
    /// How the scene is scaled up: fsr1 (FidelityFX Super Resolution 1), bilinear, or off (the render scale is
    /// ignored); fsr3, fsr4 and dlss are temporal upscalers of the bevy renderer and run as fsr1 without it
    /// [env NFSMW_UPSCALER; config `upscaler`; default fsr1].
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
    /// Computer-driven cars kept on the road around you; 0 turns traffic off
    /// [env NFSMW_TRAFFIC; config `traffic`; default 10].
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
    /// Tone-mapping curve: off or aces [env NFSMW_POST_TONEMAP; config `post_tonemap`; default off].
    #[arg(long, value_name = "off|aces")]
    pub post_tonemap: Option<crate::settings::PostTonemap>,
    /// Bloom around bright areas: off, low, medium or high [env NFSMW_POST_BLOOM; config `post_bloom`; default off].
    #[arg(long, value_name = "off|low|medium|high")]
    pub post_bloom: Option<crate::settings::PostBloom>,
    /// Anti-aliasing of the 3D scene: off, fxaa, smaa or taa; smaa and taa need the bevy renderer and run as fxaa
    /// without it [env NFSMW_POST_AA; config `post_aa`; default off].
    #[arg(long, value_name = "off|fxaa|smaa|taa")]
    pub post_aa: Option<crate::settings::PostAa>,
    /// Experimental midday visual treatment: off, low (half strength), or high (original profile).
    #[arg(long, value_name = "off|low|high")]
    pub visual_treatment: Option<crate::settings::VisualTreatment>,
    /// Experimental ambient occlusion [env NFSMW_AO; config `ao`; default off].
    #[arg(long, value_name = "off|low|medium|high")]
    pub ao: Option<crate::settings::AoQuality>,
    /// AO strength 0 to 100 [env NFSMW_AO_INTENSITY; default 75].
    #[arg(long, value_name = "0-100")]
    pub ao_intensity: Option<crate::settings::Percent>,
    /// Compare AO in the same frame: left off, right on [env NFSMW_AO_COMPARE].
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    pub ao_compare: Option<bool>,
    /// Who changes gear: automatic or manual (Q/E, the bumpers or the wheel paddles shift)
    /// [env NFSMW_TRANSMISSION; config `transmission`; default automatic].
    #[arg(long, value_name = "automatic|manual")]
    pub transmission: Option<crate::settings::Transmission>,
    /// Which camera gameplay uses: chase, dynamic-fov, bumper, hood, close, far, superfar, drift or pursuit [env NFSMW_CAMERA_MODE; config `camera_mode`; default chase].
    #[arg(long, value_name = "chase|dynamic-fov|bumper|hood|close|far|superfar|drift|pursuit")]
    pub camera_mode: Option<crate::settings::CameraMode>,
    /// Numbers for a HUD that has no car behind it: "speed_kmh,rpm,max_rpm,gear[,nos_percent[,boost_psi]]" (for
    /// reference screenshots; a nitrous or boost value shows that gauge).
    #[arg(long, hide = true, value_name = "SPEED,RPM,MAX_RPM,GEAR[,NOS[,PSI]]", value_parser = parse_hud_demo, allow_hyphen_values = true)]
    pub hud_demo: Option<crate::hud::HudState>,
}

impl ViewArgs {
    /// `hud_default` turns the HUD on without `--hud` (driving).
    pub fn run_options(&self, dir: &game_install::GameDir, hud_default: bool) -> crate::app::RunOptions {
        crate::app::RunOptions {
            screenshot: self.screenshot.clone(),
            screenshot_size: self.screenshot_size,
            screenshot_delay: self.screenshot_delay.unwrap_or(0.0),
            screenshot_count: self.screenshot_count.unwrap_or(1),
            screenshot_interval: self.screenshot_interval.unwrap_or(1.0),
            bench_seconds: self.bench_seconds,
            exec: self.exec.clone(),
            open_console: self.open_console,
            hud: (self.hud || hud_default || self.hud_demo.is_some()).then(|| dir.clone()),
            hud_demo: self.hud_demo.clone(),
            // A screenshot run has no sound unless the radio is asked for: the radio's songs drive the chyron.
            audio: (!self.no_sound && (self.screenshot.is_none() || self.radio)).then(|| dir.clone()),
            frontend: None,
        }
    }

    /// The command-line layer of the settings: the top layer, above the environment and the config file.
    pub fn settings_layer(&self) -> Partial {
        Partial {
            backend: self.backend,
            renderer: self.renderer,
            vsync: self.no_vsync.then_some(false),
            max_fps: self.max_fps,
            show_metrics: self.show_metrics,
            window_mode: self.window_mode,
            monitor: self.monitor,
            resolution: self.resolution,
            show_readout: self.show_readout,
            master_volume: if self.muted { Some(crate::settings::Percent(0)) } else { self.volume },
            hud: if self.no_hud { Some(false) } else { self.hud.then_some(true) },
            hud_layout: self.hud_layout,
            radio_hud: self.radio_hud,
            tire_smoke: switch(self.tire_smoke, self.no_tire_smoke),
            graphics_preset: self.graphics_preset,
            car_shading: self.car_shading,
            reflections: self.reflections,
            smoke_quality: self.smoke_quality,
            radio: switch(self.radio, self.no_radio),
            traffic: if self.no_traffic { Some(0) } else { self.traffic },
            cop_share: self.cop_share,
            traffic_lights: switch(self.traffic_lights, self.no_traffic_lights),
            skid_marks: switch(self.skid_marks, self.no_skid_marks),
            collision_sparks: switch(self.collision_sparks, self.no_collision_sparks),
            spark_style: self.spark_style,
            speed_trails: switch(self.speed_trails, self.no_speed_trails),
            exhaust_flames: switch(self.exhaust_flames, self.no_exhaust_flames),
            rear_view_mirror: switch(self.rear_view_mirror, self.no_rear_view_mirror),
            discord: switch(self.discord, self.no_discord),
            discord_time_played: switch(self.discord_time_played, self.no_discord_time_played),
            transmission: self.transmission,
            camera_mode: self.camera_mode,
            post_tonemap: self.post_tonemap,
            post_bloom: self.post_bloom,
            post_aa: self.post_aa,
            visual_treatment: self.visual_treatment,
            ao: self.ao,
            ao_intensity: self.ao_intensity,
            ao_compare: self.ao_compare,
            render_scale: self.render_scale,
            upscaler: self.upscaler,
            upscale_sharpness: self.upscale_sharpness,
            upscale_quality: self.upscale_quality,
            ray_tracing: self.ray_tracing,
            ..Partial::default()
        }
    }
}

fn switch(on: bool, off: bool) -> Option<bool> {
    if off {
        return Some(false);
    }
    on.then_some(true)
}
