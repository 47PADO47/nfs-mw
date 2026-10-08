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
    /// The engine's `MAX_RPM`: it picks the tachometer face and the end of the needle's sweep.
    pub max_rpm: f32,
    /// The engine's red line: where the red zone of the tachometer starts.
    pub red_line: f32,
    /// -1 reverse, 0 neutral, 1.. forward gears.
    pub gear: i32,
    /// A gear change is in progress (the gear digit dims).
    pub shifting: bool,
    /// The shift light is on: the gearbox wants the next gear.
    pub shift_light: bool,
    /// Tachometer skin (the `HUDS_Custom_NN` pack, 0..=10).
    pub skin: u8,
    /// The car has a nitrous system: the nitrous gauge is shown.
    pub has_nos: bool,
    /// Nitrous left, 0..1.
    pub nos: f32,
    /// The car has forced induction: the turbo gauge is shown.
    pub has_turbo: bool,
    /// Boost gauge, psi (negative in vacuum).
    pub boost_psi: f32,
}

impl Default for HudState {
    fn default() -> Self {
        Self {
            visible: true,
            speed: 0.0,
            use_mph: false,
            rpm: 0.0,
            max_rpm: 8000.0,
            red_line: 7500.0,
            gear: 0,
            shifting: false,
            shift_light: false,
            skin: 0,
            has_nos: false,
            nos: 0.0,
            has_turbo: false,
            boost_psi: 0.0,
        }
    }
}

impl HudState {
    /// The speed in the unit shown. The game shows whole units, cut off, not rounded (the digits are the integer
    /// parts of the speed), and three digits at most.
    pub fn display_speed(&self) -> u32 {
        let v = if self.use_mph { self.speed * 2.236_936 } else { self.speed * 3.6 };
        v.max(0.0).floor().min(999.0) as u32
    }

    /// The tachometer scale: the engine's `MAX_RPM` rounded up to the face the game has art for (7000 to
    /// 10000, in steps of 1000 and strictly above `MAX_RPM`).
    pub fn scale_rpm(&self) -> f32 {
        scale_rpm(self.max_rpm)
    }

    /// 0..1 along the tachometer scale.
    pub fn rpm_fraction(&self) -> f32 {
        (self.rpm / self.scale_rpm()).clamp(0.0, 1.0)
    }
}

/// The tachometer face for an engine: the first of 7000, 8000, 9000 that is above `max_rpm`, else 10000.
pub fn scale_rpm(max_rpm: f32) -> f32 {
    [7000.0, 8000.0, 9000.0].into_iter().find(|&n| max_rpm < n).unwrap_or(10000.0)
}

/// The angle (degrees) the redline mask of the tachometer is turned to: the bigger it is, the shorter the red zone.
/// The values are the game's table by `MAX_RPM` band and red line.
pub fn redline_rotation(max_rpm: f32, red_line: f32) -> f32 {
    // Per band: the red lines (rpm) from which each angle applies, highest first.
    let table: &[(f32, f32)] = match max_rpm {
        m if m < 7000.0 => &[(6500.0, 164.5), (6000.0, 149.5), (5500.0, 131.5), (0.0, 113.5)],
        m if m < 8000.0 => &[(7500.0, 165.0), (7000.0, 152.0), (6500.0, 138.0), (6000.0, 123.0), (0.0, 110.0)],
        m if m < 9000.0 => &[(8500.0, 166.0), (8000.0, 154.0), (7500.0, 140.5), (7000.0, 127.0), (0.0, 115.0)],
        _ => &[(9500.0, 167.0), (9000.0, 156.0), (8500.0, 145.0), (8000.0, 134.0), (0.0, 123.0)],
    };
    table.iter().find(|(from, _)| red_line >= *from).map_or(0.0, |(_, angle)| *angle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_converts_and_cuts_off() {
        let s = HudState { speed: 27.78, ..Default::default() };
        assert_eq!(s.display_speed(), 100);
        let m = HudState { speed: 26.9, use_mph: true, ..Default::default() };
        assert_eq!(m.display_speed(), 60);
        // 99.9 km/h reads 99, as the game's integer digits do.
        assert_eq!(HudState { speed: 27.75, ..Default::default() }.display_speed(), 99);
        assert_eq!(HudState { speed: -3.0, ..Default::default() }.display_speed(), 0);
        assert_eq!(HudState { speed: 1.0e6, ..Default::default() }.display_speed(), 999);
    }

    #[test]
    fn the_scale_is_the_next_face_above_max_rpm() {
        for (max, face) in [(6500.0, 7000.0), (7000.0, 8000.0), (7500.0, 8000.0), (8000.0, 9000.0), (8999.0, 9000.0)] {
            assert_eq!(scale_rpm(max), face, "{max}");
        }
        assert_eq!(scale_rpm(9000.0), 10000.0);
        assert_eq!(scale_rpm(12000.0), 10000.0);
    }

    #[test]
    fn rpm_fraction_clamps_and_follows_the_face() {
        let s = HudState { rpm: 9500.0, max_rpm: 8250.0, ..Default::default() };
        assert_eq!(s.rpm_fraction(), 1.0);
        let half = HudState { rpm: 4500.0, max_rpm: 8250.0, ..Default::default() };
        assert_eq!(half.rpm_fraction(), 0.5, "the face is the 9000 one");
    }

    #[test]
    fn the_redline_rotation_follows_the_table() {
        assert_eq!(redline_rotation(8250.0, 8000.0), 154.0);
        assert_eq!(redline_rotation(8250.0, 8700.0), 166.0);
        assert_eq!(redline_rotation(8250.0, 6000.0), 115.0);
        assert_eq!(redline_rotation(6800.0, 6500.0), 164.5);
        assert_eq!(redline_rotation(7500.0, 7100.0), 152.0);
        assert_eq!(redline_rotation(9500.0, 9200.0), 156.0);
        assert_eq!(redline_rotation(10000.0, 7000.0), 123.0);
    }
}
