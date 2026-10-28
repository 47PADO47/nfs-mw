//! A binary glTF 2.0 file (`.glb`), read as far as the importer needs: triangle meshes with positions, normals, tangents,
//! one set of texture coordinates and the name of each primitive's material. Skinned meshes are read in their bind pose
//! (the exporter writes the vertices already placed), and node transforms are ignored: the part table says where a part
//! goes. What a skin adds is read as data and nothing more: the node tree (names and parents, so the bone hierarchy), the
//! joints of the first skin and each vertex's `JOINTS_0` and `WEIGHTS_0`, which `front_end.rs` uses to tell the parts of
//! the bike that steer from the rest. Textures are not read from the file: they come from the exporter's `tex` folder.

use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;

/// The joint indices and weights of the vertices of a primitive.
type Skin = (Vec<[u16; 4]>, Vec<[f32; 4]>);

const MAGIC: u32 = 0x4654_6C67;
const JSON_CHUNK: u32 = 0x4E4F_534A;
const BIN_CHUNK: u32 = 0x004E_4942;
const FLOAT: u64 = 5126;
const U8: u64 = 5121;
const U16: u64 = 5123;
const U32: u64 = 5125;

/// What a surface is called: the name of the material instance the exporter wrote.
pub struct Material {
    pub name: String,
}

/// A node of the file's scene graph; for a skinned mesh, a bone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub name: String,
    pub parent: Option<usize>,
}

/// One draw call of a mesh: a triangle list with its own vertices.
pub struct Primitive {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// Tangent xyz and handedness; empty when the file has none.
    pub tangents: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    /// Index into [`Glb::materials`].
    pub material: usize,
    /// The four joints that move each vertex (indices into [`Glb::joints`]) and their weights; both are empty when the
    /// primitive is not skinned.
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
}

impl Primitive {
    /// Whether every vertex has joints and weights.
    pub fn is_skinned(&self) -> bool {
        let count = self.positions.len();
        count > 0 && self.joints.len() == count && self.weights.len() == count
    }
}

pub struct Glb {
    pub primitives: Vec<Primitive>,
    pub materials: Vec<Material>,
    /// Every node of the file, in file order, with its parent.
    pub nodes: Vec<Node>,
    /// The node of each joint of the first skin (`JOINTS_0` values index this list); empty without a skin.
    pub joints: Vec<usize>,
}

impl Glb {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let (json, bin) = chunks(bytes)?;
        let doc: Value = serde_json::from_slice(json).context("the JSON chunk")?;
        let reader = Reader { doc: &doc, bin };
        let materials = reader.materials();
        let nodes = reader.nodes();
        let joints = reader.joints(nodes.len())?;
        let primitives = reader.primitives(materials.len(), joints.len())?;
        Ok(Self { primitives, materials, nodes, joints })
    }

    /// The name of the bone joint `joint` stands for.
    pub fn joint_name(&self, joint: usize) -> Option<&str> {
        self.nodes.get(*self.joints.get(joint)?).map(|node| node.name.as_str())
    }

    /// Whether any primitive is skinned.
    pub fn is_skinned(&self) -> bool {
        self.primitives.iter().any(Primitive::is_skinned)
    }
}

fn chunks(bytes: &[u8]) -> Result<(&[u8], &[u8])> {
    ensure!(u32_at(bytes, 0)? == MAGIC, "not a glb file");
    ensure!(u32_at(bytes, 4)? == 2, "not glTF version 2");
    let (mut json, mut bin) = (None, &bytes[..0]);
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let length = u32_at(bytes, at)? as usize;
        let kind = u32_at(bytes, at + 4)?;
        let body = bytes.get(at + 8..at + 8 + length).context("a chunk runs past the end of the file")?;
        match kind {
            JSON_CHUNK => json = Some(body),
            BIN_CHUNK => bin = body,
            _ => {}
        }
        at += 8 + length.next_multiple_of(4);
    }
    Ok((json.context("no JSON chunk")?, bin))
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32> {
    let slice = bytes.get(at..at + 4).context("the file is cut short")?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

struct Reader<'a> {
    doc: &'a Value,
    bin: &'a [u8],
}

