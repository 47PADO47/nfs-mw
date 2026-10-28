use super::*;
use glam::Quat;

fn arc(time: f32) -> Mat4 {
    let angle = time * 2.0;
    Mat4::from_rotation_translation(Quat::from_rotation_z(angle), Vec3::new(angle.sin(), -angle.cos(), 0.0) * 20.0)
}

#[test]
fn exposure_retains_a_corner_at_different_presentation_rates() {
    let mut lengths = Vec::new();
    for fps in [30, 60, 120, 144, 240] {
        let mut history = Smooth::default();
        for frame in 0..=fps {
            history.advance(1.0 / fps as f32, arc(frame as f32 / fps as f32), true);
        }
        let path = history.path(Vec3::ZERO, 100, 100);
        let length: f32 = path.windows(2).map(|pair| pair[0].distance(pair[1])).sum();
        assert!((length - 4.8).abs() < 0.08, "{fps}: {length}");
        assert!(path.len() <= 65);
        assert!(path[0].abs_diff_eq(arc(1.0).w_axis.truncate(), 1e-4));
        let chord = path.last().unwrap() - path[0];
        let deviation = path.iter().map(|p| (*p - path[0]).cross(chord).length() / chord.length()).fold(0.0, f32::max);
        assert!(deviation > 0.08, "retained history should describe an arc: {fps}, {deviation}");
        lengths.push(length);
    }
    assert!(
        lengths.iter().copied().fold(f32::NEG_INFINITY, f32::max)
            - lengths.iter().copied().fold(f32::INFINITY, f32::min)
            < 0.08
    );
}

#[test]
fn cap_release_pause_and_teleports_remain_bounded() {
    let mut history = Smooth::default();
    for frame in 0..60 {
        history.advance(1.0 / 120.0, Mat4::from_translation(Vec3::X * frame as f32), true);
    }
    let path = history.path(Vec3::ZERO, 100, 200);
    let length: f32 = path.windows(2).map(|p| p[0].distance(p[1])).sum();
    assert!((length - 6.0).abs() < 0.01);
    let head = *path.first().unwrap();
    history.advance(0.0, Mat4::from_translation(head), true);
    assert_eq!(path, history.path(Vec3::ZERO, 100, 200));
    history.advance(0.06, Mat4::from_translation(head), false);
    assert!((history.fade() - 0.5).abs() < 1e-5);
    history.advance(0.061, Mat4::from_translation(head), false);
    assert!(history.path(Vec3::ZERO, 100, 200).is_empty());
    history.advance(0.01, Mat4::from_translation(Vec3::X * 1000.0), true);
    assert!(history.path(Vec3::ZERO, 100, 100).len() <= 1);
}

#[test]
fn engagement_settles_and_release_erases_oldest_history_first() {
    let mut history = Smooth::default();
    history.advance(0.01, Mat4::IDENTITY, true);
    let start = history.engagement(100);
    assert!(start > 1.15 && start < 1.23);
    assert_eq!(history.engagement(0), 1.0);
    for i in 1..=30 {
        history.advance(0.01, Mat4::from_translation(Vec3::X * i as f32 * 0.4), true);
    }
    assert_eq!(history.engagement(200), 1.0);
    let before = history.path(Vec3::ZERO, 100, 100);
    history.advance(0.06, Mat4::from_translation(before[0] + Vec3::X), false);
    let after = history.path(Vec3::ZERO, 100, 100);
    assert_eq!(before[0], after[0], "release detaches from the moving car");
    let length = |p: &[Vec3]| p.windows(2).map(|p| p[0].distance(p[1])).sum::<f32>();
    assert!((length(&after) / length(&before) - 0.5).abs() < 0.03);
}

#[test]
fn preview_cycle_has_a_pulse_release_and_dark_interval() {
    let mut history = Smooth::default();
    let config = crate::settings::TailPreview { cycle: true, ..Default::default() };
    history.preview(Mat4::IDENTITY, config, 0.02);
    assert!(history.engagement(100) > 1.1);
    history.preview(Mat4::IDENTITY, config, 1.76);
    assert!((history.fade() - 0.5).abs() < 0.001);
    history.preview(Mat4::IDENTITY, config, 1.95);
    assert_eq!(history.fade(), 0.0);
    assert!(history.path(Vec3::ZERO, 100, 100).is_empty());
    history.preview(Mat4::IDENTITY, config, 2.02);
    assert_eq!(history.fade(), 1.0);
}
