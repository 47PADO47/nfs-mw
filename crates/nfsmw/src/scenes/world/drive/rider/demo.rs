//! A bike's state written as text, for the car viewer: `nfsmw view-car <ID> --rider-state "speed=-5,clutch=1"` shows the rider as
//! he is in that state, once his smoothers have settled (a screenshot of the walk back, the fingers on the levers, a lean).
//! A debugging aid with no key of its own; `docs/specs/ride6-rider-hands.md` lists the states the pictures of the hands use.

use super::params::RiderSample;

/// The keys, `name=value` separated by commas. Speeds are km/h; the levers, the lean and the tuck run 0 to 1 (the lean -1 to 1,
/// positive to the right).
const KEYS: [&str; 8] = ["speed", "throttle", "brake", "clutch", "lean", "tuck", "steer", "weight"];

/// The rider's sample for the state `text` names; a key left out is 0 (a bike standing, its rider upright, no lever pulled).
pub fn parse(text: &str) -> Result<RiderSample, String> {
    let mut sample = RiderSample { max_lean: 1.0, front_down: true, rear_down: true, ..RiderSample::default() };
    for part in text.split(',').map(str::trim).filter(|part| !part.is_empty()) {
        let (key, value) = part.split_once('=').ok_or_else(|| format!("{part:?} is not name=value"))?;
        let value: f32 = value.trim().parse().map_err(|_| format!("{value:?} is not a number"))?;
        if !value.is_finite() {
            return Err(format!("{key}={value} is not finite"));
        }
        match key.trim() {
            "speed" => sample.speed = value / 3.6,
            "throttle" => sample.throttle = value.clamp(0.0, 1.0),
            "brake" => {
                sample.front_brake = value.clamp(0.0, 1.0);
                sample.rear_brake = sample.front_brake;
            }
            "clutch" => sample.clutch = value.clamp(0.0, 1.0),
            "lean" => {
                let lean = value.clamp(-1.0, 1.0);
                (sample.lean, sample.bike_lean, sample.rider_lean) = (lean * 0.8, lean, lean);
            }
            "tuck" => sample.tuck_level = value.clamp(0.0, 1.0),
            "steer" => sample.steer = value.clamp(-1.0, 1.0),
            "weight" => sample.weight = value.clamp(-1.0, 1.0),
            other => return Err(format!("{other:?} is not a rider state key (one of {})", KEYS.join(", "))),
        }
    }
    Ok(sample)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_state_names_the_levers_the_speed_and_the_lean() {
        let sample = parse("speed=36, throttle=1,clutch=0.5, brake=0.25,lean=-0.5").unwrap();
        assert!((sample.speed - 10.0).abs() < 1e-5, "km/h become m/s");
        assert_eq!((sample.throttle, sample.clutch, sample.front_brake, sample.rear_brake), (1.0, 0.5, 0.25, 0.25));
        assert_eq!((sample.bike_lean, sample.rider_lean), (-0.5, -0.5));
        assert!(sample.lean < 0.0 && sample.front_down && sample.rear_down);
    }

    #[test]
    fn a_state_with_nothing_in_it_is_a_bike_standing_and_levers_are_held_to_their_range() {
        let standing = parse("").unwrap();
        assert_eq!((standing.speed, standing.throttle, standing.clutch, standing.bike_lean), (0.0, 0.0, 0.0, 0.0));
        assert_eq!(parse("throttle=7,lean=-4").unwrap().throttle, 1.0);
        assert_eq!(parse("lean=-4").unwrap().bike_lean, -1.0);
        assert_eq!(parse("speed=-5").unwrap().speed, -5.0 / 3.6, "a negative speed is a bike rolling back");
    }

    #[test]
    fn a_state_that_is_not_understood_is_refused_with_the_reason() {
        assert!(parse("speed").unwrap_err().contains("name=value"));
        assert!(parse("speed=fast").unwrap_err().contains("not a number"));
        assert!(parse("wheelie=1").unwrap_err().contains("not a rider state key"));
        assert!(parse("speed=inf").unwrap_err().contains("not finite"));
    }
}
