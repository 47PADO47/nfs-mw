use super::*;

fn picture(rgba: [u8; 4]) -> Picture {
    Picture { width: 2, height: 2, rgba: rgba.repeat(4) }
}

fn set(rgba: [u8; 4]) -> MarkPictures {
    MarkPictures { slip: picture(rgba), spin: picture(rgba) }
}

#[test]
fn brown_marks_are_loose_road_and_black_ones_paved() {
    assert!(is_brown(&set([117, 79, 65, 200])));
    assert!(!is_brown(&set([0, 0, 0, 180])));
    assert!(!is_brown(&set([120, 120, 120, 255])));
    assert!(!is_brown(&set([200, 100, 10, 0])), "a picture with no visible pixel is not brown");
}

#[test]
fn the_most_named_material_wins_and_the_lowest_id_breaks_a_tie() {
    let counts: HashMap<u64, usize> = [(7, 3), (5, 3), (9, 1)].into_iter().collect();
    assert_eq!(most_named(&counts, |_| true), Some(5));
    assert_eq!(most_named(&counts, |id| id != 5), Some(7));
    assert_eq!(most_named(&counts, |_| false), None);
}

#[test]
fn the_profile_has_a_set_for_each_road_found_and_the_game_accepts_it() {
    let sets = MarkSets {
        paved: Some(set([0, 0, 0, 200])),
        paved_wet: Some(set([0, 0, 0, 100])),
        loose: Some(set([117, 79, 65, 200])),
    };
    let profile = blackbox_modding::content::parse_skid_marks_profile(&profile_text(&sets)).unwrap();
    assert_eq!(profile.sets.len(), 3);
    assert!(profile.set_for(blackbox_modding::content::MarkSurface::Paved, true).unwrap().wet);
    assert!(
        profile.sets.iter().all(|set| set.slip.as_deref().is_some_and(|path| path.starts_with("SKIDMARKS/MW2012/")))
    );
    let only_paved = MarkSets { paved: Some(set([0, 0, 0, 200])), ..Default::default() };
    assert_eq!(blackbox_modding::content::parse_skid_marks_profile(&profile_text(&only_paved)).unwrap().sets.len(), 1);
    assert!(
        blackbox_modding::content::parse_skid_marks_profile(&profile_text(&MarkSets::default())).is_err(),
        "no set, no profile"
    );
}

#[test]
fn the_files_are_written_under_the_variants_folder() {
    let out = std::env::temp_dir().join(format!("nfsmw-marks-{}", std::process::id()));
    let sets =
        MarkSets { paved: Some(set([0, 0, 0, 200])), loose: Some(set([117, 79, 65, 200])), ..Default::default() };
    let bytes = write_files(&out, &sets).unwrap();
    let folder = out.join("files").join(FOLDER);
    for name in ["PAVED_SLIP.PNG", "PAVED_SPIN.PNG", "LOOSE_SLIP.PNG", "LOOSE_SPIN.PNG", "profile.toml"] {
        assert!(folder.join(name).is_file(), "{name}");
    }
    assert!(!folder.join("PAVED_WET_SLIP.PNG").exists());
    assert!(bytes > 0);
    let _ = std::fs::remove_dir_all(&out);
}

/// The sets of the real install: a paved set and a loose one (the wet paved set's pictures equal the dry one's), read by ranges.
#[test]
#[ignore = "requires the 2012 install (MW2012_GAME_DIR)"]
fn the_real_install_has_three_sets_and_costs_little() {
    let root = std::env::var_os("NFSMW2012_DIR")
        .map_or_else(|| Path::new(r"D:\need for Speed - Most Wanted").to_owned(), Into::into);
    let source = Source::open(&root).unwrap();
    let (sets, cost) = find(&source).unwrap();
    println!("skid marks: read {} of {} bytes", cost.read, cost.total);
    if let Some(dump) = std::env::var_os("FX_DUMP") {
        write_files(Path::new(&dump), &sets).unwrap();
    }
    let (paved, loose) = (sets.paved.unwrap(), sets.loose.unwrap());
    assert!(sets.paved_wet.is_none(), "the wet set has the dry set's diffuse pictures: it is left out");
    assert!(!is_brown(&paved) && is_brown(&loose));
    assert!(paved.slip.rgba.len() >= 16 && loose.spin.width >= 16);
    assert!(cost.read < cost.total / 4, "read {} of {} bytes", cost.read, cost.total);
}
