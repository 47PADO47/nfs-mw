//! The shading groups of a part: its exported surfaces, moved onto the chassis frame, split by light material.

use std::collections::BTreeMap;

use anyhow::Result;
use blackbox_hash::bstring_hash;
use blackbox_solid::{GroupSource, Vertex};

use crate::bake::Surfaces;
use crate::front_end::Take;
use crate::glass;
use crate::glb::{Glb, Primitive};
use crate::layout::direction_to_car;
use crate::partfile::{Material, Part, PartTable};
use crate::shading;

/// Group flags of the stock car groups (`blackbox_solid::COMMON_GROUP_FLAGS`).
const GROUP_FLAGS: u32 = 0x4180;
/// The effect of normal-mapped surfaces (`blackbox_solid` layout version 0x16, `WorldNormalMap`).
const BUMP_EFFECT: u32 = 3;

/// The textures and light materials a solid lists, in the order its groups first use them.
#[derive(Default)]
pub struct Lists {
    pub textures: Vec<u32>,
    pub light_materials: Vec<u32>,
}

impl Lists {
    fn texture(&mut self, name: &str) -> u8 {
        slot_of(&mut self.textures, bstring_hash(name))
    }

    fn light_material(&mut self, name: &str) -> u8 {
        slot_of(&mut self.light_materials, bstring_hash(name))
    }
}

/// The index of `hash` in `list`, adding it when it is new.
pub fn slot_of(list: &mut Vec<u32>, hash: u32) -> u8 {
    let at = list.iter().position(|&known| known == hash).unwrap_or_else(|| {
        list.push(hash);
        list.len() - 1
    });
    at as u8
}

/// Where a part's vertices go: the part's own frame to the solid's, point by point.
pub struct Placer<'a> {
    pub part: &'a Part,
    /// A point of the chassis frame as a point of the solid.
    pub to_solid: &'a dyn Fn([f32; 3]) -> [f32; 3],
}

impl Placer<'_> {
    fn point(&self, p: [f32; 3]) -> [f32; 3] {
        (self.to_solid)(self.part.anchor.point(p))
    }

    fn direction(&self, d: [f32; 3]) -> [f32; 3] {
        let r = direction_to_car(self.part.anchor.direction(d));
        let length = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
        if length < 1e-9 {
            return [0.0, 0.0, 1.0];
        }
        r.map(|c| c / length)
    }
}

/// Everything the groups of one solid are built with.
pub struct Context<'a> {
    pub table: &'a PartTable,
    pub surfaces: &'a mut Surfaces,
    pub car: &'a str,
    pub lists: &'a mut Lists,
}

/// The vertex count a part at some level adds to a solid before groups are split by light material.
pub fn vertex_estimate(glb: &Glb, table: &PartTable, part: &Part) -> usize {
    glb.primitives
        .iter()
        .filter(|p| {
            let name = &glb.materials[p.material].name;
            !table.material(part, name).is_some_and(Material::is_glass_decal)
        })
        .map(|p| p.positions.len())
        .sum()
}

/// The groups of `part` read from `glb`, of the triangles `take` selects (a skinned part is split between the body and the
/// fork, `front_end.rs`). Triangles of one surface that take different light materials become different groups, each with
/// its own copy of the vertices it uses. Positions of the groups go to `points` as well.
pub fn part_groups(
    ctx: &mut Context<'_>,
    glb: &Glb,
    (placer, take): (&Placer<'_>, Take<'_>),
    points: &mut Vec<[f32; 3]>,
) -> Result<Vec<GroupSource>> {
    let mut groups = Vec::new();
    for primitive in &glb.primitives {
        if primitive.indices.is_empty() {
            continue;
        }
        let selected = take.triangles(primitive);
        if selected.is_empty() {
            continue;
        }
        let name = &glb.materials[primitive.material].name;
        let found = ctx.table.material(placer.part, name).cloned();
        let material = found.unwrap_or_else(|| Material { name: name.clone(), ..Material::default() });
        if material.is_glass_decal() {
            continue;
        }
        let surface = ctx.surfaces.surface(ctx.car, &material, placer.part.kind)?;
        let bump = surface.normal.is_some() && !primitive.tangents.is_empty();
        let mut classes: BTreeMap<&'static str, Vec<[u32; 3]>> = BTreeMap::new();
        for triangle in &selected {
            let uvs = triangle.map(|i| primitive.uvs[i as usize]);
            let class = match &surface.glass {
                Some(glass) => glass::classify(&glass.picture, glass.max, uvs),
                None => shading::classify(placer.part.kind, surface.pbr.as_deref(), uvs),
            };
            classes.entry(class).or_default().push(*triangle);
        }
        for (class, triangles) in classes {
            let (vertices, tangents, indices, used) = split(primitive, placer, &triangles, bump);
            points.extend(vertices.iter().map(|v| v.position));
            let mut group = GroupSource::new(vertices, indices);
            let diffuse = ctx.lists.texture(&surface.diffuse);
            group.texture_slots = [diffuse; 5];
            if bump {
                group.effect_id = BUMP_EFFECT;
                group.tangents = tangents;
                let normal = surface.normal.as_deref().map_or(diffuse, |n| ctx.lists.texture(n));
                let specular = surface.specular.as_deref().map_or(diffuse, |s| ctx.lists.texture(s));
                group.texture_slots = [diffuse, normal, diffuse, specular, diffuse];
            }
            group.light_material = ctx.lists.light_material(class);
            group.flags = GROUP_FLAGS;
            debug_assert!(used > 0);
            groups.push(group);
        }
    }
    Ok(groups)
}

