//! Showing a change of the replaced textures in the shared sets while the world is open (`docs/modding/api-replace.md`, "Textures").
//!
//! The textures of most of the road, the ground and the props are in the **shared sets** (`V`, `X`, `Y`), which load once when the world
//! opens. A tile that is streamed again picks up a changed replacement (`replace.rs`), but a tile's materials resolve the textures of the
//! shared sets through the handles the shared upload made, and those were made from the pictures in effect when the world opened. So a
//! player who turns a texture set on in the options with the world open (the options are in the game's menus, in the same process) saw
//! no change until the next start.
//!
//! The fix is a **redirect** (`RenderBackend::redirect_texture`, the mechanism the animated textures use): the texture the shared upload
//! made keeps its handle, which every material uses, and the new picture is drawn wherever that handle is used. Turning a set off again
//! removes the redirect, or, when the world opened with the set on, redirects to the game's own texture (`replace::original`, kept when
//! the set first replaced it). Nothing is uploaded again but the pictures that changed; a picture is **made on a worker thread**
//! (a 2012 road strip is 130 ms of resizing), and shown on the frame after it is ready, a few at a time within the upload budget. The
//! models of the shared sets, and the global textures, still follow at the next start.

use std::collections::HashMap;
use std::sync::Arc;

use blackbox_gfx::{BlendMode, RenderBackend, TextureHandle};
use blackbox_scene::upload_texture_predecoded_mips;
use blackbox_tpk::Texture;

use super::*;
use crate::modding::content::replace::{self, Active, TextureSlot};

/// What a texture's pixels are, cheaply: its size, its length and 64 bytes spread over it. Two pictures a set could make for one texture
/// differ in size or in these bytes; the same picture twice is the same here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Print {
    width: u32,
    height: u32,
    bytes: usize,
    sample: u64,
}

pub(super) fn print(texture: &Texture) -> Print {
    let step = (texture.data.len() / 64).max(1);
    let sample = texture
        .data
        .iter()
        .step_by(step)
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3));
    Print { width: texture.width, height: texture.height, bytes: texture.data.len(), sample }
}

/// What a shared texture should show.
enum Wish {
    /// What the shared upload made.
    Base,
    /// The game's own texture, which the upload did not make (the world opened with a set on).
    Original(Arc<Texture>),
    /// A set's picture, which a worker thread is making or has made.
    Slot(Arc<TextureSlot>),
}

struct Want {
    hash: u32,
    wish: Wish,
}

/// A redirect in force: the texture drawn in place of a shared one.
struct Shown {
    handle: TextureHandle,
    print: Print,
}

/// The shared textures, and what is shown in place of them.
#[derive(Default)]
pub(super) struct SharedTextures {
    /// What each shared texture was when it was uploaded.
    base: HashMap<u32, Print>,
    shown: HashMap<u32, Shown>,
    wants: Vec<Want>,
}

/// What became of a want.
enum Settled {
    Done(usize),
    Wait,
}

impl SharedTextures {
    /// Remember what `textures` are as the shared upload is made from them.
    pub(super) fn note_base(&mut self, textures: &[Texture]) {
        for texture in textures {
            self.base.entry(texture.name_hash).or_insert_with(|| print(texture));
        }
    }

    /// Whether every want is settled.
    pub(super) fn idle(&self) -> bool {
        self.wants.is_empty()
    }

    /// Work out what each shared texture that a set replaces or replaced should show now, under `active`. `resident` says which hashes the
    /// shared upload made. Returns the slots whose pictures must be made (the caller does it off the frame thread).
    pub(super) fn plan(&mut self, active: Option<&Active>, resident: impl Fn(u32) -> bool) -> Vec<Arc<TextureSlot>> {
        let mut hashes = active.map(Active::replaced_hashes).unwrap_or_default();
        hashes.extend(self.shown.keys().copied());
        hashes.extend(replace::original_hashes());
        hashes.sort_unstable();
        hashes.dedup();
        hashes.retain(|hash| resident(*hash));
        self.wants.clear();
        let mut slots = Vec::new();
        for hash in hashes {
            let wish = match active.and_then(|active| active.slot(hash)) {
                Some(slot) => {
                    slots.push(slot.clone());
                    Wish::Slot(slot)
                }
                None => match replace::original(hash) {
                    Some(original) if self.base.get(&hash) != Some(&print(&original)) => Wish::Original(original),
                    _ => Wish::Base,
                },
            };
            self.wants.push(Want { hash, wish });
        }
        slots
    }

