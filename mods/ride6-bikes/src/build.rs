//! A bike as the 2005 car files: `GEOMETRY.BIN` with, at four levels of detail, a body solid and a front and a rear wheel
//! solid, `TEXTURES.BIN` and `BOUNDS.BIN`.
//!
//! The game draws a car as a body and wheels; a bike has two. The body solid holds the frame, tank, seat, engine,
//! swingarm, exhaust and the rear end. The bars solid (`<ID>_KIT00_BARS_<L>`, [`crate::fork`]) holds what steers with the
//! handlebar and nothing more: handlebar, levers, grips, hand guards, bar pad, headlamp and front fairing, gauges. The fork
//! solid (`<ID>_KIT00_FORK_<L>`) holds what steers and slides with the suspension: the fork legs, front fender and front
//! caliper. Both have the front hub on their origin. Each wheel solid holds what turns with the wheel (tyre, rim, brake disc)
//! around its hub: the hub is the origin, the axle runs along y, and the solid is centred on the plane y = 0 of the car
//! frame. Where the hubs are is in [`Built`]; the manifest carries them.

use anyhow::{Context, Result, bail};
use blackbox_attrib::vlt_hash;
use blackbox_collision::{BodyLimits, FitOptions, fit_body, write_bounds_pack};
use blackbox_solid::{GroupSource, SolidSource, write_geometry_pack};
use blackbox_tpk::write_tpk;

use crate::bake::{Options, Surfaces};
use crate::exports::Exports;
use crate::fork::{self, Names};
use crate::front_end::Take;
use crate::layout::{Frame, LEVELS, wheel_point};
use crate::partfile::{Kind, Part, PartTable, Role};
use crate::solids::{Context as GroupContext, Lists, Placer, part_groups};
use crate::textures::TexStore;

/// Vertices one solid may hold: the writer puts all groups of one effect in one buffer with 16-bit indices.
pub const MAX_SOLID_VERTICES: usize = 60_000;
/// Triangles one solid may hold (`SolidInfo` counts them in 16 bits).
pub const MAX_SOLID_TRIANGLES: usize = 65_535;
/// `simsurface` key of `carbody`, the surface the body nodes of a car's bounds have.
const CARBODY_SURFACE: u32 = 0x99DA_6531;

/// One wheel of the bike, in the car frame of the body solid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wheel {
    /// The hub as a point of the body solid (x forward, y left, z up), metres.
    pub hub: [f32; 3],
    /// Radius of the tyre, metres.
    pub radius: f32,
    /// Width of the tyre in its size code (`120-60-17` is 0.120), metres; 0 when the table has none.
    pub width: f32,
}

/// What the car's files hold, and what the manifest needs to know.
pub struct Built {
    pub geometry: Vec<u8>,
    pub textures: Vec<u8>,
    pub bounds: Vec<u8>,
    /// Half dimensions of the bounds' root box (x across, y up, z along), metres.
    pub half: [f32; 3],
    pub front: Wheel,
    pub rear: Wheel,
    /// Triangles of the finest body and fork together.
    pub triangles: usize,
    /// What the finest level's body and fork solids hold: the check that the split of the front end took the front end.
    pub front_end: fork::Report,
}

impl Built {
    pub fn wheelbase(&self) -> f32 {
        self.front.hub[0] - self.rear.hub[0]
    }
}

/// The wheel of a role: its tyre part and the hub bone the tyre hangs on.
fn tyre(table: &PartTable, role: Role) -> Result<&Part> {
    table
        .parts_of(role)
        .find(|p| p.kind == Kind::Tyre)
        .with_context(|| format!("the part table has no tyre for the {role:?} solid"))
}

fn hub_of(table: &PartTable, tyre: &Part) -> Result<[f32; 3]> {
    let bone = tyre.anchor.bone.as_deref().with_context(|| format!("tyre {} has no hub bone", tyre.id))?;
    table.bone(bone)
}

