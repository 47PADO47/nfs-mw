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
use crate::build_pieces::{Mounts, apart_ids, pieces};
use crate::exhaust::{self, Pipe};
use crate::exports::Exports;
use crate::fork::{self, Names};
use crate::front_end::Take;
use crate::layout::{Frame, LEVELS, wheel_point};
use crate::partfile::{Kind, Material, Part, PartTable, Role};
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

/// One livery the car offers: a choice of the shop's vinyl category, and the texture of the car's own `TEXTURES.BIN` that holds
/// its picture (`[customization] liveries`, `docs/modding/car-customization.md#liveries`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LiveryChoice {
    pub name: String,
    pub texture: String,
}

/// A solid of the car after the first of its kind, which the manifest binds as a base part (`[parts."base.body.2"]`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExtraSolid {
    /// The part's id: `base.body.2`, `base.bars.2`, `base.fork.3`.
    pub id: String,
    /// The solid's name without the level letter.
    pub solid: String,
    /// The part's role in the manifest: `body`, `steered` or `steered_slid`.
    pub role: String,
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
    /// The silencers' outlets in the body solid's frame; each is a tail-pipe marker of every body solid ([`exhaust`]). Empty for a
    /// bike with no silencer mesh.
    pub exhaust: Vec<Pipe>,
    /// The liveries the bike offers, the stock one first; empty for a bike with one look.
    pub liveries: Vec<LiveryChoice>,
    /// For each body letter `A` to `D`: the level of detail of the chassis and of the other body parts its solids were built at.
    pub levels: Vec<[u32; 2]>,
    /// The solids after the first of the frame, the handlebar group and the fork (`<ID>_KIT00_BODY2`, ...): at the finest levels of
    /// detail they do not fit one solid. The manifest binds each as a base part.
    pub extra_solids: Vec<ExtraSolid>,
    /// The live screen of the instrument cluster, when its picture has a display (`[bike.dash]`, [`crate::dash`]).
    pub dash: Option<crate::dash::Screen>,
}

impl Built {
    pub fn wheelbase(&self) -> f32 {
        self.front.hub[0] - self.rear.hub[0]
    }
}

/// The wheel of a role: its tyre part and the hub bone the tyre hangs on.
pub(crate) fn tyre(table: &PartTable, role: Role) -> Result<&Part> {
    table
        .parts_of(role)
        .find(|p| p.kind == Kind::Tyre)
        .with_context(|| format!("the part table has no tyre for the {role:?} solid"))
}

pub(crate) fn hub_of(table: &PartTable, tyre: &Part) -> Result<[f32; 3]> {
    let bone = tyre.anchor.bone.as_deref().with_context(|| format!("tyre {} has no hub bone", tyre.id))?;
    table.bone(bone)
}

/// Lowest and highest height of a tyre's vertices in the chassis frame.
pub(crate) fn tyre_heights(exports: &Exports, tyre: &Part) -> Result<(f32, f32)> {
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
    levels: Vec<[u32; 2]>,
    /// The most solids the frame, the handlebar group and the fork have at any level.
    most: [usize; 3],
}

