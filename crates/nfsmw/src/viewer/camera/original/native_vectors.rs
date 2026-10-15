//! Synthetic pose sequences replayed through unchanged native Cubic update.
use super::follow::BodyInput;
use glam::Vec3;
use nfsmw_data::camera::View;

fn view(mode: usize) -> View {
    let (lag, height, angle) = match mode < 2 {
        true => (0.3, 1.2, 3.5),
        false => (-6.0, 2.0, 1.25),
    };
    View {
        fov: [84.0; 2],
        height: [height; 2],
        lag: [lag; 2],
        angle: [angle; 2],
        stiffness: [0.5; 2],
        lat_offset: [0.0; 2],
        tilting: [true; 2],
        selectable: [true; 2],
    }
}

fn body() -> BodyInput {
    BodyInput {
        position: Vec3::ZERO,
        forward: Vec3::X,
        up: Vec3::Z,
        velocity: Vec3::ZERO,
        acceleration: Vec3::ZERO,
        speed: 0.0,
        top_speed: 71.0,
        grounded: true,
        nos_engaged: false,
        collision_damping: 0.0,
        drift: 0.0,
        up_influence: Vec3::ZERO,
        zoom: 1.0,
        look_back: false,
        gear: 1,
    }
}

#[path = "native_vectors/group_0.rs"]
mod group_0;

#[path = "native_vectors/group_1.rs"]
mod group_1;

#[path = "native_vectors/group_2.rs"]
mod group_2;

#[path = "native_vectors/group_3.rs"]
mod group_3;

#[path = "native_vectors/group_4.rs"]
mod group_4;

#[path = "native_vectors/group_5.rs"]
mod group_5;

#[path = "native_vectors/group_6.rs"]
mod group_6;

#[path = "native_vectors/group_7.rs"]
mod group_7;

#[path = "native_vectors/group_8.rs"]
mod group_8;
