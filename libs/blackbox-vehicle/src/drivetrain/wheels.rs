use crate::math::MS_TO_MPH;

/// Which axles take drive: `(front, rear)`.
pub type DrivenAxles = (bool, bool);

fn axle_mean(av: &[f32; 4], axle: usize) -> f32 {
    0.5 * (av[axle * 2] + av[axle * 2 + 1])
}

/// The "free" and "locked" driven-wheel speeds (rad/s). Free is the mean of the driven axle(s) (the larger
/// magnitude when both drive). Locked selects the largest non-negative axle mean in forward gears,
/// or the smallest non-positive axle mean in reverse, independently of free-speed selection.
pub fn driven_speeds(av: &[f32; 4], driven: DrivenAxles, reverse: bool) -> (f32, f32) {
    let free = match driven {
        (true, true) => {
            let (f, r) = (axle_mean(av, 0), axle_mean(av, 1));
            if f.abs() >= r.abs() { f } else { r }
        }
        (true, false) => axle_mean(av, 0),
        (false, true) => axle_mean(av, 1),
        (false, false) => 0.0,
    };
    let front = if driven.0 { axle_mean(av, 0) } else { 0.0 };
    let rear = if driven.1 { axle_mean(av, 1) } else { 0.0 };
    let locked = if reverse { front.min(rear).min(0.0) } else { front.max(rear).max(0.0) };
    (free, locked)
}

/// Shifts every driven wheel by the difference between `wanted` and the original free speed. Below 40 mph,
/// or when both wheels of an axle are off the ground, each driven pair is first averaged (a locked diff).
pub fn write_back(av: &mut [f32; 4], grounded: [bool; 4], driven: DrivenAxles, reverse: bool, wanted: f32, speed: f32) {
    let (free, _) = driven_speeds(av, driven, reverse);
    let delta = wanted - free;
    let axles = [driven.0, driven.1];
    for (axle, &is_driven) in axles.iter().enumerate() {
        if !is_driven {
            continue;
        }
        let (a, b) = (axle * 2, axle * 2 + 1);
        if speed * MS_TO_MPH < 40.0 || (!grounded[a] && !grounded[b]) {
            let m = 0.5 * (av[a] + av[b]);
            av[a] = m;
            av[b] = m;
        }
    }
    for (axle, &is_driven) in axles.iter().enumerate() {
        if is_driven {
            av[axle * 2] += delta;
            av[axle * 2 + 1] += delta;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rear_drive_reads_the_rear_pair() {
        let av = [1.0, 1.0, 10.0, 12.0];
        assert_eq!(driven_speeds(&av, (false, true), false), (11.0, 11.0));
        assert_eq!(driven_speeds(&av, (true, true), false).0, 11.0);
    }

    #[test]
    fn locked_speed_is_sign_forced() {
        let av = [0.0, 0.0, -5.0, -5.0];
        assert_eq!(driven_speeds(&av, (false, true), false), (-5.0, 0.0));
        assert_eq!(driven_speeds(&av, (false, true), true), (-5.0, -5.0));
    }

    #[test]
    fn opposed_axles_keep_the_rate_in_the_selected_direction() {
        let av = [20.0, 24.0, -40.0, -44.0];
        assert_eq!(driven_speeds(&av, (true, true), false), (-42.0, 22.0));
        assert_eq!(driven_speeds(&av, (true, true), true), (-42.0, -42.0));
        let flipped = [-40.0, -44.0, 20.0, 24.0];
        assert_eq!(driven_speeds(&flipped, (true, true), false), (-42.0, 22.0));
    }

    #[test]
    fn write_back_subtracts_free_speed_during_a_direction_change() {
        for (driven, av, wanted) in
            [((false, true), [1.0, 3.0, 10.0, 14.0], -16.0), ((true, false), [-10.0, -14.0, 1.0, 3.0], 16.0)]
        {
            let mut slow = av;
            write_back(&mut slow, [true; 4], driven, wanted < 0.0, wanted, 5.0);
            let axle = if driven.0 { 0 } else { 2 };
            assert_eq!([slow[axle], slow[axle + 1]], [wanted; 2]);
            let mut fast = av;
            write_back(&mut fast, [true; 4], driven, wanted < 0.0, wanted, 30.0);
            assert_eq!(axle_mean(&fast, axle / 2), wanted);
            assert_eq!(fast[axle + 1] - fast[axle], av[axle + 1] - av[axle]);
        }
    }

    #[test]
    fn write_back_sets_the_locked_speed() {
        let mut av = [1.0, 1.0, 10.0, 14.0];
        write_back(&mut av, [true; 4], (false, true), false, 20.0, 5.0);
        assert_eq!(av, [1.0, 1.0, 20.0, 20.0]);
        let mut fast = [0.0, 0.0, 100.0, 120.0];
        write_back(&mut fast, [true; 4], (false, true), false, 90.0, 30.0);
        assert!((fast[2] - 80.0).abs() < 1e-4 && (fast[3] - 100.0).abs() < 1e-4, "{fast:?}");
    }
}
