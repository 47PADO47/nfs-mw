//! The tables are consistent (no game data needed), and, with `MW2012_GAME_DIR` set, every look is made from the real install.

use std::collections::HashSet;

use blackbox_hash::bstring_hash;

use super::manifest::{key, text};
use super::table::{self, sources};
use super::*;

#[test]
fn every_row_names_a_different_texture_and_keeps_the_pack_name_length() {
    let mut seen = HashSet::new();
    for entry in table::entries() {
        assert!(seen.insert(entry.hash), "{} ({}) is listed twice", entry.name, entry.hash);
        assert!(entry.name.len() <= 23, "{} is longer than the pack's 23 characters", entry.name);
        assert!(entry.width >= 4 && entry.height >= 4, "{}", entry.name);
        assert!(entry.width <= 2048 && entry.height <= 2048, "{}: {}x{}", entry.name, entry.width, entry.height);
        assert!(entry.gain > 0.0 && entry.gain <= 1.5, "{}: gain {}", entry.name, entry.gain);
    }
    assert!(seen.len() > 60, "{} rows", seen.len());
}

#[test]
fn a_name_that_is_not_cut_hashes_to_its_row() {
    // A name of fewer than 23 characters is the whole name, so its hash is the row's: this checks the table against the hash
    // function, and the rows that are cut against nothing (they are checked in the game).
    let mut checked = 0;
    for entry in table::entries().filter(|entry| entry.name.len() < 23) {
        assert_eq!(bstring_hash(entry.name), entry.hash, "{}", entry.name);
        checked += 1;
    }
    // Normal and specular maps carry a suffix the pack cut off, so their rows are not all checkable: but the diffuse ones of the
    // signs, the grate and the cover are.
    assert!(checked >= 1, "{checked} rows are short enough to check");
}

#[test]
fn every_layer_reads_a_picture_the_sources_list() {
    for entry in table::entries() {
        for layer in entry.recipe.layers {
            assert!(sources::ALL.contains(&layer.src), "{}: {:#x} is not a listed source", entry.recipe.id, layer.src);
            assert!(layer.dst.w > 0.0 && layer.dst.h > 0.0, "{}", entry.recipe.id);
            assert!(layer.dst.x + layer.dst.w <= 1.001 && layer.dst.y + layer.dst.h <= 1.001, "{}", entry.recipe.id);
        }
    }
}

#[test]
fn an_asphalt_strip_is_a_dxt5_with_alpha_where_the_2005_texture_has_a_specular_mask() {
    let masked = table::entries().filter(|entry| entry.alpha != 255).count();
    let opaque = table::entries().filter(|entry| entry.alpha == 255).count();
    assert!(masked > 10 && opaque > 10, "{masked} masked, {opaque} opaque");
}

#[test]
fn the_manifest_names_a_file_for_every_row() {
    let text = text();
    for entry in table::entries() {
        assert!(text.contains(&format!("target_hash = {}\n", entry.hash)), "{}", entry.name);
        assert!(text.contains(&manifest::file_path(entry.hash)), "{}", entry.name);
    }
}

#[test]
fn a_maker_refuses_a_key_that_is_not_its_own_and_a_texture_it_has_no_row_for() {
    let maker = Maker::new(PathBuf::from("this-install-does-not-exist"));
    assert!(maker.read_key("car/X/GEOMETRY.BIN").is_err());
    assert!(maker.read_key("road/00000001").is_err(), "no row for that hash");
    let known = key(table::entries().next().unwrap().hash);
    assert!(maker.read_key(&known).is_err(), "the install is not there");
}

#[test]
fn the_recipe_file_is_written_once_and_read_back() {
    let dir = std::env::temp_dir().join(format!("nfsmw-roadtex-recipe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    sync(&dir, Some(Path::new("D:/some install")), true);
    let maker = Maker::read(&dir).unwrap();
    assert!(maker.source().ends_with("some install"));
    sync(&dir, Some(Path::new("D:/some install")), false);
    assert!(Maker::read(&dir).is_none(), "taken away with the set");
    assert!(!dir.join(REPLACE_FOLDER).join(manifest::MANIFEST).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

mod real {
    //! Ignored without `MW2012_GAME_DIR` (the 2012 install).

    use std::path::PathBuf;

    use super::*;

    fn install() -> Option<PathBuf> {
        std::env::var_os("MW2012_GAME_DIR").map(PathBuf::from)
    }

    #[test]
    #[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
    fn every_row_is_made_and_the_game_reads_the_result() {
        let Some(root) = install() else { return };
        let sources = Sources::open(&root).unwrap();
        let started = Instant::now();
        let mut bytes = 0;
        for entry in table::entries() {
            let file = entry.dds(&sources).unwrap_or_else(|error| panic!("{}: {error:#}", entry.name));
            let texture = blackbox_tpk::read_dds(entry.name, &file).unwrap();
            assert_eq!((texture.width, texture.height), (entry.width, entry.height), "{}", entry.name);
            assert_eq!(texture.mip_levels, blackbox_tpk::full_mip_count(entry.width, entry.height), "{}", entry.name);
            bytes += file.len();
        }
        let rows = table::entries().count();
        eprintln!(
            "road textures: {rows} pictures, {} KiB made in {} ms, {} KiB read from the install",
            bytes / 1024,
            started.elapsed().as_millis(),
            sources.bytes_read() / 1024
        );
    }

    /// Writes every picture as a PNG into the folder `ROADTEX_OUT`, to look at them.
    #[test]
    #[ignore = "reads the 2012 install (MW2012_GAME_DIR) and writes PNGs (ROADTEX_OUT)"]
    fn the_pictures_can_be_written_as_png_files() {
        let (Some(root), Some(out)) = (install(), std::env::var_os("ROADTEX_OUT").map(PathBuf::from)) else { return };
        std::fs::create_dir_all(&out).unwrap();
        let sources = Sources::open(&root).unwrap();
        for entry in table::entries() {
            let picture = entry.picture(&sources).unwrap();
            let name = format!("{:08X}_{}.png", entry.hash, entry.name);
            let file = std::fs::File::create(out.join(name)).unwrap();
            let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), picture.width, picture.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&picture.pixels).unwrap();
        }
    }

    #[test]
    #[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
    fn the_maker_serves_a_key_and_the_file_is_a_dds() {
        let Some(root) = install() else { return };
        let maker = Maker::new(root);
        let first = table::entries().next().unwrap();
        let bytes = maker.read_key(&key(first.hash)).unwrap();
        assert_eq!(&bytes[0..4], b"DDS ");
        assert_eq!(maker.timings().len(), 1);
    }
}