impl Reader<'_> {
    fn list(&self, key: &str) -> &[Value] {
        self.doc.get(key).and_then(Value::as_array).map_or(&[], Vec::as_slice)
    }

    fn view(&self, index: usize) -> Result<(&[u8], Option<usize>)> {
        let view = self.list("bufferViews").get(index).with_context(|| format!("no buffer view {index}"))?;
        let offset = view.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let length = view.get("byteLength").and_then(Value::as_u64).context("a buffer view has no length")? as usize;
        let stride = view.get("byteStride").and_then(Value::as_u64).map(|s| s as usize);
        let data = self.bin.get(offset..offset + length).context("a buffer view runs past the binary chunk")?;
        Ok((data, stride))
    }

    fn materials(&self) -> Vec<Material> {
        let mut materials: Vec<Material> = self
            .list("materials")
            .iter()
            .map(|material| Material {
                name: material.get("name").and_then(Value::as_str).unwrap_or("material").to_owned(),
            })
            .collect();
        // A primitive without a material points one past the list.
        materials.push(Material { name: "none".into() });
        materials
    }

    /// The scene graph: each node's name and parent, from the `children` lists.
    fn nodes(&self) -> Vec<Node> {
        let list = self.list("nodes");
        let mut nodes: Vec<Node> = list
            .iter()
            .map(|node| Node { name: node.get("name").and_then(Value::as_str).unwrap_or("").to_owned(), parent: None })
            .collect();
        for (parent, node) in list.iter().enumerate() {
            let children = node.get("children").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
            for child in children.iter().filter_map(Value::as_u64) {
                if let Some(child) = nodes.get_mut(child as usize) {
                    child.parent = Some(parent);
                }
            }
        }
        nodes
    }

    /// The nodes of the joints of the first skin; each must be a node of the file.
    fn joints(&self, nodes: usize) -> Result<Vec<usize>> {
        let Some(skin) = self.list("skins").first() else { return Ok(Vec::new()) };
        let joints = skin.get("joints").and_then(Value::as_array).context("a skin has no joints")?;
        let joints: Vec<usize> = joints.iter().filter_map(Value::as_u64).map(|n| n as usize).collect();
        ensure!(joints.iter().all(|&n| n < nodes), "a joint is not a node of the file");
        Ok(joints)
    }

    fn primitives(&self, materials: usize, joints: usize) -> Result<Vec<Primitive>> {
        let mut out = Vec::new();
        for mesh in self.list("meshes") {
            let primitives = mesh.get("primitives").and_then(Value::as_array).context("a mesh has no primitives")?;
            for primitive in primitives {
                out.push(self.primitive(primitive, materials, joints)?);
            }
        }
        Ok(out)
    }

    /// `JOINTS_0` and `WEIGHTS_0` of a primitive; empty lists when it lacks either or the file has no skin.
    fn skin(&self, attributes: &Value, count: usize, joints: usize) -> Result<Skin> {
        let accessor = |name: &str| attributes.get(name).and_then(Value::as_u64).map(|n| n as usize);
        let (Some(j), Some(w)) = (accessor("JOINTS_0"), accessor("WEIGHTS_0")) else {
            return Ok((Vec::new(), Vec::new()));
        };
        if joints == 0 {
            return Ok((Vec::new(), Vec::new()));
        }
        let (indices, weights) = (self.numbers(j, 4, false)?, self.numbers(w, 4, true)?);
        ensure!(indices.len() == count * 4 && weights.len() == count * 4, "the skin attributes disagree in length");
        for (joint, weight) in indices.iter().zip(&weights) {
            ensure!(
                *weight <= 0.0 || (*joint as usize) < joints,
                "a vertex is skinned to a joint the skin does not have"
            );
        }
        let indices = indices.as_chunks::<4>().0.iter().map(|c| c.map(|v| v as u16)).collect();
        Ok((indices, weights.as_chunks::<4>().0.to_vec()))
    }

    fn primitive(&self, primitive: &Value, materials: usize, joints: usize) -> Result<Primitive> {
        let attributes = primitive.get("attributes").context("a primitive has no attributes")?;
        let accessor = |name: &str| attributes.get(name).and_then(Value::as_u64).map(|n| n as usize);
        let positions = self.floats(accessor("POSITION").context("a primitive has no positions")?, 3)?;
        let count = positions.len() / 3;
        let normals = match accessor("NORMAL") {
            Some(n) => self.floats(n, 3)?,
            None => [0.0, 1.0, 0.0].repeat(count),
        };
        let uvs = match accessor("TEXCOORD_0") {
            Some(n) => self.floats(n, 2)?,
            None => vec![0.0; count * 2],
        };
        ensure!(normals.len() == positions.len() && uvs.len() == count * 2, "the vertex attributes disagree in length");
        let tangents = match accessor("TANGENT") {
            Some(n) => self.floats(n, 4)?,
            None => Vec::new(),
        };
        ensure!(tangents.is_empty() || tangents.len() == count * 4, "the tangents disagree in length");
        let indices = match primitive.get("indices").and_then(Value::as_u64) {
            Some(n) => self.indices(n as usize)?,
            None => (0..count as u32).collect(),
        };
        ensure!(indices.len() % 3 == 0 && indices.iter().all(|&i| (i as usize) < count), "bad triangle indices");
        let material = primitive.get("material").and_then(Value::as_u64).map_or(materials - 1, |m| m as usize);
        let (joint_indices, weights) = self.skin(attributes, count, joints)?;
        Ok(Primitive {
            positions: positions.as_chunks::<3>().0.to_vec(),
            normals: normals.as_chunks::<3>().0.to_vec(),
            uvs: uvs.as_chunks::<2>().0.to_vec(),
            tangents: tangents.as_chunks::<4>().0.to_vec(),
            indices,
            material: material.min(materials - 1),
            joints: joint_indices,
            weights,
        })
    }

    /// The values of a float accessor, `width` to a vertex.
    fn floats(&self, index: usize, width: usize) -> Result<Vec<f32>> {
        let (accessor, data, stride) = self.accessor(index)?;
        ensure!(accessor.component == FLOAT, "accessor {index} is not float");
        let element = width * 4;
        let step = stride.unwrap_or(element);
        let mut out = Vec::with_capacity(accessor.count * width);
        for n in 0..accessor.count {
            let at = accessor.offset + n * step;
            let bytes = data.get(at..at + element).context("an accessor runs past its buffer view")?;
            out.extend(bytes.as_chunks::<4>().0.iter().map(|&b| f32::from_le_bytes(b)));
        }
        Ok(out)
    }

    /// The values of an accessor of `width` components per element, as floats, whatever its component type (float,
    /// unsigned byte or short). `JOINTS_0` is unsigned integers; `WEIGHTS_0` is floats or normalised integers, which are
    /// divided by their largest value when `normalise` is set.
    fn numbers(&self, index: usize, width: usize, normalise: bool) -> Result<Vec<f32>> {
        let (accessor, data, stride) = self.accessor(index)?;
        let (size, largest) = match accessor.component {
            FLOAT => (4, 1.0),
            U8 => (1, 255.0),
            U16 => (2, 65535.0),
            other => bail!("component type {other} is not supported for skin data"),
        };
        let step = stride.unwrap_or(size * width);
        let mut out = Vec::with_capacity(accessor.count * width);
        for n in 0..accessor.count {
            for k in 0..width {
                let at = accessor.offset + n * step + k * size;
                let bytes = data.get(at..at + size).context("an accessor runs past its buffer view")?;
                let value = match accessor.component {
                    FLOAT => f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
                    U8 => f32::from(bytes[0]),
                    _ => f32::from(u16::from_le_bytes([bytes[0], bytes[1]])),
                };
                out.push(if normalise && accessor.component != FLOAT { value / largest } else { value });
            }
        }
        Ok(out)
    }

    fn indices(&self, index: usize) -> Result<Vec<u32>> {
        let (accessor, data, stride) = self.accessor(index)?;
        let size = match accessor.component {
            U8 => 1,
            U16 => 2,
            U32 => 4,
            other => bail!("index component type {other} is not supported"),
        };
        let step = stride.unwrap_or(size);
        let mut out = Vec::with_capacity(accessor.count);
        for n in 0..accessor.count {
            let at = accessor.offset + n * step;
            let bytes = data.get(at..at + size).context("the indices run past their buffer view")?;
            let mut value = [0u8; 4];
            value[..size].copy_from_slice(bytes);
            out.push(u32::from_le_bytes(value));
        }
        Ok(out)
    }

    fn accessor(&self, index: usize) -> Result<(Accessor, &[u8], Option<usize>)> {
        let accessor = self.list("accessors").get(index).with_context(|| format!("no accessor {index}"))?;
        let view = accessor.get("bufferView").and_then(Value::as_u64).context("a sparse accessor is not supported")?;
        let (data, stride) = self.view(view as usize)?;
        let field = |key: &str| accessor.get(key).and_then(Value::as_u64).unwrap_or(0);
        let parsed = Accessor {
            component: field("componentType"),
            count: field("count") as usize,
            offset: field("byteOffset") as usize,
        };
        Ok((parsed, data, stride))
    }
}

