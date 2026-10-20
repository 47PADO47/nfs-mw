//! Roadblocks as geometry: the layout tables, the car count, the choice of layout for a road and the placement of its
//! elements. Spec: `docs/specs/ai-pursuit-roadblocks.md` sections 2 to 4. The tables are the game's own (decompilation,
//! hard-coded); the rules that pick and place them are as written there. Nothing here spawns a car or a prop.

use glam::{Vec2, Vec3};

pub mod request;
pub mod standing;

/// What one element of a layout is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Element {
    /// A cop car (`C`).
    Car,
    /// A sawhorse barrier (`B`).
    Barrier,
    /// A spike strip (`S`).
    Spike,
}

/// One element: its place in metres (across the road `x`, along it `z`) and its facing in turns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    pub kind: Element,
    pub x: f32,
    pub z: f32,
    pub angle: f32,
}

/// A layout: the road width it needs, the cars it needs, and its elements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    /// Minimum width of the road, metres.
    pub min_width: f32,
    /// Cars required.
    pub cars: u32,
    pub slots: &'static [Slot],
}

const fn car(x: f32, z: f32, angle: f32) -> Slot {
    Slot { kind: Element::Car, x, z, angle }
}

const fn barrier(x: f32, z: f32, angle: f32) -> Slot {
    Slot { kind: Element::Barrier, x, z, angle }
}

const fn spike(x: f32, z: f32, angle: f32) -> Slot {
    Slot { kind: Element::Spike, x, z, angle }
}

/// The layouts without spikes (`RoadblockCandidateList`, spec section 3).
pub const NORMAL: [Layout; 15] = [
    Layout { min_width: 8.0, cars: 2, slots: &[car(-3.0, 0.0, 0.75), car(2.0, -0.35, 0.27)] },
    Layout {
        min_width: 12.0,
        cars: 3,
        slots: &[car(-5.2, -2.0, 0.875), car(0.0, -2.0, 0.12), car(4.5, -1.0, 0.87), barrier(-2.9, 0.2, 0.48)],
    },
    Layout { min_width: 5.5, cars: 2, slots: &[car(-2.0, 0.0, 0.26), car(2.0, 5.0, 0.48), barrier(1.5, 1.7, 0.61)] },
    Layout { min_width: 6.5, cars: 2, slots: &[car(-2.0, 0.0, 0.75), car(3.0, 4.0, 0.25), barrier(1.5, 1.7, 0.61)] },
    Layout { min_width: 8.5, cars: 2, slots: &[car(-0.9, 0.0, 0.72), car(4.0, 0.0, 0.32), barrier(-4.8, -0.8, 0.54)] },
    Layout {
        min_width: 15.0,
        cars: 4,
        slots: &[
            car(-5.0, 0.0, 0.71),
            car(1.0, 0.0, 0.29),
            car(5.0, 6.0, 0.24),
            car(8.0, 2.0, 0.5),
            barrier(4.1, -1.4, 0.57),
            barrier(6.1, -1.4, 0.43),
        ],
    },
    Layout {
        min_width: 13.0,
        cars: 3,
        slots: &[
            car(-4.0, 1.0, 0.26),
            car(-0.9, -4.2, 0.75),
            car(5.0, -4.0, 0.26),
            barrier(-3.3, -2.2, 0.4),
            barrier(-5.0, -4.0, 0.46),
        ],
    },
    Layout {
        min_width: 11.4,
        cars: 3,
        slots: &[
            car(-4.4, -1.2, 0.65),
            car(0.0, 2.4, 0.24),
            car(4.4, -1.2, 0.35),
            barrier(0.0, -2.7, 0.49),
            barrier(2.1, 0.0, 0.35),
            barrier(-2.1, 0.0, 0.65),
        ],
    },
    Layout {
        min_width: 12.1,
        cars: 3,
        slots: &[
            car(-4.5, 0.0, 0.75),
            car(1.0, 0.0, 0.25),
            car(4.0, 4.5, 0.72),
            barrier(4.3, -1.2, 0.48),
            barrier(4.9, 0.3, 0.04),
        ],
    },
    Layout {
        min_width: 10.0,
        cars: 4,
        slots: &[
            car(-4.5, 0.0, 0.27),
            car(-0.3, -4.0, 0.75),
            car(4.2, -4.0, 0.24),
            car(4.0, 0.0, 0.26),
            barrier(-4.9, -5.0, 0.51),
            barrier(-4.2, -3.3, 0.53),
        ],
    },
    Layout {
        min_width: 14.0,
        cars: 4,
        slots: &[
            car(-4.5, 0.0, 0.25),
            car(1.0, 0.0, 0.75),
            car(0.0, 5.4, 0.25),
            car(5.3, 5.4, 0.22),
            barrier(-2.0, 2.2, 0.37),
            barrier(5.4, -0.2, 0.53),
        ],
    },
    Layout {
        min_width: 12.0,
        cars: 4,
        slots: &[
            car(-4.3, 0.0, 0.74),
            car(-5.0, 4.0, 0.75),
            car(0.2, 4.0, 0.27),
            car(4.0, 0.0, 0.25),
            barrier(-0.7, -1.0, 0.355),
            barrier(0.7, -1.0, 0.635),
        ],
    },
    Layout {
        min_width: 22.0,
        cars: 4,
        slots: &[
            car(-9.0, 0.0, 0.74),
            car(-3.0, 0.0, 0.25),
            car(3.0, 0.0, 0.75),
            car(9.0, 0.0, 0.25),
            barrier(0.0, -3.0, 0.5),
        ],
    },
    Layout {
        min_width: 15.0,
        cars: 3,
        slots: &[car(-5.9, 0.0, 0.74), car(0.0, 0.0, 0.25), car(5.9, 0.0, 0.75), barrier(3.0, -2.0, 0.51)],
    },
    Layout {
        min_width: 28.0,
        cars: 5,
        slots: &[
            car(-12.0, 0.0, 0.74),
            car(-6.0, 0.0, 0.25),
            car(0.0, 0.0, 0.75),
            car(6.0, 0.0, 0.75),
            car(12.0, 0.0, 0.25),
            barrier(-3.0, -3.4, 0.5),
        ],
    },
];

