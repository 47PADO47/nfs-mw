//! Original producer parameters, independently of the authored graph output.

pub(super) fn car(speed: f32, levels: [i32; 6]) -> [i32; 14] {
    let speed = match speed.abs() {
        v if !v.is_finite() => 2.0,
        v if v > 40.0 => 39.95,
        v => v.max(2.0),
    };
    let ratio = speed / 40.0;
    let volume = (ratio * 32767.0) as i32;
    let [left_az, right_az, left, right, rumble, pitch] = levels;
    [
        pitch,
        super::road::multiply(left, volume),
        super::road::multiply(right, volume),
        left_az,
        right_az,
        (ratio * 1040.0) as i32,
        0,
        0,
        super::road::multiply(rumble, volume),
        25000,
        0,
        32767,
        0,
        15,
    ]
}

pub(super) fn weather(speed: f32, volume: i32) -> [i32; 4] {
    let mph = match speed.abs() {
        v if v.is_finite() => v * 2.23699,
        _ => 0.0,
    };
    let scale = 1.0 - 0.75 * (mph / 70.0).clamp(0.0, 1.0);
    [((volume.clamp(0, 32767) as f32) * scale) as i32, 0, 4000, 0]
}