struct Accessor {
    component: u64,
    count: usize,
    offset: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A glb file from its JSON text and binary chunk.
    fn wrap(json: &str, bin: &[u8]) -> Vec<u8> {
        let mut json = json.as_bytes().to_vec();
        while json.len() % 4 != 0 {
            json.push(b' ');
        }
        let mut bin = bin.to_vec();
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let total = 12 + 8 + json.len() + 8 + bin.len();
        let mut file = Vec::new();
        file.extend(MAGIC.to_le_bytes());
        file.extend(2u32.to_le_bytes());
        file.extend((total as u32).to_le_bytes());
        file.extend((json.len() as u32).to_le_bytes());
        file.extend(JSON_CHUNK.to_le_bytes());
        file.extend(&json);
        file.extend((bin.len() as u32).to_le_bytes());
        file.extend(BIN_CHUNK.to_le_bytes());
        file.extend(&bin);
        file
    }

    /// One triangle with positions only, as a glb file.
    fn triangle() -> Vec<u8> {
        let mut bin = Vec::new();
        for v in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bin.extend(v.to_le_bytes());
        }
        for i in [0u16, 1, 2] {
            bin.extend(i.to_le_bytes());
        }
        bin.extend([0, 0]);
        let json = r#"{"asset":{"version":"2.0"},"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":6}],
            "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"},{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1}]}]}"#;
        wrap(json, &bin)
    }

    /// One triangle skinned to a two-joint skin: nodes 0 (`root`) > 1 (`arm`), the skin's joints are `[1, 0]`. Vertex 0 is
    /// all on joint 0 (`arm`), vertex 1 is split half and half, vertex 2 is all on joint 1 (`root`). Joints are unsigned
    /// bytes and weights normalised unsigned shorts, as exporters write them.
    fn skinned_triangle() -> Vec<u8> {
        let mut bin = Vec::new();
        for v in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bin.extend(v.to_le_bytes());
        }
        for i in [0u16, 1, 2] {
            bin.extend(i.to_le_bytes());
        }
        bin.extend([0, 0]);
        bin.extend([0u8, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0]);
        for w in [[65535u16, 0, 0, 0], [32768, 32767, 0, 0], [65535, 0, 0, 0]] {
            for v in w {
                bin.extend(v.to_le_bytes());
            }
        }
        let json = r#"{"asset":{"version":"2.0"},
            "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":6},
                           {"buffer":0,"byteOffset":44,"byteLength":12},{"buffer":0,"byteOffset":56,"byteLength":24}],
            "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"},{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"},
                         {"bufferView":2,"componentType":5121,"count":3,"type":"VEC4"},{"bufferView":3,"componentType":5123,"normalized":true,"count":3,"type":"VEC4"}],
            "nodes":[{"name":"root","children":[1]},{"name":"arm"},{"mesh":0,"skin":0}],
            "skins":[{"joints":[1,0]}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0,"JOINTS_0":2,"WEIGHTS_0":3},"indices":1}]}]}"#;
        wrap(json, &bin)
    }

    #[test]
    fn a_triangle_reads_back() {
        let glb = Glb::parse(&triangle()).unwrap();
        assert_eq!(glb.primitives.len(), 1);
        let p = &glb.primitives[0];
        assert_eq!(p.positions, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
        assert_eq!(p.indices, vec![0, 1, 2]);
        assert_eq!(p.normals.len(), 3);
        assert_eq!(p.material, 0);
        assert!(!p.is_skinned() && !glb.is_skinned(), "a mesh without a skin has no joints");
        assert!(glb.nodes.is_empty() && glb.joints.is_empty());
    }

    #[test]
    fn a_skin_gives_the_bone_tree_the_joints_and_each_vertexs_weights() {
        let glb = Glb::parse(&skinned_triangle()).unwrap();
        let names: Vec<_> = glb.nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, ["root", "arm", ""]);
        assert_eq!(glb.nodes[1].parent, Some(0));
        assert_eq!(glb.nodes[0].parent, None);
        assert_eq!(glb.joints, vec![1, 0]);
        assert_eq!(glb.joint_name(0), Some("arm"));
        assert_eq!(glb.joint_name(1), Some("root"));
        assert_eq!(glb.joint_name(2), None);
        let p = &glb.primitives[0];
        assert!(p.is_skinned() && glb.is_skinned());
        assert_eq!(p.joints, vec![[0, 0, 0, 0], [0, 1, 0, 0], [1, 0, 0, 0]]);
        assert!((p.weights[0][0] - 1.0).abs() < 1e-6);
        assert!((p.weights[1][0] - 0.5).abs() < 1e-3 && (p.weights[1][1] - 0.5).abs() < 1e-3, "{:?}", p.weights[1]);
    }

    #[test]
    fn a_vertex_skinned_to_a_missing_joint_is_refused() {
        let mut file = skinned_triangle();
        // Joint 5 for vertex 0 (the joints start 44 bytes into the binary chunk, 12 + 8 + json + 8 into the file).
        let json_length = u32::from_le_bytes([file[12], file[13], file[14], file[15]]) as usize;
        file[12 + 8 + json_length + 8 + 44] = 5;
        assert!(Glb::parse(&file).is_err());
    }

    #[test]
    fn other_files_are_refused() {
        assert!(Glb::parse(b"not a glb file at all").is_err());
    }
}
