//! A bounded spotlight supplied by a game, independent of the renderer backend.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalLight {
    pub position: [f32; 3],
    pub range: f32,
    /// Unit direction from the fixture into the scene.
    pub direction: [f32; 3],
    pub outer_cos: f32,
    /// Artistic diffuse illumination in the renderer's authored colour domain.
    pub color: [f32; 3],
    pub inner_cos: f32,
    /// Optional world up for an upper beam fade, from about 6 to 17 degrees above horizontal.
    pub upper_plane: Option<[f32; 3]>,
    /// Whether this light spends a local shadow-map slot.
    pub casts_shadow: bool,
    /// Scene object that must not block its own light.
    pub shadow_exclude: Option<crate::InstanceKey>,
}
impl LocalLight {
    pub fn valid(&self) -> bool {
        let upper_valid = self
            .upper_plane
            .is_none_or(|v| v.iter().all(|x| x.is_finite()) && v.iter().map(|x| x * x).sum::<f32>() > 0.001);
        upper_valid
            && self.position.iter().chain(&self.direction).chain(&self.color).all(|v| v.is_finite())
            && self.range.is_finite()
            && self.range > 0.3
            && self.range <= 1000.0
            && self.direction.iter().map(|v| v * v).sum::<f32>() > 0.001
            && self.color.iter().all(|v| *v >= 0.0)
            && (0.05..0.999).contains(&self.outer_cos)
            && self.inner_cos.is_finite()
            && self.inner_cos > self.outer_cos
            && self.inner_cos <= 1.0
    }
}
