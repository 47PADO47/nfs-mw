//! One frame of a skin: the values in, the quads out, in the order the plugin draws them. Spec:
//! `docs/specs/custom-hud-gauges.md` sections 3 and 10.

use crate::draw::{Animation, Inputs, Pictures, Quad};
use crate::element::Gauge;
use crate::gauge::{self, Drawn, Feed};
use crate::place::Placement;
use crate::readout;
use crate::skin::Skin;

/// The most the boost gauge reads either way.
pub const BOOST_LIMIT: f32 = 20.0;

/// The quads of a skin for this frame, back to front. A picture the skin names that `pictures` does not know the size of is
/// still emitted by the gauges (they need no size) and left out by the digit rows (they cannot cut a cell without one).
pub fn compose(
    skin: &Skin,
    inputs: &Inputs,
    placement: &Placement,
    pictures: &dyn Pictures,
    animation: &mut Animation,
) -> Vec<Quad> {
    let mut out = Vec::new();
    let rpm = inputs.rpm / 1000.0;
    let red_line = inputs.red_line / 1000.0;
    let nitrous = inputs.nitrous.unwrap_or(0.0);
    let boost = inputs.boost.unwrap_or(0.0).clamp(-BOOST_LIMIT, BOOST_LIMIT);
    let dt = inputs.dt.max(0.0);

    if let Some(background) = &skin.background {
        readout::square(background, background.common.colour, placement, &mut out);
    }

    let tachometer = skin.tachometer.as_ref().map(|g| {
        let feed = Feed {
            value: rpm,
            red_line: Some(red_line),
            scale_end: Some(g.value),
            fill: Some((0.0, rpm)),
            visible: true,
            perfect: inputs.perfect_zone,
        };
        gauge::draw(g, &feed, &mut animation.fades[0], dt, placement)
    });
    if let Some(drawn) = &tachometer {
        out.extend(drawn.layers.iter().cloned());
    }
    if let Some(row) = &skin.tachometer_digits {
        readout::revs(row, inputs.rpm, pictures, placement, &mut out);
    }
    if let Some(bar) = &skin.tachometer_bar {
        readout::bar(bar, rpm, placement, &mut out);
    }

    let speed = inputs.speed.clamp(0.0, 999.0);
    let speedometer = skin.speedometer.as_ref().map(|g| {
        let feed = Feed { value: speed, red_line: None, scale_end: None, fill: None, visible: true, perfect: false };
        gauge::draw(g, &feed, &mut animation.fades[1], dt, placement)
    });
    if let Some(drawn) = &speedometer {
        out.extend(drawn.layers.iter().cloned());
    }
    if let Some(row) = &skin.speed_digits {
        readout::speed(row, inputs.speed, pictures, placement, &mut out);
    }

    let dial =
        |g: &Option<Gauge>, slot: usize, value: f32, visible: bool, animation: &mut Animation, out: &mut Vec<Quad>| {
            let Some(g) = g else { return };
            let feed =
                Feed { value, red_line: None, scale_end: None, fill: Some((0.0, value)), visible, perfect: false };
            let Drawn { layers, needle } = gauge::draw(g, &feed, &mut animation.fades[slot], dt, placement);
            out.extend(layers);
            out.extend(needle);
        };
    dial(&skin.boost, 3, boost, inputs.boost.is_some(), animation, &mut out);
    dial(&skin.nitrous, 2, nitrous, inputs.nitrous.is_some(), animation, &mut out);
    if let (Some(bar), Some(_)) = (&skin.nitrous_bar, inputs.nitrous) {
        readout::bar(bar, nitrous, placement, &mut out);
    }
    dial(&skin.speedbreaker, 4, inputs.speedbreaker, true, animation, &mut out);
    if let Some(bar) = &skin.speedbreaker_bar {
        readout::bar(bar, inputs.speedbreaker, placement, &mut out);
    }

    if let Some(row) = &skin.gear {
        readout::gear(row, inputs.gear, pictures, placement, &mut out);
    }
    if let Some(icon) = &skin.units {
        readout::units(icon, inputs.mph, placement, &mut out);
    }
    if let Some(icon) = &skin.shift_light {
        let on = readout::shift_light_on(inputs.rpm, inputs.red_line);
        let show = readout::blink(icon.blink, on, dt, &mut animation.shift_timer);
        readout::shift_light(icon, show, placement, &mut out);
    }

    // The tachometer's and the speedometer's needles are drawn after everything else.
    out.extend(tachometer.and_then(|drawn| drawn.needle));
    out.extend(speedometer.and_then(|drawn| drawn.needle));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Sizes;
    impl Pictures for Sizes {
        fn size(&self, _file: &str) -> Option<(u32, u32)> {
            Some((1200, 100))
        }
    }

    const SAMPLE: &str = "[BACKGROUND]\nEnabled = 1\nSize = 400\nTexture = back.dds\nColor = 0xFFFFFFFF\n[TACHOMETER_GAUGE]\nEnabled = 1\nSize = 330\nPositionX = 30\nPositionY = 60\nValue = 10\nArrowTexture = needle.dds\nArrowMinAngle = -30\nArrowMaxAngle = 210\nBackgroundTexture = dial.dds\n[SPEEDOMETER_DIGITAL]\nEnabled = 1\nSize = 65\nPositionX = 130\nPositionY = 83\nDigitsTexture = digits.dds\nColor = 0xFFFFFFFF\n[BOOST_GAUGE]\nEnabled = 1\nSize = 90\nPositionX = 345\nPositionY = 70\nValue = 20\nArrowTexture = needle.dds\nArrowMinAngle = 90\nArrowMaxAngle = 210\nBackgroundTexture = boost.dds\n[NOS_GAUGE]\nEnabled = 1\nSize = 90\nPositionX = 345\nPositionY = 170\nValue = 1\nArrowTexture = needle.dds\nArrowMinAngle = -30\nArrowMaxAngle = 210\nBackgroundTexture = nos.dds\n[SPEEDBREAK_GAUGE]\nEnabled = 1\nSize = 90\nPositionX = 345\nPositionY = 275\nValue = 1\nArrowTexture = needle.dds\nArrowMinAngle = -30\nArrowMaxAngle = 210\nBackgroundTexture = sb.dds\n[GEAR]\nEnabled = 1\nSize = 130\nPositionX = 130\nPositionY = 110\nDigitsTexture = gear.dds\nColor = 0xFFFFFFFF\n[UNITS]\nEnabled = 1\nSize = 64\nPositionX = 128\nPositionY = 45\nUnitsTexture = units.dds\nColor = 0xFFFFFFFF\n";

    fn frame(inputs: Inputs) -> Vec<Quad> {
        let placement = Placement::new(480.0 * 16.0 / 9.0, 480.0, 1.0, 0.0, 0.0);
        compose(&Skin::parse(SAMPLE), &inputs, &placement, &Sizes, &mut Animation::default())
    }

    fn pictures_of(quads: &[Quad]) -> Vec<&str> {
        quads.iter().map(|q| q.picture.as_str()).collect()
    }

    #[test]
    fn the_plugin_order_puts_the_tachometer_needle_last() {
        let quads =
            frame(Inputs { rpm: 3000.0, speed: 42.0, nitrous: Some(0.5), boost: Some(3.0), ..Inputs::default() });
        let pictures = pictures_of(&quads);
        assert_eq!(pictures[0], "back.dds");
        assert_eq!(pictures[1], "dial.dds", "the tachometer's dial comes right after the background");
        let tail = &pictures[pictures.len() - 1];
        assert_eq!(*tail, "needle.dds", "the last quad is a needle");
        let boost = pictures.iter().position(|p| *p == "boost.dds").expect("the boost is drawn");
        let nos = pictures.iter().position(|p| *p == "nos.dds").expect("the nitrous is drawn");
        let breaker = pictures.iter().position(|p| *p == "sb.dds").expect("the speed breaker is drawn");
        assert!(boost < nos && nos < breaker, "boost, nitrous, speed breaker");
        let gear = pictures.iter().position(|p| *p == "gear.dds").expect("the gear is drawn");
        let units = pictures.iter().position(|p| *p == "units.dds").expect("the units are drawn");
        assert!(breaker < gear && gear < units);
    }

    #[test]
    fn the_nitrous_and_boost_gauges_wait_for_a_car_that_has_them() {
        let quads = frame(Inputs { rpm: 3000.0, speed: 42.0, ..Inputs::default() });
        let pictures = pictures_of(&quads);
        assert!(!pictures.contains(&"nos.dds"));
        assert!(!pictures.contains(&"boost.dds"));
        assert!(pictures.contains(&"sb.dds"), "the speed breaker is always there");
    }

    #[test]
    fn the_needles_after_the_tachometer_are_the_other_gauges_own() {
        let quads = frame(Inputs { nitrous: Some(0.0), boost: Some(0.0), ..Inputs::default() });
        let needles: Vec<&Quad> = quads.iter().filter(|q| q.turn.is_some()).collect();
        assert_eq!(needles.len(), 4, "boost, nitrous, speed breaker and tachometer");
        assert!(std::ptr::eq(*needles.last().expect("needles"), quads.last().expect("quads")));
    }

    #[test]
    fn the_speed_digits_show_the_whole_speed() {
        let quads = frame(Inputs { speed: 123.8, ..Inputs::default() });
        let digits: Vec<f32> =
            quads.iter().filter(|q| q.picture == "digits.dds").map(|q| (q.uv[0] * 12.0).round()).collect();
        assert_eq!(digits, vec![3.0, 2.0, 1.0]);
    }

    #[test]
    fn the_boost_is_clamped_to_its_limit() {
        let quads = frame(Inputs { boost: Some(80.0), ..Inputs::default() });
        let boost_needle = quads.iter().find(|q| q.turn.is_some_and(|t| (t.radians.to_degrees() - 210.0).abs() < 1e-3));
        assert!(boost_needle.is_some(), "80 psi stops at the scale's end, 210 degrees");
    }
}
