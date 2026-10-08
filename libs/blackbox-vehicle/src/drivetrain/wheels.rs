use crate::math::MS_TO_MPH;

/// Which axles take drive: `(front, rear)`.
pub type DrivenAxles = (bool, bool);

fn axle_mean(av: &[f32; 4], axle: usize) -> f32 {
    0.5 * (av[axle * 2] + av[axle * 2 + 1])
}

/// The "free" and "locked" driven-wheel speeds (rad/s). Free is the mean of the driven axle(s) (the larger
/// magnitude when both drive); locked is the same forced non-negative (non-positive in reverse).
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
    let locked = if reverse { free.min(0.0) } else { free.max(0.0) };
    (free, locked)
}

/// Shifts every driven wheel by the difference between `wanted` and the current locked speed. Below 40 mph,
/// or when both wheels of an axle are off the ground, each driven pair is first averaged (a locked diff).
pub fn write_back(av: &mut [f32; 4], grounded: [bool; 4], driven: DrivenAxles, reverse: bool, wanted: f32, speed: f32) {
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
    let (_, locked) = driven_speeds(av, driven, reverse);
    let delta = wanted - locked;
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
    fn write_back_sets_the_locked_speed() {
        let mut av = [1.0, 1.0, 10.0, 14.0];
        write_back(&mut av, [true; 4], (false, true), false, 20.0, 5.0);
        assert_eq!(av, [1.0, 1.0, 20.0, 20.0]);
        let mut fast = [0.0, 0.0, 100.0, 120.0];
        write_back(&mut fast, [true; 4], (false, true), false, 90.0, 30.0);
        assert!((fast[2] - 80.0).abs() < 1e-4 && (fast[3] - 100.0).abs() < 1e-4, "{fast:?}");
    }
}