    /// Show what is ready, until `budget` bytes of pictures are uploaded (one is always taken, so a picture bigger than the budget
    /// still shows). `handles` are the shared textures, by hash.
    pub(super) fn advance(
        &mut self,
        renderer: &mut dyn RenderBackend,
        handles: &HashMap<u32, (TextureHandle, BlendMode)>,
        discard: u32,
        budget: usize,
    ) {
        let mut spent = 0;
        let mut waiting = Vec::new();
        for want in std::mem::take(&mut self.wants) {
            let Some(&(base, _)) = handles.get(&want.hash) else { continue };
            if spent >= budget && spent > 0 {
                waiting.push(want);
                continue;
            }
            match self.settle(renderer, &want, base, discard) {
                Settled::Done(bytes) => spent += bytes,
                Settled::Wait => waiting.push(want),
            }
        }
        self.wants = waiting;
    }

    fn settle(&mut self, renderer: &mut dyn RenderBackend, want: &Want, base: TextureHandle, discard: u32) -> Settled {
        let picture = match &want.wish {
            Wish::Base => None,
            Wish::Original(texture) => Some(texture.clone()),
            Wish::Slot(slot) => match slot.made() {
                None => return Settled::Wait,
                // A picture that could not be made was logged; what the upload made stays.
                Some(picture) => picture,
            },
        };
        let Some(picture) = picture else {
            self.restore(renderer, want.hash, base);
            return Settled::Done(0);
        };
        let wanted = print(&picture);
        if self.base.get(&want.hash) == Some(&wanted) {
            self.restore(renderer, want.hash, base);
            return Settled::Done(0);
        }
        if self.shown.get(&want.hash).is_some_and(|shown| shown.print == wanted) {
            return Settled::Done(0);
        }
        let Some(handle) = upload_texture_predecoded_mips(renderer, &picture, None, discard) else {
            return Settled::Done(0);
        };
        renderer.redirect_texture(base, Some(handle));
        if let Some(old) = self.shown.insert(want.hash, Shown { handle, print: wanted }) {
            renderer.destroy_texture(old.handle);
        }
        Settled::Done(picture.data.len())
    }

    /// Show what the shared upload made again.
    fn restore(&mut self, renderer: &mut dyn RenderBackend, hash: u32, base: TextureHandle) {
        let Some(old) = self.shown.remove(&hash) else { return };
        renderer.redirect_texture(base, None);
        renderer.destroy_texture(old.handle);
    }

    /// Free what is shown, for a residency that is released.
    pub(super) fn release(&mut self, renderer: &mut dyn RenderBackend) {
        for (_, shown) in self.shown.drain() {
            renderer.destroy_texture(shown.handle);
        }
        self.wants.clear();
    }
}

impl Residency {
    /// The replacements in effect changed (the caller saw it): plan what the shared textures should show, and have the pictures made.
    pub(super) fn plan_shared_textures(&mut self, active: Option<&Active>) {
        let materials = &self.shared.materials;
        let slots = self.shared_textures.plan(active, |hash| materials.contains_key(&hash));
        if slots.is_empty() {
            return;
        }
        let made = std::thread::Builder::new().name("replacement textures".into()).spawn(move || {
            for slot in &slots {
                slot.make();
            }
        });
        if let Err(error) = made {
            log::warn!("replacements: the pictures of the shared textures are not made: {error}");
        }
    }

    /// Runs every frame: shows the pictures of the shared textures that are ready.
    pub(super) fn advance_shared_textures(&mut self, renderer: &mut dyn RenderBackend) {
        if self.shared_textures.idle() {
            return;
        }
        let budget = self.upload_budget();
        let discard = self.texture_discard.unwrap_or(0);
        self.shared_textures.advance(renderer, &self.shared.materials, discard, budget);
    }
}

#[cfg(test)]
#[path = "shared_textures_tests.rs"]
mod tests;
