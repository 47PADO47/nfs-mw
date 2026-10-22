//! A software compositor for the tests: what the presenter put in the frame's UI layer (the meshes and the textures it
//! uploaded) drawn into an image, with no window and no GPU, to look at a HUD as the player would. It draws the meshes
//! and leaves out the masked pictures (`UiLayer::compositions`: the minimap tiles and the nitrous bar), which the
//! renderer builds on the GPU.

use std::collections::HashMap;
use std::path::Path;

use crate::gui::UiOutput;

/// Uploaded textures by id: size and premultiplied RGBA.
#[derive(Default)]
pub struct Textures(HashMap<u64, (u32, u32, Vec<u8>)>);

impl Textures {
    /// Takes the uploads of a frame (a texture uploaded once stays).
    pub fn take(&mut self, out: &mut UiOutput) {
        for patch in out.patches.drain(..) {
            if patch.offset.is_none() {
                self.0.insert(patch.id.0, (patch.size[0], patch.size[1], patch.rgba));
            }
        }
    }
}

pub struct Canvas {
    pub width: u32,
    pub height: u32,
    /// Premultiplied, 0..1.
    pixels: Vec<[f32; 4]>,
}

impl Canvas {
    /// A canvas of one flat colour (the world's place).
    pub fn new(width: u32, height: u32, background: [u8; 3]) -> Self {
        let [r, g, b] = background.map(|c| f32::from(c) / 255.0);
        Self { width, height, pixels: vec![[r, g, b, 1.0]; (width * height) as usize] }
    }

    /// Draws the layer's meshes in order, later ones on top.
    pub fn draw(&mut self, out: &UiOutput, textures: &Textures) {
        let Some(layer) = out.layer.as_ref() else { return };
        for mesh in &layer.meshes {
            let Some((tw, th, texels)) = textures.0.get(&mesh.texture.0) else { continue };
            for triangle in mesh.indices.as_chunks::<3>().0 {
                let v = [0, 1, 2].map(|k| &mesh.vertices[triangle[k] as usize]);
                self.triangle(v.map(|v| (v.position, v.uv, v.color_rgba)), (*tw, *th, texels));
            }
        }
    }

    fn triangle(&mut self, v: [([f32; 2], [f32; 2], [u8; 4]); 3], texture: (u32, u32, &[u8])) {
        let (tw, th, texels) = texture;
        let [a, b, c] = v.map(|v| v.0);
        let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        if area.abs() < 1e-6 {
            return;
        }
        let min_x = a[0].min(b[0]).min(c[0]).floor().max(0.0) as i32;
        let max_x = a[0].max(b[0]).max(c[0]).ceil().min(self.width as f32 - 1.0) as i32;
        let min_y = a[1].min(b[1]).min(c[1]).floor().max(0.0) as i32;
        let max_y = a[1].max(b[1]).max(c[1]).ceil().min(self.height as f32 - 1.0) as i32;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let p = [x as f32 + 0.5, y as f32 + 0.5];
                let w0 = ((b[0] - p[0]) * (c[1] - p[1]) - (b[1] - p[1]) * (c[0] - p[0])) / area;
                let w1 = ((c[0] - p[0]) * (a[1] - p[1]) - (c[1] - p[1]) * (a[0] - p[0])) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let weights = [w0, w1, w2];
                let uv: [f32; 2] = std::array::from_fn(|k| (0..3).map(|i| v[i].1[k] * weights[i]).sum());
                let tx = ((uv[0] * tw as f32) as i32).clamp(0, tw as i32 - 1) as usize;
                let ty = ((uv[1] * th as f32) as i32).clamp(0, th as i32 - 1) as usize;
                let at = (ty * tw as usize + tx) * 4;
                let colour: [f32; 4] = std::array::from_fn(|k| (0..3).map(|i| f32::from(v[i].2[k]) * weights[i]).sum());
                let src: [f32; 4] = std::array::from_fn(|k| f32::from(texels[at + k]) / 255.0 * colour[k] / 255.0);
                let dst = &mut self.pixels[y as usize * self.width as usize + x as usize];
                for k in 0..3 {
                    dst[k] = (src[k] + dst[k] * (1.0 - src[3])).min(1.0);
                }
            }
        }
    }

    /// The pixel at a point, as 8-bit RGB.
    pub fn rgb(&self, x: u32, y: u32) -> [u8; 3] {
        let p = self.pixels[(y * self.width + x) as usize];
        [p[0], p[1], p[2]].map(|c| (c * 255.0 + 0.5) as u8)
    }

    /// Writes the image as a PNG.
    pub fn save(&self, path: &Path) {
        let file = std::fs::File::create(path).expect("the image file");
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), self.width, self.height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("the PNG header");
        let mut bytes = Vec::with_capacity((self.width * self.height * 3) as usize);
        for y in 0..self.height {
            for x in 0..self.width {
                bytes.extend(self.rgb(x, y));
            }
        }
        writer.write_image_data(&bytes).expect("the PNG data");
    }
}
