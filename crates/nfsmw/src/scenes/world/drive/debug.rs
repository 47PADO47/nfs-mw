//! `debug collisions`: the car's contact points drawn in the world as small markers, so a wall that should not
//! be there (or one that is missing) can be found by driving at it.
//!
//! Every physics step reports where the body met a barrier, a steep face or a prop (`walls.rs`) and where each
//! tyre's ray found the road. A contact becomes a cube at the point and a stick along its normal. Wall and prop
//! contacts stay for a short while, so a hit that lasted one step can still be seen; the tyre hits are the latest.
//! The contacts are kept whether or not they are drawn, so switching the view on shows what just happened.

use blackbox_render::{BlendMode, DrawRange, Instance, MeshDesc, MeshHandle, Renderer, Shading, Vertex};
use glam::{Mat4, Vec3};

use super::walls::{ContactKind, ContactPoint};
use crate::scenes::world::space;

/// Physics steps a wall or prop contact stays on screen (half a second at 60 Hz).
const LIFETIME_STEPS: u32 = 30;
/// The most contacts kept.
const MAX_MARKERS: usize = 256;
/// The cube at a contact point and the stick along its normal, metres.
const CUBE: f32 = 0.22;
const STICK_LENGTH: f32 = 1.2;
const STICK_WIDTH: f32 = 0.05;

/// What a marker marks; each has its own colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    Barrier,
    Face,
    PropRigid,
    PropLight,
    Tyre,
}

const KINDS: [MarkerKind; 5] =
    [MarkerKind::Barrier, MarkerKind::Face, MarkerKind::PropRigid, MarkerKind::PropLight, MarkerKind::Tyre];

impl MarkerKind {
    /// Vertex colour, B G R A: the world is pre-lit with 0x80 as full brightness.
    fn colour(self) -> [u8; 4] {
        match self {
            Self::Barrier => [0x18, 0x18, 0x80, 0xFF],
            Self::Face => [0x18, 0x60, 0x80, 0xFF],
            Self::PropRigid => [0x80, 0x18, 0x80, 0xFF],
            Self::PropLight => [0x18, 0x80, 0x80, 0xFF],
            Self::Tyre => [0x20, 0x80, 0x20, 0xFF],
        }
    }

    fn index(self) -> usize {
        KINDS.iter().position(|k| *k == self).unwrap_or(0)
    }
}

impl From<ContactKind> for MarkerKind {
    fn from(kind: ContactKind) -> Self {
        match kind {
            ContactKind::Barrier => Self::Barrier,
            ContactKind::Face => Self::Face,
            ContactKind::PropRigid => Self::PropRigid,
            ContactKind::PropLight => Self::PropLight,
        }
    }
}

/// What the colours mean, for the console.
pub const LEGEND: &str =
    "red: barrier, orange: steep face, magenta: rigid prop, yellow: light prop, green: tyre ray hits";

/// The meshes the markers are drawn with, one per kind (a box from the origin along +x, 1 m long, 1 m square).
pub struct MarkerMeshes {
    meshes: [MeshHandle; 5],
}

impl MarkerMeshes {
    pub fn upload(renderer: &mut Renderer) -> Self {
        Self { meshes: KINDS.map(|kind| upload_box(renderer, "contact marker", kind.colour())) }
    }
}

