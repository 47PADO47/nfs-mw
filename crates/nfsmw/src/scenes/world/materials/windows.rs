//! Global authored reflection substitution for architectural glazing.

use blackbox_gfx::{BlendMode, RenderBackend, Shading, TextureHandle, WindowMaterialDesc, WindowMaterialHandle};
use blackbox_solid::Solid;
use std::collections::HashMap;

pub(in crate::scenes::world) fn window_materials(
    renderer: &mut dyn RenderBackend,
    solid: &Solid,
    lookup: &impl Fn(u32) -> Option<(TextureHandle, BlendMode)>,
    cache: &mut HashMap<[TextureHandle; 2], WindowMaterialHandle>,
) -> Vec<Option<Shading>> {
    let reflection = lookup(blackbox_hash::bstring_hash("WINDOWREFLECTION")).map(|m| m.0);
    solid
        .groups
        .iter()
        .map(|group| {
            if group.effect_id != 5 || solid.name.starts_with("SKY") {
                return None;
            }
            let diffuse = lookup(group.diffuse_texture(solid)?)?.0;
            let textures = [diffuse, reflection?];
            if let Some(handle) = cache.get(&textures) {
                return Some(Shading::WorldWindow(*handle));
            }
            let handle = renderer.create_window_material(&WindowMaterialDesc { diffuse, reflection: textures[1] })?;
            cache.insert(textures, handle);
            Some(Shading::WorldWindow(handle))
        })
        .collect()
}
