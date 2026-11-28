//! Three asynchronous timestamp slots. Busy slots are skipped; rendering never waits for readback.
//!
//! Each slot holds [`PAIRS`] start/end pairs, in [`PASSES`] order: pair 0 is the whole frame, the others bracket one
//! part of it each (`stamp(encoder, 2 * pair)` and `stamp(encoder, 2 * pair + 1)`). A pair a frame does not
//! stamp reads zero. `road_reflections` and `cloud_lighting` run inside `colour_pass`, the upscaler inside `post`.
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

/// The parts of a frame with their own timer, by pair.
pub(super) const PASSES: [&str; PAIRS] = [
    "base_render",
    "shadows",
    "surface_prepass",
    "ambient_occlusion",
    "cloud_lighting",
    "road_reflections",
    "ray_scene",
    "ray_shadows",
    // Lamp shadows, the cascade history and the cloud shadow map.
    "local_shadows",
    // The car's environment cube faces and its irradiance.
    "reflection_faces",
    "colour_pass",
    "post",
];
const PAIRS: usize = 12;
/// Pairs after [`PAIRS`] for the post chain's passes, one each in chain order (`Timing::post_pass`).
const POST_PAIRS: usize = 12;
const QUERIES: usize = (PAIRS + POST_PAIRS) * 2;
/// The pair of each named part: `stamp(encoder, 2 * pair)` opens it, `2 * pair + 1` closes it.
pub(super) mod pair {
    pub const SHADOWS: usize = 1;
    pub const SURFACES: usize = 2;
    pub const AMBIENT: usize = 3;
    pub const CLOUDS: usize = 4;
    pub const ROAD: usize = 5;
    pub const RAY_SCENE: usize = 6;
    pub const RAY_SHADOWS: usize = 7;
    pub const LOCAL_SHADOWS: usize = 8;
    pub const REFLECTION_FACES: usize = 9;
    pub const COLOUR: usize = 10;
    pub const POST: usize = 11;
}

pub(super) const FEATURES: wgpu::Features =
    wgpu::Features::TIMESTAMP_QUERY.union(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS);
