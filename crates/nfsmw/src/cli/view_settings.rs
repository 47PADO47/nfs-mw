//! How the options the viewers and the game share become run options and settings.

use super::view_args::ViewArgs;
use crate::settings::Partial;

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
            offscreen: self.offscreen,
            exec: self.exec.clone(),
            open_console: self.open_console,
            open_garage: self.open_garage,
            garage_tab: self.garage_tab,
            hud: (self.hud || hud_default || self.hud_demo.is_some() || self.hud_scenario.is_some())
                .then(|| dir.clone()),
            hud_demo: self.hud_demo.clone(),
            hud_scenario: self.hud_scenario,
            // A screenshot run has no sound unless the radio is asked for: the radio's songs drive the chyron.
            audio: (!self.no_sound && (self.screenshot.is_none() || self.radio)).then(|| dir.clone()),
            frontend: None,
            mods: None,
            #[cfg(feature = "multiplayer")]
            net: None,
        }
    }

    /// The run's seed and stock-rules switch: the command line over the environment (`parity/mod.rs`), with the
    /// vehicle budget the command line sets, which a stock run keeps.
    pub fn parity_options(&self) -> crate::parity::ParityOptions {
        let options =
            crate::parity::ParityOptions::resolve(self.seed, self.stock_rules, |name| std::env::var(name).ok());
        crate::parity::ParityOptions { traffic: self.traffic_budget(), ..options }
    }

    /// The vehicle budget `--traffic N` or `--no-traffic` (0) asks for.
    fn traffic_budget(&self) -> Option<u32> {
        match self.no_traffic {
            true => Some(0),
            false => self.traffic,
        }
    }

    /// The command-line layer of the settings: the top layer, above the environment and the config file.
    pub fn settings_layer(&self) -> Partial {
        Partial {
            appearance: None,
            backend: self.backend,
            renderer: self.renderer,
            reflex: self.reflex,
            frame_generation: self.frame_generation,
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
            ui_style: self.ui_style.or(self.radio_hud.map(crate::settings::UiStyle::from)),
            language: self.language,
            world: self.world,
            tire_smoke: switch(self.tire_smoke, self.no_tire_smoke),
            graphics_preset: self.graphics_preset,
            car_shading: self.car_shading,
            reflections: self.reflections,
            texture_filtering: self.texture_filtering,
            reflection_cube_size: self.reflection_cube_size,
            reflection_mirror_size: self.reflection_mirror_size,
            reflection_pow2: switch(self.reflection_pow2, self.no_reflection_pow2),
            mirror_tint: self.mirror_tint,
            reflection_distance: self.reflection_distance,
            reflection_lod: self.reflection_lod,
            reflection_content: self.reflection_content,
            vehicle_reflection_brightness: self.vehicle_reflection_brightness,
            smoke_quality: self.smoke_quality,
            radio: switch(self.radio, self.no_radio),
            traffic: self.traffic_budget(),
            cop_share: self.cop_share,
            traffic_lights: switch(self.traffic_lights, self.no_traffic_lights),
            cop_health_bars: switch(self.cop_health_bars, self.no_cop_health_bars),
            motion_blur: switch(self.motion_blur, self.no_motion_blur),
            motion_blur_strength: self.motion_blur_strength,
            grading_strength: self.grading_strength,
            hd_content: self.hd_content.layer(),
            custom_fonts: self.custom_fonts.layer(),
            fe_shadows: self.fe_shadows.layer(),
            net: self.net.layer(),
            skid_marks: switch(self.skid_marks, self.no_skid_marks),
            collision_sparks: switch(self.collision_sparks, self.no_collision_sparks),
            spark_style: self.spark_style,
            speed_trails: switch(self.speed_trails, self.no_speed_trails),
            exhaust_flames: switch(self.exhaust_flames, self.no_exhaust_flames),
            rear_view_mirror: switch(self.rear_view_mirror, self.no_rear_view_mirror),
            discord: switch(self.discord, self.no_discord),
            discord_time_played: switch(self.discord_time_played, self.no_discord_time_played),
            discord_join: switch(self.discord_join, self.no_discord_join),
            mods: self.no_mods.then_some(crate::settings::mods::ModsMode::Off),
            transmission: self.transmission,
            physics_rate_hz: self.physics_rate_hz,
            camera_mode: self.camera_mode,
            post_tonemap: self.post_tonemap,
            post_bloom: self.post_bloom,
            post_aa: self.post_aa,
            visual_treatment: self.visual_treatment,
            shadows: self.shadows,
            ao: self.ao,
            ao_intensity: self.ao_intensity,
            ao_compare: self.ao_compare,
            ao_depth_mips: self.ao_depth_mips,
            ao_method: self.ao_method,
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