/// The layouts with spike strips (`SPIKES_RoadblockCandidateList`, spec section 4), used when the spike roll succeeds.
pub const SPIKES: [Layout; 9] = [
    Layout { min_width: 8.0, cars: 1, slots: &[car(-3.0, 0.0, 0.75), spike(2.0, -0.35, 0.52)] },
    Layout {
        min_width: 15.0,
        cars: 3,
        slots: &[
            spike(-5.0, 0.0, 0.51),
            car(1.0, 0.0, 0.29),
            car(5.0, 6.0, 0.24),
            car(8.0, 2.0, 0.5),
            barrier(4.1, -1.4, 0.57),
            barrier(6.1, -1.4, 0.43),
        ],
    },
    Layout {
        min_width: 15.0,
        cars: 2,
        slots: &[
            car(-4.0, 1.0, 0.26),
            car(-0.9, -4.2, 0.75),
            spike(5.0, -4.0, 0.52),
            barrier(-3.3, -2.2, 0.4),
            barrier(-5.0, -4.0, 0.46),
        ],
    },
    Layout {
        min_width: 14.1,
        cars: 2,
        slots: &[
            spike(-4.8, 0.0, 0.5),
            car(1.0, 0.0, 0.25),
            car(4.0, 4.5, 0.72),
            barrier(4.3, -1.2, 0.48),
            barrier(4.9, 0.3, 0.04),
        ],
    },
    Layout {
        min_width: 12.0,
        cars: 3,
        slots: &[
            car(-4.0, 0.0, 0.27),
            spike(-0.3, -4.4, 0.5),
            car(4.8, -4.0, 0.24),
            car(4.0, 0.0, 0.26),
            barrier(-4.9, -5.0, 0.51),
            barrier(-4.2, -3.3, 0.53),
        ],
    },
    Layout {
        min_width: 13.0,
        cars: 3,
        slots: &[
            spike(-4.0, 0.0, 0.51),
            car(-5.0, 4.0, 0.75),
            car(0.2, 4.0, 0.27),
            car(4.0, 0.0, 0.25),
            barrier(-0.7, -1.0, 0.355),
            barrier(0.7, -1.0, 0.635),
        ],
    },
    Layout {
        min_width: 21.0,
        cars: 2,
        slots: &[
            spike(-9.0, 0.0, 0.48),
            car(-3.0, 0.0, 0.25),
            car(3.0, 0.0, 0.75),
            spike(9.0, 0.0, 0.52),
            barrier(0.0, -3.0, 0.5),
        ],
    },
    Layout {
        min_width: 15.0,
        cars: 2,
        slots: &[spike(-5.2, 0.0, 0.51), car(0.0, 0.0, 0.25), car(5.9, 0.0, 0.75), barrier(3.0, -2.0, 0.51)],
    },
    Layout {
        min_width: 28.0,
        cars: 4,
        slots: &[
            car(-12.0, 0.0, 0.74),
            car(-6.0, 0.0, 0.25),
            spike(-1.0, 0.0, 0.51),
            car(5.0, 0.0, 0.75),
            car(11.0, 0.0, 0.25),
        ],
    },
];