/// The vertices (and tangents, when `bump`) the triangles use, placed, with indices into them, and how many there are.
fn split(
    primitive: &Primitive,
    placer: &Placer<'_>,
    triangles: &[[u32; 3]],
    bump: bool,
) -> (Vec<Vertex>, Vec<[f32; 4]>, Vec<u16>, usize) {
    let mut remap: std::collections::HashMap<u32, u16> = std::collections::HashMap::new();
    let (mut vertices, mut tangents, mut indices) = (Vec::new(), Vec::new(), Vec::new());
    for &old in triangles.iter().flatten() {
        let new = *remap.entry(old).or_insert_with(|| {
            let i = old as usize;
            vertices.push(Vertex {
                position: placer.point(primitive.positions[i]),
                normal: placer.direction(primitive.normals[i]),
                color_bgra: [255; 4],
                uv: primitive.uvs[i],
            });
            if bump {
                let [x, y, z, w] = primitive.tangents[i];
                let [tx, ty, tz] = placer.direction([x, y, z]);
                tangents.push([tx, ty, tz, w]);
            }
            (vertices.len() - 1) as u16
        });
        indices.push(new);
    }
    let used = vertices.len();
    (vertices, tangents, indices, used)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::partfile::{Anchor, Kind, Role};

    fn part(translation: [f32; 3]) -> Part {
        Part {
            id: "front_rim".into(),
            kind: Kind::Rim,
            role: Role::FrontWheel,
            export: "x".into(),
            anchor: Anchor { bone: None, translation, rotation: [0.0, 0.0, 0.0, 1.0] },
            materials: Default::default(),
        }
    }

    #[test]
    fn slots_are_reused() {
        let mut list = Vec::new();
        assert_eq!(slot_of(&mut list, 5), 0);
        assert_eq!(slot_of(&mut list, 9), 1);
        assert_eq!(slot_of(&mut list, 5), 0);
    }

    #[test]
    fn a_placer_moves_by_the_anchor_then_the_solid_and_turns_directions() {
        let p = part([1.0, 0.0, 0.0]);
        let to_solid = |q: [f32; 3]| [q[0] - 1.0, -q[2], q[1]];
        let placer = Placer { part: &p, to_solid: &to_solid };
        assert_eq!(placer.point([0.0, 0.5, 0.25]), [0.0, -0.25, 0.5]);
        assert_eq!(placer.direction([0.0, 2.0, 0.0]), [0.0, 0.0, 1.0]);
        assert_eq!(placer.direction([0.0, 0.0, 0.0]), [0.0, 0.0, 1.0], "a zero normal points up rather than NaN");
    }

    #[test]
    fn a_split_copies_only_the_vertices_its_triangles_use() {
        let primitive = Primitive {
            positions: vec![
                [0.0; 3],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [5.0, 5.0, 5.0],
                [6.0, 5.0, 5.0],
                [5.0, 6.0, 5.0],
            ],
            normals: vec![[0.0, 1.0, 0.0]; 6],
            uvs: vec![[0.0; 2]; 6],
            tangents: (0..6).map(|i| [1.0, 0.0, 0.0, if i % 2 == 0 { 1.0 } else { -1.0 }]).collect(),
            indices: vec![0, 1, 2, 3, 4, 5],
            material: 0,
            joints: Vec::new(),
            weights: Vec::new(),
        };
        let p = part([0.0; 3]);
        let to_solid = |q: [f32; 3]| q;
        let placer = Placer { part: &p, to_solid: &to_solid };
        let (vertices, tangents, indices, used) = split(&primitive, &placer, &[[3, 4, 5]], true);
        assert_eq!((vertices.len(), used), (3, 3));
        assert_eq!(indices, vec![0, 1, 2]);
        assert_eq!(vertices[0].position, [5.0, 5.0, 5.0]);
        assert_eq!(tangents.iter().map(|t| t[3]).collect::<Vec<_>>(), vec![-1.0, 1.0, -1.0]);
        let (_, none, ..) = split(&primitive, &placer, &[[0, 1, 2]], false);
        assert!(none.is_empty());
    }
}
