//! A digest of an image: the mean colour of each cell of a 32x18 grid.
//!
//! 576 cells of three bytes are small enough to keep as a Rust constant (a hex string), yet fine enough
//! that any visible change to a scene moves some cell. Tests compare a fresh render to the stored digest
//! with a per-channel tolerance, so they catch unintended changes without storing images.

use std::fmt;

use blackbox_gfx::RgbaImage;

pub const COLUMNS: usize = 32;
pub const ROWS: usize = 18;
pub const CELLS: usize = COLUMNS * ROWS;
/// Hex characters in a digest string.
pub const HEX_LEN: usize = CELLS * 6;
/// Hex characters per line in [`Digest::to_const`].
const HEX_PER_LINE: usize = 96;

/// The mean colour of every cell, row by row, top first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digest {
    cells: Vec<[u8; 3]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DigestError {
    /// The image is smaller than the grid.
    TooSmall { width: u32, height: u32 },
    /// The string is not `HEX_LEN` hex characters.
    BadHex,
}

impl fmt::Display for DigestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooSmall { width, height } => {
                write!(f, "a {width}x{height} image has fewer pixels than the {COLUMNS}x{ROWS} digest grid")
            }
            Self::BadHex => write!(f, "a digest is {HEX_LEN} hex characters"),
        }
    }
}

impl std::error::Error for DigestError {}

/// How a render compares to an expected digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestReport {
    /// The largest per-channel difference over all cells.
    pub max_difference: u8,
    /// Cells with a channel off by more than the tolerance.
    pub cells_over: usize,
    /// The column and row of the worst cell.
    pub worst_cell: (usize, usize),
}

impl DigestReport {
    pub fn is_within(&self) -> bool {
        self.cells_over == 0
    }
}

impl fmt::Display for DigestReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} cells over the tolerance, worst {} at column {} row {}",
            self.cells_over, self.max_difference, self.worst_cell.0, self.worst_cell.1
        )
    }
}

impl Digest {
    /// The digest of `image`: each cell averages the pixels `x * width / 32 .. (x + 1) * width / 32` by
    /// the same split of the height. Alpha is ignored.
    pub fn of(image: &RgbaImage) -> Result<Self, DigestError> {
        let (width, height) = (image.width as usize, image.height as usize);
        if width < COLUMNS || height < ROWS {
            return Err(DigestError::TooSmall { width: image.width, height: image.height });
        }
        let mut cells = Vec::with_capacity(CELLS);
        for row in 0..ROWS {
            let (y0, y1) = (row * height / ROWS, (row + 1) * height / ROWS);
            for column in 0..COLUMNS {
                let (x0, x1) = (column * width / COLUMNS, (column + 1) * width / COLUMNS);
                let mut sum = [0u64; 3];
                for y in y0..y1 {
                    for x in x0..x1 {
                        let at = (y * width + x) * 4;
                        (0..3).for_each(|c| sum[c] += u64::from(image.rgba[at + c]));
                    }
                }
                let count = ((y1 - y0) * (x1 - x0)) as u64;
                cells.push(sum.map(|s| ((s + count / 2) / count) as u8));
            }
        }
        Ok(Self { cells })
    }

    pub fn cells(&self) -> &[[u8; 3]] {
        &self.cells
    }

    /// The cell at `column`, `row`.
    pub fn cell(&self, column: usize, row: usize) -> [u8; 3] {
        self.cells[row * COLUMNS + column]
    }

    /// Lower-case hex, six characters per cell.
    pub fn to_hex(&self) -> String {
        self.cells.iter().map(|c| format!("{:02x}{:02x}{:02x}", c[0], c[1], c[2])).collect()
    }