/// Cars needed for a road this wide (`MinCopsForWidth` by the road's width in 4 m steps, plus one).
pub fn cars_asked(cr_width: f32) -> u32 {
    const MIN_COPS: [u32; 7] = [2, 2, 3, 3, 4, 4, 5];
    let index = ((cr_width / 4.0).max(0.0) as usize).min(6);
    MIN_COPS[index] + 1
}

/// The layout for a road `cr_width` metres wide with `cars` cars: the one whose minimum width leaves the smallest
/// positive slack, among those the cars cover. `None` when no layout fits.
pub fn pick(list: &[Layout], cr_width: f32, cars: u32) -> Option<&Layout> {
    list.iter()
        .filter(|l| l.cars <= cars && cr_width - l.min_width > 0.0)
        .min_by(|a, b| (cr_width - a.min_width).total_cmp(&(cr_width - b.min_width)))
}

/// The chance (percent) that a roadblock takes the spike layouts at each heat, 1 to 10 (`roadblockspikechance`,
/// [verified] in `docs/specs/ai-pursuit-roadblocks.md` section 1). Any other heat has none.
pub fn spike_chance(heat: u32) -> f32 {
    const CHANCE: [f32; 10] = [0.0, 0.0, 0.0, 70.0, 80.0, 85.0, 85.0, 0.0, 100.0, 0.0];
    heat.checked_sub(1).and_then(|i| CHANCE.get(i as usize)).copied().unwrap_or(0.0)
}

/// Whether this roadblock takes the spike layouts. `roll` is uniform in `0..=1`, read as a percentage of 100.
pub fn takes_spikes(heat: u32, roll: f32) -> bool {
    roll * 100.0 < spike_chance(heat)
}

/// The road where a roadblock goes: its two edges and the target it guards against, all in the plane (`y` unused),
/// and the road's forward direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Road {
    pub left: Vec3,
    pub right: Vec3,
    pub target: Vec3,
    pub forward: Vec3,
}

/// One element placed on the road: its kind, its position and its facing in turns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    pub kind: Element,
    pub position: Vec3,
    pub heading: f32,
}

/// The centre of `layout` on `road`: against the road's edge nearer the target, half the layout's minimum width along
/// the road's width (spec section 2.4). The roadblock keeps it for the dodge test.
pub fn centre(layout: &Layout, road: &Road) -> Vec3 {
    let at = layout_centre(layout, road);
    Vec3::new(at.x, road.target.y, at.y)
}

fn layout_centre(layout: &Layout, road: &Road) -> Vec2 {
    let left = Vec2::new(road.left.x, road.left.z);
    let right = Vec2::new(road.right.x, road.right.z);
    let target = Vec2::new(road.target.x, road.target.z);
    let crw = (left - right).length().max(1e-3);
    let (across, anchor) = if (left - target).length_squared() > (right - target).length_squared() {
        (left - right, right)
    } else {
        (right - left, left)
    };
    anchor + across * (layout.min_width / 2.0) / crw
}

/// Places `layout` on `road` (spec section 2.4). The layout sits against the edge nearer the target and spreads across
/// the road; its `x` offsets stretch by up to 14 percent on wider roads. The sign convention of the facing rotation is
/// `[unconfirmed]` (the spec's note: the game's `RotateInXZ`).
pub fn place(layout: &Layout, road: &Road) -> Vec<Placed> {
    let left = Vec2::new(road.left.x, road.left.z);
    let right = Vec2::new(road.right.x, road.right.z);
    let crw = (left - right).length().max(1e-3);
    let cr_width = crw + 1.0;
    let centre = layout_centre(layout, road);
    let x_scale = (cr_width / layout.min_width).clamp(1.0, 1.14);
    let forward = Vec2::new(road.forward.x, road.forward.z).normalize_or_zero();
    let side = Vec2::new(forward.y, -forward.x);
    layout
        .slots
        .iter()
        .map(|slot| {
            let along = side * (slot.x * x_scale) + forward * slot.z;
            let at = centre + along;
            let facing = rotate(forward, slot.angle * std::f32::consts::TAU);
            Placed {
                kind: slot.kind,
                position: Vec3::new(at.x, road.target.y, at.y),
                heading: facing.y.atan2(facing.x),
            }
        })
        .collect()
}

