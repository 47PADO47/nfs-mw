//! The death of a destroyed cop: a border flash, then a linear fade, then gone. Spec section 7.

use super::params::BarParams;

/// What a destroyed bar shows, `dead_for` seconds after its death.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Death {
    /// The flash. `on` is the half of the cycle that shows the destroyed border colour.
    Flash { on: bool },
    /// The fade, with the whole bar's alpha going from 1 down to 0.
    Fade { alpha: f32 },
    /// Hidden. The state is kept until the cop is no longer sampled.
    Gone,
}

/// Whether the flash is in its "on" half `dead_for` seconds after death. The colour switches every `period`.
pub fn flash_on(dead_for: f32, period: f32) -> bool {
    if period <= 0.0 {
        return true;
    }
    // A small slack, so that a time on a switch boundary counts as the new half (0.15 / 0.05 is 2.9999995 in f32).
    let half = ((dead_for / period) + 1e-4).floor() as i64;
    half.rem_euclid(2) == 0
}

/// The alpha of a destroyed bar: 1 until the fade starts, linear down to 0 at the hide time, 0 from then on.
pub fn death_alpha(dead_for: f32, params: &BarParams) -> f32 {
    if dead_for < params.fade_start_secs {
        return 1.0;
    }
    let span = params.hide_secs - params.fade_start_secs;
    (1.0 - (dead_for - params.fade_start_secs) / span).clamp(0.0, 1.0)
}

/// The phase of a destroyed bar `dead_for` seconds after its death.
pub fn death_phase(dead_for: f32, params: &BarParams) -> Death {
    if dead_for >= params.hide_secs {
        return Death::Gone;
    }
    if dead_for >= params.fade_start_secs {
        return Death::Fade { alpha: death_alpha(dead_for, params) };
    }
    Death::Flash { on: flash_on(dead_for, params.flash_period_secs) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health_bar::approx;

    #[test]
    fn flash_switches_every_period() {
        let params = BarParams::default();
        assert_eq!(death_phase(0.0, &params), Death::Flash { on: true });
        assert_eq!(death_phase(0.049, &params), Death::Flash { on: true });
        assert_eq!(death_phase(0.05, &params), Death::Flash { on: false });
        assert_eq!(death_phase(0.10, &params), Death::Flash { on: true });
        assert_eq!(death_phase(0.15, &params), Death::Flash { on: false });
    }

    #[test]
    fn fade_runs_from_full_to_nothing_over_the_last_second() {
        let params = BarParams::default();
        assert!(approx(death_alpha(1.0, &params), 1.0));
        assert!(approx(death_alpha(1.5, &params), 0.5));
        assert!(approx(death_alpha(2.0, &params), 0.0));
        assert!(matches!(death_phase(1.0, &params), Death::Fade { alpha } if approx(alpha, 1.0)));
        assert!(matches!(death_phase(1.5, &params), Death::Fade { alpha } if approx(alpha, 0.5)));
    }

    #[test]
    fn gone_from_two_seconds_on() {
        let params = BarParams::default();
        assert_eq!(death_phase(1.999, &params), Death::Fade { alpha: death_alpha(1.999, &params) });
        assert_eq!(death_phase(2.0, &params), Death::Gone);
        assert_eq!(death_phase(9.0, &params), Death::Gone);
    }
}