/// Lowest and highest height of a tyre's vertices in the chassis frame.
fn tyre_heights(exports: &Exports, tyre: &Part) -> Result<(f32, f32)> {
    let glb = exports.load(&tyre.export, 0)?;
    let heights: Vec<f32> =
        glb.primitives.iter().flat_map(|p| &p.positions).map(|&p| tyre.anchor.point(p)[1]).collect();
    if heights.is_empty() {
        bail!("tyre {} has no vertices", tyre.id);
    }
    let low = heights.iter().copied().fold(f32::MAX, f32::min);
    let high = heights.iter().copied().fold(f32::MIN, f32::max);
    Ok((low, high))
}

/// The width a tyre size code (`120-60-17`: width in millimetres, aspect, rim inches) names, metres.
pub fn tyre_width(size: &str) -> f32 {
    size.split('-').next().and_then(|mm| mm.parse::<f32>().ok()).map_or(0.0, |mm| mm / 1000.0)
}

struct Solids {
    list: Vec<SolidSource>,
    /// Points of the finest body and fork, in the body solid's frame, for the bounds.
    points: Vec<[f32; 3]>,
    report: fork::Report,
}

/// Build the car files of `folder` from the part table and exports in `dir`.
pub fn build(dir: &std::path::Path, folder: &str, options: Options) -> Result<Built> {
    let table = PartTable::read(dir)?;
    let exports = Exports::scan(&dir.join("glb"))?;
    let store = TexStore::open(&dir.join("tex"))?;
    let mut surfaces = Surfaces::new(store, options);

    let (front_tyre, rear_tyre) = (tyre(&table, Role::FrontWheel)?, tyre(&table, Role::BackWheel)?);
    let (front_hub, rear_hub) = (hub_of(&table, front_tyre)?, hub_of(&table, rear_tyre)?);
    let (front_heights, rear_heights) = (tyre_heights(&exports, front_tyre)?, tyre_heights(&exports, rear_tyre)?);
    let frame = Frame { centre: (front_hub[0] + rear_hub[0]) / 2.0, lift: -front_heights.0.min(rear_heights.0) };
    let size = |side: &str| tyre_width(table.tyre_sizes.get(side).map_or("", String::as_str));
    let wheel = |hub: [f32; 3], heights: (f32, f32), side: &str| Wheel {
        hub: frame.body_point(hub),
        radius: (heights.1 - heights.0) / 2.0,
        width: size(side),
    };
    let (front, rear) = (wheel(front_hub, front_heights, "front"), wheel(rear_hub, rear_heights, "back"));

    let mut solids = Solids { list: Vec::new(), points: Vec::new(), report: fork::Report::default() };
    let hubs = [(Role::FrontWheel, front_hub, "FRONT_TIRE"), (Role::BackWheel, rear_hub, "REAR_TIRE")];
    for (n, (letter, level)) in LEVELS.into_iter().enumerate() {
        let body = format!("{folder}_KIT00_BODY_{letter}");
        let (fork_name, bars_name) = (format!("{folder}_KIT00_FORK_{letter}"), format!("{folder}_KIT00_BARS_{letter}"));
        let names = Names { body: &body, fork: &fork_name, bars: &bars_name, car: folder };
        let pair = fork::body_and_fork(&table, &exports, &mut surfaces, names, (frame, front_hub), level)?;
        if n == 0 {
            solids.report = pair.report;
            // The fork's and the bars' points are relative to the front hub; the bounds are of the whole bike in the body's
            // frame.
            let hub = front.hub;
            let moved =
                pair.fork_points.iter().chain(&pair.bars_points).map(|p| [p[0] + hub[0], p[1] + hub[1], p[2] + hub[2]]);
            solids.points = pair.body_points.iter().copied().chain(moved).collect();
        }
        solids.list.push(pair.body);
        solids.list.extend(pair.fork);
        solids.list.extend(pair.bars);
    }
    for (role, hub, tag) in hubs {
        for (n, (letter, _)) in LEVELS.into_iter().enumerate() {
            let name = format!("{folder}_KIT00_{tag}_{letter}");
            let level = n as u32;
            solids.list.push(wheel_solid(&table, &exports, &mut surfaces, (&name, folder), (role, hub), level)?);
        }
    }
    let geometry = write_geometry_pack(&solids.list).context("writing GEOMETRY.BIN")?;
    let textures = write_tpk(folder, &surfaces.palette.into_textures()).context("writing TEXTURES.BIN")?;
    let (bounds, half) = bounds(&solids.points, folder)?;
    let report = solids.report;
    let triangles = report.body_triangles + report.fork_triangles + report.bars_triangles;
    Ok(Built { geometry, textures, bounds, half, front, rear, triangles, front_end: report })
}

