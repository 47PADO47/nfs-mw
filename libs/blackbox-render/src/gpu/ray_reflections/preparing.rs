//! Preparing the hit data of large meshes on a worker.
//!
//! A mesh's hit data ([`Mesh`]) is a converted copy of its vertices and triangles, classified for the shadow and
//! normal-map rules. A car of half a million vertices across its lamp states spent over 10 ms of the frame that
//! installed it there. Meshes of at least [`ASYNC_VERTICES`] vertices are copied as they are and prepared on a rayon
//! worker instead. They join the ray scene in the first frame after they are ready: until then the mesh has no hit
//! data, and an instance of it is left out of the TLAS without marking the scene incomplete, as a mesh without
//! triangles is. Each request has a ticket, so the result for a mesh destroyed in the meantime, or for a slot that was
//! reused, is dropped.
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};

use blackbox_gfx::{MeshDesc, NormalMaterialDesc};

use super::mesh::Mesh;

/// Meshes from this many vertices up are prepared on a worker.
pub(super) const ASYNC_VERTICES: usize = 8192;

type Finished = (usize, u64, Option<Mesh>);

/// The hit data of a mesh drawn with `layers` (the atlas texture of each draw) and `surfaces` (its normal material).
pub(super) fn prepare(
    desc: &MeshDesc<'_>,
    layers: &[Option<u32>],
    surfaces: &[Option<NormalMaterialDesc>],
) -> Option<Mesh> {
    let mut mesh = Mesh::new(desc, layers)?;
    super::surfaces::apply(&mut mesh, desc, layers, surfaces);
    super::shadow::classify(&mut mesh, desc, layers, surfaces);
    mesh.partition_cutouts();
    Some(mesh)
}

/// Meshes being prepared on workers.
pub(super) struct Preparing {
    /// The latest request of each mesh still waiting.
    tickets: HashMap<usize, u64>,
    next: u64,
    sender: Sender<Finished>,
    results: Mutex<Receiver<Finished>>,
}

impl Default for Preparing {
    fn default() -> Self {
        let (sender, results) = channel();
        Self { tickets: HashMap::new(), next: 0, sender, results: Mutex::new(results) }
    }
}

impl Preparing {
    /// Prepare mesh `handle` on a worker, from a copy of `desc`.
    pub fn spawn(
        &mut self,
        handle: usize,
        desc: &MeshDesc<'_>,
        layers: Vec<Option<u32>>,
        surfaces: Vec<Option<NormalMaterialDesc>>,
    ) {
        let ticket = self.next;
        self.next += 1;
        self.tickets.insert(handle, ticket);
        let vertices = desc.vertices.to_vec();
        let indices = desc.indices.to_vec();
        let draws = desc.draws.clone();
        let sender = self.sender.clone();
        rayon::spawn(move || {
            let _scope = blackbox_trace::scope("Vulkan RT: prepare mesh");
            let desc = MeshDesc {
                label: "",
                vertices: &vertices,
                tangents: None,
                secondary_uvs: None,
                indices: &indices,
                draws,
            };
            // The scene may be gone already; then nobody wants the result.
            let _ = sender.send((handle, ticket, prepare(&desc, &layers, &surfaces)));
        });
    }

    /// Forget the request for `handle`: the mesh was destroyed, or prepared again on the calling thread.
    pub fn cancel(&mut self, handle: usize) {
        self.tickets.remove(&handle);
    }

    /// The results finished so far that are still wanted, by mesh.
    pub fn finished(&mut self) -> Vec<(usize, Option<Mesh>)> {
        let results = self.results.get_mut().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut out = Vec::new();
        while let Ok((handle, ticket, mesh)) = results.try_recv() {
            if self.tickets.get(&handle) != Some(&ticket) {
                continue;
            }
            self.tickets.remove(&handle);
            out.push((handle, mesh));
        }
        out
    }

    /// Whether requests are still out, which the tests wait for.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.tickets.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_gfx::{BlendMode, DrawRange, Shading, Vertex};

    fn desc(vertices: &[Vertex], indices: &[u16]) -> MeshDesc<'static> {
        let vertices: &'static [Vertex] = Vec::leak(vertices.to_vec());
        let indices: &'static [u16] = Vec::leak(indices.to_vec());
        MeshDesc {
            label: "test",
            vertices,
            tangents: None,
            secondary_uvs: None,
            indices,
            draws: vec![DrawRange {
                first_index: 0,
                index_count: indices.len() as u32,
                base_vertex: 0,
                texture: None,
                blend: BlendMode::Opaque,
                shading: Shading::Lit,
                two_sided: true,
            }],
        }
    }

    fn wait(preparing: &mut Preparing) -> Vec<(usize, Option<Mesh>)> {
        let mut out = Vec::new();
        let started = std::time::Instant::now();
        while !preparing.is_empty() && started.elapsed() < std::time::Duration::from_secs(10) {
            out.extend(preparing.finished());
            std::thread::yield_now();
        }
        out
    }

    #[test]
    fn a_worker_prepares_the_same_hit_data_as_the_calling_thread() {
        let vertex =
            Vertex { position: [1.0, 2.0, 3.0], normal: [0.0, 0.0, 1.0], color_bgra: [10, 20, 30, 255], uv: [0.5; 2] };
        let desc = desc(&[vertex; 4], &[0, 1, 2, 2, 1, 3]);
        let here = prepare(&desc, &[Some(7)], &[None]).expect("triangles");
        let mut preparing = Preparing::default();
        preparing.spawn(3, &desc, vec![Some(7)], vec![None]);
        let done = wait(&mut preparing);
        let [(3, Some(there))] = done.as_slice() else { panic!("one result for mesh 3") };
        assert_eq!(there.triangles, here.triangles);
        assert_eq!(there.textures, here.textures);
        assert_eq!(bytemuck::cast_slice::<_, f32>(&there.vertices), bytemuck::cast_slice::<_, f32>(&here.vertices));
    }

    #[test]
    fn a_cancelled_or_replaced_request_is_dropped() {
        let vertex = Vertex { position: [0.0; 3], normal: [0.0, 0.0, 1.0], color_bgra: [255; 4], uv: [0.0; 2] };
        let desc = desc(&[vertex; 3], &[0, 1, 2]);
        let mut preparing = Preparing::default();
        preparing.spawn(1, &desc, vec![Some(1)], vec![None]);
        preparing.cancel(1);
        preparing.spawn(2, &desc, vec![Some(1)], vec![None]);
        preparing.spawn(2, &desc, vec![Some(9)], vec![None]);
        let done = wait(&mut preparing);
        assert_eq!(done.len(), 1, "only the latest request of mesh 2 lands");
        let (2, Some(mesh)) = &done[0] else { panic!("mesh 2") };
        assert_eq!(mesh.textures, vec![9]);
    }
}
