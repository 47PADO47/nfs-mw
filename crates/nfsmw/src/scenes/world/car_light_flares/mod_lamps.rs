//! The flares and glows a mod car describes itself (`[[lamp_flare]]` and `[[light_bar]]`, `docs/modding/car-lamps.md`).
//!
//! A sprite is sized in metres and fades in a straight line between two distances; its brightness is also scaled by the cosine
//! of the angle off the lamp's axis raised to `falloff`. A flare is lit by a lamp state of the car; a light bar lamp follows its
//! flash pattern while the car's emergency lights are on. The game's shared profiles (`light_flares_cg`) are not used for a
//! kind of lamp the car gives sprites for, and are for every kind it does not.

use blackbox_gfx::{BlendMode, EffectLayer, RenderBackend, TextureHandle, TexturedEffect};
use blackbox_hash::bstring_hash;
use glam::{Mat4, Vec3};
use nfsmw_data::car::CarModel;
use nfsmw_data::car::mods::lamp_flares::{self, Bar, Flare, Lamps, Lit, Look, Side, Siren};

use super::super::car_lights::{Kind, LampState};
use super::{FlareView, emergency, focal_scale};

/// The damage bits of the head, brake and reverse lamps of one side (`car_lights/damage.rs`): left, right, centre.
const HEAD_BITS: [u32; 3] = [1, 2, 4];
const BRAKE_BITS: [u32; 3] = [8, 16, 32];
const REVERSE_BITS: [u32; 3] = [64, 128, 0];
/// The emergency bit that breaks a red, blue or white siren lamp.
const SIREN_BITS: [u32; 3] = [emergency::RED, emergency::BLUE, emergency::WHITE];
/// A sprite this far or farther, metres, is not drawn.
const MAX_DISTANCE: f32 = 400.0;
/// A sprite nearer the camera plane than this is not drawn.
const MIN_DEPTH: f32 = 0.3;

/// A car's own sprites, with their textures uploaded.
#[derive(Clone, Default)]
pub(super) struct ModLamps {
    lamps: Option<&'static Lamps>,
    textures: Vec<(String, TextureHandle)>,
}

impl ModLamps {
    /// The sprites `model` publishes, with their textures found in the car's own texture pack. A sprite the pack lacks is not
    /// drawn.
    pub(super) fn upload(renderer: &mut dyn RenderBackend, model: &CarModel) -> Self {
        let Some(lamps) = lamp_flares::of(&model.name) else { return Self::default() };
        let mut textures: Vec<(String, TextureHandle)> = Vec::new();
        let looks = lamps.flares.iter().map(|flare| &flare.look).chain(lamps.bars.iter().map(|bar| &bar.look));
        for look in looks {
            if textures.iter().any(|(name, _)| *name == look.sprite) {
                continue;
            }
            let Some(texture) = model.textures.get(&bstring_hash(&look.sprite)) else { continue };
            let Some(handle) = blackbox_scene::upload_texture(renderer, texture) else { continue };
            textures.push((look.sprite.clone(), handle));
        }
        Self { lamps: Some(lamps), textures }
    }

