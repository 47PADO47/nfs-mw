use super::*;

const TABLE: &str = r#"{
 "schema": 1, "vehicle": "bike-option-parts",
 "parts": [
  {"id": "muffler974_001", "role": "body", "export": "muffler974",
   "anchor": {"bone": null, "translation": [0, 0, 0], "rotation": [0, 0, 0, 1]}, "materials": {}},
  {"id": "muffler974_001b", "role": "body", "export": "muffler974",
   "anchor": {"bone": null, "translation": [0, 0, 0], "rotation": [0, 0, 0, 1]}, "materials": {}},
  {"id": "rim738_001_front_rim", "role": "front_wheel", "export": "f_rim738",
   "anchor": {"bone": null, "translation": [0, 0, 0], "rotation": [0, 0, 0, 1]}, "materials": {}},
  {"id": "rim738_001_back_rim", "role": "back_wheel", "export": "b_rim738",
   "anchor": {"bone": null, "translation": [0, 0, 0], "rotation": [0, 0, 0, 1]}, "materials": {}}
 ]
}"#;

#[test]
fn a_part_pack_is_a_folder_named_after_the_asset() {
    assert_eq!(pack_path("muffler974_001"), "PARTS/R6/MUFFLER974_001");
    assert_eq!(solid_name("rim738_001_front_rim"), "RIM738_001_FRONT_RIM");
}

#[test]
fn an_asset_owns_the_parts_named_after_it_and_no_other() {
    let table = PartTable::parse(TABLE).unwrap();
    assert_eq!(part_ids(&table, "muffler974_001"), ["muffler974_001"]);
    assert_eq!(part_ids(&table, "rim738_001"), ["rim738_001_front_rim", "rim738_001_back_rim"]);
    assert!(part_ids(&table, "muffler974").is_empty());
}

#[test]
fn the_textures_of_small_parts_are_small() {
    let tiny = [Kind::Header, Kind::Muffler, Kind::BarPad, Kind::Rim, Kind::Brake].map(max_texture);
    assert_eq!(tiny, [128, 256, 256, 1024, 1024]);
    // The rims' textures are half of it and the discs' a quarter: 512 and 256 pixels.
    assert_eq!(crate::bake::size_cap(crate::partfile::Kind::Rim, max_texture(Kind::Rim)), 512);
    assert_eq!(crate::bake::size_cap(crate::partfile::Kind::Brake, max_texture(Kind::Brake)), 256);
}

