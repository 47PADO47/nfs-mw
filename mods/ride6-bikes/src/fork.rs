//! The body, bars and fork solids of one level of detail, made together so all three read the chassis at the same level.
//!
//! `front_end.rs` tells three groups of the chassis apart. The frame stays in `<ID>_KIT00_BODY_<L>`. The **bars**
//! (handlebar, levers, grips, hand guards, bar pad, headlamp and front fairing, gauges, upper clamp) go to
//! `<ID>_KIT00_BARS_<L>`: they turn with the handlebar and nothing more. The **fork** (legs, front fender, front caliper)
//! goes to `<ID>_KIT00_FORK_<L>`: it turns and slides along the steering axis with the suspension. The frame of both
//! solids is the **front hub's**: the origin is the centre of the front hub, the axes are the car's (x forward, y left,
//! z up), and nothing is scaled, spun or mirrored. The game poses the fork like the front wheel without the spin, and the
//! bars like it without the travel (`docs/modding/ride6-bikes.md`, "The steered front end").

use anyhow::{Context, Result};
use blackbox_solid::{GroupSource, SolidSource};

use crate::bake::Surfaces;
use crate::build::finish;
use crate::chunks::{MAX_BODY_SOLIDS, pack};
use crate::exports::Exports;
use crate::front_end::{self, Class, Side, Take};
use crate::glb::Glb;
use crate::layout::{COARSEST_CHASSIS_LEVEL, Frame, wheel_point};
use crate::partfile::{Part, PartTable, Role};
use crate::solids::{Context as GroupContext, Lists, Placer, part_groups, vertex_estimate};

/// How many vertices and triangles each of the two solids of a level holds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Report {
    pub body_vertices: usize,
    pub body_triangles: usize,
    pub fork_vertices: usize,
    pub fork_triangles: usize,
    pub bars_vertices: usize,
    pub bars_triangles: usize,
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "body {} vertices in {} triangles, fork {} vertices in {} triangles, bars {} vertices in {} triangles",
            self.body_vertices,
            self.body_triangles,
            self.fork_vertices,
            self.fork_triangles,
            self.bars_vertices,
            self.bars_triangles
        )
    }
}

/// The solids of one level of detail.
pub struct Pair {
    /// The solids of the frame: `<ID>_KIT00_BODY_<L>` first, then `<ID>_KIT00_BODY2_<L>` and so on for what does not fit it
    /// ([`crate::chunks`]). Never empty.
    pub bodies: Vec<SolidSource>,
    /// The solids of the fork, `<ID>_KIT00_FORK_<L>` and then `FORK2` and so on; empty when nothing slides (the chassis is not skinned,
    /// or no bone of the rules is in it).
    pub fork: Vec<SolidSource>,
    /// The solids of the handlebar group; empty when nothing steers without sliding.
    pub bars: Vec<SolidSource>,
    /// The vertices of the body, in the body solid's frame, for the bounds.
    pub body_points: Vec<[f32; 3]>,
    /// The vertices of the fork and of the bars, in their solids' frame (the front hub's).
    pub fork_points: Vec<[f32; 3]>,
    pub bars_points: Vec<[f32; 3]>,
    pub report: Report,
    /// The level of detail of the chassis and the level of the other parts (exhaust, bar pad, mirrors) the solids were built at.
    pub levels: [u32; 2],
}

/// The part of a bike a solid is made of: the frame, the handlebar group or the fork.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Body,
    Bars,
    Fork,
}

impl Kind {
    /// The solid name's part: `BODY`, `BARS`, `FORK`.
    fn tag(self) -> &'static str {
        match self {
            Kind::Body => "BODY",
            Kind::Bars => "BARS",
            Kind::Fork => "FORK",
        }
    }
}

/// The name of the `n`th solid (1 is the first, `<ID>_KIT00_BODY_<L>`) of `kind` at the level of detail `letter`: the second is
/// `<ID>_KIT00_BODY2_<L>`.
pub fn solid_name(car: &str, kind: Kind, n: usize, letter: char) -> String {
    let tag = kind.tag();
    match n {
        0 | 1 => format!("{car}_KIT00_{tag}_{letter}"),
        _ => format!("{car}_KIT00_{tag}{n}_{letter}"),
    }
}

/// The names of the solids and the car they belong to.
#[derive(Clone, Copy)]
pub struct Names<'a> {
    /// The letter of the level of detail the solids are for.
    pub letter: char,
    pub car: &'a str,
    /// The ids of the parts built as solids of their own ([`crate::build::apart_ids`]); the body, bars and fork leave them out.
    pub apart: &'a [&'a str],
}

/// What one solid collects.
#[derive(Default)]
struct Half {
    groups: Vec<GroupSource>,
    points: Vec<[f32; 3]>,
    lists: Lists,
}

impl Half {
    fn add(
        &mut self,
        (table, surfaces, car): (&PartTable, &mut Surfaces, &str),
        (part, glb): (&Part, &Glb),
        to_solid: &dyn Fn([f32; 3]) -> [f32; 3],
        take: Take<'_>,
    ) -> Result<()> {
        let mut ctx = GroupContext { table, surfaces, car, lists: &mut self.lists };
        let placer = Placer { part, to_solid };
        self.groups.extend(part_groups(&mut ctx, glb, (&placer, take), &mut self.points)?);
        Ok(())
    }

    fn vertices(&self) -> usize {
        self.groups.iter().map(|g| g.vertices.len()).sum()
    }

    fn triangles(&self) -> usize {
        self.groups.iter().map(|g| g.indices.len() / 3).sum()
    }
}

