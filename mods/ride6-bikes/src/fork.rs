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
    pub body: SolidSource,
    /// `None` when nothing slides (the chassis is not skinned, or no bone of the rules is in it).
    pub fork: Option<SolidSource>,
    /// `None` when nothing steers without sliding.
    pub bars: Option<SolidSource>,
    /// The vertices of the body, in the body solid's frame, for the bounds.
    pub body_points: Vec<[f32; 3]>,
    /// The vertices of the fork and of the bars, in their solids' frame (the front hub's).
    pub fork_points: Vec<[f32; 3]>,
    pub bars_points: Vec<[f32; 3]>,
    pub report: Report,
}

/// The names of the two solids and the car they belong to.
#[derive(Clone, Copy)]
pub struct Names<'a> {
    pub body: &'a str,
    pub fork: &'a str,
    pub bars: &'a str,
    pub car: &'a str,
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

/// A solid that may have nothing in it: `None` for no groups, else [`finish`].
fn optional(name: &str, groups: Vec<GroupSource>, lists: Lists) -> Result<Option<SolidSource>> {
    if groups.is_empty() {
        return Ok(None);
    }
    finish(name, groups, lists).map(Some)
}

/// The body, bars and fork solids at the finest level of the chassis from `first_level` on whose parts fit one solid. Smaller
/// parts (exhaust, bar pad) are read one level finer than the chassis. `hub` is the front hub in the chassis frame.
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
        let part_level = level.saturating_sub(1);
        let at = |part: &Part| if part.id == chassis.id { level } else { part_level };
        let mut loaded: Vec<(&Part, Glb)> = Vec::new();
        for part in table.parts_of(Role::Body) {
            loaded.push((part, exports.load(&part.export, at(part))?));
        }
        let estimate: usize = loaded.iter().map(|(part, glb)| vertex_estimate(glb, table, part)).sum();
        if estimate > crate::build::MAX_SOLID_VERTICES {
            continue;
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
        let Ok(body_solid) = finish(names.body, body.groups, body.lists) else { continue };
        let Ok(fork_solid) = optional(names.fork, fork.groups, fork.lists) else { continue };
        let Ok(bars_solid) = optional(names.bars, bars.groups, bars.lists) else { continue };
        return Ok(Pair {
            body: body_solid,
            fork: fork_solid,
            bars: bars_solid,
            body_points: body.points,
            fork_points: fork.points,
            bars_points: bars.points,
            report,
        });
    }
    anyhow::bail!(
        "{}: no level of detail of the chassis from {first_level} to {COARSEST_CHASSIS_LEVEL} fits one solid",
        names.body
    )
}
