//! The two files a grading profile's cubes may be (`docs/modding/effect-variants.md`, "Profile format of the grading"):
//!
//! - **`.lut`**, this game's own compact form: 8 bytes (`B`, `B`, `L`, `U`, `T`, the version 1, the edge as a 16-bit little
//!   endian number), then `edge`^3 entries of 3 bytes (red, green, blue), red varying fastest, then green, then blue. A 16
//!   cube is 12,296 bytes.
//! - **`.cube`**, the Adobe / DaVinci text format every grading tool writes: `LUT_3D_SIZE n`, then `n`^3 lines of three numbers
//!   from 0 to 1, red varying fastest. `TITLE` and comments are skipped; a 1D table or a domain other than 0 to 1 is refused.
//!
//! Both decode to [`CubeData`], which the game turns into a `blackbox_gfx::ColourCube`.

use super::ContentError;

/// The edge a cube may have (the renderer's limit, `blackbox_gfx::CUBE_SIZE_RANGE`).
pub const CUBE_EDGE: std::ops::RangeInclusive<u32> = 2..=64;
const MAGIC: [u8; 6] = *b"BBLUT\x01";
const HEADER: usize = 8;

/// A decoded cube: `size`^3 entries of `[r, g, b]` bytes, red varying fastest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CubeData {
    pub size: u32,
    pub rgb: Vec<u8>,
}

fn field(reason: &'static str) -> ContentError {
    ContentError::Field { field: "cube file", reason }
}

/// The bytes of a `.lut` file for a cube of edge `size` (the data must be `size`^3 * 3 bytes).
pub fn encode_lut(size: u32, rgb: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HEADER + rgb.len());
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&(size as u16).to_le_bytes());
    bytes.extend_from_slice(rgb);
    bytes
}

/// Decode a cube file by the extension of its name (`.cube` is the Adobe text, anything else the binary form).
pub fn decode_cube(path: &str, bytes: &[u8]) -> Result<CubeData, ContentError> {
    match path.to_ascii_lowercase().ends_with(".cube") {
        true => decode_text(bytes),
        false => decode_lut(bytes),
    }
}

fn checked(size: u32, rgb: Vec<u8>) -> Result<CubeData, ContentError> {
    if !CUBE_EDGE.contains(&size) {
        return Err(field("a cube has 2 to 64 entries a side"));
    }
    if rgb.len() != (size as usize).pow(3) * 3 {
        return Err(field("the cube holds the wrong number of entries"));
    }
    Ok(CubeData { size, rgb })
}

fn decode_lut(bytes: &[u8]) -> Result<CubeData, ContentError> {
    if bytes.len() < HEADER || bytes[..6] != MAGIC {
        return Err(field("not a .lut file (the first bytes are BBLUT and the version 1)"));
    }
    checked(u32::from(u16::from_le_bytes([bytes[6], bytes[7]])), bytes[HEADER..].to_vec())
}

