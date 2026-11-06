//! The pursuit breakers' icons on the minimap. In a chase the game shows one icon at every track position marker named
//! `IconMarker`; a breaker that fires takes every icon within 20 m of it away until its section is reset. Spec:
//! `docs/specs/pursuit-breakers.md` ("Map icons").

use std::collections::BTreeMap;

use glam::Vec3;

use super::space;
use crate::hud::{MapBlip, PURSUIT_BREAKER};

/// A fired breaker removes the icons within this many metres of it.
pub const HIDE_RADIUS: f32 = 20.0;

/// A breaker that fired: where (physics space) and the streaming section it belongs to, when it is known.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Fired {
    pub position: Vec3,
    pub section: Option<usize>,
}

/// The icon places and which of them a fired breaker hides.
#[derive(Default)]
pub(super) struct BreakerIcons {
    /// Physics-space places of the track's icon markers; empty until the track's markers are read.
    places: Vec<Vec3>,
    /// Hidden icons by index, with the section of the breaker that hid them: a reset of that section shows them again.
    hidden: BTreeMap<usize, Option<usize>>,
    placed: bool,
}

impl BreakerIcons {
    /// Whether the icon places were given.
    pub fn is_placed(&self) -> bool {
        self.placed
    }

    /// The icon markers' places, render space (the track's own records).
    pub fn place(&mut self, markers: impl IntoIterator<Item = Vec3>) {
        self.places = markers.into_iter().map(|at| Vec3::from_array(space::to_physics(at))).collect();
        self.hidden.clear();
        self.placed = true;
    }

    /// One step: breakers fired at these places, and these streaming sections were reset.
    pub fn step(&mut self, fired: &[Fired], restored: &[usize]) {
        self.hidden.retain(|_, section| !section.is_some_and(|section| restored.contains(&section)));
        self.fired(fired);
    }

    /// Breakers opened zones here: the icons within [`HIDE_RADIUS`] go.
    pub fn fired(&mut self, fired: &[Fired]) {
        for (index, icon) in self.places.iter().enumerate() {
            if let Some(stop) = fired.iter().find(|stop| stop.position.distance(*icon) <= HIDE_RADIUS) {
                self.hidden.insert(index, stop.section);
            }
        }
    }

    fn shown(&self) -> impl Iterator<Item = Vec3> + '_ {
        self.places.iter().enumerate().filter(|(index, _)| !self.hidden.contains_key(index)).map(|(_, at)| *at)
    }

    /// The icons for the minimap, while a chase is on. Nearest first is the style's business.
    pub fn blips(&self, pursuing: bool, out: &mut Vec<MapBlip>) {
        if !pursuing {
            return;
        }
        for icon in self.shown() {
            let render = space::to_render(icon.to_array());
            out.push(MapBlip { style: PURSUIT_BREAKER, position: [render.x, render.y], active: false, heading: None });
        }
    }

    /// The icons within `radius` metres of `place`, nearest first and at most `count`, both in render space, for the
    /// Custom HUD's world markers (`hud_marks.rs`). None outside a chase, as on the minimap.
    pub fn near(&self, pursuing: bool, place: Vec3, radius: f32, count: usize) -> Vec<Vec3> {
        if !pursuing {
            return Vec::new();
        }
        let mut near: Vec<Vec3> = self
            .shown()
            .map(|icon| space::to_render(icon.to_array()))
            .filter(|at| at.distance(place) <= radius)
            .collect();
        near.sort_by(|a, b| a.distance(place).total_cmp(&b.distance(place)));
        near.truncate(count);
        near
    }

    /// How many icons stand within `radius` metres of `place` (physics space).
    #[cfg(test)]
    pub fn shown_near(&self, place: Vec3, radius: f32) -> usize {
        self.shown().filter(|p| p.distance(place) <= radius).count()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.shown().count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Icons at these physics-space places.
    fn icons(places: &[Vec3]) -> BreakerIcons {
        let mut icons = BreakerIcons::default();
        icons.place(places.iter().map(|at| space::to_render(at.to_array())));
        icons
    }

    fn fired(position: Vec3, section: usize) -> Fired {
        Fired { position, section: Some(section) }
    }

    #[test]
    fn icons_show_only_in_a_chase_at_the_marker() {
        let icons = icons(&[Vec3::new(-100.0, 5.0, 300.0)]);
        let mut out = Vec::new();
        icons.blips(false, &mut out);
        assert!(out.is_empty(), "no icons outside a chase");
        icons.blips(true, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].style, PURSUIT_BREAKER);
        assert_eq!(out[0].position, [300.0, 100.0], "render space: forward is physics z, left is minus x");
    }

    #[test]
    fn the_icons_near_the_car_are_in_render_space_nearest_first_and_only_in_a_chase() {
        // Physics (0, 0, 100) is render (100, 0, 0); physics (-50, 0, 0) is render (0, 50, 0).
        let icons = icons(&[Vec3::new(0.0, 0.0, 100.0), Vec3::new(-50.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 900.0)]);
        assert!(icons.near(false, Vec3::ZERO, 150.0, 4).is_empty(), "no breakers outside a chase");
        let near = icons.near(true, Vec3::ZERO, 150.0, 4);
        assert_eq!(near, vec![Vec3::new(0.0, 50.0, 0.0), Vec3::new(100.0, 0.0, 0.0)]);
        assert_eq!(icons.near(true, Vec3::ZERO, 150.0, 1), vec![Vec3::new(0.0, 50.0, 0.0)], "at most `count`");
    }

    #[test]
    fn a_fired_breaker_takes_every_icon_within_twenty_metres_with_it() {
        let mut icons = icons(&[
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 19.0),
            Vec3::new(0.0, 0.0, 21.0),
            Vec3::new(500.0, 0.0, 0.0),
        ]);
        icons.fired(&[fired(Vec3::ZERO, 0)]);
        assert_eq!(icons.len(), 2);
        assert_eq!(icons.shown_near(Vec3::ZERO, HIDE_RADIUS), 0);
    }

    #[test]
    fn a_section_reset_restores_only_the_icons_its_breakers_hid() {
        let mut icons = icons(&[Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0)]);
        icons.step(&[fired(Vec3::ZERO, 0), fired(Vec3::new(100.0, 0.0, 0.0), 1)], &[]);
        assert_eq!(icons.len(), 0);
        icons.step(&[], &[1]);
        assert_eq!(icons.len(), 1);
        assert_eq!(icons.shown_near(Vec3::new(100.0, 0.0, 0.0), 1.0), 1, "section 1's icon is back");
        icons.step(&[], &[2]);
        assert_eq!(icons.shown_near(Vec3::ZERO, 1.0), 0, "section 0's icon stays hidden");
    }

    #[test]
    fn placing_the_markers_again_shows_every_icon() {
        let mut icons = icons(&[Vec3::ZERO]);
        icons.fired(&[Fired { position: Vec3::ZERO, section: None }]);
        assert_eq!(icons.len(), 0);
        icons.place([Vec3::ZERO]);
        assert_eq!(icons.len(), 1);
    }
}