    /// The uploaded textures, to release with the car.
    pub(super) fn textures(&self) -> impl Iterator<Item = TextureHandle> + '_ {
        self.textures.iter().map(|(_, handle)| *handle)
    }

    /// Whether the car draws this kind of lamp itself, so the markers' flares of that kind are left out.
    pub(super) fn covers(&self, kind: Kind) -> bool {
        let Some(lamps) = self.lamps else { return false };
        match kind {
            Kind::Head => lamps.covers(Lit::Head),
            Kind::Tail | Kind::CentreBrake => lamps.covers(Lit::Tail) || lamps.covers(Lit::Brake),
            Kind::Reverse => lamps.covers(Lit::Reverse),
            Kind::Siren(_) | Kind::SirenBright(_) | Kind::SirenWhite | Kind::SirenOrange => !lamps.bars.is_empty(),
            Kind::Signal(_) => false,
        }
    }

    /// Append the batches of the car's sprites, one per texture, for each frame of the car.
    pub(super) fn draw(
        &self,
        state: LampState,
        frames: impl Iterator<Item = Mat4> + Clone,
        view: FlareView,
        out: &mut Vec<TexturedEffect>,
    ) {
        let Some(lamps) = self.lamps else { return };
        let FlareView { camera, forward, fov_degrees, height } = view;
        if focal_scale(height, fov_degrees).is_none() {
            return;
        }
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::Y);
        let up = right.cross(forward).normalize_or(Vec3::Z);
        for (name, texture) in &self.textures {
            let mut vertices = Vec::new();
            for world in frames.clone() {
                let sprites = lamps
                    .flares
                    .iter()
                    .filter(|flare| flare.look.sprite == *name)
                    .map(|flare| (&flare.look, flare_level(flare, state), false))
                    .chain(
                        lamps
                            .bars
                            .iter()
                            .filter(|bar| bar.look.sprite == *name)
                            .map(|bar| (&bar.look, bar_level(bar, state), true)),
                    );
                for (look, level, omni) in sprites {
                    let at = world.transform_point3(Vec3::from(look.position));
                    let to_eye = camera - at;
                    let distance = to_eye.length();
                    if level <= 0.0 || !distance.is_finite() || (-to_eye).dot(forward) <= MIN_DEPTH {
                        continue;
                    }
                    let facing = match omni {
                        true => 1.0,
                        false => world
                            .transform_vector3(Vec3::from(look.direction))
                            .normalize_or_zero()
                            .dot(to_eye.normalize_or_zero()),
                    };
                    let Some((radius, color)) = appearance(look, level, facing, distance) else { continue };
                    let at = at + to_eye.normalize_or_zero() * look.z_bias;
                    let (x, y) = (right * radius, up * radius);
                    EffectLayer::particle_quad(
                        &mut vertices,
                        [at - x + y, at + x + y, at + x - y, at - x - y],
                        color,
                        [0.0; 2],
                    );
                }
            }
            if !vertices.is_empty() {
                out.push(TexturedEffect { texture: *texture, blend: BlendMode::Additive, vertices });
            }
        }
    }
}

/// How lit a flare is: the lamp's state, 0 when the lamp's damage bit is set.
fn flare_level(flare: &Flare, state: LampState) -> f32 {
    if state.dark {
        return 0.0;
    }
    let side = |bits: [u32; 3]| match flare.side {
        Side::Left => bits[0],
        Side::Right => bits[1],
        Side::Centre => bits[2],
    };
    let (level, bit) = match flare.lit {
        Lit::Head => (f32::from(u8::from(state.headlights)), side(HEAD_BITS)),
        Lit::Tail => (f32::from(u8::from(state.headlights)), side(BRAKE_BITS)),
        Lit::Brake => (f32::from(u8::from(state.brake > 0.0)), side(BRAKE_BITS)),
        Lit::Reverse => (f32::from(u8::from(state.reverse)), side(REVERSE_BITS)),
        Lit::Always => (1.0, 0),
    };
    match state.broken & bit {
        0 => level,
        _ => 0.0,
    }
}

/// How lit a siren lamp is: its pattern's step, 0 when the emergency lights are off or its colour's bit is broken.
fn bar_level(bar: &Bar, state: LampState) -> f32 {
    let mask = SIREN_BITS[match bar.siren {
        Siren::Red => 0,
        Siren::Blue => 1,
        Siren::White => 2,
    }];
    if (state.emergency & !state.broken) & mask == 0 {
        return 0.0;
    }
    Lamps::bar_level(bar, state.time)
}

/// The radius (metres) and the colour of a sprite seen at `distance` metres, `facing` the cosine of the angle off its axis;
/// `None` when it cannot be seen.
fn appearance(look: &Look, level: f32, facing: f32, distance: f32) -> Option<(f32, [u8; 4])> {
    if level <= 0.0 || facing <= 0.0 || distance >= MAX_DISTANCE {
        return None;
    }
    let [near, far] = look.range;
    let t = ((distance - near) / (far - near)).clamp(0.0, 1.0);
    let lerp = |pair: [f32; 2]| pair[0] + (pair[1] - pair[0]) * t;
    let radius = lerp(look.radius);
    let bright = lerp(look.alpha) * level * facing.powf(look.falloff);
    let color = std::array::from_fn(|i| match i {
        3 => (bright * 255.0).clamp(0.0, 255.0) as u8,
        _ => (look.colour[i] * bright.min(1.0) * 255.0).clamp(0.0, 255.0) as u8,
    });
    (radius > 0.0 && color[3] > 0).then_some((radius, color))
}

#[cfg(test)]
#[path = "mod_lamps_tests.rs"]
mod tests;
