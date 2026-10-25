//! One authored mono wet bus, independent of per-voice dry gain and panning.
#[cfg(test)]
mod native_tests;
mod processor;
#[cfg(test)]
mod tests;

use crate::audio::{Audio, Group};
use blackbox_fxedit::{Patch, Processor};
use kira::effect::EffectBuilder;
pub(super) use processor::Builder;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};

const CAPACITY: usize = 256;

struct Prepared {
    rate: u32,
    rows: Vec<Option<Processor>>,
}

struct Shared {
    mono: Mutex<[f32; CAPACITY]>,
    rate: AtomicU32,
    selected: AtomicU32,
    effective: AtomicU32,
    errors: AtomicU64,
    groups: [AtomicU32; 4],
}

#[derive(Clone)]
pub(super) struct Bus {
    shared: Arc<Shared>,
    tx: mpsc::SyncSender<Prepared>,
}

#[derive(Clone)]
pub(super) struct Sender {
    shared: Arc<Shared>,
    group: usize,
}

impl Sender {
    pub fn mono(&self, offset: usize, input: &[f32]) {
        let gain = f32::from_bits(self.shared.groups[self.group].load(Ordering::Relaxed));
        let Ok(mut buffer) = self.shared.mono.lock() else { return };
        if offset + input.len() > buffer.len() {
            self.shared.errors.fetch_add(1, Ordering::Relaxed);
            return;
        }
        for (dest, source) in buffer[offset..].iter_mut().zip(input) {
            *dest += source * gain;
        }
    }
}

impl Bus {
    pub fn builder() -> Builder {
        let (tx, rx) = mpsc::sync_channel(1);
        let shared = Arc::new(Shared {
            mono: Mutex::new([0.0; CAPACITY]),
            rate: AtomicU32::new(0),
            selected: AtomicU32::new(u32::MAX),
            effective: AtomicU32::new(u32::MAX),
            errors: AtomicU64::new(0),
            groups: std::array::from_fn(|_| AtomicU32::new(1.0_f32.to_bits())),
        });
        Builder { bus: Self { shared, tx }, rx }
    }
    pub fn sender(&self, group: Group) -> Sender {
        let group = match group {
            Group::Sfx => 0,
            Group::Engine => 1,
            Group::Music => 2,
            Group::Hud => 3,
        };
        Sender { shared: self.shared.clone(), group }
    }
    pub fn volumes(&self, volumes: crate::audio::Volumes) {
        for (slot, gain) in self.shared.groups.iter().zip([volumes.sfx, volumes.engine, volumes.music, volumes.hud]) {
            slot.store(gain.to_bits(), Ordering::Relaxed);
        }
    }
    pub fn supported(&self) -> bool {
        self.shared.effective.load(Ordering::Relaxed) != u32::MAX && self.shared.errors.load(Ordering::Relaxed) == 0
    }
}

#[derive(Default)]
pub(super) struct Slot {
    rate: u32,
    requested: Option<u8>,
    effective: bool,
}

impl Audio {
    pub(super) fn update_wet(&mut self, requested: Option<u8>) -> bool {
        let Some(bus) = self.output.as_ref().map(|out| out.wet.clone()) else { return false };
        let rate = bus.shared.rate.load(Ordering::Relaxed);
        if rate == 0 {
            return false;
        }
        if self.wet.rate != rate {
            let mut rows = Vec::new();
            for name in PATCHES {
                let result = self
                    .dir
                    .read(&format!("SOUND/FXEDIT/{name}"))
                    .map_err(|error| error.to_string())
                    .and_then(|bytes| Patch::parse(&bytes).map_err(|error| error.to_string()))
                    .and_then(|patch| Processor::new(&patch, rate, 2).map_err(|error| error.to_string()));
                rows.push(
                    result
                        .map_err(|error| {
                            self.trace.record(crate::audio::trace::Record {
                                event: "wet_patch_failure",
                                class: "FXEDIT",
                                bank: name,
                                success: Some(false),
                                detail: &error,
                                logical_ms: self.trace.now_ms(),
                                ..Default::default()
                            })
                        })
                        .ok(),
                );
            }
            if bus.tx.try_send(Prepared { rate, rows }).is_ok() {
                self.wet.rate = rate;
            }
        }
        if self.wet.requested != requested {
            self.wet.requested = requested;
            bus.shared.selected.store(requested.map_or(u32::MAX, u32::from), Ordering::Relaxed);
            self.trace.note("wet_patch_request", &format!("row={requested:?}; fixed16samplewetlatency; rate={rate}"));
        }
        let supported = bus.supported();
        if self.wet.effective != supported {
            self.wet.effective = supported;
            self.trace.record(crate::audio::trace::Record {
                event: "wet_effective",
                class: "FXEDIT",
                logical_ms: self.trace.now_ms(),
                success: Some(supported),
                detail: &format!(
                    "row={requested:?}; rate={rate}; errors={}",
                    bus.shared.errors.load(Ordering::Relaxed)
                ),
                ..Default::default()
            });
        }
        supported
    }
}

// Filled from the original environment-patch table, not inferred from filenames.
const PATCHES: [&str; 12] = [
    "City_Dense.fx",
    "Alley.fx",
    "City_Open.fx",
    "Garage.fx",
    "Garage_sml.fx",
    "Simple_Tunnel.fx",
    "Simple_Tunnel_Sml.fx",
    "Hills_Close.fx",
    "Hills.fx",
    "City_Open.fx",
    "City_Dense.fx",
    "Alley.fx",
];
