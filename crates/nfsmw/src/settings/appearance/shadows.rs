//! Original PC fitted shadow-camera constants; higher resolutions need no new assets.
pub(crate) const PROJECTION: blackbox_gfx::ShadowProjection = blackbox_gfx::ShadowProjection {
    frustum: [0.01, 50.0],
    depth_bias: [4.0, 4.0],
    near: 20.1,
    width_scale: 1.5,
    height_scale: 10.0,
    far_scale: 10.0,
    offset: [0.0; 2],
    post_transform: [[1.0, 0.0, 0.0, 0.0], [0.0, 0.0, -0.5, 0.0], [0.0, 2.0, 0.0, 0.0], [0.0, -1.0, 0.5, 1.0]],
};
