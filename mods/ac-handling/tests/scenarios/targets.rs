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
    Target { name: "S1 skidpad_g", unit: "g", ac: None, design: 1.00, band: (0.90, 1.10), planar: 0.973 },
    Target { name: "S2 brake_m", unit: "m", ac: None, design: 41.0, band: (37.0, 45.0), planar: 40.91 },
    Target { name: "S2 brake_peak_g", unit: "g", ac: None, design: 1.00, band: (0.90, 1.10), planar: 1.036 },
    Target { name: "S3 understeer_deg_per_g", unit: "deg/g", ac: None, design: 1.5, band: (0.8, 2.5), planar: 1.134 },
    Target { name: "S4 yaw_overshoot_pct", unit: "%", ac: None, design: 10.0, band: (3.0, 20.0), planar: 11.54 },
    Target { name: "S4 yaw_settle_s", unit: "s", ac: None, design: 0.7, band: (0.4, 1.0), planar: 0.683 },
    Target { name: "S5 breakaway_rung", unit: "throttle", ac: None, design: 0.7, band: (0.5, 0.9), planar: 0.75 },
    Target { name: "S5 slide_recover_s", unit: "s", ac: None, design: 0.8, band: (0.3, 1.5), planar: 0.62 },
    Target { name: "S6 launch_d3_m", unit: "m", ac: None, design: 24.0, band: (21.0, 28.0), planar: 23.09 },
    Target { name: "S6 launch_t100_s", unit: "s", ac: None, design: 5.0, band: (4.4, 5.8), planar: 5.517 },
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
    /// S2t `brake_m` (every brake torque times 1.3).
    pub s2t_brake_m: f64,
    /// S2t `brake_peak_g`.
    pub s2t_peak_g: f64,
    /// `understeer_deg_per_g`.
    pub understeer: f64,
    /// `yaw_overshoot_pct`.
    pub overshoot_pct: f64,
    /// `yaw_settle_s`.
    pub settle_s: f64,
    /// `launch_d3_m`.
    pub launch_d3_m: f64,
}

#[allow(clippy::too_many_arguments)]
const fn row(
    compound: &'static str,
    level: f64,
    s1: f64,
    s2: (f64, f64),
    s2t: (f64, f64),
    s3: f64,
    s4: (f64, f64),
    d3: f64,
) -> ClassRow {
    ClassRow {
        compound,
        level,
        skidpad_g: s1,
        brake_m: s2.0,
        brake_peak_g: s2.1,
        s2t_brake_m: s2t.0,
        s2t_peak_g: s2t.1,
        understeer: s3,
        overshoot_pct: s4.0,
        settle_s: s4.1,
        launch_d3_m: d3,
    }
}

/// The five rows of section 3 (both tables).
pub const CLASSES: [ClassRow; 5] = [
    row("economy", 0.60, 0.843, (51.84, 0.880), (49.39, 0.895), 1.49, (16.2, 0.95), 18.04),
    row("road", 0.80, 0.911, (42.17, 1.012), (42.37, 1.019), 1.26, (13.5, 0.80), 20.51),
    row("sport", 1.00, 0.973, (40.91, 1.036), (37.69, 1.190), 1.13, (11.5, 0.68), 23.04),
    row("semislick", 1.25, 1.066, (40.55, 1.112), (31.50, 1.294), 0.98, (9.1, 0.55), 26.66),
    row("slick", 1.55, 1.236, (40.17, 1.202), (31.15, 1.406), 0.75, (6.1, 0.38), 32.47),
];

/// One row of the inertia sweep of scenarios.md section 6.
#[derive(Debug, Clone, Copy)]
pub struct SweepRow {
    /// The `inertia_scale`.
    pub scale: f64,
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
    /// `breakaway_rung`.
    pub breakaway: f64,
    /// `slide_recover_s`.
    pub recover_s: f64,
    /// `launch_d3_m`.
    pub d3_m: f64,
    /// `launch_t100_s`; `None` is "not in 6 s".
    pub t100_s: Option<f64>,
    /// `T_LEAD(20)`.
    pub lead_20: f64,
}

/// The four rows of the inertia sweep.
pub const SWEEP: [SweepRow; 4] = [
    SweepRow {
        scale: 0.3,
        skidpad_g: 0.967,
        brake_m: 34.85,
        brake_peak_g: 1.201,
        understeer: 1.118,
        overshoot_pct: 12.84,
        settle_s: 0.667,
        breakaway: 0.70,
        recover_s: 0.85,
        d3_m: 25.22,
        t100_s: Some(4.983),
        lead_20: 3.57,
    },
    SweepRow {
        scale: 0.5,
        skidpad_g: 0.970,
        brake_m: 36.61,
        brake_peak_g: 1.167,
        understeer: 1.166,
        overshoot_pct: 12.49,
        settle_s: 0.667,
        breakaway: 0.70,
        recover_s: 0.53,
        d3_m: 24.61,
        t100_s: Some(5.117),
        lead_20: 3.63,
    },
    SweepRow {
        scale: 1.0,
        skidpad_g: 0.973,
        brake_m: 40.91,
        brake_peak_g: 1.036,
        understeer: 1.134,
        overshoot_pct: 11.54,
        settle_s: 0.683,
        breakaway: 0.75,
        recover_s: 0.62,
        d3_m: 23.09,
        t100_s: Some(5.517),
        lead_20: 3.87,
    },
    SweepRow {
        scale: 2.0,
        skidpad_g: 0.976,
        brake_m: 49.29,
        brake_peak_g: 0.844,
        understeer: 1.114,
        overshoot_pct: 9.39,
        settle_s: 0.700,
        breakaway: 0.80,
        recover_s: 0.53,
        d3_m: 20.53,
        t100_s: None,
        lead_20: 4.37,
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
