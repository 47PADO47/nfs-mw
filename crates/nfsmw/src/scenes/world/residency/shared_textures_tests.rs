//! A change of the replaced textures with the world open, on authored pictures and a backend that counts textures and redirects.

use blackbox_modding::content::TextureState;
use blackbox_tpk::{PixelFormat, TextureSource};

use super::*;
use crate::app::test_backend::NullBackend;
use crate::modding::content::replace::{set_replacing, set_replacing_all};

/// A DXT1 texture of `side` by `side` filled with `fill`, called `hash`.
fn texture(hash: u32, side: u32, fill: u8) -> Texture {
    let blocks = (side / 4) * (side / 4);
    let mut texture =
        TextureSource::new("ROAD", side, side, PixelFormat::Dxt1, 1, vec![fill; blocks as usize * 8]).into_texture();
    texture.name_hash = hash;
    texture
}

fn renderer() -> NullBackend {
    NullBackend::new(crate::settings::test_caps::native()).with_texture_tracking()
}

fn live(renderer: &NullBackend) -> usize {
    renderer.resources.lock().unwrap().textures
}

/// The shared upload of `stock`: its handle, and the table the swaps look handles up in.
fn uploaded(
    renderer: &mut NullBackend,
    stock: &Texture,
) -> (TextureHandle, HashMap<u32, (TextureHandle, BlendMode)>, SharedTextures) {
    let base = upload_texture_predecoded_mips(renderer, stock, None, 0).unwrap();
    let mut shared = SharedTextures::default();
    shared.note_base(std::slice::from_ref(stock));
    (base, HashMap::from([(stock.name_hash, (base, BlendMode::Opaque))]), shared)
}

fn slot_of(hash: u32, side: u32, fill: u8) -> TextureSlot {
    TextureSlot::ready("mod", "ROAD", texture(hash, side, fill), TextureState::Keep)
}

#[test]
fn a_set_turned_on_with_the_world_open_is_drawn_over_the_texture_the_upload_made_and_turned_off_it_is_not() {
    const HASH: u32 = 0x77AA_0001;
    let mut gpu = renderer();
    let (base, handles, mut shared) = uploaded(&mut gpu, &texture(HASH, 8, 7));
    let on = set_replacing(HASH, slot_of(HASH, 16, 9));
    for slot in shared.plan(Some(&on), |hash| hash == HASH) {
        slot.make();
    }
    shared.advance(&mut gpu, &handles, 0, usize::MAX);
    let shown = *gpu.redirects.get(&base).expect("the base is redirected to the picture");
    assert_eq!(gpu.texture_sizes[&shown][..2], [16, 16], "the set's picture, twice the size");
    assert_eq!(live(&gpu), 2);
    assert!(shared.idle());
    shared.plan(None, |hash| hash == HASH);
    shared.advance(&mut gpu, &handles, 0, usize::MAX);
    assert!(gpu.redirects.is_empty(), "off again: the upload's own texture is drawn");
    assert_eq!(live(&gpu), 1, "the picture is freed");
}

#[test]
fn a_world_opened_with_the_set_on_shows_the_games_own_texture_when_it_is_turned_off() {
    const HASH: u32 = 0x77AA_0002;
    let mut gpu = renderer();
    let stock = texture(HASH, 8, 7);
    let on = set_replacing(HASH, slot_of(HASH, 16, 9));
    // The loader replaced the texture as the section loaded (and kept the game's own), and the upload made the replaced one.
    let mut loaded = vec![stock.clone()];
    on.replace_textures(&mut loaded);
    let (base, handles, mut shared) = uploaded(&mut gpu, &loaded[0]);
    for slot in shared.plan(Some(&on), |hash| hash == HASH) {
        slot.make();
    }
    shared.advance(&mut gpu, &handles, 0, usize::MAX);
    assert!(gpu.redirects.is_empty(), "what the upload made is the picture already: nothing to draw over it");
    assert_eq!(live(&gpu), 1);
    shared.plan(None, |hash| hash == HASH);
    shared.advance(&mut gpu, &handles, 0, usize::MAX);
    let shown = *gpu.redirects.get(&base).expect("the game's own texture is drawn over the replaced one");
    assert_eq!(gpu.texture_sizes[&shown][..2], [8, 8]);
    // And on again: the picture is the one the upload made, so the redirect goes.
    let again = set_replacing(HASH, slot_of(HASH, 16, 9));
    for slot in shared.plan(Some(&again), |hash| hash == HASH) {
        slot.make();
    }
    shared.advance(&mut gpu, &handles, 0, usize::MAX);
    assert!(gpu.redirects.is_empty());
    assert_eq!(live(&gpu), 1);
}

#[test]
fn a_picture_that_is_not_made_yet_waits_and_the_budget_shows_one_picture_at_a_time() {
    const A: u32 = 0x77AA_0003;
    const B: u32 = 0x77AA_0004;
    let mut gpu = renderer();
    let (base_a, mut handles, mut shared) = uploaded(&mut gpu, &texture(A, 8, 1));
    let second = texture(B, 8, 2);
    let base_b = upload_texture_predecoded_mips(&mut gpu, &second, None, 0).unwrap();
    shared.note_base(std::slice::from_ref(&second));
    handles.insert(B, (base_b, BlendMode::Opaque));
    let on = set_replacing(A, slot_of(A, 16, 9));
    let slots = shared.plan(Some(&on), |hash| hash == A);
    shared.advance(&mut gpu, &handles, 0, usize::MAX);
    assert!(gpu.redirects.is_empty() && !shared.idle(), "the picture is not made: the frame goes on and waits");
    slots[0].make();
    shared.advance(&mut gpu, &handles, 0, usize::MAX);
    assert!(gpu.redirects.contains_key(&base_a) && shared.idle());
    // Two pictures and a budget of one byte: the first is always taken, the second waits for the next call.
    let both = set_replacing_all(vec![(A, slot_of(A, 32, 4)), (B, slot_of(B, 16, 5))]);
    let slots = shared.plan(Some(&both), |hash| hash == A || hash == B);
    assert_eq!(slots.len(), 2);
    slots.iter().for_each(|slot| slot.make());
    shared.advance(&mut gpu, &handles, 0, 1);
    assert!(!shared.idle(), "the budget was spent on the first picture");
    shared.advance(&mut gpu, &handles, 0, 1);
    assert!(shared.idle(), "both were shown");
    assert!(gpu.redirects.contains_key(&base_a) && gpu.redirects.contains_key(&base_b));
}

#[test]
fn nothing_the_upload_does_not_hold_is_planned() {
    let mut shared = SharedTextures::default();
    let on = set_replacing(0x77AA_0005, slot_of(0x77AA_0005, 16, 9));
    assert!(shared.plan(Some(&on), |_| false).is_empty());
    assert!(shared.idle());
}