fn decode_text(bytes: &[u8]) -> Result<CubeData, ContentError> {
    let text = std::str::from_utf8(bytes).map_err(|_| field("a .cube file is text"))?;
    let mut size = 0u32;
    let mut rgb: Vec<u8> = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with('#')) {
        let mut words = line.split_whitespace();
        let Some(first) = words.next() else { continue };
        match first {
            "TITLE" => {}
            "LUT_1D_SIZE" => return Err(field("a 1D table is not a cube")),
            "LUT_3D_SIZE" => {
                size = words.next().and_then(|word| word.parse().ok()).ok_or(field("LUT_3D_SIZE is a number"))?;
                if !CUBE_EDGE.contains(&size) {
                    return Err(field("a cube has 2 to 64 entries a side"));
                }
                rgb.reserve((size as usize).pow(3) * 3);
            }
            "DOMAIN_MIN" | "DOMAIN_MAX" => {
                let want = if first == "DOMAIN_MIN" { 0.0 } else { 1.0 };
                let values: Vec<f32> = words.filter_map(|word| word.parse().ok()).collect();
                if values.len() != 3 || values.iter().any(|v| (v - want).abs() > 1e-6) {
                    return Err(field("only the domain 0 to 1 is read"));
                }
            }
            _ => {
                let values: Vec<f32> = line.split_whitespace().filter_map(|word| word.parse().ok()).collect();
                if values.len() != 3 || size == 0 || values.iter().any(|v| !v.is_finite()) {
                    return Err(field("a data line is three numbers after LUT_3D_SIZE"));
                }
                rgb.extend(values.iter().map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8));
            }
        }
    }
    checked(size, rgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(size: u32) -> Vec<u8> {
        let n = size as usize;
        let mut rgb = Vec::new();
        for z in 0..n {
            for y in 0..n {
                for x in 0..n {
                    rgb.extend([x, y, z].map(|i| (i as f32 * 255.0 / (n - 1) as f32).round() as u8));
                }
            }
        }
        rgb
    }

    #[test]
    fn a_lut_file_round_trips() {
        let rgb = identity(16);
        let bytes = encode_lut(16, &rgb);
        assert_eq!(bytes.len(), 8 + 12_288);
        assert_eq!(decode_cube("GRADING/X/A.LUT", &bytes).unwrap(), CubeData { size: 16, rgb });
    }

    #[test]
    fn a_lut_file_is_checked() {
        let good = encode_lut(2, &identity(2));
        assert!(decode_cube("a.lut", &good).is_ok());
        assert!(decode_cube("a.lut", &good[..good.len() - 1]).is_err(), "short");
        assert!(decode_cube("a.lut", &[good.clone(), vec![0]].concat()).is_err(), "long");
        let mut wrong = good.clone();
        wrong[5] = 2;
        assert!(decode_cube("a.lut", &wrong).is_err(), "another version");
        assert!(decode_cube("a.lut", b"BBLUT").is_err());
        assert!(decode_cube("a.lut", &encode_lut(1, &[0, 0, 0])).is_err(), "an edge of 1");
        assert!(decode_cube("a.lut", &encode_lut(65, &vec![0; 65 * 65 * 65 * 3])).is_err(), "an edge of 65");
    }

    #[test]
    fn an_adobe_cube_file_reads() {
        let mut text =
            String::from("# made by hand\nTITLE \"test\"\nDOMAIN_MIN 0 0 0\nDOMAIN_MAX 1.0 1.0 1.0\nLUT_3D_SIZE 2\n");
        for z in 0..2 {
            for y in 0..2 {
                for x in 0..2 {
                    text.push_str(&format!("{} {} {}\n", x as f32, y as f32 * 0.5, z as f32));
                }
            }
        }
        let cube = decode_cube("look.CUBE", text.as_bytes()).unwrap();
        assert_eq!(cube.size, 2);
        assert_eq!(&cube.rgb[..6], &[0, 0, 0, 255, 0, 0], "red varies fastest");
        assert_eq!(&cube.rgb[6..9], &[0, 128, 0], "then green (0.5 is 128)");
        assert_eq!(&cube.rgb[21..], &[255, 128, 255]);
    }

    #[test]
    fn a_cube_text_that_is_not_a_cube_is_refused() {
        assert!(decode_cube("a.cube", b"LUT_1D_SIZE 4\n0 0 0\n").is_err());
        assert!(decode_cube("a.cube", b"LUT_3D_SIZE 2\n0 0 0\n").is_err(), "missing entries");
        assert!(decode_cube("a.cube", b"0 0 0\n").is_err(), "data before the size");
        assert!(decode_cube("a.cube", b"LUT_3D_SIZE 2\nDOMAIN_MAX 2 2 2\n").is_err(), "another domain");
        assert!(decode_cube("a.cube", b"LUT_3D_SIZE 99\n").is_err());
        assert!(decode_cube("a.cube", b"LUT_3D_SIZE 2\nNaN 0 0\n").is_err());
        assert!(decode_cube("a.cube", &[0xff, 0xfe]).is_err());
    }
}
