use super::spec::AxleConstants;

/// State of one corner: how far the spring is compressed (0 = not touching) and for how long it has been
/// in the air.
#[derive(Clone, Copy, Debug, Default)]
pub struct Corner {
    pub compression: f32,
    pub air_time: f32,
}

/// Which side of the axle a corner is on, for the sway bar sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// The vertical force (N, along the body up axis, never negative) of a loaded corner: progressive spring,
/// digressive damper with blow-off, and the anti-roll bar.
///
/// `c_old` and `c_new` are this wheel's compression before and after this step; `c_left_old` and
/// `c_right_old` are the stored compressions of the axle's two wheels used by the sway bar.
pub struct SpringInput {
    pub c_old: f32,
    pub c_new: f32,
    pub c_left_old: f32,
    pub c_right_old: f32,
    pub side: Side,
    pub dt: f32,
    /// Body mass, for the damper blow-off threshold.
    pub mass: f32,
    pub blowout: f32,
}

pub fn spring_force(a: &AxleConstants, i: &SpringInput) -> f32 {
    let mut rise = (i.c_new - i.c_old) / i.dt;
    if a.valving > 1e-6 && a.digression < 1.0 && rise.abs() > a.valving {
        rise = rise.signum() * a.valving * (rise.abs() / a.valving).powf(a.digression);
    }
    let spring = i.c_new * a.spring * (1.0 + i.c_new * a.progression);
    let mut damper = rise * if rise > 0.0 { a.shock } else { a.shock_ext };
    if i.blowout > 0.0 && damper > i.blowout * 9.81 * i.mass {
        damper = 0.0;
    }
    let roll = (i.c_left_old - i.c_right_old) * a.sway;
    let sway = if i.side == Side::Left { roll } else { -roll };
    (damper + spring + sway).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suspension::ChassisSpec;

    fn axle() -> AxleConstants {
        let c = ChassisSpec {
            spring_stiffness: [600.0; 2],
            spring_progression: [0.0; 2],
            shock_stiffness: [40.0; 2],
            shock_ext_stiffness: [60.0; 2],
            shock_valving: [0.0; 2],
            shock_digression: [0.0; 2],
            shock_blowout: 0.0,
            swaybar_stiffness: [100.0; 2],
            travel: [5.0; 2],
            ride_height: [4.0; 2],
            track_width: [1.6; 2],
            wheel_base: 2.6,
            front_axle: 1.3,
            front_weight_bias: 50.0,
            roll_center: 18.0,
        };
        c.axle(0, 0.0)
    }

    fn input(c_old: f32, c_new: f32) -> SpringInput {
        SpringInput {
            c_old,
            c_new,
            c_left_old: c_old,
            c_right_old: c_old,
            side: Side::Left,
            dt: 1.0 / 60.0,
            mass: 1500.0,
            blowout: 0.0,
        }
    }

    #[test]
    fn static_spring_force_is_stiffness_times_compression() {
        let a = axle();
        let f = spring_force(&a, &input(0.05, 0.05));
        assert!((f - 0.05 * 600.0 * 175.1268).abs() < 1.0, "{f}");
    }

    #[test]
    fn damper_pushes_back_harder_when_compressing_than_extending_per_its_rates() {
        let a = axle();
        let comp = spring_force(&a, &input(0.04, 0.05));
        let ext = spring_force(&a, &input(0.06, 0.05));
        let rest = spring_force(&a, &input(0.05, 0.05));
        assert!(comp > rest && ext < rest);
        // Extension damping is stiffer (60 vs 40 lb s/in) so the force falls more than it rose.
        assert!(rest - ext > comp - rest);
    }

    #[test]
    fn force_never_goes_negative() {
        let a = axle();
        assert_eq!(spring_force(&a, &input(0.2, 0.0)), 0.0);
    }

    #[test]
    fn sway_bar_loads_the_more_compressed_wheel() {
        let a = axle();
        let mut left = input(0.05, 0.05);
        left.c_left_old = 0.07;
        left.c_right_old = 0.03;
        let l = spring_force(&a, &left);
        let mut right = input(0.05, 0.05);
        right.c_left_old = 0.07;
        right.c_right_old = 0.03;
        right.side = Side::Right;
        let r = spring_force(&a, &right);
        assert!(l > r);
        assert!((l - r - 2.0 * 0.04 * a.sway).abs() < 1.0);
    }

    #[test]
    fn digression_flattens_fast_damper_motion() {
        let mut c = axle();
        let linear = spring_force(&c, &input(0.0, 0.05));
        c.valving = 0.1;
        c.digression = 0.5;
        let flat = spring_force(&c, &input(0.0, 0.05));
        assert!(flat < linear);
    }

    #[test]
    fn blow_off_valve_opens_on_a_huge_bump() {
        let a = axle();
        let mut hard = input(0.0, 0.05);
        hard.blowout = 0.5;
        let with = spring_force(&a, &hard);
        let spring_only = 0.05 * a.spring;
        assert!((with - spring_only).abs() < 1.0, "damper should be vented: {with} vs {spring_only}");
    }
}
