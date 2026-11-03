//! Cascade update schedule. The nearest cascade renders every frame; farther ones render less often and
//! receivers keep sampling them with the matrix, texel size and depth span the map was rendered with.
use super::fit::Params;
use glam::{Mat4, Vec3};

/// Frames between refreshes of each cascade: 0 every frame, 1 every 2nd, 2 and 3 every 4th.
pub(super) const PERIOD: [u32; 4] = [1, 2, 4, 4];
/// Texels a stale map may trail the fitted one before it is refreshed regardless of the schedule.
/// The fit keeps a 16 texel guard band; the rest is left for snapping error and the filter footprint.
const DRIFT_LIMIT: f32 = 10.0;
/// Squared sine of the largest sun rotation a stale map tolerates.
const SUN_LIMIT: f32 = 1.0e-6;
/// Squared sine (about one degree) of the largest sun step a far cascade may trail for one frame.
const SUN_STEP_LIMIT: f32 = 3.0e-4;
/// Angular step of the sun direction the cascades are fitted to: 0.1 degree in elevation and azimuth.
const SUN_STEP: f32 = 0.1 * std::f32::consts::PI / 180.0;

/// `light_dir` snapped to a fixed elevation/azimuth lattice for the cascade fit. A slowly moving sun then keeps
/// bit-identical cascade matrices between steps, so the static-depth cache and the schedule can reuse maps. The
/// fit itself uses the snapped direction, so maps and receivers agree and the texel grid cannot swim.
pub(super) fn quantized_light(light_dir: Vec3) -> Vec3 {
    let Some(sun) = (-light_dir).try_normalize() else { return light_dir };
    let snap = |angle: f32| (angle / SUN_STEP).round() * SUN_STEP;
    let elevation = snap(sun.z.clamp(-1.0, 1.0).asin());
    // The azimuth is meaningless at the poles; keep the exact vertical there.
    if elevation.cos() < 1.0e-4 {
        return -Vec3::Z * elevation.signum();
    }
    let azimuth = snap(sun.y.atan2(sun.x));
    -Vec3::new(elevation.cos() * azimuth.cos(), elevation.cos() * azimuth.sin(), elevation.sin())
}

/// Whether a far cascade whose only staleness is a small sun step waits one frame: cascade 2 refreshes on even
/// frames, cascade 3 on odd ones, so a sun step never re-renders both far maps in the same frame.
fn sun_deferred(cascade: usize, frame: u32) -> bool {
    cascade >= 2 && frame % 2 != cascade as u32 % 2
}

/// How a stored map relates to the freshly fitted one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Hold {
    Holds,
    /// Usable for one more frame: only the sun turned, by a small step.
    SunStep,
    Stale,
}

/// Whether the schedule alone refreshes `cascade` on `frame`. Offsets by cascade index spread the load.
pub(super) fn scheduled(cascade: usize, frame: u32) -> bool {
    let period = PERIOD[cascade];
    frame % period == cascade as u32 % period
}

/// Distance in texels between a stale map's frame and the freshly fitted one, measured at the fresh
/// map's centre. Infinite when the stale matrix cannot place that point.
pub(super) fn drift_texels(stale: &Mat4, fresh: &Mat4, size: f32) -> f32 {
    let centre = fresh.inverse().project_point3(Vec3::new(0.0, 0.0, 0.5));
    let ndc = stale.project_point3(centre);
    if !ndc.is_finite() {
        return f32::INFINITY;
    }
    ndc.x.abs().max(ndc.y.abs()) * size * 0.5
}

/// What each cascade map currently holds, as the receiver needs it.
#[derive(Default)]
pub(super) struct Rendered {
    frame: u32,
    valid: [bool; 4],
    matrices: [[f32; 16]; 4],
    texels: [f32; 4],
    depth_spans: [f32; 4],
    resolutions: [f32; 4],
    sun: [[f32; 4]; 4],
}

impl Rendered {
    /// Forget every map: after the atlas was recreated or shadows were switched off.
    pub fn reset(&mut self) {
        *self = Self { frame: self.frame, ..Default::default() };
    }