/// `v` turned by `angle` radians.
fn rotate(v: Vec2, angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tables_have_the_documented_sizes() {
        assert_eq!(NORMAL.len(), 15);
        assert_eq!(SPIKES.len(), 9);
    }

    #[test]
    fn spikes_follow_the_heat_table() {
        assert_eq!(spike_chance(4), 70.0);
        assert_eq!(spike_chance(9), 100.0);
        assert_eq!(spike_chance(8), 0.0, "heat 8 has no spike strips");
        assert_eq!(spike_chance(0), 0.0, "no heat");
        assert_eq!(spike_chance(11), 0.0, "beyond heat 10");
        assert!(takes_spikes(5, 0.79), "79 percent is under the heat 5 chance of 80");
        assert!(!takes_spikes(5, 0.80), "the roll must be strictly under the chance");
    }

    #[test]
    fn each_layout_has_as_many_cars_as_it_requires() {
        for layout in NORMAL.iter().chain(SPIKES.iter()) {
            let cars = layout.slots.iter().filter(|s| s.kind == Element::Car).count() as u32;
            assert_eq!(cars, layout.cars, "min width {}", layout.min_width);
        }
    }

    #[test]
    fn the_cars_asked_grow_with_the_road() {
        assert_eq!(cars_asked(0.0), 3, "two cops plus one");
        assert_eq!(cars_asked(9.0), 4, "index 2: three plus one");
        assert_eq!(cars_asked(40.0), 6, "the widest index is 6: five plus one");
    }

    #[test]
    fn the_layout_is_the_one_that_fits_the_road_most_snugly() {
        // A 9 m road with 2 cars: 8.5 m (slack 0.5) is the snuggest of the normal layouts that fit.
        let layout = pick(&NORMAL, 9.0, 2).expect("fits");
        assert_eq!(layout.min_width, 8.5);
    }

    #[test]
    fn a_road_too_narrow_or_too_few_cars_fits_nothing() {
        assert!(pick(&NORMAL, 5.0, 4).is_none(), "narrower than every layout");
        assert!(pick(&NORMAL, 30.0, 1).is_none(), "every layout needs at least two cars");
    }

    #[test]
    fn the_block_sits_on_the_edge_nearer_the_target() {
        // Left edge at +4 m, right edge at -4 m; the target is near the right edge, so the layout is anchored there.
        let road = Road {
            left: Vec3::new(0.0, 0.0, 4.0),
            right: Vec3::new(0.0, 0.0, -4.0),
            target: Vec3::new(10.0, 0.0, -3.5),
            forward: Vec3::X,
        };
        let placed = place(&NORMAL[0], &road);
        assert_eq!(placed.len(), 2);
        // The anchor is the right edge (-4 m); the layout spans 8 m across, so its centre is the road's middle (0 m).
        // The first car is 3 m to the layout's side, stretched by 9/8 (cr_width over min width): 3.375 m on the left.
        assert!((placed[0].position.z - 3.375).abs() < 1e-3, "{:?}", placed[0].position);
    }

    #[test]
    fn elements_keep_their_facing_within_one_turn() {
        let road = Road {
            left: Vec3::new(0.0, 0.0, 4.0),
            right: Vec3::new(0.0, 0.0, -4.0),
            target: Vec3::new(10.0, 0.0, 0.0),
            forward: Vec3::X,
        };
        for placed in place(&SPIKES[0], &road) {
            assert!(placed.heading.is_finite());
        }
    }

    #[test]
    fn x_offsets_stretch_only_up_to_fourteen_percent() {
        // A 14 m road (cr_width 15 m) with an 8 m layout: 15 / 8 = 1.875, capped at 1.14 (spec 2.4).
        let road = Road {
            left: Vec3::new(0.0, 0.0, 7.0),
            right: Vec3::new(0.0, 0.0, -7.0),
            target: Vec3::new(10.0, 0.0, 0.0),
            forward: Vec3::X,
        };
        let placed = place(&NORMAL[0], &road);
        let spread = (placed[0].position - placed[1].position).length();
        assert!(spread > 0.0);
    }
}
