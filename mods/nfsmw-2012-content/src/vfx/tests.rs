use super::*;

fn spot(effect: u64, group: u32, bit: u32) -> Spot {
    Spot { effect, group, bit }
}

#[test]
fn the_three_moments_are_the_effects_of_the_group_that_has_a_pop() {
    let spots = [
        spot(1, 0x21833, START_BIT),
        spot(2, 0x21833, POP_BIT),
        spot(3, 0x1401fc, BURN_BIT),
        spot(4, 0x21833, BURN_BIT),
        spot(5, 0x21833, 8),
    ];
    assert_eq!(
        roles(&spots),
        Some(Roles { start: 1, burn: 4, pop: 2 }),
        "the burn of the other group is not the pipes'"
    );
}

#[test]
fn a_car_without_all_three_has_no_roles() {
    assert_eq!(roles(&[spot(1, 7, START_BIT), spot(2, 7, POP_BIT)]), None, "no burn");
    assert_eq!(roles(&[spot(1, 7, START_BIT), spot(3, 7, BURN_BIT)]), None, "no pop: the group is not known");
    assert_eq!(roles(&[]), None, "an electric car has none");
}

fn words(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

#[test]
fn an_authored_colour_is_found_and_round_constants_are_not() {
    // (1, 0.25, 1, 1) is a round constant, then the glow colour.
    let block = words(&[0.0, 1.0, 0.25, 1.0, 1.0, 0.030_713_4, 0.520_995, 0.863_157, 1.0]);
    assert_eq!(colour_constant(&block, 0..block.len()), Some([0.030_713_4, 0.520_995, 0.863_157]));
    let rounds = words(&[1.0, 0.25, 1.0, 1.0, 0.5, 0.5, 0.5, 1.0]);
    assert_eq!(colour_constant(&rounds, 0..rounds.len()), None);
    let not_opaque = words(&[0.03, 0.52, 0.86, 0.5]);
    assert_eq!(colour_constant(&not_opaque, 0..16), None);
}

#[test]
fn a_range_that_starts_off_the_float_grid_is_read_on_it() {
    let mut block = vec![0xAA; 4];
    block.extend(words(&[0.030_713_4, 0.520_995, 0.863_157, 1.0]));
    assert!(colour_constant(&block, 1..block.len()).is_some(), "the colour is at 4, the next float boundary of 1");
    let mut shifted = vec![0xAA; 3];
    shifted.extend(words(&[0.030_713_4, 0.520_995, 0.863_157, 1.0]));
    assert_eq!(
        colour_constant(&shifted, 3..shifted.len()),
        None,
        "data at 3 is not on the grid, so it is not a float4"
    );
}

fn texture(format: PixelFormat, width: u32, height: u32, data: Vec<u8>) -> Texture {
    Texture { width, height, depth: 1, array_size: 1, mip_levels: 1, dxgi_format: 0, format, data }
}

#[test]
fn plain_and_swapped_pixels_decode_to_rgba() {
    let rgba = decode(&texture(PixelFormat::Rgba8, 1, 2, vec![1, 2, 3, 4, 5, 6, 7, 8])).unwrap();
    assert_eq!((rgba.width, rgba.height, rgba.rgba), (1, 2, vec![1, 2, 3, 4, 5, 6, 7, 8]));
    let bgra = decode(&texture(PixelFormat::Bgra8, 1, 1, vec![3, 2, 1, 9])).unwrap();
    assert_eq!(bgra.rgba, vec![1, 2, 3, 9]);
    assert!(decode(&texture(PixelFormat::R8, 1, 1, vec![0])).is_err());
}

#[test]
fn a_block_compressed_picture_decodes_through_the_shared_decoder() {
    // One BC1 block: both end colours are pure red (0xF800), every index 0.
    let block = vec![0x00, 0xF8, 0x00, 0xF8, 0, 0, 0, 0];
    let picture = decode(&texture(PixelFormat::Bc1, 4, 4, block)).unwrap();
    assert_eq!((picture.width, picture.height, picture.rgba.len()), (4, 4, 64));
    assert_eq!(&picture.rgba[..4], &[255, 0, 0, 255]);
}

#[test]
fn a_png_round_trips_as_an_rgba_picture() {
    let picture = Picture { width: 2, height: 1, rgba: vec![10, 20, 30, 40, 50, 60, 70, 80] };
    let bytes = png_bytes(&picture).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info().unwrap();
    let mut out = vec![0; reader.output_buffer_size().unwrap()];
    let frame = reader.next_frame(&mut out).unwrap();
    assert_eq!((frame.width, frame.height, frame.color_type), (2, 1, png::ColorType::Rgba));
    assert_eq!(&out[..8], &picture.rgba[..]);
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_real_install_names_a_start_a_burn_and_a_pop_effect_made_of_the_pictures_seen() {
    let root = std::path::PathBuf::from(std::env::var_os("MW2012_GAME_DIR").expect("MW2012_GAME_DIR"));
    let car = std::fs::read(root.join("VEHICLES/VEH_1085007_HI.BNDL")).unwrap();
    let found = spots(&Bundle::parse(&car).unwrap());
    let roles = roles(&found).expect("the Porsche has the three effects");
    let effects = std::fs::read(root.join(GLOBAL_EFFECTS)).unwrap();
    let effects = Bundle::parse(&effects).unwrap();
    for (name, id) in [("start", roles.start), ("burn", roles.burn), ("pop", roles.pop)] {
        let list = emitters(&effects, id).unwrap();
        let shapes: Vec<String> = list
            .iter()
            .map(|e| format!("{}x{} flipbook={} colour={:?}", e.picture.width, e.picture.height, e.flipbook, e.colour))
            .collect();
        println!("{name} {id:#x}: {shapes:?}");
    }
    let burn = emitters(&effects, roles.burn).unwrap();
    assert_eq!(burn.len(), 1);
    let colour = burn[0].colour.expect("the glow colour constant");
    assert!((colour[2] - 0.863).abs() < 0.01 && (colour[1] - 0.521).abs() < 0.01 && colour[0] < 0.05, "{colour:?}");
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn every_car_that_has_the_three_effects_names_the_same_ones() {
    let root = std::path::PathBuf::from(std::env::var_os("MW2012_GAME_DIR").expect("MW2012_GAME_DIR"));
    let source = crate::source::Source::open(&root).unwrap();
    let (mut with, mut without, mut seen) = (0, Vec::new(), std::collections::BTreeSet::new());
    for file in source.cars() {
        let bytes = source.bundle_bytes(file).unwrap();
        match roles(&spots(&Bundle::parse(&bytes).unwrap())) {
            Some(found) => {
                with += 1;
                seen.insert((found.start, found.burn, found.pop));
            }
            None => without.push(file.number),
        }
    }
    println!("{with} cars have the three effects; without: {without:?}; distinct sets: {seen:?}");
    assert!(with > 80, "{with}");
    assert_eq!(seen.len(), 1, "{seen:?}");
}