    /// Decide which of the first `count` cascades render this frame. Cascades that do not keep the
    /// matrix, texel size and depth span of their last render in `params`, so the receiver agrees with
    /// the map. `force` refreshes everything, for example on a camera cut.
    pub fn plan(&mut self, params: &mut Params, count: usize, force: bool) -> [bool; 4] {
        let frame = self.frame;
        self.frame = frame.wrapping_add(1);
        let mut due = [true; 4];
        for (cascade, slot) in due.iter_mut().enumerate().take(count) {
            let stale = match self.holds(cascade, params) {
                Hold::Holds => false,
                Hold::SunStep => !sun_deferred(cascade, frame),
                Hold::Stale => true,
            };
            *slot = force || scheduled(cascade, frame) || stale;
            if *slot {
                self.record(cascade, params);
                continue;
            }
            self.restore(cascade, params);
        }
        due
    }

    /// Whether the stored map can still stand in for the freshly fitted one.
    fn holds(&self, cascade: usize, fresh: &Params) -> Hold {
        if !self.valid[cascade]
            || self.resolutions[cascade] != fresh.resolutions[cascade]
            || self.depth_spans[cascade] != fresh.depth_spans[cascade]
        {
            return Hold::Stale;
        }
        let rotation = Vec3::from_slice(&self.sun[cascade]).cross(Vec3::from_slice(&fresh.sun)).length_squared();
        if rotation > SUN_STEP_LIMIT {
            return Hold::Stale;
        }
        let stale = Mat4::from_cols_array(&self.matrices[cascade]);
        let fresh_matrix = Mat4::from_cols_array(&fresh.matrices[cascade]);
        if drift_texels(&stale, &fresh_matrix, fresh.resolutions[cascade]) > DRIFT_LIMIT {
            return Hold::Stale;
        }
        match rotation > SUN_LIMIT {
            true => Hold::SunStep,
            false => Hold::Holds,
        }
    }

    fn record(&mut self, cascade: usize, params: &Params) {
        self.valid[cascade] = true;
        self.matrices[cascade] = params.matrices[cascade];
        self.texels[cascade] = params.texels[cascade];
        self.depth_spans[cascade] = params.depth_spans[cascade];
        self.resolutions[cascade] = params.resolutions[cascade];
        self.sun[cascade] = params.sun;
    }

