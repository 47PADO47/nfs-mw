//! Streaming adaptation retains partial blocks; never advances with padding samples.
use super::*;
use kira::{Frame, effect::Effect, info::Info};

pub(in crate::audio) struct Builder {
    pub(super) bus: Bus,
    pub(super) rx: mpsc::Receiver<Prepared>,
}

impl EffectBuilder for Builder {
    type Handle = Bus;
    fn build(self) -> (Box<dyn Effect>, Bus) {
        let bus = self.bus.clone();
        (
            Box::new(WetEffect {
                bus: self.bus,
                rx: self.rx,
                prepared: None,
                row: None,
                pending: [0.0; 16],
                output: [Frame::ZERO; 16],
                cursor: 0,
            }),
            bus,
        )
    }
}

struct WetEffect {
    bus: Bus,
    rx: mpsc::Receiver<Prepared>,
    prepared: Option<Prepared>,
    row: Option<usize>,
    pending: [f32; 16],
    output: [Frame; 16],
    cursor: usize,
}

impl Effect for WetEffect {
    fn init(&mut self, sample_rate: u32, _buffer_size: usize) {
        self.bus.shared.rate.store(sample_rate, Ordering::Relaxed);
    }
    fn on_change_sample_rate(&mut self, sample_rate: u32) {
        self.prepared = None;
        self.row = None;
        self.cursor = 0;
        self.output.fill(Frame::ZERO);
        self.bus.shared.effective.store(u32::MAX, Ordering::Relaxed);
        self.bus.shared.rate.store(sample_rate, Ordering::Relaxed);
    }
    fn on_start_processing(&mut self) {
        if self.prepared.is_none()
            && let Ok(prepared) = self.rx.try_recv()
            && prepared.rate == self.bus.shared.rate.load(Ordering::Relaxed)
        {
            self.prepared = Some(prepared);
            self.row = None;
        }
        let selected = self.bus.shared.selected.load(Ordering::Relaxed);
        let row = (selected < 12).then_some(selected as usize);
        if row == self.row {
            return;
        }
        self.row = row;
        self.cursor = 0;
        self.pending.fill(0.0);
        self.output.fill(Frame::ZERO);
        let available = self.prepared.as_mut().and_then(|bank| row.and_then(|row| bank.rows[row].as_mut()));
        let supported = available.is_some();
        if let Some(processor) = available {
            processor.reset();
        }
        self.bus.shared.effective.store(if supported { selected } else { u32::MAX }, Ordering::Relaxed);
    }
    fn process(&mut self, input: &mut [Frame], _dt: f64, _info: &Info) {
        let Ok(mut mono) = self.bus.shared.mono.lock() else { return };
        if input.len() > mono.len() {
            self.bus.shared.errors.fetch_add(1, Ordering::Relaxed);
            return;
        }
        let processor = self.prepared.as_mut().and_then(|bank| self.row.and_then(|row| bank.rows[row].as_mut()));
        let Some(processor) = processor else {
            mono.fill(0.0);
            return;
        };
        for (index, frame) in input.iter_mut().enumerate() {
            *frame += self.output[self.cursor];
            self.pending[self.cursor] = mono[index];
            mono[index] = 0.0;
            self.cursor += 1;
            if self.cursor < 16 {
                continue;
            }
            let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
            if processor.process_mono(&self.pending, &mut left, &mut right).is_err() {
                self.bus.shared.errors.fetch_add(1, Ordering::Relaxed);
                self.bus.shared.effective.store(u32::MAX, Ordering::Relaxed);
                left.fill(0.0);
                right.fill(0.0);
            }
            for i in 0..16 {
                self.output[i] = Frame { left: left[i], right: right[i] };
            }
            self.cursor = 0;
        }
    }
}