/// The solids of `kind` made of `half`'s groups ([`pack`]: as many as they need), none for no groups; `None` when there are more than
/// [`MAX_BODY_SOLIDS`] or one does not fit.
fn solids_of(names: Names<'_>, kind: Kind, half: Half) -> Option<Vec<SolidSource>> {
    let packed = pack(half.groups);
    if packed.len() > MAX_BODY_SOLIDS {
        return None;
    }
    let mut out = Vec::new();
    for (n, groups) in packed.into_iter().enumerate() {
        let lists =
            Lists { textures: half.lists.textures.clone(), light_materials: half.lists.light_materials.clone() };
        out.push(finish(&solid_name(names.car, kind, n + 1, names.letter), groups, lists).ok()?);
    }
    Some(out)
}

/// The whole-bike vertex count above which a level is not tried: what the most solids of a body, the bars and the fork hold. The RC
/// 390's chassis has 174,000 vertices at level 0, which is four solids with the shading groups' copies of the vertices; the solids
/// themselves say whether a level fits.
const SPLIT_ESTIMATE_LIMIT: usize = (MAX_BODY_SOLIDS + 2) * crate::build::MAX_SOLID_VERTICES;

/// The body, bars and fork solids at the finest level of the chassis from `first_level` on that fits. For each level of the chassis
/// the small parts (exhaust, bar pad, mirrors) are tried one level finer than the chassis first, then at the chassis' level, then
/// ever coarser: a few thousand vertices of a silencer are worth less than the finer level of the whole body. `hub` is the front
/// hub in the chassis frame.
pub fn body_and_fork(
    table: &PartTable,
    exports: &Exports,
    surfaces: &mut Surfaces,
    names: Names<'_>,
    (frame, hub): (Frame, [f32; 3]),
    first_level: u32,
) -> Result<Pair> {
    let chassis = table.chassis()?;
    for level in first_level..=COARSEST_CHASSIS_LEVEL {
        let whole = vertex_estimate(&exports.load(&chassis.export, level)?, table, chassis);
        if whole > SPLIT_ESTIMATE_LIMIT {
            continue;
        }
        for part_level in level.saturating_sub(1)..=COARSEST_CHASSIS_LEVEL {
            let fitted = fit(table, exports, surfaces, names, (frame, hub), (level, part_level))?;
            if let Some(pair) = fitted {
                return Ok(pair);
            }
        }
    }
    anyhow::bail!(
        "{}: no level of detail of the chassis from {first_level} to {COARSEST_CHASSIS_LEVEL} fits its solids",
        solid_name(names.car, Kind::Body, 1, names.letter)
    )
}

/// The three solids with the chassis at `level` and the other parts at `part_level`, or `None` when one of them does not fit.
fn fit(
    table: &PartTable,
    exports: &Exports,
    surfaces: &mut Surfaces,
    names: Names<'_>,
    (frame, hub): (Frame, [f32; 3]),
    (level, part_level): (u32, u32),
) -> Result<Option<Pair>> {
    let chassis = table.chassis()?;
    let at = |part: &Part| if part.id == chassis.id { level } else { part_level };
    let mut loaded: Vec<(&Part, Glb)> = Vec::new();
    for part in table.parts_of(Role::Body).filter(|part| !names.apart.contains(&part.id.as_str())) {
        loaded.push((part, exports.load(&part.export, at(part))?));
    }
    let estimate: usize = loaded.iter().map(|(part, glb)| vertex_estimate(glb, table, part)).sum();
    if estimate > SPLIT_ESTIMATE_LIMIT {
        return Ok(None);
    }
    let chassis_glb =
        &loaded.iter().find(|(part, _)| part.id == chassis.id).context("the chassis is not a body part")?.1;
    let classes = front_end::classes(&chassis_glb.nodes, &table.front_end);
    let mut halves: [Half; 3] = Default::default();
    let to_body = |p: [f32; 3]| frame.body_point(p);
    let to_hub = |p: [f32; 3]| wheel_point(p, hub);
    for &(part, ref glb) in &loaded {
        let joints = front_end::joint_classes(glb, &classes);
        let side = front_end::side_of(part.anchor.bone.as_deref(), glb, &classes);
        for class in Class::ALL {
            let take = match side {
                Side::Whole(whole) if whole == class => Take::All,
                Side::Whole(_) => continue,
                Side::Split => Take::Of(class, &joints),
            };
            let to_solid: &dyn Fn([f32; 3]) -> [f32; 3] = match class {
                Class::Frame => &to_body,
                Class::Bars | Class::Fork => &to_hub,
            };
            halves[class.index()].add((table, &mut *surfaces, names.car), (part, glb), to_solid, take)?;
        }
    }
    let [body, bars, fork] = halves;
    let report = Report {
        body_vertices: body.vertices(),
        body_triangles: body.triangles(),
        fork_vertices: fork.vertices(),
        fork_triangles: fork.triangles(),
        bars_vertices: bars.vertices(),
        bars_triangles: bars.triangles(),
    };
    let (points, fork_points, bars_points) = (body.points.clone(), fork.points.clone(), bars.points.clone());
    let Some(bodies) = solids_of(names, Kind::Body, body).filter(|bodies| !bodies.is_empty()) else { return Ok(None) };
    let Some(fork_solids) = solids_of(names, Kind::Fork, fork) else { return Ok(None) };
    let Some(bars_solids) = solids_of(names, Kind::Bars, bars) else { return Ok(None) };
    Ok(Some(Pair {
        bodies,
        fork: fork_solids,
        bars: bars_solids,
        body_points: points,
        fork_points,
        bars_points,
        report,
        levels: [level, part_level],
    }))
}
