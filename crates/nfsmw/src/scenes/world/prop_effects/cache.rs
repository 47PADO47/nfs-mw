use super::*;
use nfsmw_data::car::exhaust::{ADDITIVE_BLEND, particle_textures};

impl ObjectFx {
    pub(super) fn load_pending(
        &mut self,
        renderer: &mut dyn RenderBackend,
        dir: &GameDir,
        db: &blackbox_attrib::Database,
    ) {
        let keys: BTreeSet<_> = self
            .pending
            .iter()
            .filter_map(|command| match command {
                Command::Play(source) => Some(source.effect.effect),
                _ => None,
            })
            .filter(|key| !self.sets.contains_key(key))
            .collect();
        let mut groups = BTreeMap::new();
        let mut wanted = BTreeSet::new();
        for key in keys {
            if self.sets.len() + groups.len() >= 128 {
                self.rejected += 1;
                continue;
            }
            let mut emitters = nfsmw_data::world::object_fx::emitters(db, key);
            emitters.truncate(16);
            if emitters.is_empty() {
                log::debug!("prop effect {key:08X}: no supported particle group");
                self.sets.insert(key, None);
                continue;
            }
            for emitter in &emitters {
                if !self.textures.contains_key(&emitter.texture) && !self.missing_textures.contains(&emitter.texture) {
                    wanted.insert(emitter.texture);
                }
            }
            groups.insert(key, (nfsmw_data::world::object_fx::parameters(db, key).unwrap_or_default(), emitters));
        }
        if !wanted.is_empty() {
            let hashes: Vec<_> = wanted.iter().copied().collect();
            let textures = particle_textures(dir, &hashes).unwrap_or_else(|error| {
                log::warn!("prop particle textures: {error:#}");
                Vec::new()
            });
            for texture in textures {
                let Some(handle) = blackbox_scene::upload_texture(renderer, &texture) else { continue };
                let blend = match texture.alpha_blend == ADDITIVE_BLEND {
                    true => BlendMode::Additive,
                    false => BlendMode::AlphaBlend,
                };
                self.textures.insert(texture.name_hash, (handle, blend));
            }
            for hash in wanted {
                if !self.textures.contains_key(&hash) {
                    log::warn!("prop particle texture {hash:08X} unavailable");
                    self.missing_textures.insert(hash);
                }
            }
        }
        for (key, (parameters, emitters)) in groups {
            let templates: Vec<_> = emitters
                .into_iter()
                .filter(|e| self.textures.contains_key(&e.texture))
                .map(|e| (e.texture, e.spec))
                .collect();
            self.sets.insert(key, (!templates.is_empty()).then_some(Set { templates, parameters }));
        }
    }
}
