//! The scalars and bands of docs/specs/ac-handling/targets.md: section 2.1 for `ref_sport` and section 3 for the compounds.
//! Every Assetto Corsa reference there is `unmeasured`, so no comparison to the reference game is made: the rows are skipped
//! with a printed reason, never guessed. What is enforced is the `design` band and the `planar` regression value.

/// One scalar of section 2.1.
#[derive(Debug, Clone, Copy)]
pub struct Target {
    /// The scalar's name.
    pub name: &'static str,
    /// The unit, for the report.
    pub unit: &'static str,
    /// The reference game's value: `None` is `unmeasured`.
    pub ac: Option<f64>,
    /// The design value.
    pub design: f64,
    /// The band the host run must fall in.
    pub band: (f64, f64),
    /// What the planar car gives.
    pub planar: f64,
}

/// The ten rows of section 2.1.
pub const REF_SPORT: [Target; 10] = [
    Target { name: "S1 skidpad_g", unit: "g", ac: None, design: 1.00, band: (0.90, 1.10), planar: 0.967 },
    Target { name: "S2 brake_m", unit: "m", ac: None, design: 36.0, band: (32.0, 40.0), planar: 34.85 },
    Target { name: "S2 brake_peak_g", unit: "g", ac: None, design: 1.15, band: (1.05, 1.30), planar: 1.201 },
    Target { name: "S3 understeer_deg_per_g", unit: "deg/g", ac: None, design: 1.5, band: (0.8, 2.5), planar: 1.119 },
    Target { name: "S4 yaw_overshoot_pct", unit: "%", ac: None, design: 10.0, band: (3.0, 20.0), planar: 12.84 },
    Target { name: "S4 yaw_settle_s", unit: "s", ac: None, design: 0.7, band: (0.4, 1.0), planar: 0.65 },
    Target { name: "S5 breakaway_rung", unit: "throttle", ac: None, design: 0.7, band: (0.5, 0.9), planar: 0.7 },
    Target { name: "S5 slide_recover_s", unit: "s", ac: None, design: 0.8, band: (0.3, 1.5), planar: 0.83 },
    Target { name: "S6 launch_d3_m", unit: "m", ac: None, design: 24.0, band: (21.0, 28.0), planar: 25.22 },
    Target { name: "S6 launch_t100_s", unit: "s", ac: None, design: 5.0, band: (4.4, 5.8), planar: 4.97 },
];

/// One row of section 3: a compound at its reference grip level.
#[derive(Debug, Clone, Copy)]
pub struct ClassRow {
    /// The compound's name.
    pub compound: &'static str,
    /// The grip level `L` of the row.
    pub level: f64,
    /// `skidpad_g`.
    pub skidpad_g: f64,
    /// `brake_m`.
    pub brake_m: f64,
    /// `brake_peak_g`.
    pub brake_peak_g: f64,
    /// `understeer_deg_per_g`.
    pub understeer: f64,
    /// `yaw_overshoot_pct`.
    pub overshoot_pct: f64,
    /// `yaw_settle_s`.
    pub settle_s: f64,
    /// `launch_d3_m`.
    pub launch_d3_m: f64,
}

/// The five rows of section 3.
pub const CLASSES: [ClassRow; 5] = [
    ClassRow {
        compound: "economy",
        level: 0.60,
        skidpad_g: 0.832,
        brake_m: 46.27,
        brake_peak_g: 0.938,
        understeer: 1.45,
        overshoot_pct: 17.8,
        settle_s: 0.90,
        launch_d3_m: 19.57,
    },
    ClassRow {
        compound: "road",
        level: 0.80,
        skidpad_g: 0.903,
        brake_m: 41.60,
        brake_peak_g: 1.087,
        understeer: 1.30,
        overshoot_pct: 14.9,
        settle_s: 0.75,
        launch_d3_m: 22.23,
    },
    ClassRow {
        compound: "sport",
        level: 1.00,
        skidpad_g: 0.967,
        brake_m: 34.85,
        brake_peak_g: 1.201,
        understeer: 1.12,
        overshoot_pct: 12.8,
        settle_s: 0.65,
        launch_d3_m: 25.14,
    },
    ClassRow {
        compound: "semislick",
        level: 1.25,
        skidpad_g: 1.061,
        brake_m: 34.44,
        brake_peak_g: 1.305,
        understeer: 0.97,
        overshoot_pct: 10.2,
        settle_s: 0.52,
        launch_d3_m: 29.34,
    },
    ClassRow {
        compound: "slick",
        level: 1.55,
        skidpad_g: 1.238,
        brake_m: 33.81,
        brake_peak_g: 1.403,
        understeer: 0.78,
        overshoot_pct: 7.0,
        settle_s: 0.37,
        launch_d3_m: 35.58,
    },
];

/// The allowed difference from a planar value (targets.md section 5): 3 %, or 0.05 on a time and 0.5 on a distance if larger.
pub fn planar_tolerance(value: f64, unit: &str) -> f64 {
    let floor = match unit {
        "s" => 0.05,
        "m" => 0.5,
        _ => 0.0,
    };
    (0.03 * value.abs()).max(floor)
}
