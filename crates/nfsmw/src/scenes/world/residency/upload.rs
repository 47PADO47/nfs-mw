//! Taking arrived tiles to the GPU, within a per-frame byte budget.

use super::*;

impl Residency {
    /// The bytes of section data to upload this frame. The world is concealed until it has been ready once, so it can
    /// take a bigger share of the frame then than it can while it is shown.
    pub(super) fn upload_budget(&self) -> usize {
        match self.settled {
            true => UPLOAD_BYTES_PER_FRAME,
            false => CONCEALED_UPLOAD_BYTES_PER_FRAME,
        }
    }

    /// Upload arrived sections, the wanted ones first and the oldest first among them, until the frame's byte budget
    /// is spent.
    pub(super) fn upload_arrived(&mut self, renderer: &mut dyn RenderBackend) {
        if self.arrived.is_empty() {
            return;
        }
        let wanted = &self.wanted_set;
        self.arrived.make_contiguous().sort_by_key(|data| !wanted.contains(&data.index));
        let budget = self.upload_budget();
        let mut spent = 0;
        while let Some(data) = self.arrived.pop_front() {
            if !matches!(self.tiles.get(&data.index), Some(TileState::Requested)) {
                continue; // no longer wanted
            }
            let bytes = data.upload_bytes();
            if spent > 0 && spent + bytes > budget {
                self.arrived.push_front(data);
                break;
            }
            spent += bytes;
            self.upload_tile(renderer, data);
        }
    }

    fn upload_tile(&mut self, renderer: &mut dyn RenderBackend, mut data: SectionData) {
        let mut resources = SectionResources::with_texture_detail(self.texture_discard.unwrap_or(0));
        resources.anims = std::mem::take(&mut data.anims);
        resources.upload_textures(renderer, &data);
        resources.upload_meshes(renderer, &data, &self.shared);
        // What a loader thread could work out without the meshes is already in `data`; the rest joins it to them.
        let prepared = take_prepared(&mut data, &self.catalog, &self.override_groups);
        let (mut placed, unresolved) = resolve_placed(&prepared.placed, &resources, &self.shared);
        for (index, object) in placed.iter_mut().enumerate() {
            object.key = keys::tile(data.index, index);
        }
        self.props.add_tile(data.index, &mut placed);
        resources.reflection_placed = resolve_reflections(&prepared.reflections, &resources, &self.shared);
        for (index, reflected) in resources.reflection_placed.iter_mut().enumerate() {
            reflected.bind_player(&placed, keys::tile(data.index, index | (1 << 31)));
        }
        self.unresolved += unresolved;
        let packs = data.collision.iter().map(|p| p.section).collect();
        for pack in data.collision {
            self.insert_collision(pack, data.index);
        }
        self.tiles.insert(data.index, TileState::Resident { resources: Box::new(resources), placed, packs });
        self.visual_revision = self.visual_revision.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::super::shadow_tests::fixture;
    use super::*;

    fn arrived_tile(index: usize) -> SectionData {
        SectionData {
            index,
            solids: Vec::new(),
            solid_vertices: Vec::new(),
            textures: Vec::new(),
            decoded: Vec::new(),
            anims: Vec::new(),
            scenery: Vec::new(),
            spawners: Vec::new(),
            collision: Vec::new(),
            prepared: None,
        }
    }

    #[test]
    fn the_world_uploads_more_per_frame_while_it_is_concealed() {
        let (mut residency, _) = fixture();
        assert!(!residency.settled);
        assert_eq!(residency.upload_budget(), CONCEALED_UPLOAD_BYTES_PER_FRAME);
        residency.settled = true;
        assert_eq!(residency.upload_budget(), UPLOAD_BYTES_PER_FRAME);
        const { assert!(CONCEALED_UPLOAD_BYTES_PER_FRAME > UPLOAD_BYTES_PER_FRAME) };
    }

    #[test]
    fn residency_settles_once_complete() {
        let (mut residency, mut renderer) = fixture();
        assert!(!residency.settled);
        residency.update(&mut renderer, -1.0, 0.0);
        assert!(residency.complete());
        assert!(residency.settled, "the first complete frame latches the shown-world budget");
    }

    #[test]
    fn an_arrival_nobody_requested_is_dropped_without_a_tile() {
        let (mut residency, mut renderer) = fixture();
        residency.update(&mut renderer, -1.0, 0.0);
        // A request the zone dropped: its section arrives late, and no tile waits for it.
        residency.arrived.push_back(arrived_tile(99));
        residency.upload_arrived(&mut renderer);
        assert!(residency.arrived.is_empty());
        assert!(!residency.tiles.contains_key(&99));
    }

    #[test]
    fn a_requested_arrival_becomes_resident() {
        let (mut residency, mut renderer) = fixture();
        residency.update(&mut renderer, -1.0, 0.0);
        residency.tiles.insert(99, TileState::Requested);
        residency.arrived.push_back(arrived_tile(99));
        residency.upload_arrived(&mut renderer);
        assert!(matches!(residency.tiles.get(&99), Some(TileState::Resident { .. })));
    }
}
