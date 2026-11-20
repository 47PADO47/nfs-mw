//! The cube block reading on authored bytes; the install's `ENVIRONMENT.BNDL` is read by the ignored test below
//! (`CHAMELEON_GAME_DIR` or `MW2012_GAME_DIR`).

use chameleon_bundle::Bundle;

use super::*;

/// A cube block of edge `size`: the 16-byte header and entries `f(x, y, z)` as red, green, blue, stored blue first.
fn block(size: usize, f: impl Fn(usize, usize, usize) -> [u8; 3]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend((size as u32).to_le_bytes());
    bytes.extend((size as u32).to_le_bytes());
    bytes.extend([0u8; 8]);
    for z in 0..size {
        for y in 0..size {
            for x in 0..size {
                let [r, g, b] = f(x, y, z);
                bytes.extend([b, g, r]);
            }
        }
    }
    bytes
}

#[test]
fn a_cube_block_reads_as_red_green_blue_with_red_fastest() {
    // An identity: input x is red, y green, z blue, and the stored entry is (blue, green, red).
    let identity = block(16, |x, y, z| [(x * 17) as u8, (y * 17) as u8, (z * 17) as u8]);
    assert_eq!(identity.len(), 12_304);
    let cube = cube_from_block(&identity).unwrap();
    assert_eq!(cube.size, 16);
    assert_eq!(&cube.rgb[..3], &[0, 0, 0]);
    assert_eq!(&cube.rgb[3..6], &[17, 0, 0], "the second entry is the next red");
    assert_eq!(&cube.rgb[16 * 3..16 * 3 + 3], &[0, 17, 0], "sixteen on is the next green");
    assert_eq!(&cube.rgb[256 * 3..256 * 3 + 3], &[0, 0, 17], "256 on is the next blue");
    assert_eq!(&cube.rgb[4095 * 3..], &[255, 255, 255]);
}

#[test]
fn the_stored_byte_order_is_reversed_into_red_green_blue() {
    // One entry whose stored bytes are 1, 2, 3 (blue first): red 3, green 2, blue 1.
    let mut bytes = block(2, |_, _, _| [0, 0, 0]);
    bytes[16..19].copy_from_slice(&[1, 2, 3]);
    assert_eq!(&cube_from_block(&bytes).unwrap().rgb[..3], &[3, 2, 1]);
}

#[test]
fn a_block_that_is_not_a_cube_is_refused() {
    let good = block(4, |_, _, _| [9, 9, 9]);
    assert!(cube_from_block(&good).is_some());
    assert!(cube_from_block(&good[..good.len() - 1]).is_none(), "short");
    let mut other = good.clone();
    other[4] = 5;
    assert!(cube_from_block(&other).is_none(), "the two edges differ");
    assert!(cube_from_block(&block(1, |_, _, _| [0; 3])).is_none(), "an edge of 1");
    assert!(cube_from_block(&[]).is_none());
}

fn environment() -> Vec<u8> {
    let root =
        std::env::var_os("CHAMELEON_GAME_DIR").or_else(|| std::env::var_os("MW2012_GAME_DIR")).expect("a game dir");
    std::fs::read(std::path::Path::new(&root).join("HAWAII/ENVIRONMENT.BNDL")).expect("ENVIRONMENT.BNDL")
}

#[test]
#[ignore = "reads the 2012 install"]
fn the_install_has_timelines_whose_keys_name_cubes() {
    let data = environment();
    let bundle = Bundle::parse(&data).unwrap();
    let all = timelines(&bundle);
    assert_eq!(all.len(), 25, "the install has 25 timelines");
    let first = all.iter().find(|t| t.id & 0xff_ffff == 0x69CBB).expect("the first timeline");
    assert_eq!(first.keys.len(), 21);
    assert_eq!(first.keys[0].hour, 0.0);
    assert_eq!(first.keys[0].cube, 0x0100_0000_0000_3E93);
    assert_eq!(first.keys[0].wet_cube, Some(0x0100_0000_0000_3E9E), "the wet night");
    assert_eq!(first.keys[11].hour, 12.0);
    assert_eq!(first.keys[11].cube, 0x0100_0000_0004_E220);
    assert_eq!(first.keys.last().unwrap().hour, 23.0);
    assert!(first.keys.windows(2).all(|pair| pair[0].hour < pair[1].hour), "the hours rise");
    let overcast = all.iter().find(|t| t.id & 0xff_ffff == 0x9EFCA).expect("the short timeline");
    assert_eq!(overcast.keys.len(), 8);
    assert_eq!(overcast.cubes(), [0x0100_0000_0012_43FB]);
    assert_eq!(first.cubes().len(), 6);
    assert_eq!(first.keys[11].wet_cube, Some(0x0100_0000_0009_C4D4), "the wet noon");
    let wet: Vec<u64> = first.wet_cubes().into_iter().map(|id| id & 0xff_ffff).collect();
    assert_eq!(wet, [0x3E9E, 0x9C4D5, 0x69F27, 0x9C4D4, 0x124652]);
    for timeline in &all {
        for cube in timeline.cubes() {
            let rgb = cube_rgb(&bundle, cube).expect("every named cube reads");
            assert_eq!(rgb.size, 16);
        }
    }
}
