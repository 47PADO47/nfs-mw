//! Three asynchronous timestamp slots. Busy slots are skipped; rendering never waits for readback.
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

pub(super) const FEATURES: wgpu::Features =
    wgpu::Features::TIMESTAMP_QUERY.union(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS);
struct Slot {
    queries: wgpu::QuerySet,
    resolved: wgpu::Buffer,
    readback: wgpu::Buffer,
    state: Arc<AtomicU8>,
}
pub(super) struct Timing {
    slots: Vec<Slot>,
    active: Option<usize>,
    used: [bool; 12],
    next: usize,
    period: f32,
    pub latest: Option<[f32; 6]>,
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
                    count: 12,
                }),
                resolved: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("lab resolved timestamps"),
                    size: 96,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                readback: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("lab timestamp readback"),
                    size: 96,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                state: Arc::new(AtomicU8::new(0)),
            })
            .collect();
        Some(Self {
            slots,
            active: None,
            used: [false; 12],
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
                let mut sample = [0.0; 6];
                for i in 0..6 {
                    sample[i] = ticks[i * 2 + 1].saturating_sub(ticks[i * 2]) as f32 * self.period / 1_000_000.0;
                }
                self.latest = Some(sample);
                self.completed += 1;
                blackbox_trace::global().counter(
                    "GPU passes (delayed readback ms)",
                    &[
                        ("readback_id", self.completed as f64),
                        ("base_render", sample[0].into()),
                        ("shadows", sample[1].into()),
                        ("surface_prepass", sample[2].into()),
                        ("ambient_occlusion", sample[3].into()),
                        ("cloud_lighting", sample[4].into()),
                        ("road_reflections", sample[5].into()),
                    ],
                );
            }
            slot.readback.unmap();
            slot.state.store(0, Ordering::Release);
        }
        self.active = (0..self.slots.len())
            .map(|i| (self.next + i) % self.slots.len())
            .find(|&i| self.slots[i].state.load(Ordering::Acquire) == 0);
        self.used = [false; 12];
        self.stamp(encoder, 0);
    }

    pub(super) fn stamp(&mut self, encoder: &mut wgpu::CommandEncoder, index: usize) {
        let Some(active) = self.active else { return };
        encoder.write_timestamp(&self.slots[active].queries, index as u32);
        self.used[index] = true;
    }

    pub(super) fn finish(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let Some(active) = self.active else { return };
        self.stamp(encoder, 1);
        for i in 2..12 {
            if !self.used[i] {
                self.stamp(encoder, i);
            }
        }
        let slot = &self.slots[active];
        encoder.resolve_query_set(&slot.queries, 0..12, &slot.resolved, 0);
        encoder.copy_buffer_to_buffer(&slot.resolved, 0, &slot.readback, 0, 96);
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
