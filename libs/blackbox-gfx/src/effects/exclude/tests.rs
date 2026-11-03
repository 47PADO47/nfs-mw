use super::*;
use crate::EffectLayer;

#[test]
fn owner_box_subtraction_preserves_only_exterior_triangle_area() {
    let mut out = Vec::new();
    EffectLayer::quad(
        &mut out,
        [Vec3::new(-2.0, -2.0, 0.0), Vec3::new(2.0, -2.0, 0.0), Vec3::new(2.0, 2.0, 0.0), Vec3::new(-2.0, 2.0, 0.0)],
        [255; 4],
    );
    EffectLayer::exclude_box(&mut out, 0, Mat4::IDENTITY, Vec3::splat(-1.0), Vec3::splat(1.0));
    let area: f32 = out
        .as_chunks::<3>()
        .0
        .iter()
        .map(|tri| {
            let p: [Vec3; 3] = std::array::from_fn(|i| Vec3::from(tri[i].position));
            let centre = (p[0] + p[1] + p[2]) / 3.0;
            assert!(centre.x.abs() >= 1.0 - 1e-5 || centre.y.abs() >= 1.0 - 1e-5);
            assert!(tri.iter().all(|v| v.uv.iter().all(|u| (0.0..=1.0).contains(u))));
            (p[1] - p[0]).cross(p[2] - p[0]).length() * 0.5
        })
        .sum();
    assert!((area - 12.0).abs() < 1e-4, "{area}");
}
