//! The geometry of the icon menus' scroller: where an icon sits, how big and how opaque it is
//! (docs/specs/frontend-menus.md, section 2). Plain numbers, no package.

/// The width of the scroller (`IconScrollerMenu` passes 350).
pub const WIDTH: f32 = 350.0;
/// Space between neighbouring icons (negative: they overlap by five).
pub const SPACING: f32 = -5.0;
/// Seconds the scroll takes to reach a new selection.
pub const SCROLL_SECONDS: f32 = 0.2;
/// Frames the icons take to fade in, at 60 frames per second.
pub const FADE_FRAMES: f32 = 9.0;

/// 1 near the centre, falling linearly to 0 at the ends of the scroller, 0 outside it.
pub fn scale(x: f32, center: f32, scroll_size: f32) -> f32 {
    let far = center + scroll_size * 0.5;
    let near = center - scroll_size * 0.5;
    if x < near || x > far {
        return 0.0;
    }
    if x < center - 1.5 {
        return (x - near) / (scroll_size * 0.5);
    }
    if x > center + 1.5 {
        return (far - x) / (scroll_size * 0.5);
    }
    1.0
}

/// Where the centre of an icon goes: `x` is its place on the strip, pulled towards the centre by the cube of
/// how much it shrank.
pub fn pulled(x: f32, center: f32, width: f32, scale: f32) -> f32 {
    let pull = width * (1.0 - scale).powi(3);
    if x < center { x + pull } else { x - pull }
}

/// Idle to fade colour (`0xAARRGGBB`) by `scale`: `idle * scale + fade * (1 - scale)` per channel.
pub fn fade_colour(idle: u32, fade: u32, scale: f32) -> [u8; 4] {
    let channel = |shift: u32| {
        let a = ((idle >> shift) & 0xFF) as f32;
        let b = ((fade >> shift) & 0xFF) as f32;
        (a * scale + b * (1.0 - scale)).clamp(0.0, 255.0) as u8
    };
    [channel(16), channel(8), channel(0), channel(24)]
}

/// The scroll value: eases from where it was to a new target over [`SCROLL_SECONDS`] with a cubic curve.
#[derive(Clone, Copy, Debug, Default)]
pub struct Scroll {
    from: f32,
    to: f32,
    t: f32,
}

impl Scroll {
    pub fn at(value: f32) -> Self {
        Self { from: value, to: value, t: 1.0 }
    }

    pub fn value(&self) -> f32 {
        let t = self.t.clamp(0.0, 1.0);
        let eased = t * t * (3.0 - 2.0 * t);
        self.from + (self.to - self.from) * eased
    }

    /// Starts moving towards `to` from the current value.
    pub fn seek(&mut self, to: f32) {
        if to == self.to {
            return;
        }
        self.from = self.value();
        self.to = to;
        self.t = 0.0;
    }

    pub fn update(&mut self, dt: f32) {
        self.t = (self.t + dt / SCROLL_SECONDS).min(1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_centre_is_full_size_and_the_ends_vanish() {
        assert_eq!(scale(100.0, 100.0, WIDTH), 1.0);
        assert_eq!(scale(101.0, 100.0, WIDTH), 1.0);
        assert_eq!(scale(100.0 + WIDTH, 100.0, WIDTH), 0.0);
        let near = scale(100.0 + 59.0, 100.0, WIDTH);
        assert!((near - (175.0 - 59.0) / 175.0).abs() < 1e-5, "{near}");
        assert_eq!(scale(100.0 - 59.0, 100.0, WIDTH), near, "symmetric");
    }

    #[test]
    fn small_icons_are_pulled_towards_the_centre() {
        assert_eq!(pulled(10.0, 100.0, 64.0, 1.0), 10.0);
        assert!(pulled(10.0, 100.0, 64.0, 0.5) > 10.0);
        assert!(pulled(190.0, 100.0, 64.0, 0.5) < 190.0);
    }

    #[test]
    fn colours_fade_to_transparent_white() {
        assert_eq!(fade_colour(0xFFFFFFFF, 0x00FFFFFF, 1.0), [255, 255, 255, 255]);
        assert_eq!(fade_colour(0xFFFFFFFF, 0x00FFFFFF, 0.0), [255, 255, 255, 0]);
        assert_eq!(fade_colour(0xFFFFAE40, 0x00FFAE40, 0.5), [255, 0xAE, 0x40, 127]);
    }

    #[test]
    fn the_scroll_eases_and_arrives() {
        let mut s = Scroll::at(0.0);
        s.seek(-100.0);
        assert_eq!(s.value(), 0.0);
        s.update(0.1);
        assert!((s.value() + 50.0).abs() < 1e-3, "half way at half the time");
        s.update(0.5);
        assert_eq!(s.value(), -100.0);
        s.seek(-100.0);
        assert_eq!(s.value(), -100.0, "no restart for the same target");
    }
}
