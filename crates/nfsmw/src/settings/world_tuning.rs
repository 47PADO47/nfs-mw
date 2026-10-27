//! Runtime world settings.
use super::*;

/// The settings a running world reads each frame: the physics clock and the animation clock rates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldTuning {
    pub game_speed: f32,
    /// Real-time simulation cadence; rendering has its own independent limit.
    pub physics_rate_hz: u32,
    pub animation_speed: f32,
    /// The player car's model scale, 0.25 to 4 (Extra Options `CarScale`).
    pub car_scale: f32,
    pub show_hud_on_look_back: bool,
    /// Lift the engine's rev limiter (Extra Options `NoRevLimiter`).
    pub no_rev_limiter: bool,
    /// Show turn signals on cars while steering.
    pub turn_signals: bool,
    /// Draw the cop health bars (`Settings::cop_health_bars`).
    pub cop_health_bars: bool,
    pub enhanced_exhaust: bool,
    pub exhaust_style: ExhaustStyle,
    pub backfire_style: BackfireStyle,
    /// The Speedbreaker tunables (`NFSMWSpeedFixerSettings.ini`).
    pub speedbreaker: SpeedbreakerTuning,
    /// The high-speed radial blur: on or off, and its strength.
    pub motion_blur: MotionBlur,
    pub rain_droplet_scale: f32,
    /// The restored sparks and the speed trail (`[xenon_effects]`).
    pub xenon_effects: XenonEffects,
    /// The camera's contact shake, a multiplier from 0 (none) to 1 (as it is) (`[custom_hud] camera_shake`).
    pub camera_shake: f32,
    /// How far the Dynamic FOV camera widens with speed, a multiplier of the widening (`[custom_hud] fov_intensity`).
    pub fov_intensity: f32,
}

impl Default for WorldTuning {
    fn default() -> Self {
        Self {
            game_speed: DEFAULT_GAME_SPEED,
            physics_rate_hz: DEFAULT_PHYSICS_RATE_HZ,
            animation_speed: DEFAULT_ANIMATION_SPEED,
            show_hud_on_look_back: DEFAULT_SHOW_HUD_ON_LOOK_BACK,
            car_scale: DEFAULT_CAR_SCALE,
            no_rev_limiter: DEFAULT_NO_REV_LIMITER,
            turn_signals: false,
            cop_health_bars: DEFAULT_COP_HEALTH_BARS,
            enhanced_exhaust: false,
            exhaust_style: ExhaustStyle::default(),
            backfire_style: BackfireStyle::default(),
            speedbreaker: SpeedbreakerTuning::default(),
            motion_blur: MotionBlur::default(),
            rain_droplet_scale: 0.5,
            xenon_effects: XenonEffects::default(),
            camera_shake: hud::DEFAULT_CAMERA_SHAKE,
            fov_intensity: hud::DEFAULT_FOV_INTENSITY,
        }
    }
}

impl From<&Settings> for WorldTuning {
    fn from(s: &Settings) -> Self {
        Self {
            game_speed: s.game_speed,
            physics_rate_hz: s.physics_rate_hz,
            animation_speed: s.animation_speed,
            show_hud_on_look_back: s.show_hud_on_look_back,
            car_scale: s.car_scale,
            no_rev_limiter: s.no_rev_limiter,
            turn_signals: s.turn_signals,
            cop_health_bars: s.cop_health_bars,
            enhanced_exhaust: s.enhanced_exhaust,
            exhaust_style: s.exhaust_style,
            backfire_style: s.backfire_style,
            speedbreaker: s.speedbreaker,
            motion_blur: MotionBlur::from(s),
            rain_droplet_scale: s.rain_droplet_scale,
            xenon_effects: s.xenon_effects,
            camera_shake: s.hud_options.camera_shake,
            fov_intensity: s.hud_options.fov_intensity,
        }
    }
}
