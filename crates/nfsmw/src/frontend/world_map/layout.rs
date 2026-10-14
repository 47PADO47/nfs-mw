//! Where the map's picture sits in the window, in points: a square on the left, the list of blips on its right.

/// The picture's rectangle `[x, y, side, side]` in points for a window of `width` by `height`. It is as tall as the
/// window allows (with room for the title and hints) and at most 60% of the width, so the list has the rest.
pub fn picture_rect(width: f32, height: f32) -> [f32; 4] {
    let side = (height * 0.86).min(width * 0.6).max(1.0);
    [width * 0.05, (height - side) * 0.5, side, side]
}

/// A window point `(x, y)` as a view position of the picture rectangle: `(0, 0)` its top left, `(1, 1)` its bottom
/// right. Outside the rectangle the values are below 0 or above 1.
pub fn view_of(x: f32, y: f32, rect: [f32; 4]) -> [f32; 2] {
    [(x - rect[0]) / rect[2], (y - rect[1]) / rect[3]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picture_is_a_square_inside_the_window_on_the_left() {
        for (w, h) in [(1920.0, 1080.0), (1280.0, 1024.0), (800.0, 600.0)] {
            let [x, y, side, side_h] = picture_rect(w, h);
            assert_eq!(side, side_h);
            assert!(x >= 0.0 && y >= 0.0 && x + side < w * 0.7 && y + side <= h, "{w}x{h}: {:?}", [x, y, side]);
        }
    }

    #[test]
    fn a_window_point_maps_to_the_view() {
        let rect = [10.0, 20.0, 100.0, 100.0];
        assert_eq!(view_of(10.0, 20.0, rect), [0.0, 0.0]);
        assert_eq!(view_of(110.0, 120.0, rect), [1.0, 1.0]);
        assert!(view_of(5.0, 50.0, rect)[0] < 0.0, "left of the picture");
    }
}