    fn restore(&self, cascade: usize, params: &mut Params) {
        params.matrices[cascade] = self.matrices[cascade];
        params.texels[cascade] = self.texels[cascade];
        params.depth_spans[cascade] = self.depth_spans[cascade];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    fn ortho(centre: Vec3, radius: f32) -> [f32; 16] {
        let projection = glam::camera::rh::proj::directx::orthographic(-radius, radius, -radius, radius, 0.1, 100.0);
        (projection * Mat4::from_translation(-centre - Vec3::Z * 50.0)).to_cols_array()
    }

    fn params(shift: f32) -> Params {
        let mut params = Params::zeroed();
        for matrix in &mut params.matrices {
            *matrix = ortho(Vec3::new(shift, 0.0, 0.0), 10.0);
        }
        params.resolutions = [1024.0; 4];
        params.depth_spans = [96.0; 4];
        params.texels = [0.02; 4];
        params.sun = [0.0, 0.0, -1.0, 0.0];
        params
    }

    #[test]
    fn schedule_refreshes_near_every_frame_and_far_on_their_period() {
        for frame in 0..64 {
            assert!(scheduled(0, frame), "cascade 0 renders every frame");
        }
        let counts: Vec<_> = (0..4).map(|c| (0..64).filter(|&f| scheduled(c, f)).count()).collect();
        assert_eq!(counts, [64, 32, 16, 16]);
        for (cascade, &period) in PERIOD.iter().enumerate().take(4).skip(1) {
            let mut last = 0;
            for frame in 1..64 {
                if scheduled(cascade, frame) {
                    assert!(frame - last <= period, "cascade {cascade} starved at {frame}");
                    last = frame;
                }
            }
        }
        assert!((0..64).all(|f| (0..4).filter(|&c| scheduled(c, f)).count() <= 3), "load stays spread");
    }

    #[test]
    fn first_frame_renders_everything_then_stale_cascades_keep_their_matrix() {
        let mut rendered = Rendered::default();
        let mut first = params(0.0);
        assert_eq!(rendered.plan(&mut first, 4, false), [true; 4], "nothing rendered yet");
        // Frame 1: cascades 0 and 1 are scheduled; 2 and 3 keep the frame-0 matrix.
        let mut next = params(0.05);
        let due = rendered.plan(&mut next, 4, false);
        assert_eq!(due, [true, true, false, false]);
        assert_eq!(next.matrices[2], first.matrices[2]);
        assert_eq!(next.matrices[3], first.matrices[3]);
        assert_ne!(next.matrices[1], first.matrices[1]);
    }

    #[test]
    fn drift_depth_span_sun_and_cuts_force_a_refresh() {
        let mut rendered = Rendered::default();
        rendered.plan(&mut params(0.0), 4, false);
        // Frame 1: nothing moves, only the schedule decides.
        assert_eq!(rendered.plan(&mut params(0.0), 4, false), [true, true, false, false]);
        // Frame 2: cascade 2 is scheduled; 1 and 3 are not and can stay.
        assert_eq!(rendered.plan(&mut params(0.0), 4, false), [true, false, true, false]);
        // Frame 3 with a shift of 1 m on a 20 m, 1024 texel map is 51 texels: every stale cascade refreshes.
        let moved = rendered.plan(&mut params(1.0), 4, false);
        assert_eq!(moved, [true; 4]);
        // Frame 4: a changed depth span refreshes a cascade that was not scheduled.
        let mut spans = params(1.0);
        spans.depth_spans[1] = 112.0;
        assert!(rendered.plan(&mut spans, 4, false)[1]);
        // Frame 5: a rotated sun does too.
        let mut sun = params(1.0);
        sun.sun = [0.1, 0.0, -0.995, 0.0];
        assert!(rendered.plan(&mut sun, 4, false)[3]);
        // A camera cut renders everything.
        assert_eq!(rendered.plan(&mut params(1.0), 4, true), [true; 4]);
        rendered.reset();
        assert_eq!(rendered.plan(&mut params(1.0), 4, false), [true; 4], "reset invalidates the maps");
    }

    #[test]
    fn quantized_sun_holds_between_steps_and_stays_close() {
        let base = Vec3::new(-0.932, -0.262, 0.249).normalize();
        let held = quantized_light(-base);
        assert!(held.angle_between(-base) <= SUN_STEP, "snapping error stays within one step");
        // 0.002 degrees per frame: most frames keep the exact same fitted direction.
        let same = (1..48)
            .filter(|&step| {
                let sun = glam::Quat::from_rotation_z(step as f32 * 0.00003) * base;
                let previous = glam::Quat::from_rotation_z((step - 1) as f32 * 0.00003) * base;
                quantized_light(-sun).to_array().map(f32::to_bits)
                    == quantized_light(-previous).to_array().map(f32::to_bits)
            })
            .count();
        assert!(same >= 44, "a slow sun must keep bit-identical fits: {same}");
        assert_eq!(quantized_light(Vec3::NEG_Z), Vec3::NEG_Z);
        assert_eq!(quantized_light(Vec3::ZERO), Vec3::ZERO);
    }

    #[test]
    fn a_sun_step_refreshes_far_cascades_on_alternate_frames() {
        let mut rendered = Rendered::default();
        rendered.plan(&mut params(0.0), 4, false);
        rendered.plan(&mut params(0.0), 4, false);
        rendered.plan(&mut params(0.0), 4, false);
        rendered.plan(&mut params(0.0), 4, false);
        // Frame 4 (even): a 0.1 degree step. Cascade 2 refreshes now, cascade 3 waits one frame.
        let step = |p: &mut Params| {
            let sun = glam::Quat::from_rotation_x(SUN_STEP) * Vec3::NEG_Z;
            p.sun = sun.extend(0.0).to_array();
        };
        let mut first = params(0.0);
        step(&mut first);
        assert_eq!(rendered.plan(&mut first, 4, false), [true, true, true, false]);
        let mut second = params(0.0);
        step(&mut second);
        // Frame 5 (odd): cascade 1 is scheduled anyway; cascade 3 catches up, cascade 2 already holds the step.
        assert_eq!(rendered.plan(&mut second, 4, false), [true, true, false, true], "cascade 3 catches up");
        // A large turn still refreshes everything at once.
        let mut turn = params(0.0);
        turn.sun = [0.1, 0.0, -0.995, 0.0];
        assert_eq!(rendered.plan(&mut turn, 4, false), [true; 4]);
    }

    #[test]
    fn drift_is_measured_in_texels() {
        let a = Mat4::from_cols_array(&ortho(Vec3::ZERO, 10.0));
        let b = Mat4::from_cols_array(&ortho(Vec3::new(0.2, 0.0, 0.0), 10.0));
        assert!(drift_texels(&a, &a, 1024.0) < 1e-3);
        let texels = drift_texels(&a, &b, 1024.0);
        assert!((texels - 0.2 / 20.0 * 1024.0).abs() < 0.05, "{texels}");
        assert!(drift_texels(&Mat4::ZERO, &b, 1024.0).is_infinite());
    }
}
