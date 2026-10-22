//! The zone each streaming centre is in (the camera; in co-op, each player) and the map tiles
//! those zones load and draw (`docs/specs/visible-sections.md`).

use std::collections::{HashMap, HashSet};

use blackbox_streaming::{Numbering, StreamingSection, VisibleSections};

pub struct Zones {
    visible: VisibleSections,
    /// Section number → index into the streaming index (map tiles only).
    tiles: HashMap<i16, usize>,
    /// The zone of each streaming centre, in centre order. One centre until `update_many` says otherwise.
    currents: Vec<Option<i16>>,
}

impl Zones {
    pub fn new(visible: VisibleSections, sections: &[StreamingSection]) -> Self {
        let tiles = sections.iter().enumerate().filter(|(_, s)| s.is_spatial()).map(|(i, s)| (s.number, i)).collect();
        Self { visible, tiles, currents: vec![None] }
    }

    /// Follow the camera at map position (x, y); true when its zone changed. Same as `update_many`
    /// with that one centre.
    #[cfg(test)]
    pub fn update(&mut self, x: f32, y: f32) -> bool {
        self.update_many(&[[x, y]])
    }

    /// Follow each streaming centre (a map position) to its zone, in centre order; true when any
    /// zone changed or the number of centres did.
    ///
    /// The game uses the drivable section under the player. A free camera also flies over zones a
    /// car never reaches (whose lists hold almost nothing) and off the map, so only zones in the
    /// region count; elsewhere a centre keeps its zone, and it starts in the nearest one.
    pub fn update_many(&mut self, centres: &[[f32; 2]]) -> bool {
        let mut changed = centres.len() != self.currents.len();
        self.currents.resize(centres.len(), None);
        for (current, &centre) in self.currents.iter_mut().zip(centres) {
            let next = zone_at(&self.visible, centre, *current);
            changed |= next != *current;
            *current = next;
        }
        changed
    }

    /// The first centre's zone name (`D14`), once there is one. Formats a new string on every call,
    /// so it is for logs.
    pub fn name(&self) -> Option<String> {
        self.currents.first().copied().flatten().map(Numbering::name)
    }

    fn indices<'a>(&'a self, numbers: &'a [i16]) -> impl Iterator<Item = usize> + 'a {
        numbers.iter().filter_map(|n| self.tiles.get(n).copied())
    }

    /// The zones the centres are in, in centre order, skipping the centres with none.
    fn current_zones(&self) -> impl Iterator<Item = i16> + '_ {
        self.currents.iter().flatten().copied()
    }

    /// Tiles a zone draws: its visible list.
    fn draw_tiles(&self, zone: i16) -> Vec<usize> {
        self.indices(self.visible.sections_to_draw(zone)).collect()
    }

    /// Tiles a zone loads: its loading section's list.
    fn load_tiles(&self, zone: i16) -> Vec<usize> {
        let to_load = self.visible.sections_to_load(zone);
        self.indices(&to_load).collect()
    }

    /// Tiles to draw from the current zones: the union of their visible lists.
    pub fn drawn(&self) -> HashSet<usize> {
        self.current_zones().flat_map(|z| self.draw_tiles(z)).collect()
    }

    /// Tiles to keep loaded for the current zones: every zone's drawn tiles first, then every zone's
    /// loaded tiles, each in centre order and listed once.
    pub fn wanted(&self) -> Vec<usize> {
        let draws = self.current_zones().map(|z| self.draw_tiles(z));
        let loads = self.current_zones().map(|z| self.load_tiles(z));
        union_in_order(draws.chain(loads))
    }
}

/// The zone a centre at `p` follows: the drivable section under it in the region, else the zone it
/// was in, else the nearest drivable section in the region.
fn zone_at(visible: &VisibleSections, p: [f32; 2], current: Option<i16>) -> Option<i16> {
    let here = visible.drivable_at(p).map(|d| d.section).filter(|&s| visible.in_region(s));
    here.or(current).or_else(|| visible.closest_drivable(p, |s| visible.in_region(s)).map(|(d, _)| d.section))
}

/// Every item of every list, in list order, each kept at its first appearance.
fn union_in_order(lists: impl IntoIterator<Item = impl IntoIterator<Item = usize>>) -> Vec<usize> {
    let mut seen = HashSet::new();
    lists.into_iter().flatten().filter(|&i| seen.insert(i)).collect()
}

#[cfg(test)]
mod tests {
    use blackbox_streaming::visible::{Boundary, DrivableSection};

    use super::*;

    #[test]
    fn one_list_keeps_its_order() {
        assert_eq!(union_in_order(vec![vec![3_usize, 1, 2]]), vec![3, 1, 2]);
    }

    #[test]
    fn two_lists_union_without_duplicates() {
        assert_eq!(union_in_order(vec![vec![5_usize, 4], vec![4, 6, 5]]), vec![5, 4, 6]);
    }

    #[test]
    fn empty_lists_give_nothing() {
        assert!(union_in_order(Vec::<Vec<usize>>::new()).is_empty());
        assert!(union_in_order(vec![Vec::<usize>::new(), Vec::new()]).is_empty());
    }

    fn section(number: i16) -> StreamingSection {
        StreamingSection {
            name: format!("tile{number}"),
            number,
            file_type: 0,
            file_offset: 0,
            size: 0,
            compressed_size: 0,
            perm_size: 0,
            priority: 0,
            center: [0.0, 0.0],
            radius: 10.0,
            checksum: 0,
        }
    }

    fn boundary(section: i16, min_x: f32, max_x: f32) -> Boundary {
        Boundary {
            section,
            panorama: false,
            bbox_min: [min_x, -1000.0],
            bbox_max: [max_x, 1000.0],
            centre: [(min_x + max_x) * 0.5, 0.0],
            points: vec![[min_x, -1000.0], [max_x, -1000.0], [max_x, 1000.0], [min_x, 1000.0]],
        }
    }

    fn visible() -> VisibleSections {
        VisibleSections {
            lod_offset: 40,
            region: vec![101, 102],
            boundaries: vec![boundary(101, -1000.0, 0.0), boundary(102, 0.0, 1000.0)],
            drivable: vec![
                DrivableSection { section: 101, visible: vec![101, 102, 103] },
                DrivableSection { section: 102, visible: vec![102] },
            ],
            loading: vec![],
        }
    }

    #[test]
    fn one_centre_matches_the_single_zone() {
        let sections: Vec<StreamingSection> = (101..=103).map(section).collect();
        let mut single = Zones::new(visible(), &sections);
        let mut many = Zones::new(visible(), &sections);
        for point in [[-500.0, 0.0], [500.0, 0.0], [5000.0, 0.0], [-500.0, 0.0]] {
            assert_eq!(single.update(point[0], point[1]), many.update_many(&[point]));
            assert_eq!(single.name(), many.name());
            assert_eq!(single.drawn(), many.drawn());
            assert_eq!(single.wanted(), many.wanted());
        }
        assert_eq!(single.name().as_deref(), Some("A1"));
    }
}
