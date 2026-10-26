//! The scripted drives of the measurement: steady engine speeds, a rev sweep, a coast-down, a standing start with
//! shifts, a nitrous pull, a hit and a scrape.

use blackbox_carsound::{GEAR_FIRST, ScrapeKind};

use super::rig::{FPS, Frame, hit, wall_contact};

/// Speed of a car at the redline in each gear, in mph (first gear first): a plain six-speed.
const REDLINE_MPH: [f32; 6] = [38.0, 62.0, 88.0, 115.0, 145.0, 175.0];

/// The speed at `rpm` (0 to 1 of the way to redline) in `gear` (the scene's id, first is 2).
pub fn mph_at(rpm: f32, gear: i32) -> f32 {
    let top = REDLINE_MPH[(gear - GEAR_FIRST).clamp(0, 5) as usize];
    (0.15 + 0.85 * rpm.clamp(0.0, 1.0)) * top
}

fn frames(seconds: f32, make: impl Fn(f32) -> Frame) -> Vec<Frame> {
    (0..(seconds * FPS as f32) as usize).map(|n| make(n as f32 / FPS as f32)).collect()
}

/// The engine held at `rpm` with `throttle` in third gear.
pub fn steady(rpm: f32, throttle: f32, seconds: f32) -> Vec<Frame> {
    frames(seconds, |_| Frame::drive(rpm, throttle, GEAR_FIRST + 2, mph_at(rpm, GEAR_FIRST + 2)))
}

/// Idle to redline with `throttle` over `seconds`, in third gear.
pub fn sweep(throttle: f32, seconds: f32) -> Vec<Frame> {
    frames(seconds, |t| {
        let rpm = t / seconds;
        Frame::drive(rpm, throttle, GEAR_FIRST + 2, mph_at(rpm, GEAR_FIRST + 2))
    })
}

/// Off the throttle from 0.8 of the way to redline down to 0.1 over `seconds`, in fourth gear.
pub fn coast(seconds: f32) -> Vec<Frame> {
    frames(seconds, |t| {
        let rpm = 0.8 - 0.7 * t / seconds;
        Frame::drive(rpm, 0.0, GEAR_FIRST + 3, mph_at(rpm, GEAR_FIRST + 3))
    })
}

/// Idle for a second, then a full-throttle pull through six gears with a lift of a third of a second at each shift.
pub fn start_with_shifts() -> Vec<Frame> {
    let mut out = frames(1.0, |_| Frame::idle());
    for gear in 0..6 {
        let id = GEAR_FIRST + gear;
        let (from, to) = (if gear == 0 { 0.05 } else { 0.5 }, 0.95);
        let pull = 2.0 + 0.4 * gear as f32;
        out.extend(frames(pull, |t| {
            let rpm = from + (to - from) * t / pull;
            Frame::drive(rpm, 1.0, id, mph_at(rpm, id))
        }));
        if gear < 5 {
            out.extend(frames(0.35, |t| {
                let rpm = to - 0.4 * t / 0.35;
                Frame::drive(rpm, 0.0, id + 1, mph_at(rpm, id))
            }));
        }
    }
    out
}

/// A second of cruise, three seconds of a full-throttle pull with the nitrous burning, a second of cruise.
pub fn nitrous() -> Vec<Frame> {
    frames(5.0, |t| {
        let on = (1.0..4.0).contains(&t);
        let rpm = if on { 0.5 + 0.15 * (t - 1.0) } else { 0.5 };
        let mut frame = Frame::drive(rpm, if on { 1.0 } else { 0.4 }, GEAR_FIRST + 3, mph_at(rpm, GEAR_FIRST + 3));
        frame.input.nos_active = on;
        frame
    })
}

/// Cruising, one hard hit at 1 s, cruising on to 4 s.
pub fn crash() -> Vec<Frame> {
    let mut out = frames(4.0, |_| Frame::drive(0.5, 0.4, GEAR_FIRST + 3, mph_at(0.5, GEAR_FIRST + 3)));
    out[FPS as usize].events.push(hit(0.9));
    out
}

/// Cruising with the car rubbing a wall from 1 s to 3 s.
pub fn scrape() -> Vec<Frame> {
    let mut out = frames(4.0, |_| Frame::drive(0.5, 0.4, GEAR_FIRST + 3, mph_at(0.5, GEAR_FIRST + 3)));
    for frame in &mut out[FPS as usize..3 * FPS as usize] {
        frame.scrape = Some((ScrapeKind::Wall, 0.8));
        frame.contacts.push(wall_contact());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scripts_have_the_lengths_they_say() {
        assert_eq!(steady(0.5, 1.0, 2.0).len(), 120);
        assert_eq!(sweep(1.0, 9.0).len(), 540);
        assert_eq!(crash().len(), 240);
        assert_eq!(scrape().iter().filter(|frame| frame.scrape.is_some()).count(), 120);
        assert_eq!(crash().iter().map(|frame| frame.events.len()).sum::<usize>(), 1);
    }

    #[test]
    fn a_pull_rises_through_each_gear_and_drops_at_each_shift() {
        let start = start_with_shifts();
        let gears: Vec<i32> = start.iter().map(|frame| frame.input.gear).collect();
        assert_eq!((gears[0], *gears.last().unwrap()), (GEAR_FIRST, GEAR_FIRST + 5));
        assert!(gears.windows(2).all(|pair| pair[1] >= pair[0]));
        assert!(mph_at(1.0, GEAR_FIRST + 5) > mph_at(1.0, GEAR_FIRST));
    }
}
