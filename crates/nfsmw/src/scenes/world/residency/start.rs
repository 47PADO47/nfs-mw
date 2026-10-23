//! Starting the streaming early. The loader threads can read the shared sets and the tiles around the start while the
//! rest of the world (the physics, the garage, the mods) is still being read, so [`begin`] runs on the load thread
//! right after the streaming index, and the residency adopts the streamer it started ([`Residency::adopt`]).
//!
//! [`Residency::adopt`]: super::Residency::adopt

use std::collections::HashSet;
use std::path::PathBuf;

use blackbox_streaming::{StreamingSection, VisibleSections};
use nfsmw_data::world::Streamer;

use super::MAX_IN_FLIGHT;
use crate::scenes::world::zone::Zones;

/// A streamer that is already working, and the map tiles it was asked for besides the shared sets.
pub struct Started {
    pub streamer: Streamer,
    pub requested: Vec<usize>,
}

/// Starts `threads` loaders over `stream` and asks for every shared set and for the tiles the zone at map position
/// `at` loads first, nearest first, as many as the residency has request slots. The residency takes the rest as it
/// goes; asking early costs nothing if the player starts elsewhere, because it cancels what its first zone does not
/// want.
pub fn begin(
    stream: PathBuf,
    sections: &[StreamingSection],
    visible: &VisibleSections,
    at: [f32; 2],
    threads: usize,
) -> Started {
    let streamer = Streamer::spawn(stream, sections.to_vec(), threads);
    request_shared(&streamer, sections);
    let mut zones = Zones::new(visible.clone(), sections);
    let requested = first_tiles(&mut zones, sections, at);
    for &tile in &requested {
        streamer.request(tile);
    }
    Started { streamer, requested }
}

/// Every shared set stays loaded, including the sky geometry in X0.
pub(super) fn request_shared(streamer: &Streamer, sections: &[StreamingSection]) {
    for (index, _) in sections.iter().enumerate().filter(|(_, section)| !section.is_spatial()) {
        streamer.request(index);
    }
}

/// The tiles to load first for a camera at `at`: the zone's wanted list in load order, cut to the request slots.
fn first_tiles(zones: &mut Zones, sections: &[StreamingSection], at: [f32; 2]) -> Vec<usize> {
    zones.update_many(&[at]);
    let mut wanted = zones.wanted();
    order_wanted(&mut wanted, &zones.drawn(), sections, &[at]);
    wanted.truncate(MAX_IN_FLIGHT);
    wanted
}

/// Orders `wanted` for streaming centres at `centres` (a co-op player each): the drawn tiles first, each group nearest
/// to its closest centre first.
pub(super) fn order_wanted(
    wanted: &mut [usize],
    drawn: &HashSet<usize>,
    sections: &[StreamingSection],
    centres: &[[f32; 2]],
) {
    let distance = |i: usize| centres.iter().map(|&[x, y]| sections[i].distance_to(x, y)).fold(f32::INFINITY, f32::min);
    let key = |i: usize| (!drawn.contains(&i), distance(i));
    wanted.sort_by(|&a, &b| {
        let (ka, kb) = (key(a), key(b));
        ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1))
    });
}

#[cfg(test)]
mod tests {
    use super::super::shadow_tests::fixture;
    use super::*;

    #[test]
    fn the_first_tiles_are_the_start_zones_nearest_drawn_ones() {
        let (mut residency, _) = fixture();
        let first = first_tiles(&mut residency.zones, &residency.sections, [-1.0, 0.0]);
        assert!(first.len() <= MAX_IN_FLIGHT);
        assert_eq!(first[..3], [0, 1, 2], "zone A1 draws tiles 0, 1 and 2, nearest to the camera first");
    }

    #[test]
    fn a_start_off_the_map_takes_the_nearest_zone() {
        let (mut residency, _) = fixture();
        let near = first_tiles(&mut residency.zones, &residency.sections, [5000.0, 0.0]);
        assert!(!near.is_empty(), "the nearest zone in the region still loads its tiles");
    }

    #[test]
    fn drawn_tiles_come_before_nearer_undrawn_ones() {
        let (residency, _) = fixture();
        let drawn: HashSet<usize> = [2].into();
        let mut wanted = vec![0, 1, 2];
        order_wanted(&mut wanted, &drawn, &residency.sections, &[[-1.0, 0.0]]);
        assert_eq!(wanted, [2, 0, 1]);
    }

    #[test]
    fn an_adopted_residency_counts_the_early_requests_as_loading() {
        let (donor, _) = fixture();
        let sections = donor.sections.clone();
        let visible = VisibleSections {
            lod_offset: 40,
            region: Vec::new(),
            boundaries: Vec::new(),
            drivable: Vec::new(),
            loading: Vec::new(),
        };
        // The workers open this file and wait; the requests they get read nothing the test looks at.
        let streamer = Streamer::spawn(std::env::current_exe().unwrap(), sections.clone(), 1);
        let started = Started { streamer, requested: vec![0, 1] };
        let catalog = nfsmw_data::world::PropCatalog::new(Vec::new(), &blackbox_attrib::Database::new());
        let residency = super::super::Residency::adopt(
            sections,
            visible,
            started,
            nfsmw_data::world::GlobalTextures::default(),
            None,
            catalog,
            Vec::new(),
        );
        assert_eq!(residency.counts(), (0, 2, false), "two tiles on their way, nothing ready yet");
    }
}