struct Slot {
    queries: wgpu::QuerySet,
    resolved: wgpu::Buffer,
    readback: wgpu::Buffer,
    state: Arc<AtomicU8>,
    /// The post passes this slot's frame timed, by post pair; empty for a pair not used.
    post_names: [&'static str; POST_PAIRS],
}
pub(super) struct Timing {
    slots: Vec<Slot>,
    active: Option<usize>,
    used: [bool; QUERIES],
    next: usize,
    period: f32,
    pub latest: Option<[f32; PAIRS]>,
    pub completed: u64,
}

impl Timing {
    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(FEATURES) {
            return None;
        }
        let slots = (0..3)
            .map(|_| Slot {
                queries: device.create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some("lab GPU timings"),
                    ty: wgpu::QueryType::Timestamp,
                    count: QUERIES as u32,
                }),
                resolved: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("lab resolved timestamps"),
                    size: QUERIES as u64 * 8,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                readback: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("lab timestamp readback"),
                    size: QUERIES as u64 * 8,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                state: Arc::new(AtomicU8::new(0)),
                post_names: [""; POST_PAIRS],
            })
            .collect();
        Some(Self {
            slots,
            active: None,
            used: [false; QUERIES],
            next: 0,
            period: queue.get_timestamp_period(),
            latest: None,
            completed: 0,
        })
    }

    pub(super) fn begin(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder) {
        let _trace = blackbox_trace::scope("GPU API: timestamp poll / readback");
        let _ = device.poll(wgpu::PollType::Poll);
        // Poll in submission order so a later sample wins when several complete together.
        for offset in 0..self.slots.len() {
            let index = (self.next + offset) % self.slots.len();
            let slot = &self.slots[index];
            let state = slot.state.load(Ordering::Acquire);
            if state < 2 {
                continue;
            }
            if state == 2
                && let Ok(mapped) = slot.readback.slice(..).get_mapped_range()
            {
                let ticks: &[u64] = bytemuck::cast_slice(&mapped);
                let span = |i: usize| ticks[i * 2 + 1].saturating_sub(ticks[i * 2]) as f32 * self.period / 1_000_000.0;
                let mut sample = [0.0; PAIRS];
                for (i, value) in sample.iter_mut().enumerate() {
                    *value = span(i);
                }
                let mut post: Vec<(&'static str, f64)> = Vec::with_capacity(POST_PAIRS);
                for (i, name) in slot.post_names.iter().enumerate().filter(|(_, name)| !name.is_empty()) {
                    let ms = f64::from(span(PAIRS + i));
                    match post.iter_mut().find(|(seen, _)| seen == name) {
                        Some(entry) => entry.1 += ms,
                        None => post.push((name, ms)),
                    }
                }
                if !post.is_empty() {
                    blackbox_trace::global().counter("GPU post passes (delayed readback ms)", &post);
                }
                self.latest = Some(sample);
                self.completed += 1;
                let mut values = [("readback_id", self.completed as f64); PAIRS + 1];
                for (value, (name, ms)) in values[1..].iter_mut().zip(PASSES.iter().zip(sample)) {
                    *value = (name, ms.into());
                }
                blackbox_trace::global().counter("GPU passes (delayed readback ms)", &values);
            }
            slot.readback.unmap();
            slot.state.store(0, Ordering::Release);
            self.slots[index].post_names = [""; POST_PAIRS];
        }
        self.active = (0..self.slots.len())
            .map(|i| (self.next + i) % self.slots.len())
            .find(|&i| self.slots[i].state.load(Ordering::Acquire) == 0);
        self.used = [false; QUERIES];
        self.stamp(encoder, 0);
    }

    pub(super) fn stamp(&mut self, encoder: &mut wgpu::CommandEncoder, index: usize) {
        let Some(active) = self.active else { return };
        encoder.write_timestamp(&self.slots[active].queries, index as u32);
        self.used[index] = true;
    }

    /// Open pair `pair` (see [`pair`]) at this point of `encoder`.
    pub(super) fn open(&mut self, encoder: &mut wgpu::CommandEncoder, pair: usize) {
        self.stamp(encoder, 2 * pair);
    }

    /// Close pair `pair` at this point of `encoder`.
    pub(super) fn close(&mut self, encoder: &mut wgpu::CommandEncoder, pair: usize) {
        self.stamp(encoder, 2 * pair + 1);
    }

    /// Open (`start`) or close the timer of post pass `index` of the chain, `name`. Passes past the timers'
    /// count are not timed.
    pub(super) fn post_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        index: usize,
        name: &'static str,
        start: bool,
    ) {
        let Some(active) = self.active else { return };
        if index >= POST_PAIRS {
            return;
        }
        if start {
            self.slots[active].post_names[index] = name;
        }
        self.stamp(encoder, 2 * (PAIRS + index) + usize::from(!start));
    }

    pub(super) fn finish(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let Some(active) = self.active else { return };
        self.stamp(encoder, 1);
        for i in 2..QUERIES {
            if !self.used[i] {
                self.stamp(encoder, i);
            }
        }
        let slot = &self.slots[active];
        encoder.resolve_query_set(&slot.queries, 0..QUERIES as u32, &slot.resolved, 0);
        encoder.copy_buffer_to_buffer(&slot.resolved, 0, &slot.readback, 0, QUERIES as u64 * 8);
    }

    pub(super) fn submitted(&mut self) {
        let _trace = blackbox_trace::scope("GPU API: arm asynchronous readback");
        let Some(active) = self.active.take() else { return };
        let slot = &self.slots[active];
        slot.state.store(1, Ordering::Release);
        let state = Arc::clone(&slot.state);
        slot.readback.slice(..).map_async(wgpu::MapMode::Read, move |result| {
            state.store(
                match result {
                    Ok(()) => 2,
                    Err(_) => 3,
                },
                Ordering::Release,
            );
        });
        self.next = (active + 1) % self.slots.len();
    }
}