/// Build the car files of `folder` from the part table and exports in `dir`.
pub fn build(dir: &std::path::Path, folder: &str, options: Options) -> Result<Built> {
    let table = PartTable::read(dir)?;
    let exports = Exports::scan(&dir.join("glb"))?;
    let store = TexStore::open(&dir.join("tex"))?;
    // A bike with several different liveries draws the stock one on the car's paint placeholder, which the game repaints with
    // the livery the player picks; the others are textures of the pack.
    let livery_pictures = table.livery_pictures();
    let paint = match livery_pictures.is_empty() {
        true => None,
        false => table.livery_material().and_then(|(_, material)| material.base_color.clone()),
    };
    let mut surfaces = Surfaces::new(store, options).with_paint(paint);

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
    let pipes = exhaust::stock_pipes(&table, &exports, frame)?;

    let mut solids = Solids {
        list: Vec::new(),
        points: Vec::new(),
        report: fork::Report::default(),
        levels: Vec::new(),
        most: [1; 3],
    };
    let apart = apart_ids(options.split_parts);
    let hubs = [(Role::FrontWheel, front_hub, "FRONT_TIRE"), (Role::BackWheel, rear_hub, "REAR_TIRE")];
    for (n, (letter, level)) in LEVELS.into_iter().enumerate() {
        let names = Names { letter, car: folder, apart: &apart };
        let mut pair = fork::body_and_fork(&table, &exports, &mut surfaces, names, (frame, front_hub), level)?;
        pair.bodies[0].markers = exhaust::markers(&pipes);
        solids.levels.push(pair.levels);
        let counts = [pair.bodies.len(), pair.bars.len(), pair.fork.len()];
        solids.most = std::array::from_fn(|i| solids.most[i].max(counts[i]));
        if n == 0 {
            solids.report = pair.report;
            // The fork's and the bars' points are relative to the front hub; the bounds are of the whole bike in the body's
            // frame.
            let hub = front.hub;
            let moved =
                pair.fork_points.iter().chain(&pair.bars_points).map(|p| [p[0] + hub[0], p[1] + hub[1], p[2] + hub[2]]);
            solids.points = pair.body_points.iter().copied().chain(moved).collect();
        }
        solids.list.extend(pair.bodies);
        solids.list.extend(pair.fork);
        solids.list.extend(pair.bars);
    }
    for (role, hub, tag) in hubs {
        for (n, (letter, _)) in LEVELS.into_iter().enumerate() {
            let name = format!("{folder}_KIT00_{tag}_{letter}");
            let level = n as u32;
            let wheel = (name.as_str(), folder);
            solids.list.push(wheel_solid(&table, &exports, &mut surfaces, wheel, (role, hub), (level, &apart))?);
        }
    }
    if options.split_parts != 0 {
        let at = Mounts { frame, front_hub, rear_hub };
        let made = pieces(&table, &exports, &mut surfaces, folder, (at, options.split_parts))?;
        solids.points.extend(made.points);
        solids.list.extend(made.list);
    }
    let liveries = add_liveries(&mut surfaces, folder, &livery_pictures)?;
    let dash = surfaces.dash.clone();
    let geometry = write_geometry_pack(&solids.list).context("writing GEOMETRY.BIN")?;
    let textures = write_tpk(folder, &surfaces.palette.into_textures()).context("writing TEXTURES.BIN")?;
    let (bounds, half) = bounds(&solids.points, folder)?;
    let report = solids.report;
    let triangles = report.body_triangles + report.fork_triangles + report.bars_triangles;
    let levels = solids.levels;
    let extra_solids = extra_solids(folder, solids.most);
    Ok(Built {
        geometry,
        textures,
        bounds,
        half,
        front,
        rear,
        triangles,
        front_end: report,
        exhaust: pipes,
        liveries,
        levels,
        extra_solids,
        dash,
    })
}

/// The base parts for the solids after the first of the frame, the handlebar group and the fork, `most` being how many of each the car
/// has at its finest level.
fn extra_solids(folder: &str, most: [usize; 3]) -> Vec<ExtraSolid> {
    let kinds = [("body", "BODY", "body"), ("bars", "BARS", "steered"), ("fork", "FORK", "steered_slid")];
    let mut out = Vec::new();
    for ((lower, upper, role), count) in kinds.into_iter().zip(most) {
        for n in 2..=count {
            out.push(ExtraSolid {
                id: format!("base.{lower}.{n}"),
                solid: format!("{folder}_KIT00_{upper}{n}"),
                role: role.to_owned(),
            });
        }
    }
    out
}

/// The livery textures of the car's pack and the choices that name them. The first livery is the stock one: its picture is the
/// paint placeholder `<ID>_SKIN1` the body surfaces already draw; every other gets a texture `<ID>_LIV<n>`. A livery whose picture
/// is the same as an earlier one's, or cannot be read, is left out.
fn add_liveries(surfaces: &mut Surfaces, folder: &str, pictures: &[(u32, Material)]) -> Result<Vec<LiveryChoice>> {
    let id = folder.to_ascii_uppercase();
    let mut out = Vec::new();
    let mut seen: Vec<Option<String>> = Vec::new();
    for (index, material) in pictures {
        if seen.contains(&material.base_color) {
            continue;
        }
        let texture = match out.is_empty() {
            true => format!("{id}_SKIN1"),
            false => format!("{id}_LIV{index}"),
        };
        if !out.is_empty() && !surfaces.add_livery_picture(&texture, material)? {
            continue;
        }
        seen.push(material.base_color.clone());
        out.push(LiveryChoice { name: format!("Livery {index}"), texture });
    }
    Ok(out)
}

/// Read the part at `level` (or the finest level below it that was exported) and make its groups.
pub(crate) fn load_groups(
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
    (level, apart): (u32, &[&str]),
) -> Result<SolidSource> {
    let (mut groups, mut points, mut lists) = (Vec::new(), Vec::new(), Lists::default());
    let to_solid = |p: [f32; 3]| wheel_point(p, hub);
    let mut ctx = GroupContext { table, surfaces, car, lists: &mut lists };
    for part in table.parts_of(role).filter(|part| !apart.contains(&part.id.as_str())) {
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
            exhaust: Vec::new(),
            liveries: Vec::new(),
            levels: Vec::new(),
            extra_solids: Vec::new(),
            dash: None,
        };
        assert!((built.wheelbase() - 1.4745).abs() < 1e-4);
    }

    #[test]
    fn a_solid_with_no_surfaces_or_too_many_vertices_is_refused() {
        assert!(finish("S", Vec::new(), Lists::default()).is_err());
    }
}
