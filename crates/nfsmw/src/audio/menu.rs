//! Menu event routing and authored bank playback. Spec: `frontend-audio.md`.

use std::cell::RefCell;
use std::collections::HashSet;

use blackbox_feng::ids::*;

use super::aems::{EffectModule, EffectSampler};
use super::{Audio, Group};

#[cfg(test)]
mod tests;

const LAST_COMMON_BUTTON: u32 = PAD_BUTTON0 + 5;

thread_local! {
    static QUEUE: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
}

/// Maps the original sound messages and the port's semantic pad events to UI triggers.
fn trigger(message: u32) -> Option<i32> {
    Some(match message {
        PAD_UP | PAD_DOWN | PAD_UPLEFT | PAD_UPRIGHT | PAD_DOWNLEFT | PAD_DOWNRIGHT | 0xB205_316C | 0x480D_F13F => 0,
        PAD_LEFT | PAD_RIGHT | 0x4812_2792 | 0x4AC5_E165 => 2,
        PAD_ACCEPT | BUTTON_PRESSED | 0x7B6B_89D7 => 4,
        PAD_BACK | 0x480C_9A58 => 5,
        PAD_START => 6,
        PAD_BUTTON0..=LAST_COMMON_BUTTON => 7,
        0x610F_B233..=0x610F_B238 => 7,
        0x6BC1_CD5D => 10,
        0xE3FB_93BE => 11,
        0x4810_A91B => 28,
        0x145B_13C0 => 88,
        0x8C40_ED95 => 89,
        _ => return None,
    })
}

/// Requests a menu effect; unknown messages and button releases stay silent.
pub fn request(message: u32) {
    if let Some(id) = trigger(message) {
        QUEUE.with(|queue| queue.borrow_mut().push(id));
    }
}

/// A package can announce one gesture through both a pad event and a script response.
pub fn request_batch(messages: impl IntoIterator<Item = (blackbox_feng::PackageId, u32)>) {
    let mut seen = Vec::new();
    for (package, message) in messages {
        let Some(id) = trigger(message) else { continue };
        if seen.contains(&(package, id)) {
            continue;
        }
        seen.push((package, id));
        QUEUE.with(|queue| queue.borrow_mut().push(id));
    }
}

pub(crate) fn take() -> Vec<i32> {
    QUEUE.with(|queue| std::mem::take(&mut *queue.borrow_mut()))
}

fn target(id: i32) -> (&'static str, &'static str, i32) {
    if id < 23 {
        return ("GLOBAL/FE_COMMON_MB.abk", "PlayCommonSample", id);
    }
    ("FE/FE_MB.abk", "PlayFrontEndSample", id - 23)
}

struct Running {
    module: EffectModule,
    words: [i32; 4],
    age: f32,
    released: bool,
}

/// Sound lifetimes survive screen changes and end independently of the menu stack.
#[derive(Default)]
pub(super) struct MenuPlayer {
    active: Vec<Running>,
    unavailable: HashSet<i32>,
    seed: u32,
}

impl MenuPlayer {
    fn start(&mut self, audio: &mut Audio, id: i32) {
        if self.unavailable.contains(&id) {
            return;
        }
        let (bank, class, index) = target(id);
        self.seed = self.seed.wrapping_add(1);
        match EffectModule::load(audio, bank, class, self.seed) {
            Ok(module) => {
                self.active.push(Running { module, words: [index, 32767, 4096, 0], age: 0.0, released: false })
            }
            Err(error) => {
                self.unavailable.insert(id);
                log::warn!("menu sound {id}: {error}");
            }
        }
    }

    fn update(&mut self, audio: &mut Audio, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.active.retain_mut(|running| {
            running.age += dt;
            let usable = running.module.update(&mut EffectSampler(audio, Group::Sfx), dt, &running.words);
            // The graph must see its class data once before its short-lived Csis owner is released.
            if !running.released && running.age >= 1.0 / 60.0 {
                running.module.release();
                running.released = true;
            }
            usable && !running.module.finished() && running.age < 10.0
        });
    }

    pub(super) fn service(&mut self, audio: &mut Audio, dt: f32) {
        for id in take() {
            self.start(audio, id);
        }
        self.update(audio, dt);
    }
}
