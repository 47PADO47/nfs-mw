//! Unit conversion factors. Parameter structs keep the units of the data they are filled from
//! (inches, ft*lb, mph, rpm); the simulation works in metres, newtons, seconds and radians.

/// Metres per inch.
pub const INCH_TO_M: f32 = 0.0254;
/// Newton metres per foot-pound.
pub const FT_LB_TO_NM: f32 = 1.3558;
/// Metres per second per mile per hour.
pub const MPH_TO_MS: f32 = 0.44703;
/// Miles per hour per metre per second.
pub const MS_TO_MPH: f32 = 2.2369;
/// Newtons per metre per pound per inch (spring rate), and N*s/m per lb*s/in (damper rate).
pub const LB_IN_TO_N_M: f32 = 175.1268;
/// Divisor from rpm to rad/s.
pub const RPM_PER_RAD_S: f32 = 9.549_296;
/// Horse power constant: hp = ft*lb * rpm / 5252.
pub const HP_RPM_CONSTANT: f32 = 5252.0;

/// Engine speed in rad/s from rpm.
pub fn rpm_to_rad(rpm: f32) -> f32 {
    rpm / RPM_PER_RAD_S
}

/// Engine speed in rpm from rad/s.
pub fn rad_to_rpm(rad: f32) -> f32 {
    rad * RPM_PER_RAD_S
}

/// Miles per hour to metres per second.
pub fn mph_to_ms(mph: f32) -> f32 {
    mph * MPH_TO_MS
}

/// Metres per second to miles per hour.
pub fn ms_to_mph(ms: f32) -> f32 {
    ms * MS_TO_MPH
}

/// Inches to metres.
pub fn inch_to_m(inch: f32) -> f32 {
    inch * INCH_TO_M
}
