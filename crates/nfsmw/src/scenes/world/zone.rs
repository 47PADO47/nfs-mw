//! The camera's zone and the map tiles it loads and draws
//! (`docs/specs/visible-sections.md`).

use std::collections::{HashMap, HashSet};

use blackbox_streaming::{Numbering, StreamingSection, VisibleSections};

pub struct Zones {
    visible: VisibleSections,
    /// Section number → index into the streaming index (map tiles only).
    tiles: HashMap<i16, usize>,
    current: Option<i16>,
}

impl Zones {
    pub fn new(visible: VisibleSections, sections: &[StreamingSection]) -> Self {
        let tiles = sections.iter().enumerate().filter(|(_, s)| s.is_spatial()).map(|(i, s)| (s.number, i)).collect();
        Self { visible, tiles, current: None }
    }

    /// Follow the camera at map position (x, y); true when the zone changed.
    ///
    /// The game uses the drivable section under the player. A free camera also flies over zones a
    /// car never reaches (whose lists hold almost nothing) and off the map, so only zones in the
    /// region count; elsewhere the camera keeps its zone, and it starts in the nearest one.
    pub fn update(&mut self, x: f32, y: f32) -> bool {
        let v = &self.visible;
        let here = v.drivable_at([x, y]).map(|d| d.section).filter(|&s| v.in_region(s));
        let next =
            here.or(self.current).or_else(|| v.closest_drivable([x, y], |s| v.in_region(s)).map(|(d, _)| d.section));
        let changed = next != self.current;
        self.current = next;
        changed
    }

    /// The current zone's name (`D14`), once there is one.
    pub fn name(&self) -> Option<String> {
        self.current.map(Numbering::name)
    }

    fn indices<'a>(&'a self, numbers: &'a [i16]) -> impl Iterator<Item = usize> + 'a {
        numbers.iter().filter_map(|n| self.tiles.get(n).copied())
    }

    /// Tiles to draw from the current zone: its visible list.
    pub fn drawn(&self) -> HashSet<usize> {
        self.current.map(|z| self.indices(self.visible.sections_to_draw(z)).collect()).unwrap_or_default()
    }

    /// Tiles to keep loaded for the current zone, the drawn ones first.
    pub fn wanted(&self) -> Vec<usize> {
        let Some(zone) = self.current else { return Vec::new() };
        let mut out: Vec<usize> = self.indices(self.visible.sections_to_draw(zone)).collect();
        let to_load = self.visible.sections_to_load(zone);
        let rest: Vec<usize> = self.indices(&to_load).filter(|i| !out.contains(i)).collect();
        out.extend(rest);
        out
    }
}
