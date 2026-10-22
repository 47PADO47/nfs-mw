//! The pursuit breakers' icons on the minimap. In a chase the game marks the breaker structures with icons; the icon
//! of a breaker that fires, and every icon within 20 m of it, disappears. Spec: `docs/specs/pursuit-breakers.md`
//! ("Map icons").

use glam::{Vec2, Vec3};
use nfsmw_data::car::physics::PhysicsData;
use std::collections::BTreeSet;

use super::props::PropWorld;
use super::space;
use crate::hud::{MapBlip, PURSUIT_BREAKER};

/// A fired breaker removes the icons within this many metres of it.
pub const HIDE_RADIUS: f32 = 20.0;
/// The resident breakers are looked up again this often, simulated seconds.
const REFRESH: f32 = 1.0;

/// The icons of the resident breakers that have not fired.
#[derive(Default)]
pub(super) struct BreakerIcons {
    /// Physics-space positions of the breakers that show an icon.
    shown: Vec<Icon>,
    /// Hidden icons retain their section identity across streaming.
    hidden: BTreeSet<(usize, u32)>,
    age: f32,
    looked: bool,
}

#[derive(Clone, Copy)]
struct Icon {
    section: usize,
    id: u32,
    position: Vec3,
}

impl BreakerIcons {
    /// Looks the resident breakers up once a second.
    pub fn step(&mut self, dt: f32, props: &PropWorld, data: &PhysicsData, fired: &[Vec3], restored: &[usize]) {
        self.age += dt;
        self.hidden.retain(|(section, _)| !restored.contains(section));
        if self.looked && self.age < REFRESH && fired.is_empty() && restored.is_empty() {
            return;
        }
        self.age = 0.0;
        self.looked = true;
        let sites = props.breaker_sites(data, Vec2::ZERO);
        self.shown = sites
            .iter()
            .filter(|s| !s.fired)
            .filter_map(|s| Some(Icon { section: props.section_for(s.id)?, id: s.id, position: s.position }))
            .collect();
        self.fired(fired);
    }

    /// Breakers opened zones at these places (physics space): their icons and the icons near them go.
    pub fn fired(&mut self, places: &[Vec3]) {
        for icon in &self.shown {
            if places.iter().any(|p| p.distance(icon.position) <= HIDE_RADIUS) {
                self.hidden.insert((icon.section, icon.id));
            }
        }
        self.apply_hidden();
    }

    fn apply_hidden(&mut self) {
        self.shown.retain(|icon| !self.hidden.contains(&(icon.section, icon.id)));
    }

    /// The icons for the minimap, while a chase is on. Nearest first is the style's business.
    pub fn blips(&self, pursuing: bool, out: &mut Vec<MapBlip>) {
        if !pursuing {
            return;
        }
        for icon in &self.shown {
            let render = space::to_render(icon.position.to_array());
            out.push(MapBlip { style: PURSUIT_BREAKER, position: [render.x, render.y], active: false, heading: None });
        }
    }

    /// How many icons stand within `radius` metres of `place` (physics space).
    #[cfg(test)]
    pub fn shown_near(&self, place: Vec3, radius: f32) -> usize {
        self.shown.iter().filter(|p| p.position.distance(place) <= radius).count()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.shown.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icons(places: &[Vec3]) -> BreakerIcons {
        BreakerIcons {
            shown: places
                .iter()
                .enumerate()
                .map(|(id, &position)| Icon { section: 0, id: id as u32, position })
                .collect(),
            looked: true,
            ..Default::default()
        }
    }

    #[test]
    fn icons_show_only_in_a_chase_at_the_structure() {
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
    fn a_fired_breaker_takes_every_icon_within_twenty_metres_with_it() {
        let mut icons = icons(&[
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 19.0),
            Vec3::new(0.0, 0.0, 21.0),
            Vec3::new(500.0, 0.0, 0.0),
        ]);
        icons.fired(&[Vec3::new(0.0, 0.0, 0.0)]);
        assert_eq!(icons.len(), 2);
        let mut out = Vec::new();
        icons.blips(true, &mut out);
        assert!(out.iter().all(|b| b.position != [0.0, 0.0] && b.position != [19.0, 0.0]));
    }

    #[test]
    fn a_fired_place_keeps_hiding_after_the_next_look() {
        let mut icons = icons(&[Vec3::new(5.0, 0.0, 0.0), Vec3::new(50.0, 0.0, 0.0)]);
        let resident = icons.shown.clone();
        icons.fired(&[Vec3::ZERO]);
        icons.shown = resident;
        icons.apply_hidden();
        assert_eq!(icons.len(), 1);
    }

    #[test]
    fn a_section_reset_restores_only_its_own_icons() {
        let mut icons = icons(&[Vec3::ZERO, Vec3::X]);
        icons.shown[1].section = 1;
        let resident = icons.shown.clone();
        icons.fired(&[Vec3::ZERO]);
        assert_eq!(icons.len(), 0);
        icons.hidden.retain(|(section, _)| *section != 0);
        icons.shown = resident;
        icons.apply_hidden();
        assert_eq!(icons.len(), 1);
        assert_eq!(icons.shown[0].section, 0);
    }
}