#[test]
fn a_part_the_table_lacks_is_a_reason() {
    let table = PartTable::parse(TABLE).unwrap();
    let dir = std::env::temp_dir().join(format!("ride6-options-pack-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let library = Library { table, exports: Exports::scan(&dir).unwrap(), tex: dir.clone() };
    let error = library.build("nothing", Kind::Muffler, Options::default()).err().unwrap();
    assert!(format!("{error:#}").contains("no part of it"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_big_parts_have_fewer_levels_and_every_part_has_a_finest() {
    for kind in [Kind::Header, Kind::Muffler, Kind::Rim, Kind::Brake, Kind::BarPad] {
        let levels = part_levels(kind);
        assert_eq!(levels[0].0, 'A', "{kind:?}");
        assert!(levels.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 < w[1].1), "{kind:?}");
    }
    assert_eq!(part_levels(Kind::Header).len(), 2);
    assert_eq!(part_levels(Kind::Brake).len(), 4);
}

#[test]
fn a_point_of_a_part_goes_through_the_rows_of_its_mount() {
    // Turned a quarter about z, then moved: x' = -y + 1, y' = x + 2, z' = z + 3.
    let rows = [0.0, -1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 2.0, 0.0, 0.0, 1.0, 3.0];
    assert_eq!(mount(rows, [1.0, 0.0, 0.0]), [1.0, 3.0, 3.0]);
    assert_eq!(mount(rows, [0.0, 2.0, 1.0]), [-1.0, 2.0, 4.0]);
    // The rows the importer writes agree with the bike's own mounting of a point.
    let mounts = crate::build_pieces::Mounts::default();
    let anchor = crate::partfile::Anchor::rigid(None, [0.1, 0.2, 0.3], [0.0, 0.0, 0.0, 1.0]);
    let matrix = mounts.matrix(crate::build_pieces::At::Body, &anchor);
    let solid_point = to_car([0.4, 0.5, 0.6]);
    let expected = mounts.to(crate::build_pieces::At::Body, anchor.point([0.4, 0.5, 0.6]));
    let got = mount(matrix, solid_point);
    assert!((0..3).all(|i| (got[i] - expected[i]).abs() < 1e-5), "{got:?} against {expected:?}");
}

/// Where the outlets of the silencers of bikes of the user's RIDE 6 export are, stock and fitted, numerically: the emitter of a
/// fitted exhaust is where its own silencer ends, not at the stock pipe (docs/modding/ride6-bike-upgrades.md, "Where the flames
/// are"). Run with `cargo test -p ride6-bikes flame_points -- --ignored --nocapture`.
#[test]
#[ignore = "needs the user's RIDE 6 export (C:/r6work/bikes and C:/r6work/bike-parts)"]
fn flame_points_of_fitted_exhausts_move_with_the_part_on_real_bikes() {
    use crate::build_pieces::{mounts_of, present};
    use crate::exhaust::stock_pipes;
    use crate::exports::Exports;
    use crate::options::Catalogue;
    use crate::options_manifest::Plan;
    use std::path::Path;

    let (bikes, parts) = (Path::new("C:/r6work/bikes"), Path::new("C:/r6work/bike-parts"));
    let catalogue = Catalogue::read(parts).expect("the catalogue");
    let library = Library::open(parts).expect("the parts");
    let wanted = ["Ducati_Panigale-V4-S-2025", "BMW_S-1000-RR-2019", "KTM_RC-390-2023", "Yamaha_YZF-R1-2024"];
    let mut checked = 0;
    for name in wanted {
        let dir = bikes.join(name);
        let Ok(text) = std::fs::read_to_string(dir.join("parts.json")) else {
            println!("{name}: not exported, skipped");
            continue;
        };
        let table = PartTable::parse(&text).expect("parts.json");
        let exports = Exports::scan(&dir.join("glb")).expect("glb");
        let mounts = mounts_of(&table, &exports).expect("the bike's frames");
        let stock = stock_pipes(&table, &exports, mounts.frame).expect("stock pipes");
        println!("{name}: stock pipes {:?}", stock.iter().map(|p| (p.side, p.at)).collect::<Vec<_>>());
        let options = catalogue.bike(&table.vehicle).expect("catalogued");
        let mut plan = Plan::of(name, options, &present(&table), mounts);
        plan.set_pipes(|stem, number, matrix| library.silencer_pipes(stem, number, matrix).unwrap_or_default());
        let mut moved = 0;
        for part in plan.parts.iter().filter(|part| part.id.starts_with("exhaust.muffler")) {
            assert!(!part.pipes.is_empty(), "{name}: {} has no outlet", part.id);
            for pipe in &part.pipes {
                let nearest = stock
                    .iter()
                    .map(|s| (0..3).map(|i| (s.at[i] - pipe.at[i]).powi(2)).sum::<f32>().sqrt())
                    .fold(f32::MAX, f32::min);
                println!(
                    "  {} -> {:?} at {:?} dir {:?} ({nearest:.2} m from the nearest stock pipe)",
                    part.id, pipe.side, pipe.at, pipe.dir
                );
                assert!(pipe.at.iter().all(|c| c.is_finite() && c.abs() < 5.0), "{name}: {:?}", pipe.at);
                moved += usize::from(nearest > 0.10);
            }
        }
        assert!(moved > 0, "{name}: no option moves the flame point by 10 cm");
        checked += 1;
    }
    assert!(checked > 0, "none of the bikes is in the export");
}
