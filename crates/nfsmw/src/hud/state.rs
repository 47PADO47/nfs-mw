use bevy_ecs::prelude::Resource;

/// What the in-game HUD shows, as plain numbers. The scene fills it every frame; nothing here knows how
/// the HUD is built or drawn.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct HudState {
    /// Draw the HUD at all.
    pub visible: bool,
    /// Speed in metres per second.
    pub speed: f32,
    /// Show miles per hour instead of kilometres per hour.
    pub use_mph: bool,
    /// Engine speed in rpm.
    pub rpm: f32,
    /// Engine speed at the end of the tachometer scale (the red line).
    pub max_rpm: f32,
    /// -1 reverse, 0 neutral, 1.. forward gears.
    pub gear: i32,
    /// The shift light is on.
    pub shift_light: bool,
    /// Tachometer skin (the `HUDS_Custom_NN` pack, 0..=10).
    pub skin: u8,
}

impl Default for HudState {
    fn default() -> Self {
        Self {
            visible: true,
            speed: 0.0,
            use_mph: false,
            rpm: 0.0,
            max_rpm: 8000.0,
            gear: 0,
            shift_light: false,
            skin: 0,
        }
    }
}

impl HudState {
    /// The speed in the unit shown, rounded.
    pub fn display_speed(&self) -> u32 {
        let v = if self.use_mph { self.speed * 2.236_936 } else { self.speed * 3.6 };
        v.max(0.0).round().min(999.0) as u32
    }

    /// 0..1 along the tachometer scale.
    pub fn rpm_fraction(&self) -> f32 {
        if self.max_rpm > 0.0 { (self.rpm / self.max_rpm).clamp(0.0, 1.0) } else { 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_converts_and_rounds() {
        let s = HudState { speed: 27.78, ..Default::default() };
        assert_eq!(s.display_speed(), 100);
        let m = HudState { speed: 26.82, use_mph: true, ..Default::default() };
        assert_eq!(m.display_speed(), 60);
        assert_eq!(HudState { speed: -3.0, ..Default::default() }.display_speed(), 0);
        assert_eq!(HudState { speed: 1.0e6, ..Default::default() }.display_speed(), 999);
    }

    #[test]
    fn rpm_fraction_clamps() {
        let s = HudState { rpm: 9000.0, max_rpm: 8000.0, ..Default::default() };
        assert_eq!(s.rpm_fraction(), 1.0);
        assert_eq!(HudState { max_rpm: 0.0, ..Default::default() }.rpm_fraction(), 0.0);
    }
}
