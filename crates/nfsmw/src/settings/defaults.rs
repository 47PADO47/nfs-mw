//! Defaults that more than one place reads.

use super::Percent;

/// Active vehicle budget of the installed PC reference, including the player and police.
pub const DEFAULT_TRAFFIC: u32 = 20;
/// The default share of those cars that are patrol cops: 5 percent, one in twenty.
pub const DEFAULT_COP_SHARE: Percent = Percent(5);

/// Whether traffic stops at red lights by default. The original game has no traffic lights: this is a rewrite
/// extension, on because it was asked for.
pub const DEFAULT_TRAFFIC_LIGHTS: bool = true;
/// Whether a health bar is drawn above each cop car that chases the player. A rewrite extension: the original shows no
/// per-car bar, so it is off by default.
pub const DEFAULT_COP_HEALTH_BARS: bool = false;
/// Rain on every track, as the Extra Options `[Weather] AlwaysRain` switch (off by default).
pub const DEFAULT_ALWAYS_RAIN: bool = false;
/// The rain amount: 100 percent is the original amount.
pub const DEFAULT_RAIN_AMOUNT: Percent = Percent(100);
/// The game speed multiplier: 1 is the original speed.
pub const DEFAULT_GAME_SPEED: f32 = 1.0;
/// The world's animation speed: 1 is the original rate (Extra Options `WorldAnimationSpeed`).
pub const DEFAULT_ANIMATION_SPEED: f32 = 1.0;
/// The car model scale: 1 is the original size (Extra Options `CarScale`, visual only).
pub const DEFAULT_CAR_SCALE: f32 = 1.0;
/// The rev limiter is on by default, as in the original.
pub const DEFAULT_NO_REV_LIMITER: bool = false;
/// The HUD stays up while looking back (Extra Options `ShowHUDOnLookBack`); off hides it, as the original does.
pub const DEFAULT_SHOW_HUD_ON_LOOK_BACK: bool = false;