/// Read the part at `level` (or the finest level below it that was exported) and make its groups.
fn load_groups(
    ctx: &mut GroupContext<'_>,
    exports: &Exports,
    part: &Part,
    level: u32,
    to_solid: &dyn Fn([f32; 3]) -> [f32; 3],
    points: &mut Vec<[f32; 3]>,
) -> Result<Vec<GroupSource>> {
    let glb = exports.load(&part.export, level)?;
    part_groups(ctx, &glb, (&Placer { part, to_solid }, Take::All), points)
}

pub(crate) fn finish(name: &str, groups: Vec<GroupSource>, lists: Lists) -> Result<SolidSource> {
    if groups.is_empty() {
        bail!("{name}: no surfaces");
    }
    let (vertices, triangles) = (
        groups.iter().map(|g| g.vertices.len()).sum::<usize>(),
        groups.iter().map(|g| g.indices.len() / 3).sum::<usize>(),
    );
    if vertices > MAX_SOLID_VERTICES || triangles > MAX_SOLID_TRIANGLES {
        bail!("{name}: {vertices} vertices and {triangles} triangles do not fit one solid");
    }
    let mut solid = SolidSource::new(name, groups);
    solid.texture_hashes = lists.textures;
    solid.light_material_hashes = lists.light_materials;
    Ok(solid)
}

/// The wheel solid of `role` at `level`: tyre, rim and disc around the hub.
fn wheel_solid(
    table: &PartTable,
    exports: &Exports,
    surfaces: &mut Surfaces,
    (name, car): (&str, &str),
    (role, hub): (Role, [f32; 3]),
    level: u32,
) -> Result<SolidSource> {
    let (mut groups, mut points, mut lists) = (Vec::new(), Vec::new(), Lists::default());
    let to_solid = |p: [f32; 3]| wheel_point(p, hub);
    let mut ctx = GroupContext { table, surfaces, car, lists: &mut lists };
    for part in table.parts_of(role) {
        groups.extend(load_groups(&mut ctx, exports, part, level, &to_solid, &mut points)?);
    }
    finish(name, groups, lists)
}

/// The car's bounds from the points of its finest body: car space (x forward, y left, z up) to the bounds space of
/// vehicles (x right, y up, z forward).
fn bounds(points: &[[f32; 3]], folder: &str) -> Result<(Vec<u8>, [f32; 3])> {
    let physics: Vec<[f32; 3]> = points.iter().map(|p| [-p[1], p[2], p[0]]).collect();
    let options = FitOptions { surface: CARBODY_SURFACE, ..FitOptions::default() };
    let set = fit_body(vlt_hash(&folder.to_ascii_uppercase()), &physics, &options).context("fitting the bounds")?;
    set.check_vehicle_body(&BodyLimits::default()).context("the bounds do not make a vehicle body")?;
    let bytes = write_bounds_pack(std::slice::from_ref(&set)).context("writing BOUNDS.BIN")?;
    Ok((bytes, set.root().half_dimensions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tyre_size_code_gives_its_width() {
        assert!((tyre_width("120-60-17") - 0.12).abs() < 1e-6);
        assert!((tyre_width("160-60-17") - 0.16).abs() < 1e-6);
        assert_eq!(tyre_width(""), 0.0);
        assert_eq!(tyre_width("wide"), 0.0);
    }

    #[test]
    fn the_wheelbase_is_the_hub_distance() {
        let wheel = |x: f32| Wheel { hub: [x, 0.0, 0.3], radius: 0.3, width: 0.1 };
        let built = Built {
            geometry: vec![],
            textures: vec![],
            bounds: vec![],
            half: [0.0; 3],
            front: wheel(0.7355),
            rear: wheel(-0.739),
            triangles: 0,
            front_end: fork::Report::default(),
        };
        assert!((built.wheelbase() - 1.4745).abs() < 1e-4);
    }

    #[test]
    fn a_solid_with_no_surfaces_or_too_many_vertices_is_refused() {
        assert!(finish("S", Vec::new(), Lists::default()).is_err());
    }
}
