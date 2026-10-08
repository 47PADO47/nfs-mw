use super::*;

type GlyphRow = (u16, u8, u8, u16, u16, i8, i8, i8, u8, u16, i16);

/// A font chunk payload with the given glyphs `(unicode, width, height, u, v, advance_y, offset_x, offset_y,
/// kern count, kern index, advance_x)` and kerning entries `(previous char, kern)`.
fn font_bytes(name: &str, glyphs: &[GlyphRow], kerns: &[(u16, i8)]) -> Vec<u8> {
    let mut out = vec![0u8; 0x200];
    out[..name.len()].copy_from_slice(name.as_bytes());
    out[0x100..0x100 + name.len()].copy_from_slice(name.as_bytes());
    let glyph_table = 0x80usize;
    let kern_table = glyph_table + glyphs.len() * 16;
    let mut f = vec![0u8; kern_table + 4 + kerns.len() * 4];
    f[..4].copy_from_slice(b"FNTF");
    let total = f.len() as u32;
    f[4..8].copy_from_slice(&total.to_le_bytes());
    f[8..10].copy_from_slice(&414u16.to_le_bytes());
    f[10..12].copy_from_slice(&(glyphs.len() as u16).to_le_bytes());
    f[12..16].copy_from_slice(&0x40009i32.to_le_bytes());
    f[18] = 14;
    f[19] = 4;
    f[20..24].copy_from_slice(&(glyph_table as u32).to_le_bytes());
    f[24..28].copy_from_slice(&(kern_table as u32).to_le_bytes());
    for (i, g) in glyphs.iter().enumerate() {
        let at = glyph_table + i * 16;
        f[at..at + 2].copy_from_slice(&g.0.to_le_bytes());
        f[at + 2] = g.1;
        f[at + 3] = g.2;
        f[at + 4..at + 6].copy_from_slice(&g.3.to_le_bytes());
        f[at + 6..at + 8].copy_from_slice(&g.4.to_le_bytes());
        f[at + 8] = g.5 as u8;
        f[at + 9] = g.6 as u8;
        f[at + 10] = g.7 as u8;
        f[at + 11] = g.8;
        f[at + 12..at + 14].copy_from_slice(&g.9.to_le_bytes());
        f[at + 14..at + 16].copy_from_slice(&g.10.to_le_bytes());
    }
    f[kern_table..kern_table + 4].copy_from_slice(&(kerns.len() as u32).to_le_bytes());
    for (i, k) in kerns.iter().enumerate() {
        let at = kern_table + 4 + i * 4;
        f[at..at + 2].copy_from_slice(&k.0.to_le_bytes());
        f[at + 2] = k.1 as u8;
    }
    out.extend(f);
    out
}

fn sample() -> Font {
    // 'A' (65) and 'B' (66) and space (32); B kerns -2 after A.
    let glyphs: &[GlyphRow] = &[
        (32, 1, 1, 0, 0, 0, 0, 0, 0, 0, 6),
        (65, 10, 14, 20, 40, 0, 1, 2, 0, 0, 12),
        (66, 8, 14, 40, 40, 0, 0, 2, 1, 0, 10),
    ];
    Font::parse(&font_bytes("font_test", glyphs, &[(65, -2)])).unwrap()
}

#[test]
fn parses_names_glyphs_and_kerning() {
    let f = sample();
    assert_eq!(f.name, "font_test");
    assert_eq!(f.texture_name, "font_test");
    assert_eq!(f.hash, crate::fe_hash_upper("FONT_TEST"));
    assert_eq!((f.version, f.ascent, f.descent), (414, 14, 4));
    assert_eq!(f.glyphs().len(), 3);
    let a = f.glyph(65).unwrap();
    assert_eq!((a.width, a.height, a.u, a.v, a.advance_x, a.offset_x, a.offset_y), (10, 14, 20, 40, 12, 1, 2));
    assert!(f.glyph(70).is_none());
    let b = f.glyph(66).unwrap();
    assert_eq!(f.kern(b, 65), -2);
    assert_eq!(f.kern(b, 66), 0);
    assert_eq!(f.height(), 18.0);
}

#[test]
fn rejects_short_or_wrong_data() {
    assert!(Font::parse(&[0; 10]).is_err());
    let mut bad = font_bytes("x", &[], &[]);
    bad[0x200] = b'X';
    assert!(Font::parse(&bad).is_err());
}

#[test]
fn lays_out_left_aligned_text_with_kerning() {
    let f = sample();
    let l = f.layout("AB", TextStyle::default(), (256, 128));
    assert_eq!(l.quads.len(), 2);
    let a = l.quads[0];
    assert_eq!((a.x0, a.y0, a.x1, a.y1), (1.0, 2.0, 11.0, 16.0));
    assert!((a.u0 - 20.0 / 256.0).abs() < 1e-6 && (a.u1 - 31.0 / 256.0).abs() < 1e-6);
    assert!((a.v0 - 40.0 / 128.0).abs() < 1e-6 && (a.v1 - 54.0 / 128.0).abs() < 1e-6);
    // B starts after A advance (12) plus the kerning (-2).
    let b = l.quads[1];
    assert_eq!(b.x0, 12.0 - 2.0);
    // Width is the sum of the advances: 12 + (10 - 2).
    assert_eq!(l.width, 20.0);
}

#[test]
fn justification_moves_the_origin() {
    let f = sample();
    let centred = f.layout("AA", TextStyle { justification: 1, ..Default::default() }, (256, 128));
    let right = f.layout("AA", TextStyle { justification: 2, ..Default::default() }, (256, 128));
    let left = f.layout("AA", TextStyle::default(), (256, 128));
    assert_eq!(centred.quads[0].x0, left.quads[0].x0 - 12.0);
    assert_eq!(right.quads[0].x0, left.quads[0].x0 - 24.0);
    let mid = f.layout("A", TextStyle { justification: 4, ..Default::default() }, (256, 128));
    assert_eq!(mid.quads[0].y0, left.quads[0].y0 - 9.0);
}

#[test]
fn newlines_start_a_new_line() {
    let f = sample();
    let l = f.layout("A^A", TextStyle { leading: 3, ..Default::default() }, (256, 128));
    assert_eq!(l.quads.len(), 2);
    assert_eq!(l.quads[1].y0 - l.quads[0].y0, 18.0 + 3.0);
    assert_eq!(l.quads[1].x0, l.quads[0].x0);
    assert_eq!(l.height, 18.0 + 3.0 + 18.0);
}

#[test]
fn unknown_characters_and_icons_draw_nothing() {
    let f = sample();
    assert!(f.layout("zzz", TextStyle::default(), (256, 128)).quads.is_empty());
    assert_eq!(f.layout("A$ICON$A", TextStyle::default(), (256, 128)).quads.len(), 2);
    assert!(f.layout("", TextStyle::default(), (256, 128)).quads.is_empty());
}

#[test]
fn word_wrap_breaks_at_a_space() {
    let f = sample();
    let wrapped = f.layout("AA AA", TextStyle { justification: 0x10, max_width: 30, leading: 0 }, (256, 128));
    let ys: Vec<f32> = wrapped.quads.iter().map(|q| q.y0).collect();
    assert_eq!(wrapped.quads.len(), 5, "the space draws a quad too");
    assert!(ys[3] > ys[1], "second word wrapped: {ys:?}");
}
