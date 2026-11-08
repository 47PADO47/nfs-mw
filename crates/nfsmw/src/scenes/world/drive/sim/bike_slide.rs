//! What a bike's tyre tells the effects of its slide: the squeal, the smoke and the marks (`docs/specs/ride6-bike-skid.md`).
//!
//! A car's readouts are the slide speeds of its tyres in m/s, ramped between two speeds. A tyre of a bike at speed always slips a
//! little (a driven rear tyre a few percent, a leaned tyre a degree or two), and that slip is a speed that grows with the bike's:
//! read as a car's it screeched and marked at 50 m/s with nothing wrong. A tyre skids when its slip is past the peak of its force
//! curve, so the readouts here are the tyre's own slide in units of that peak (`blackbox_vehicle::bike::WheelState::slide`, 1 at
//! the peak), and the speeds the sound reads are only the part of the slide past it.

use blackbox_vehicle::bike::WheelState;
use blackbox_vehicle::math::ramp;

/// The slide (1 = the force peak) over which the marks fade in: none at the peak, full at 1.5 times it. A modelling choice.
pub(super) const SKID_SLIDE: (f32, f32) = (1.05, 1.5);
/// The slide over which the smoke thickens: a tyre well past its peak. A modelling choice.
pub(super) const SMOKE_SLIDE: (f32, f32) = (1.3, 2.2);

/// The slide of one tyre as the car's wheel readouts take it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Slide {
    /// The slip speed along the wheel, past the peak (m/s).
    pub slip: f32,
    /// The slip speed across it, past the peak (m/s), positive to the right.
    pub lateral_speed: f32,
    /// The slip speed past the peak (m/s).
    pub speed: f32,
    pub skid: f32,
    pub smoke: f32,
}

/// How wide a bike's skid mark is, as a share of its tyre's width: the contact patch of a tyre that skids upright is narrower than the
/// tyre. A modelling choice.
const MARK_SHARE: f32 = 0.5;

/// The width of the mark of library wheel `index` of `bike` (m).
pub(super) fn mark_width(bike: &blackbox_vehicle::bike::Bike, index: usize) -> f32 {
    let spec = bike.spec();
    let tyre = match index {
        0 | 1 => &spec.front,
        _ => &spec.rear,
    };
    tyre.width * MARK_SHARE
}

/// The part of a slide of `slide` peaks that lies past the peak: 0 up to it, towards 1 for a tyre that has let go.
pub(super) fn past_peak(slide: f32) -> f32 {
    match slide > 1.0 {
        true => 1.0 - 1.0 / slide,
        false => 0.0,
    }
}

/// The slide of `wheel` on a bike that moves `along` m/s along itself (at least 1 m/s, the tyre model's own regularisation).
pub(super) fn slide_of(wheel: &WheelState, along: f32) -> Slide {
    if !wheel.on_ground {
        return Slide::default();
    }
    let past = past_peak(wheel.slide);
    let slip = wheel.slip_speed * past;
    let lateral_speed = wheel.slip_angle.tan() * along * past;
    Slide {
        slip,
        lateral_speed,
        speed: slip.hypot(lateral_speed),
        skid: ramp(wheel.slide, SKID_SLIDE.0, SKID_SLIDE.1),
        smoke: ramp(wheel.slide, SMOKE_SLIDE.0, SMOKE_SLIDE.1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wheel(slide: f32) -> WheelState {
        WheelState { on_ground: true, slide, slip_speed: 3.0, slip_angle: 0.2, ..WheelState::default() }
    }

    #[test]
    fn a_tyre_up_to_its_peak_reads_no_slide_at_any_speed() {
        for along in [1.0, 30.0, 80.0] {
            assert_eq!(slide_of(&wheel(0.99), along), Slide::default(), "{along} m/s");
            assert_eq!(slide_of(&wheel(1.0), along), Slide::default(), "{along} m/s");
        }
    }

    #[test]
    fn a_tyre_past_its_peak_reads_the_slide_and_marks() {
        let slide = slide_of(&wheel(2.0), 30.0);
        assert!((slide.slip - 1.5).abs() < 1e-5, "{slide:?}");
        assert!((slide.lateral_speed - 0.2_f32.tan() * 30.0 * 0.5).abs() < 1e-4, "{slide:?}");
        assert_eq!(slide.skid, 1.0);
        assert!(slide.smoke > 0.7, "{slide:?}");
        let just = slide_of(&wheel(1.2), 30.0);
        assert!(just.skid > 0.0 && just.skid < 0.5 && just.smoke == 0.0, "{just:?}");
    }

    #[test]
    fn a_wheel_off_the_ground_reads_nothing() {
        let wheel = WheelState { on_ground: false, slide: 5.0, ..wheel(5.0) };
        assert_eq!(slide_of(&wheel, 30.0), Slide::default());
    }
}
