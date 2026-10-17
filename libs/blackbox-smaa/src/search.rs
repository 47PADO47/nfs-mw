// Ported from Scripts/SearchTex.py of SMAA.
// Copyright (C) 2013 Jorge Jimenez (jorge@iryoku.com)
// Copyright (C) 2013 Jose I. Echevarria (joseignacioechevarria@gmail.com)
// Copyright (C) 2013 Belen Masia (bmasia@unizar.es)
// Copyright (C) 2013 Fernando Navarro (fernandn@microsoft.com)
// Copyright (C) 2013 Diego Gutierrez (diegog@unizar.es)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software. As clarification, there is no
// requirement that the copyright notice and permission be included in binary
// distributions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
// FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
// IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// Ported from SMAA (https://github.com/iryoku/smaa). Upstream notice is also
// kept in LICENSE-SMAA.
//
//! SMAA's `SearchTex` lookup texture, generated in code.
//!
//! Ported from `Scripts/SearchTex.py` in <https://github.com/iryoku/smaa>.
//! Copyright (C) 2013 Jorge Jimenez, Jose I. Echevarria, Belen Masia,
//! Fernando Navarro, Diego Gutierrez. Licensed under the MIT licence, see
//! `LICENSE-SMAA`.
//!
//! The texture is R8, 64 x 16. It holds, for each line-search end state, how
//! many pixels the last search step advances (`0`, `127` or `254`).
//!
//! The script builds a 66 x 33 image: columns `0..33` hold the left-search
//! values and columns `33..66` the right-search values, both indexed by the
//! two edge-state keys `0..=32`. It then crops rows `17..33` and flips them
//! vertically, leaving the 64 x 16 texture that the shader reads.

/// Texture size in texels, `(width, height)`.
pub const SEARCH_SIZE: (u32, u32) = (64, 16);

/// Number of columns of the uncropped image that belong to the left search.
const LEFT_COLUMNS: usize = 33;

/// Generates the `SearchTex` as R8 bytes, row-major, `64 * 16` long.
///
/// Deterministic and bit-identical to the texture shipped in the `smaa` crate.
pub fn search_texture() -> Vec<u8> {
    let (width, height) = (SEARCH_SIZE.0 as usize, SEARCH_SIZE.1 as usize);
    let mut data = vec![0u8; width * height];
    for row in 0..height {
        // The script crops rows 17..33 and flips them, so output row `row`
        // comes from source row `32 - row`.
        let y = 32 - row;
        for col in 0..width {
            data[row * width + col] = match col {
                0..LEFT_COLUMNS => raw_pixel(col, y, false),
                _ => raw_pixel(col - LEFT_COLUMNS, y, true),
            };
        }
    }
    data
}

/// Value of one pixel of the uncropped image, for the left or right search.
fn raw_pixel(x: usize, y: usize, right: bool) -> u8 {
    let Some(column_edges) = edges_for_key(x) else {
        return 0;
    };
    let Some(row_edges) = edges_for_key(y) else {
        return 0;
    };
    let steps = match right {
        false => delta_left(column_edges, row_edges),
        true => delta_right(column_edges, row_edges),
    };
    // The script scales by 127 to use the full dynamic range.
    127 * steps as u8
}

/// Edge bits `[e0, e1, e2, e3]` whose bilinear sample is at key `key`.
///
/// The script samples at `0.03125 * key`. Each edge bit contributes a fixed
/// weight to the key (1, 3, 7 and 21 in units of 1/32), so the 16 combinations
/// map to 16 distinct keys. Keys that are not produced by any combination have
/// no edges, and the script leaves their pixels black.
fn edges_for_key(key: usize) -> Option<[usize; 4]> {
    (0..16usize).find_map(|bits| {
        let edges = [bits & 1, (bits >> 1) & 1, (bits >> 2) & 1, (bits >> 3) & 1];
        let weight = edges[0] + 3 * edges[1] + 7 * edges[2] + 21 * edges[3];
        (weight == key).then_some(edges)
    })
}

/// Extra pixels to advance in the last left-search step (`deltaLeft`).
fn delta_left(left: [usize; 4], top: [usize; 4]) -> usize {
    let mut d = 0;
    if top[3] == 1 {
        d += 1;
    }
    if d == 1 && top[2] == 1 && left[1] != 1 && left[3] != 1 {
        d += 1;
    }
    d
}

/// Extra pixels to advance in the last right-search step (`deltaRight`).
fn delta_right(left: [usize; 4], top: [usize; 4]) -> usize {
    let mut d = 0;
    if top[3] == 1 && left[1] != 1 && left[3] != 1 {
        d += 1;
    }
    if d == 1 && top[2] == 1 && left[0] != 1 && left[2] != 1 {
        d += 1;
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FNV-1a (64-bit) over `bytes`.
    fn fnv1a(bytes: &[u8]) -> u64 {
        bytes
            .iter()
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, &byte| (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3))
    }

    /// FNV-1a of the reference `SEARCHTEX_BYTES` shipped in smaa 0.21.0
    /// (`third_party/smaa/Textures/SearchTex.rs`).
    const REFERENCE_FNV1A: u64 = 0x21c1_fcf0_aa63_1065;

    #[test]
    fn size_is_64_by_16_r8() {
        assert_eq!(SEARCH_SIZE, (64, 16));
        assert_eq!(search_texture().len(), 64 * 16);
    }

    #[test]
    fn matches_reference_texture() {
        assert_eq!(fnv1a(&search_texture()), REFERENCE_FNV1A);
    }

    #[test]
    fn top_row_left_search_values() {
        // Output row 0 is source row 32, where the top edge state is [1, 1, 1, 1].
        let row = &search_texture()[..64];
        // Key 0 (no edges) gives 0. Key 1 gives 254 (two steps). Key 3 gives 127.
        // Key 4 gives 127. Key 7 gives 254. Keys 2, 5 and 6 have no state, so 0.
        assert_eq!(row[0], 254);
        assert_eq!(row[1], 254);
        assert_eq!(row[2], 0);
        assert_eq!(row[3], 127);
        assert_eq!(row[4], 127);
        assert_eq!(row[5], 0);
        assert_eq!(row[6], 0);
        assert_eq!(row[7], 254);
    }

    #[test]
    fn top_row_right_search_values() {
        let row = &search_texture()[..64];
        // Column 33 is right-search key 0 with top edges [1, 1, 1, 1]: 254.
        // Column 34 is key 1 = [1, 0, 0, 0]: only one step, so 127.
        assert_eq!(row[33], 254);
        assert_eq!(row[34], 127);
    }

    #[test]
    fn bottom_source_row_without_edges_is_zero() {
        // Output row 15 is source row 17. Key 17 is not a valid edge state,
        // so the whole row is black.
        let row = &search_texture()[15 * 64..16 * 64];
        assert!(row.iter().all(|&value| value == 0));
    }

    #[test]
    fn only_three_quantised_values_occur() {
        let data = search_texture();
        assert!(data.iter().all(|&value| matches!(value, 0 | 127 | 254)));
    }
}
