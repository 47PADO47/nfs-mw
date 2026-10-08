//! `--drive-script`: a scripted driver for headless runs and screenshots.
//!
//! `"3:throttle=1;2:throttle=1,steer=0.4;1:brake=1"` holds each input set for that many seconds.
//! Keys: `throttle`, `brake`, `steer` (-1..1), `handbrake`, `nos` (0 or 1), `up` and `down` (a gear
//! shift at the start of the segment). A segment without keys coasts.

use super::clock::STEP;
use super::input::DriveInput;

#[derive(Debug, Clone, PartialEq)]
struct Segment {
    seconds: f32,
    input: DriveInput,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DriveScript {
    segments: Vec<Segment>,
}

impl DriveScript {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut segments = Vec::new();
        for part in text.split(';').map(str::trim).filter(|p| !p.is_empty()) {
            let (time, keys) = part.split_once(':').unwrap_or((part, ""));
            let seconds: f32 = time.trim().parse().map_err(|_| format!("bad duration {time:?} in {part:?}"))?;
            if !(0.0..=3600.0).contains(&seconds) {
                return Err(format!("duration {seconds} is out of range"));
            }
            let mut input = DriveInput::default();
            for pair in keys.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                let (key, value) = pair.split_once('=').unwrap_or((pair, "1"));
                let n: f32 = value.trim().parse().map_err(|_| format!("bad value {value:?} for {key:?}"))?;
                match key.trim() {
                    "throttle" => input.throttle = n.clamp(0.0, 1.0),
                    "brake" => input.brake = n.clamp(0.0, 1.0),
                    "steer" => input.steer = n.clamp(-1.0, 1.0),
                    "handbrake" => input.handbrake = n > 0.5,
                    "nos" => input.nos = n > 0.5,
                    "up" => input.shift_up = n > 0.5,
                    "down" => input.shift_down = n > 0.5,
                    other => return Err(format!("unknown key {other:?} (throttle brake steer handbrake nos up down)")),
                }
            }
            segments.push(Segment { seconds, input });
        }
        if segments.is_empty() {
            return Err("the script is empty".into());
        }
        Ok(Self { segments })
    }

    /// Total length, seconds.
    pub fn duration(&self) -> f32 {
        self.segments.iter().map(|s| s.seconds).sum()
    }

    /// The input at `time` seconds into the script (zero input after the end). A gear shift is
    /// requested on the first physics step of its segment only.
    pub fn input_at(&self, time: f32) -> DriveInput {
        let mut start = 0.0;
        for s in &self.segments {
            if time < start + s.seconds {
                let first = time - start < STEP;
                return DriveInput {
                    shift_up: s.input.shift_up && first,
                    shift_down: s.input.shift_down && first,
                    ..s.input
                };
            }
            start += s.seconds;
        }
        DriveInput::default()
    }

    pub fn finished(&self, time: f32) -> bool {
        time >= self.duration()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_segments() {
        let script = DriveScript::parse("3:throttle=1;2:throttle=1,steer=-0.4;1:brake=1").unwrap();
        assert!((script.duration() - 6.0).abs() < 1e-6);
        assert_eq!(script.input_at(0.0).throttle, 1.0);
        assert_eq!(script.input_at(3.5).steer, -0.4);
        assert_eq!(script.input_at(5.5).brake, 1.0);
        assert_eq!(script.input_at(5.5).throttle, 0.0);
        assert!(!script.finished(5.9) && script.finished(6.0));
        assert_eq!(script.input_at(7.0), DriveInput::default(), "after the end the driver lets go");
    }

    #[test]
    fn flags_and_shifts() {
        let script = DriveScript::parse("1:throttle,up,nos;1:handbrake=1").unwrap();
        let a = script.input_at(0.0);
        assert!(a.shift_up && a.nos && a.throttle == 1.0);
        assert!(!script.input_at(0.5).shift_up, "a shift is requested once, at the start");
        assert!(script.input_at(1.2).handbrake);
    }

    #[test]
    fn values_are_clamped_and_empty_segments_coast() {
        let script = DriveScript::parse("1:throttle=5,steer=-9;2").unwrap();
        assert_eq!(script.input_at(0.5).throttle, 1.0);
        assert_eq!(script.input_at(0.5).steer, -1.0);
        assert_eq!(script.input_at(1.5), DriveInput::default());
        assert!((script.duration() - 3.0).abs() < 1e-6);
    }

    #[test]
    fn rejects_nonsense() {
        assert!(DriveScript::parse("").is_err());
        assert!(DriveScript::parse("x:throttle=1").is_err());
        assert!(DriveScript::parse("1:turbo=1").is_err());
        assert!(DriveScript::parse("1:throttle=fast").is_err());
        assert!(DriveScript::parse("-1:throttle=1").is_err());
    }
}
