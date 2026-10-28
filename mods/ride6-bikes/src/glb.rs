//! A binary glTF 2.0 file (`.glb`), read as far as the importer needs: triangle meshes with positions, normals, one set of
//! texture coordinates and the base colour map or colour of each material. Skinned meshes are read in their bind pose
//! (the exporter writes the vertices already placed), and node transforms are ignored: the parts the importer reads have
//! none.

use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;

const MAGIC: u32 = 0x4654_6C67;
const JSON_CHUNK: u32 = 0x4E4F_534A;
const BIN_CHUNK: u32 = 0x004E_4942;
const FLOAT: u64 = 5126;
const U8: u64 = 5121;
const U16: u64 = 5123;
const U32: u64 = 5125;

/// A PNG picture stored in the file.
pub struct Image {
    pub name: String,
    pub png: Vec<u8>,
}

/// What a surface looks like: a picture, or failing that a colour.
pub struct Material {
    pub name: String,
    pub image: Option<usize>,
    /// Linear RGBA factor of the base colour (white when the file gives none).
    pub color: [f32; 4],
}

/// One draw call of a mesh: a triangle list with its own vertices.
pub struct Primitive {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    /// Index into [`Glb::materials`].
    pub material: usize,
}

pub struct Glb {
    pub primitives: Vec<Primitive>,
    pub materials: Vec<Material>,
    pub images: Vec<Image>,
}

impl Glb {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let (json, bin) = chunks(bytes)?;
        let doc: Value = serde_json::from_slice(json).context("the JSON chunk")?;
        let reader = Reader { doc: &doc, bin };
        let images = reader.images()?;
        let materials = reader.materials();
        let primitives = reader.primitives(materials.len())?;
        Ok(Self { primitives, materials, images })
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

    fn images(&self) -> Result<Vec<Image>> {
        let mut images = Vec::new();
        for (n, image) in self.list("images").iter().enumerate() {
            let name = image.get("name").and_then(Value::as_str).map_or_else(|| format!("image{n}"), str::to_owned);
            let view = image.get("bufferView").and_then(Value::as_u64);
            let Some(view) = view else { bail!("image {name} is not stored in the file") };
            images.push(Image { name, png: self.view(view as usize)?.0.to_vec() });
        }
        Ok(images)
    }

    fn materials(&self) -> Vec<Material> {
        let textures = self.list("textures");
        let mut materials: Vec<Material> = self
            .list("materials")
            .iter()
            .map(|material| {
                let pbr = material.get("pbrMetallicRoughness");
                let texture =
                    pbr.and_then(|p| p.get("baseColorTexture")).and_then(|t| t.get("index")).and_then(Value::as_u64);
                let image = texture
                    .and_then(|index| textures.get(index as usize))
                    .and_then(|texture| texture.get("source"))
                    .and_then(Value::as_u64)
                    .map(|source| source as usize);
                let color = pbr
                    .and_then(|p| p.get("baseColorFactor"))
                    .and_then(Value::as_array)
                    .filter(|factor| factor.len() == 4)
                    .map(|factor| std::array::from_fn(|n| factor[n].as_f64().unwrap_or(1.0) as f32))
                    .unwrap_or([1.0; 4]);
                let name = material.get("name").and_then(Value::as_str).unwrap_or("material").to_owned();
                Material { name, image, color }
            })
            .collect();
        // A primitive without a material points one past the list.
        materials.push(Material { name: "none".into(), image: None, color: [1.0; 4] });
        materials
    }

    fn primitives(&self, materials: usize) -> Result<Vec<Primitive>> {
        let mut out = Vec::new();
        for mesh in self.list("meshes") {
            let primitives = mesh.get("primitives").and_then(Value::as_array).context("a mesh has no primitives")?;
            for primitive in primitives {
                out.push(self.primitive(primitive, materials)?);
            }
        }
        Ok(out)
    }

    fn primitive(&self, primitive: &Value, materials: usize) -> Result<Primitive> {
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
        let indices = match primitive.get("indices").and_then(Value::as_u64) {
            Some(n) => self.indices(n as usize)?,
            None => (0..count as u32).collect(),
        };
        ensure!(indices.len() % 3 == 0 && indices.iter().all(|&i| (i as usize) < count), "bad triangle indices");
        let material = primitive.get("material").and_then(Value::as_u64).map_or(materials - 1, |m| m as usize);
        Ok(Primitive {
            positions: positions.as_chunks::<3>().0.to_vec(),
            normals: normals.as_chunks::<3>().0.to_vec(),
            uvs: uvs.as_chunks::<2>().0.to_vec(),
            indices,
            material: material.min(materials - 1),
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
        let mut json = json.as_bytes().to_vec();
        while json.len() % 4 != 0 {
            json.push(b' ');
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

    #[test]
    fn a_triangle_reads_back() {
        let glb = Glb::parse(&triangle()).unwrap();
        assert_eq!(glb.primitives.len(), 1);
        let p = &glb.primitives[0];
        assert_eq!(p.positions, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
        assert_eq!(p.indices, vec![0, 1, 2]);
        assert_eq!(p.normals.len(), 3);
        assert_eq!(p.material, 0);
    }

    #[test]
    fn other_files_are_refused() {
        assert!(Glb::parse(b"not a glb file at all").is_err());
    }
}