/// Uploads a pre-lit box of one colour (B G R A, 0x80 is full brightness) from the origin along +x, 1 m long and
/// 1 m square: the unit shape every marker is a scaled copy of.
pub(in crate::scenes::world) fn upload_box(
    renderer: &mut Renderer,
    label: &'static str,
    colour: [u8; 4],
) -> MeshHandle {
    let mut vertices = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    // Six faces of the box x in 0..1, y and z in -0.5..0.5, each as two triangles.
    let faces: [[[f32; 3]; 4]; 6] = [
        [[0.0, -0.5, -0.5], [0.0, 0.5, -0.5], [0.0, 0.5, 0.5], [0.0, -0.5, 0.5]],
        [[1.0, -0.5, -0.5], [1.0, 0.5, -0.5], [1.0, 0.5, 0.5], [1.0, -0.5, 0.5]],
        [[0.0, -0.5, -0.5], [1.0, -0.5, -0.5], [1.0, -0.5, 0.5], [0.0, -0.5, 0.5]],
        [[0.0, 0.5, -0.5], [1.0, 0.5, -0.5], [1.0, 0.5, 0.5], [0.0, 0.5, 0.5]],
        [[0.0, -0.5, -0.5], [1.0, -0.5, -0.5], [1.0, 0.5, -0.5], [0.0, 0.5, -0.5]],
        [[0.0, -0.5, 0.5], [1.0, -0.5, 0.5], [1.0, 0.5, 0.5], [0.0, 0.5, 0.5]],
    ];
    for face in faces {
        let base = vertices.len() as u16;
        for position in face {
            vertices.push(Vertex { position, normal: [0.0, 0.0, 1.0], color_bgra: colour, uv: [0.0, 0.0] });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    renderer.create_mesh(&MeshDesc {
        label,
        vertices: &vertices,
        indices: &indices,
        draws: vec![DrawRange {
            first_index: 0,
            index_count: indices.len() as u32,
            base_vertex: 0,
            texture: None,
            blend: BlendMode::Opaque,
            shading: Shading::Prelit,
        }],
    })
}

#[derive(Debug, Clone, Copy)]
struct Marker {
    /// Physics space.
    point: Vec3,
    normal: Vec3,
    kind: MarkerKind,
    age: u32,
}

/// The contacts to draw.
#[derive(Debug, Default)]
pub struct ContactMarkers {
    enabled: bool,
    /// Wall and prop contacts that have not expired.
    recent: Vec<Marker>,
    /// Where the tyres' rays hit the road in the latest step.
    tyres: Vec<Marker>,
}

impl ContactMarkers {
    /// Draw the markers (`true`) or not.
    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
    }

    /// Takes the contacts of one physics step: the wall and prop contacts and the tyre ray hits (point and normal).
    pub fn record(&mut self, contacts: &[ContactPoint], tyre_hits: impl IntoIterator<Item = (Vec3, Vec3)>) {
        for m in &mut self.recent {
            m.age += 1;
        }
        self.recent.retain(|m| m.age < LIFETIME_STEPS);
        self.recent.extend(contacts.iter().map(|c| Marker {
            point: c.point,
            normal: c.normal,
            kind: c.kind.into(),
            age: 0,
        }));
        let excess = self.recent.len().saturating_sub(MAX_MARKERS);
        self.recent.drain(..excess);
        self.tyres = tyre_hits
            .into_iter()
            .map(|(point, normal)| Marker { point, normal, kind: MarkerKind::Tyre, age: 0 })
            .collect();
    }

    /// How many markers are on screen: (wall and prop contacts, tyre hits).
    pub fn counts(&self) -> (usize, usize) {
        (self.recent.len(), self.tyres.len())
    }

    /// What to draw: a stick along the normal and a cube at the point for each marker, in render space. Nothing
    /// while the view is off.
    fn placements(&self) -> Vec<(MarkerKind, Mat4)> {
        if !self.enabled {
            return Vec::new();
        }
        let mut out = Vec::new();
        for m in self.recent.iter().chain(&self.tyres) {
            let origin = space::to_render(m.point.to_array());
            let direction = space::to_render(m.normal.to_array()).normalize_or(Vec3::Z);
            out.push((m.kind, along(origin, direction, STICK_LENGTH, STICK_WIDTH)));
            out.push((m.kind, along(origin, direction, CUBE, CUBE)));
        }
        out
    }

    /// Append the instances of the markers.
    pub fn instances(&self, meshes: &MarkerMeshes, out: &mut Vec<Instance>) {
        out.extend(
            self.placements()
                .into_iter()
                .map(|(kind, transform)| Instance { mesh: meshes.meshes[kind.index()], transform }),
        );
    }
}

/// A box from `origin` along `direction`, `length` long and `width` square.
fn along(origin: Vec3, direction: Vec3, length: f32, width: f32) -> Mat4 {
    let helper = if direction.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
    let side = direction.cross(helper).normalize();
    let up = direction.cross(side);
    Mat4::from_cols(
        (direction * length).extend(0.0),
        (side * width).extend(0.0),
        (up * width).extend(0.0),
        origin.extend(1.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contact(kind: ContactKind) -> ContactPoint {
        ContactPoint { point: Vec3::new(1.0, 2.0, 3.0), normal: Vec3::NEG_Z, kind }
    }

    fn tyres() -> Vec<(Vec3, Vec3)> {
        vec![(Vec3::ZERO, Vec3::Y); 4]
    }

    #[test]
    fn contacts_are_kept_while_the_view_is_off_but_not_drawn() {
        let mut m = ContactMarkers::default();
        m.record(&[contact(ContactKind::Barrier)], tyres());
        assert_eq!(m.counts(), (1, 4));
        assert!(m.placements().is_empty());
        m.set_enabled(true);
        assert_eq!(m.placements().len(), 2 * (1 + 4), "a stick and a cube each");
    }

    #[test]
    fn contacts_stay_a_while_and_the_tyre_hits_are_the_latest() {
        let mut m = ContactMarkers::default();
        m.record(&[contact(ContactKind::Barrier), contact(ContactKind::PropLight)], tyres());
        assert_eq!(m.counts(), (2, 4));
        for _ in 0..LIFETIME_STEPS - 1 {
            m.record(&[], vec![(Vec3::ZERO, Vec3::Y)]);
        }
        assert_eq!(m.counts(), (2, 1), "still there, the tyres are the last step's");
        m.record(&[], vec![]);
        assert_eq!(m.counts(), (0, 0), "expired");
    }

    #[test]
    fn the_list_is_bounded() {
        let mut m = ContactMarkers::default();
        let many: Vec<ContactPoint> = (0..400).map(|_| contact(ContactKind::Face)).collect();
        m.record(&many, tyres());
        assert_eq!(m.counts().0, MAX_MARKERS);
    }

    #[test]
    fn markers_are_placed_in_render_space_with_their_kind() {
        let mut m = ContactMarkers::default();
        m.set_enabled(true);
        // Physics (1, 2, 3) is render (3, -1, 2); the normal -z is render -x.
        m.record(&[contact(ContactKind::PropLight)], vec![]);
        let placed = m.placements();
        assert!(placed.iter().all(|(kind, _)| *kind == MarkerKind::PropLight));
        let (_, stick) = placed[0];
        assert!((stick.transform_point3(Vec3::ZERO) - Vec3::new(3.0, -1.0, 2.0)).length() < 1e-5);
        let tip = stick.transform_point3(Vec3::X);
        assert!((tip - Vec3::new(3.0 - STICK_LENGTH, -1.0, 2.0)).length() < 1e-4, "{tip:?}");
    }

    #[test]
    fn a_stick_runs_from_the_point_along_the_normal() {
        let t = along(Vec3::new(10.0, 0.0, 0.0), Vec3::Y, 2.0, 0.1);
        let end = t.transform_point3(Vec3::new(1.0, 0.0, 0.0));
        let start = t.transform_point3(Vec3::ZERO);
        assert!((start - Vec3::new(10.0, 0.0, 0.0)).length() < 1e-5);
        assert!((end - Vec3::new(10.0, 2.0, 0.0)).length() < 1e-5, "{end:?}");
        let corner = t.transform_point3(Vec3::new(0.0, 0.5, 0.5));
        assert!(((corner - start).length() - 0.1 * 0.5f32.sqrt()).abs() < 1e-5, "{corner:?}");
    }

    #[test]
    fn each_kind_has_its_own_slot_and_colour() {
        let mut colours: Vec<[u8; 4]> = KINDS.iter().map(|k| k.colour()).collect();
        colours.sort();
        colours.dedup();
        assert_eq!(colours.len(), KINDS.len());
        assert!(KINDS.iter().enumerate().all(|(i, k)| k.index() == i));
    }
}
