//! The stock parts of a bike that the shop can swap, built as solids of their own (`docs/modding/ride6-bike-upgrades.md`).
//!
//! Without the shop's options the exhaust is part of the body solid, the bar pad of the bars, and the rims and brake discs of
//! the wheel solids. A bike that has options writes these six parts as solids named `<ID>_KIT00_<TAG>_<L>`, each modelled in the
//! frame the game poses it in (body, or a hub), so that a part of the catalogue can take its place. The body, bars, fork and
//! wheel solids leave them out ([`apart_ids`]); the tyre stays in the wheel solid.

use anyhow::Result;
use blackbox_solid::SolidSource;

use crate::bake::Surfaces;
use crate::build::{finish, hub_of, load_groups, tyre, tyre_heights};
use crate::exports::Exports;
use crate::layout::{Frame, LEVELS, wheel_point};
use crate::partfile::{Anchor, PartTable, Role};
use crate::solids::{Context as GroupContext, Lists};

/// The frame a piece is modelled in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum At {
    /// The body solid's: the middle of the wheelbase on the origin, the tyres on the ground.
    Body,
    /// The front hub's (the bars', the fork's and the front wheel's).
    FrontHub,
    /// The rear hub's.
    RearHub,
}

/// One swappable stock part.
#[derive(Debug, Clone, Copy)]
pub struct Piece {
    /// The solid is `<ID>_KIT00_<tag>`.
    pub tag: &'static str,
    /// The ids of the parts of `parts.json` it holds.
    pub ids: &'static [&'static str],
    pub at: At,
    /// The part id the stock component of the bike's slot shows, and the role of its binding in the manifest.
    pub part: &'static str,
    pub role: &'static str,
}

/// The pieces, in the order the manifest lists them.
pub static PIECES: [Piece; 6] = [
    Piece { tag: "EXHAUST", ids: &["exmanifold", "muffler"], at: At::Body, part: "exhaust.stock", role: "body" },
    Piece { tag: "BARPAD", ids: &["barpad"], at: At::FrontHub, part: "barpad.stock", role: "steered" },
    Piece { tag: "FRONT_RIM", ids: &["front_rim"], at: At::FrontHub, part: "rim.front.stock", role: "wheel_front" },
    Piece { tag: "REAR_RIM", ids: &["back_rim"], at: At::RearHub, part: "rim.back.stock", role: "wheel_rear" },
    Piece { tag: "FRONT_DISC", ids: &["front_brake"], at: At::FrontHub, part: "disc.front.stock", role: "wheel_front" },
    Piece { tag: "REAR_DISC", ids: &["back_brake"], at: At::RearHub, part: "disc.back.stock", role: "wheel_rear" },
];

/// The part ids the pieces of `mask` (bit `n` for `PIECES[n]`) take out of the body, bars and wheels.
pub fn apart_ids(mask: u8) -> Vec<&'static str> {
    let chosen = PIECES.iter().enumerate().filter(|(n, _)| mask & (1 << n) != 0);
    chosen.flat_map(|(_, piece)| piece.ids.iter().copied()).collect()
}

/// Where a bike's frames are: what turns a point of the chassis frame into a point of a piece's solid.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Mounts {
    pub frame: Frame,
    /// The hubs in the chassis frame.
    pub front_hub: [f32; 3],
    pub rear_hub: [f32; 3],
}

impl Mounts {
    /// A point of the chassis frame as a point of the solid in the frame `at`.
    pub fn to(&self, at: At, p: [f32; 3]) -> [f32; 3] {
        match at {
            At::Body => self.frame.body_point(p),
            At::FrontHub => wheel_point(p, self.front_hub),
            At::RearHub => wheel_point(p, self.rear_hub),
        }
    }

    /// The mount of a part of the catalogue: the three rows of the 3 by 4 matrix that takes its solid (the part's own frame with the
    /// export axes turned to the car's, `options_pack`) to the frame `at` of this bike, through the part's `anchor`.
    pub fn matrix(&self, at: At, anchor: &Anchor) -> [f32; 12] {
        // The solid holds (x, -z, y) of the export frame; the inverse turn gives the export point back.
        let map = |q: [f32; 3]| self.to(at, anchor.point([q[0], q[2], -q[1]]));
        let origin = map([0.0; 3]);
        let axes = [map([1.0, 0.0, 0.0]), map([0.0, 1.0, 0.0]), map([0.0, 0.0, 1.0])];
        let row = |i: usize| [axes[0][i] - origin[i], axes[1][i] - origin[i], axes[2][i] - origin[i], origin[i]];
        let (x, y, z) = (row(0), row(1), row(2));
        [x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3], z[0], z[1], z[2], z[3]]
    }
}

/// The solids of the pieces a bike has, at every level of detail, and the points of its exhaust at the finest, in the body frame.
#[derive(Default)]
pub struct Made {
    pub list: Vec<SolidSource>,
    pub points: Vec<[f32; 3]>,
}

/// Which pieces a bike has: those whose parts are in its part table.
pub fn present(table: &PartTable) -> Vec<&'static Piece> {
    PIECES.iter().filter(|piece| table.parts.iter().any(|part| piece.ids.contains(&part.id.as_str()))).collect()
}

/// The frames of the bike whose part table is `table`: the middle of the wheelbase, the lowest tyre point and the two hubs.
pub fn mounts_of(table: &PartTable, exports: &Exports) -> Result<Mounts> {
    let (front_tyre, rear_tyre) = (tyre(table, Role::FrontWheel)?, tyre(table, Role::BackWheel)?);
    let (front_hub, rear_hub) = (hub_of(table, front_tyre)?, hub_of(table, rear_tyre)?);
    let (front, rear) = (tyre_heights(exports, front_tyre)?, tyre_heights(exports, rear_tyre)?);
    let frame = Frame { centre: (front_hub[0] + rear_hub[0]) / 2.0, lift: -front.0.min(rear.0) };
    Ok(Mounts { frame, front_hub, rear_hub })
}

/// The solids of the pieces of `mask` of `folder`, `mounts` being where its frames are.
pub fn pieces(
    table: &PartTable,
    exports: &Exports,
    surfaces: &mut Surfaces,
    folder: &str,
    (mounts, mask): (Mounts, u8),
) -> Result<Made> {
    let mut made = Made::default();
    let wanted = apart_ids(mask);
    for piece in present(table).into_iter().filter(|piece| piece.ids.iter().all(|id| wanted.contains(id))) {
        let to_solid = |p: [f32; 3]| mounts.to(piece.at, p);
        for (n, (letter, _)) in LEVELS.into_iter().enumerate() {
            let name = format!("{folder}_KIT00_{}_{letter}", piece.tag);
            let (mut groups, mut points, mut lists) = (Vec::new(), Vec::new(), Lists::default());
            let mut ctx = GroupContext { table, surfaces: &mut *surfaces, car: folder, lists: &mut lists };
            for part in table.parts.iter().filter(|part| piece.ids.contains(&part.id.as_str())) {
                groups.extend(load_groups(&mut ctx, exports, part, n as u32, &to_solid, &mut points)?);
            }
            if groups.is_empty() {
                continue;
            }
            made.list.push(finish(&name, groups, lists)?);
            if n == 0 && piece.at == At::Body {
                made.points.extend(points);
            }
        }
    }
    Ok(made)
}

#[cfg(test)]
#[path = "build_pieces_tests.rs"]
mod tests;
