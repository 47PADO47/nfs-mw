//! Textures written and read back.

use blackbox_hash::bstring_hash;

use super::{assert_same, bytes, source};
use crate::{
    AlphaUsage, PackForm, PixelFormat, TextureSource, WriteOptions, read_texture_packs, write_tpk, write_tpk_with,
};

fn both_forms() -> [WriteOptions; 2] {
    [
        WriteOptions { form: PackForm::Streamed, filename: None },
        WriteOptions { form: PackForm::Plain, filename: Some("Global\\Pipeline\\Test.tpk".to_owned()) },
    ]
}

/// Writes `textures` in both forms, reads them back and checks every field of every texture.
fn round_trip(textures: &[TextureSource]) {
    for options in both_forms() {
        let file = write_tpk_with("TESTPACK", textures, &options).unwrap();
        let packs = read_texture_packs(&file).unwrap();
        assert_eq!(packs.len(), 1);
        let pack = &packs[0];
        assert_eq!(pack.name, "TESTPACK");
        assert_eq!(pack.version, 5);
        assert!(pack.failed.is_empty(), "{:?}", pack.failed);
        assert_eq!(pack.textures.len(), textures.len());
        let mut expected: Vec<&TextureSource> = textures.iter().collect();
        expected.sort_by_key(|s| s.hash());
        for (t, s) in pack.textures.iter().zip(expected) {
            assert_same(t, s);
        }
    }
}

#[test]
fn dxt1_4x4_with_mips_down_to_1x1() {
    let s = source("DXT1_TINY", 4, 4, PixelFormat::Dxt1, 3, 1);
    assert_eq!(s.data.len(), 24);
    round_trip(std::slice::from_ref(&s));

    let file = write_tpk("CARTEXTURES", std::slice::from_ref(&s)).unwrap();
    let t = &read_texture_packs(&file).unwrap()[0].textures[0];
    assert_eq!(t.mip(0).unwrap(), &s.data[0..8]);
    assert_eq!(t.mip(2).unwrap(), &s.data[16..24]);
    assert_eq!(t.name_hash, bstring_hash("DXT1_TINY"));
    assert_eq!(t.compression_type, 0x22);
}

#[test]
fn dxt5_with_non_square_mip_chain() {
    let mut s = source("DXT5_WIDE", 16, 8, PixelFormat::Dxt5, 5, 2);
    s.alpha_usage = AlphaUsage::Modulated;
    s.alpha_blend = 1;
    s.alpha_sorting = true;
    s.flags = 2;
    assert_eq!(s.data.len(), 128 + 32 + 16 + 16 + 16);
    round_trip(&[s]);
}

#[test]
fn argb_with_mips() {
    let s = source("ARGB", 8, 8, PixelFormat::Argb8888, 4, 3);
    assert_eq!(s.data.len(), 256 + 64 + 16 + 4);
    round_trip(&[s]);
}

#[test]
fn dxt3_that_is_not_a_power_of_two() {
    round_trip(&[source("ODD", 12, 20, PixelFormat::Dxt3, 1, 4)]);
}

#[test]
fn palettized_texture_keeps_its_palette() {
    let mut s = source("PAL", 8, 8, PixelFormat::P8, 1, 5);
    s.palette = bytes(9, 1024);
    round_trip(&[s]);
}

#[test]
fn several_textures_are_sorted_by_hash_whatever_the_input_order() {
    let textures = vec![
        source("TEX_C", 16, 16, PixelFormat::Dxt1, 5, 1),
        source("TEX_A", 8, 8, PixelFormat::Dxt5, 4, 2),
        source("TEX_D", 4, 4, PixelFormat::Argb8888, 3, 3),
        source("TEX_B", 32, 16, PixelFormat::Dxt3, 6, 4),
    ];
    round_trip(&textures);
    let mut reversed = textures.clone();
    reversed.reverse();
    assert_eq!(write_tpk("P", &textures).unwrap(), write_tpk("P", &reversed).unwrap());
}

#[test]
fn many_textures_of_odd_sizes() {
    let sizes: [(u32, u32); 7] = [(4, 4), (8, 4), (64, 64), (128, 32), (16, 16), (256, 256), (4, 16)];
    let textures: Vec<TextureSource> = (0..40u8)
        .map(|i| {
            let (w, h) = sizes[usize::from(i) % sizes.len()];
            let mips = w.min(h).ilog2() + 1;
            let format =
                [PixelFormat::Dxt1, PixelFormat::Dxt3, PixelFormat::Dxt5, PixelFormat::Argb8888][usize::from(i) % 4];
            source(&format!("T{i:02}"), w, h, format, mips, i)
        })
        .collect();
    round_trip(&textures);
}

#[test]
fn zero_textures_is_a_valid_pack() {
    for options in both_forms() {
        let file = write_tpk_with("EMPTY", &[], &options).unwrap();
        let packs = read_texture_packs(&file).unwrap();
        assert_eq!(packs.len(), 1);
        assert!(packs[0].textures.is_empty());
    }
}

#[test]
fn long_names_are_cut_in_the_record_but_hashed_whole() {
    let name = "BMWM3GTR_KIT00_HEADLIGHT_GLASS_OCCLUDED";
    let s = source(name, 8, 8, PixelFormat::Dxt1, 1, 6);
    let file = write_tpk("P", &[s]).unwrap();
    let t = &read_texture_packs(&file).unwrap()[0].textures[0];
    assert_eq!(t.name, "BMWM3GTR_KIT00_HEADLIGH");
    assert_eq!(t.name_hash, bstring_hash(name));
}

#[test]
fn a_parsed_texture_can_be_written_again() {
    let mut s = source("REWRITE_WITH_A_LONG_NAME_X", 16, 16, PixelFormat::Dxt5, 5, 7);
    s.flags = 3;
    s.alpha_usage = AlphaUsage::PunchThrough;
    let first = read_texture_packs(&write_tpk("P", &[s]).unwrap()).unwrap();
    let again: Vec<TextureSource> = first[0].textures.iter().map(TextureSource::from).collect();
    let second = read_texture_packs(&write_tpk("P", &again).unwrap()).unwrap();
    assert_eq!(first[0].textures, second[0].textures);
}

#[test]
fn pack_and_file_names_are_stored() {
    let options = WriteOptions { form: PackForm::Plain, filename: Some("Global\\Pipeline\\Car.tpk".to_owned()) };
    let file = write_tpk_with("CARTEXTURES", &[], &options).unwrap();
    let pack = &read_texture_packs(&file).unwrap()[0];
    assert_eq!(pack.name, "CARTEXTURES");
    assert_eq!(pack.filename, "Global\\Pipeline\\Car.tpk");
    let default = read_texture_packs(&write_tpk("CARTEXTURES", &[]).unwrap()).unwrap();
    assert_eq!(default[0].filename, "CARTEXTURES.tpk");
}

#[test]
fn filtered_and_ranged_reads_work_on_written_packs() {
    let textures = vec![source("ONE", 8, 8, PixelFormat::Dxt1, 4, 1), source("TWO", 8, 8, PixelFormat::Dxt5, 4, 2)];
    for options in both_forms() {
        let file = write_tpk_with("P", &textures, &options).unwrap();
        let keep = bstring_hash("TWO");
        let filtered = crate::read_texture_packs_filtered(&file, |h| h == keep).unwrap();
        assert_eq!(filtered[0].textures.len(), 1);
        assert_same(&filtered[0].textures[0], &textures[1]);
        let ranged = crate::read_texture_packs_ranged(&mut std::io::Cursor::new(&file), |h| h == keep).unwrap();
        assert_eq!(ranged, filtered);
    }
}
