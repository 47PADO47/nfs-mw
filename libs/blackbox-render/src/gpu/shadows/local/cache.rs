//! Kept depth of the casters that stand still, for the shadowed lights that stand still themselves.
//!
//! A street lamp's projection never changes, and most of what it lights does not move. Its unmoving casters are drawn
//! once into an image of their own; a frame then copies that image into the lamp's atlas layer and draws only the
//! moving casters over it, or leaves the layer alone when it already holds exactly that image. A light whose
//! projection changed since the previous frame (a headlight) is drawn in full, as before.
use blackbox_gfx::{Instance, InstanceKey};
use std::collections::HashMap;

type FastMap<K, V> = HashMap<K, V, foldhash::fast::FixedState>;

/// A light's projection matrix, bit for bit: the identity of a light that stands still.
pub(super) type Projection = [u32; 16];

#[derive(Default)]
pub(super) struct LampCache {
    entries: FastMap<Projection, Entry>,
    /// Images of lights that left the shadowed set, for the next one that enters it.
    free: Vec<wgpu::TextureView>,
    /// The light and static casters whose kept image an atlas layer holds unchanged; `None` after any other draw.
    layers: Vec<Option<(Projection, u64)>>,
    frame: u64,
    /// The casters asked about in the previous frame and in this one, by key.
    previous: FastMap<InstanceKey, Stamp>,
    current: FastMap<InstanceKey, Stamp>,
    /// This frame's answer per caster: unknown, moving or resting.
    rest: Vec<u8>,
}

/// Where a keyed caster was, and as which mesh.
#[derive(PartialEq, Eq, Clone, Copy)]
struct Stamp {
    mesh: usize,
    transform: [u32; 16],
}

const UNKNOWN: u8 = 0;
const MOVING: u8 = 1;
const RESTING: u8 = 2;

struct Entry {
    image: Option<wgpu::TextureView>,
    fingerprint: u64,
    seen: u64,
}

/// What a frame does with one still light's kept image.
pub(super) struct Kept {
    pub image: wgpu::TextureView,
    /// The image does not hold these static casters yet and must be drawn.
    pub stale: bool,
}

impl LampCache {
    /// Starts a frame with `layers` atlas layers of `resolution` and `casters` casters.
    pub fn begin(&mut self, layers: usize, resolution: u32, casters: usize) {
        self.frame += 1;
        std::mem::swap(&mut self.previous, &mut self.current);
        self.current.clear();
        self.rest.clear();
        self.rest.resize(casters, UNKNOWN);
        self.layers.resize(layers, None);
        if self.free.first().is_some_and(|image| image.texture().width() != resolution) {
            self.free.clear();
        }
    }

    /// Whether the light with this projection had it in the previous frame too. Notes the light as seen.
    pub fn still(&mut self, projection: Projection) -> bool {
        let frame = self.frame;
        let entry = self.entries.entry(projection).or_insert(Entry { image: None, fingerprint: 0, seen: 0 });
        // Zero is "never": frames count from one.
        let still = entry.seen != 0 && entry.seen + 1 == frame;
        entry.seen = frame;
        still
    }

    /// Whether caster `index` is where a still light's frame found it in the previous frame, as the same mesh. Only
    /// the casters a still light selects are ever asked about, so one is first known to rest in the second frame
    /// that asks. The answer only sorts casters into kept and redrawn; the kept image's fingerprint names what it
    /// holds exactly.
    pub fn resting(&mut self, index: usize, casters: &[Instance]) -> bool {
        if self.rest[index] != UNKNOWN {
            return self.rest[index] == RESTING;
        }
        let caster = &casters[index];
        if caster.key.is_transient() {
            self.rest[index] = MOVING;
            return false;
        }
        let stamp = Stamp { mesh: caster.mesh.raw(), transform: caster.transform.to_cols_array().map(f32::to_bits) };
        let resting = self.previous.get(&caster.key) == Some(&stamp);
        self.current.insert(caster.key, stamp);
        self.rest[index] = match resting {
            true => RESTING,
            false => MOVING,
        };
        resting
    }

    /// The kept image of a still light for the static casters `fingerprint` names.
    pub fn kept(&mut self, device: &wgpu::Device, resolution: u32, projection: Projection, fingerprint: u64) -> Kept {
        let entry = self.entries.get_mut(&projection).expect("a still light was seen this frame");
        let sized = entry.image.as_ref().is_some_and(|image| image.texture().width() == resolution);
        if !sized {
            entry.image = Some(self.free.pop().unwrap_or_else(|| image(device, resolution)));
        }
        let stale = !sized || entry.fingerprint != fingerprint;
        entry.fingerprint = fingerprint;
        Kept { image: entry.image.clone().expect("the image was just kept"), stale }
    }

    /// Whether atlas layer `layer` already holds exactly this light's kept image.
    pub fn holds(&self, layer: usize, projection: Projection, fingerprint: u64) -> bool {
        self.layers[layer] == Some((projection, fingerprint))
    }

    /// Records what `layer` holds after this frame's draw: a kept image alone, or `None`.
    pub fn set_layer(&mut self, layer: usize, held: Option<(Projection, u64)>) {
        self.layers[layer] = held;
    }

    /// Ends a frame: the images of the lights it did not see wait for the next light, at most one per layer.
    pub fn end(&mut self) {
        let frame = self.frame;
        let free = &mut self.free;
        let spare = self.layers.len();
        self.entries.retain(|_, entry| {
            if entry.seen == frame {
                return true;
            }
            if let Some(image) = entry.image.take().filter(|_| free.len() < spare) {
                free.push(image);
            }
            false
        });
    }

    /// Kept depth storage in MiB.
    pub fn mib(&self) -> u32 {
        let kept = self.entries.values().filter_map(|entry| entry.image.as_ref()).chain(&self.free);
        kept.map(|image| image.texture().width().pow(2) * 4 / (1024 * 1024)).sum()
    }
}

fn image(device: &wgpu::Device, resolution: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("kept static lamp depth"),
            size: wgpu::Extent3d { width: resolution, height: resolution, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