    pub fn from_hex(hex: &str) -> Result<Self, DigestError> {
        let hex = hex.trim();
        if hex.len() != HEX_LEN || !hex.is_ascii() {
            return Err(DigestError::BadHex);
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|_| DigestError::BadHex);
        let cells = (0..CELLS)
            .map(|n| Ok([byte(n * 3)?, byte(n * 3 + 1)?, byte(n * 3 + 2)?]))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { cells })
    }

    /// Compare against `expected`: a cell is over the tolerance when any channel differs by more than
    /// `tolerance`.
    pub fn compare(&self, expected: &Self, tolerance: u8) -> DigestReport {
        let mut report = DigestReport { max_difference: 0, cells_over: 0, worst_cell: (0, 0) };
        for (n, (a, b)) in self.cells.iter().zip(&expected.cells).enumerate() {
            let worst = (0..3).map(|c| a[c].abs_diff(b[c])).max().unwrap_or(0);
            if worst > tolerance {
                report.cells_over += 1;
            }
            if worst > report.max_difference {
                report.max_difference = worst;
                report.worst_cell = (n % COLUMNS, n / COLUMNS);
            }
        }
        report
    }

    /// Rust source for a `pub const NAME: &str` holding this digest, ready to paste.
    pub fn to_const(&self, name: &str) -> String {
        let hex = self.to_hex();
        let mut out = format!("pub const {name}: &str = \"\\\n");
        for line in hex.as_bytes().chunks(HEX_PER_LINE) {
            out.push_str(std::str::from_utf8(line).unwrap_or_default());
            out.push_str("\\\n");
        }
        out.push_str("\";\n");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient(width: u32, height: u32) -> RgbaImage {
        let rgba = (0..width * height)
            .flat_map(|i| [(i % width * 255 / (width - 1)) as u8, (i / width * 255 / (height - 1)) as u8, 7, 255])
            .collect();
        RgbaImage::new(width, height, rgba).unwrap()
    }

    #[test]
    fn a_flat_image_has_flat_cells() {
        let image = RgbaImage::new(64, 36, [10, 20, 30, 255].repeat(64 * 36)).unwrap();
        let digest = Digest::of(&image).unwrap();
        assert_eq!(digest.cells().len(), CELLS);
        assert!(digest.cells().iter().all(|c| *c == [10, 20, 30]));
    }

    #[test]
    fn cells_follow_the_image() {
        let digest = Digest::of(&gradient(256, 144)).unwrap();
        assert!(digest.cell(0, 0)[0] < digest.cell(31, 0)[0], "red grows to the right");
        assert!(digest.cell(0, 0)[1] < digest.cell(0, 17)[1], "green grows downwards");
        assert!(digest.cells().iter().all(|c| c[2] == 7));
    }

    #[test]
    fn an_image_that_does_not_divide_evenly_still_covers_every_pixel() {
        assert!(Digest::of(&gradient(100, 61)).is_ok());
        assert_eq!(Digest::of(&gradient(31, 18)), Err(DigestError::TooSmall { width: 31, height: 18 }));
    }

    #[test]
    fn hex_round_trips() {
        let digest = Digest::of(&gradient(64, 36)).unwrap();
        let hex = digest.to_hex();
        assert_eq!(hex.len(), HEX_LEN);
        assert_eq!(Digest::from_hex(&hex).unwrap(), digest);
        assert_eq!(Digest::from_hex(&format!("  {hex}\n")).unwrap(), digest, "surrounding space is ignored");
        assert_eq!(Digest::from_hex("abcd"), Err(DigestError::BadHex));
        assert_eq!(Digest::from_hex(&"zz".repeat(HEX_LEN / 2)), Err(DigestError::BadHex));
    }

    #[test]
    fn the_const_text_parses_back_to_the_same_digest() {
        let digest = Digest::of(&gradient(64, 36)).unwrap();
        let text = digest.to_const("EXAMPLE");
        assert!(text.starts_with("pub const EXAMPLE: &str = \"\\\n"));
        assert!(text.ends_with("\";\n"));
        // What the compiler makes of the literal: each line break plus indentation is dropped.
        let body = text.trim_start_matches("pub const EXAMPLE: &str = \"").trim_end_matches("\";\n");
        let literal: String = body.split("\\\n").collect();
        assert_eq!(Digest::from_hex(&literal).unwrap(), digest);
        assert!(text.lines().all(|l| l.len() <= 100));
    }

    #[test]
    fn comparison_reports_the_worst_cell_and_how_many_are_over() {
        let a = Digest::of(&gradient(64, 36)).unwrap();
        let mut b = a.clone();
        b.cells[5 * COLUMNS + 3][1] = b.cells[5 * COLUMNS + 3][1].wrapping_add(9);
        b.cells[0][0] = b.cells[0][0].saturating_add(1);
        let report = a.compare(&b, 2);
        assert_eq!((report.max_difference, report.cells_over, report.worst_cell), (9, 1, (3, 5)));
        assert!(!report.is_within());
        assert!(a.compare(&b, 9).is_within());
        assert!(a.compare(&a, 0).is_within());
    }
}
