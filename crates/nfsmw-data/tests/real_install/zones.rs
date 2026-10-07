//! The visible-section tables (`docs/formats/maps.md`, `docs/specs/visible-sections.md`):
//! record counts, zones that tile the map without overlapping, and the lists the
//! world viewer relies on.

use std::collections::HashSet;

use blackbox_streaming::Numbering;
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};

use crate::install;

fn names(sections: impl IntoIterator<Item = i16>) -> Vec<String> {
    sections.into_iter().map(Numbering::name).collect()
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn visible_sections_match_the_docs() {
    let Some(dir) = install() else { return };
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let v = &index.visible;
    let numbering = v.numbering();
    assert_eq!((v.lod_offset, v.region.len()), (40, 373));
    assert_eq!((v.boundaries.len(), v.drivable.len(), v.loading.len()), (515, 435, 39));
    assert_eq!(v.boundaries.iter().filter(|b| b.panorama).count(), 49);

    let in_stream: HashSet<i16> = index.sections.iter().map(|s| s.number).collect();
    for d in &v.drivable {
        assert!(numbering.is_drivable(d.section), "{}", Numbering::name(d.section));
        assert!(v.boundary(d.section).is_some(), "{} has no boundary", Numbering::name(d.section));
        assert!(in_stream.contains(&numbering.far_of(d.section)), "{} has no far tile", Numbering::name(d.section));
    }

    // Unreachable zones: outside the region list, with stub visible lists.
    let outside: Vec<_> = v.drivable.iter().filter(|d| !v.in_region(d.section)).collect();
    assert_eq!(outside.len(), 66);
    assert!(outside.iter().all(|d| d.visible.len() <= 20));

    // Tiles no zone lists: four panoramas the game never draws.
    let listed: HashSet<i16> = v.drivable.iter().flat_map(|d| d.visible.iter().copied()).collect();
    let unlisted = index.sections.iter().filter(|s| s.is_spatial() && !listed.contains(&s.number)).map(|s| s.number);
    assert_eq!(names(unlisted), ["A91", "A94", "C99", "O93"]);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn zones_do_not_overlap() {
    let Some(dir) = install() else { return };
    let v = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap().visible;
    let numbering = v.numbering();
    let zones: Vec<_> = v.boundaries.iter().filter(|b| numbering.is_drivable(b.section)).collect();
    let (mut inside, mut total) = (0, 0);
    for gx in (-2200..5700).step_by(25) {
        for gy in (-1600..5700).step_by(25) {
            let p = [gx as f32, gy as f32];
            let hits = zones.iter().filter(|b| b.contains(p)).count();
            assert!(hits <= 1, "{p:?} is in {hits} zones");
            inside += hits;
            total += 1;
        }
    }
    eprintln!("{inside} of {total} grid points are in a zone");
    assert!(inside * 100 > total * 70);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn downtown_zone_lists() {
    let Some(dir) = install() else { return };
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let v = &index.visible;
    let zone = v.drivable_at([2000.0, 0.0]).unwrap().section;
    assert_eq!(Numbering::name(zone), "D14");
    let drawn: Vec<i16> = v.sections_to_draw(zone).iter().copied().filter(|&s| index.by_number(s).is_some()).collect();
    assert_eq!(drawn.len(), 35);
    // The city panorama cards stand in these streets but are meant for far-away zones.
    for panorama in ["C94", "C95", "C99"] {
        assert!(!drawn.contains(&Numbering::parse(panorama).unwrap()), "{panorama} drawn from D14");
    }
    assert!(v.sections_to_load(zone).len() >= v.sections_to_draw(zone).len());
}
