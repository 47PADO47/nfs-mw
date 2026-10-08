use crate::YuvFrame;

fn flat(width: usize, height: usize, y: u8, u: u8, v: u8) -> YuvFrame {
    let chroma = width.div_ceil(2) * height.div_ceil(2);
    YuvFrame { width, height, y: vec![y; width * height], u: vec![u; chroma], v: vec![v; chroma] }
}

fn close(a: u8, b: u8) -> bool {
    a.abs_diff(b) <= 2
}

#[test]
fn studio_black_and_white_map_to_full_range() {
    assert_eq!(flat(4, 2, 16, 128, 128).to_rgba_vec()[..4], [0, 0, 0, 255]);
    assert_eq!(flat(4, 2, 235, 128, 128).to_rgba_vec()[..4], [255, 255, 255, 255]);
}

#[test]
fn bt601_primaries() {
    let red = flat(2, 2, 81, 90, 240).to_rgba_vec();
    assert!(close(red[0], 255) && close(red[1], 0) && close(red[2], 0), "{red:?}");
    let green = flat(2, 2, 145, 54, 34).to_rgba_vec();
    assert!(close(green[0], 0) && close(green[1], 255) && close(green[2], 0), "{green:?}");
    let blue = flat(2, 2, 41, 240, 110).to_rgba_vec();
    assert!(close(blue[0], 0) && close(blue[1], 0) && close(blue[2], 255), "{blue:?}");
}

#[test]
fn chroma_is_shared_by_two_by_two_pixels() {
    let mut f = flat(4, 4, 126, 128, 128);
    f.u[1] = 240; // the right chroma sample of the first chroma row
    let rgba = f.to_rgba_vec();
    let px = |x: usize, y: usize| &rgba[(y * 4 + x) * 4..][..3];
    assert_eq!(px(0, 0), px(1, 1));
    assert_eq!(px(2, 0), px(3, 1));
    assert_ne!(px(1, 0), px(2, 0));
    assert_eq!(px(2, 2), px(0, 0));
}

#[test]
fn odd_sizes_and_short_buffers_are_safe() {
    let f = flat(3, 3, 100, 128, 128);
    assert_eq!((f.chroma_width(), f.chroma_height(), f.rgba_len()), (2, 2, 36));
    assert_eq!(f.to_rgba_vec().len(), 36);
    let mut out = vec![7u8; 8];
    f.to_rgba(&mut out);
    assert_eq!(out, [7; 8]);
    let empty = YuvFrame::default();
    empty.to_rgba(&mut []);
}

#[cfg(feature = "vp6")]
mod decoder {
    use crate::Vp6Decoder;

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        let mut decoder = Vp6Decoder::new(64, 32).unwrap();
        assert!(decoder.decode(&[]).is_err());
        assert!(decoder.decode(&[0x00, 0x01]).is_err());
        // An inter frame (first bit set) before any key frame has no reference to predict from.
        assert!(decoder.decode(&[0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]).is_err());
    }
}
