use super::*;
use crate::ui::Catalog;
use blackbox_feng::Runtime;
use game_install::GameDir;

fn step(rt: &mut Runtime, binding: &mut bind::HudBinding, state: &HudState) {
    binding.apply(rt, state);
    rt.update(1.0);
    binding.finish_frame(rt, state);
}

#[test]
#[ignore = "needs installed HUD assets"]
fn installed_pursuit_hud_drives_heat_counts_and_radar_without_template_numbers() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/InGameC.bun"]);
    let assets = crate::ui::UiAssets::load(&dir).unwrap();
    let mut rt = Runtime::new();
    let package = rt.load(catalog.find("HUD_SingleRace.fng").expect("installed HUD").clone());
    let mut binding = bind::HudBinding::new(&mut rt, package);
    binding.advance_mirror_fade(1.0);
    let state = HudState {
        vehicle_heat: 3.8,
        cops_engaged: 4,
        cops_damaged: 2,
        radar: Some(RadarHud { angle: 0.4, distance: 50.0 }),
        cop_nearby: true,
        ..Default::default()
    };
    step(&mut rt, &mut binding, &state);
    let tree = rt.tree(package);
    let labels: Vec<_> =
        tree.nodes.iter().filter(|n| n.text.is_some() && matches!(n.name_hash, 0xA81A_2028 | 0x7F91_DA62)).collect();
    assert!(labels.len() >= 2, "the real package contains the duplicate heat labels");
    assert!(labels.iter().all(|n| n.text.as_deref() == Some("x3")));
    for hash in [0x2419_6AD4, 0x69AA_01E7, 0x9F59_065A] {
        let node = tree.nodes.iter().find(|n| n.name_hash == hash).expect("installed radar object");
        assert!(node.visible, "radar object {hash:x} is visible in free roam");
    }
    let mut chase = state.clone();
    chase.radar = None;
    chase.cop_nearby = false;
    chase.pursuit = Some(PursuitHud { secs: 10.0, cops_destroyed: 1, ..Default::default() });
    step(&mut rt, &mut binding, &chase);
    let tree = rt.tree(package);
    for (hash, text) in [(0x8149_18CA, "4"), (0x5FA7_0D4C, "2"), (0xA16F_9F1E, "1")] {
        let node = tree.nodes.iter().find(|n| n.name_hash == hash).expect("installed pursuit counter");
        assert!(node.visible);
        assert_eq!(node.text.as_deref(), Some(text));
    }
    assert!(!tree.nodes.iter().find(|n| n.name_hash == 0x2419_6AD4).unwrap().visible);
    authored_layouts(&mut rt, package, &mut binding, &state, &assets);
    signed_meter(&mut rt, package, &mut binding, &assets);
}

fn authored_layouts(
    rt: &mut Runtime,
    package: blackbox_feng::PackageId,
    binding: &mut bind::HudBinding,
    state: &HudState,
    assets: &crate::ui::UiAssets,
) {
    use crate::settings::HudLayout;
    use crate::ui::present::Screen;

    let backing = rt.find(package, 0x839E_7D77).unwrap();
    let group = rt.find(package, 0x0627_43F5).unwrap();
    let mirror = rt.find(package, 0x9EE0_6631).unwrap();
    binding.set_mirror_shown(false);
    for _ in 0..3 {
        step(rt, binding, state);
        assert!((rt.position(group).unwrap().y + 178.0).abs() < 1e-3);
        assert!((rt.position(backing).unwrap().y + 181.0).abs() < 1e-3);
        assert_eq!(rt.script_of(backing), Some(radar::APPEAR));
        assert_eq!(rt.script_of(mirror), Some(radar::HIDE));
    }
    let tree = rt.tree(package);
    standalone_opacity(&tree, assets);
    let viewport = viewport::HudViewport::new(&tree, &[]);
    for (width, height) in [(1440.0, 1080.0), (3840.0, 2160.0), (5120.0, 1440.0)] {
        let screen = Screen { width, height, pixels_per_point: 1.0 };
        let mut laid_out = tree.clone();
        viewport.apply(&mut laid_out, screen, HudLayout::Pc);
        let node = laid_out.nodes.iter().find(|n| n.name_hash == 0x839E_7D77).unwrap();
        let x = width * 0.5 + node.position.x * screen.scale();
        let y = height * 0.5 + node.position.y * screen.scale();
        assert!((x / width - 0.5).abs() < 0.003, "detector stays centered at {width}x{height}");
        assert!(y > height * 0.05 && y < height * 0.1, "standalone detector is at the top: {y}/{height}");
    }
    binding.set_mirror_shown(true);
    step(rt, binding, state);
    assert!((rt.position(group).unwrap().y + 121.0).abs() < 1e-3, "mirror mode uses its authored INIT pose");
    let tree = rt.tree(package);
    assert!(!tree.nodes[backing.index].visible, "standalone backing hides behind the mirror layout");
    assert!(tree.nodes[mirror.index].visible);
    // A stale detection must not resurrect the group in an active chase.
    let mut chase = state.clone();
    chase.pursuit = Some(PursuitHud::default());
    step(rt, binding, &chase);
    let tree = rt.tree(package);
    assert!(!tree.nodes[group.index].visible);
    assert!(!tree.nodes[rt.find(package, 0x2419_6AD4).unwrap().index].visible);
    assert!(tree.nodes[mirror.index].visible, "pursuit retains the mirror backing");
    chase.pursuit.as_mut().unwrap().cooldown = true;
    step(rt, binding, &chase);
    assert!(rt.tree(package).nodes[group.index].visible);
    binding.set_mirror_shown(false);
    step(rt, binding, &chase);
    assert!((rt.position(group).unwrap().y + 178.0).abs() < 1e-3);
    assert!(rt.tree(package).nodes[backing.index].visible);
}

fn standalone_opacity(tree: &blackbox_feng::UiTree, assets: &crate::ui::UiAssets) {
    use crate::gui::UiOutput;
    use crate::ui::present::{BlackboxPresenter, Screen};
    use blackbox_feng::NodeKind;
    use blackbox_gfx::UiTextureId;

    let backing = tree.nodes.iter().find(|node| node.name_hash == 0x839E_7D77).unwrap();
    assert_eq!(backing.world_colour[3], 200);
    let NodeKind::Image { texture, .. } = backing.kind else { panic!("radar backing is an image") };
    let id = UiTextureId((1 << 62) | u64::from(assets.resolve(texture)));
    let mut output = UiOutput::default();
    let screen = Screen { width: 3840.0, height: 2160.0, pixels_per_point: 1.0 };
    BlackboxPresenter::default().present(tree, assets, screen, &mut output);
    let layer = output.layer.unwrap();
    let vertices: Vec<_> =
        layer.meshes.iter().filter(|mesh| mesh.texture == id).flat_map(|mesh| &mesh.vertices).collect();
    assert!(!vertices.is_empty(), "the actual standalone detector backing is presented");
    assert!(vertices.iter().all(|vertex| vertex.color_rgba[3] == 200), "authored opacity is preserved");
}

fn signed_meter(
    rt: &mut Runtime,
    package: blackbox_feng::PackageId,
    binding: &mut bind::HudBinding,
    assets: &crate::ui::UiAssets,
) {
    let hashes = [0xE0E0_169B, 0x8EEE_BD33, 0x47A4_E2A9, 0x8E36_53A6, 0x30F3_9A6F];
    let bars = hashes.map(|hash| rt.find(package, hash).unwrap());
    // Authored source widths and centers, rather than whatever a prior HUD state wrote.
    let original = bars.map(|object| {
        let data = &rt.package(package).unwrap().objects[object.index].data;
        (data.position().x, data.size().x)
    });
    let mut state = HudState { pursuit: Some(PursuitHud::default()), ..Default::default() };
    for t in [-1.0, -0.75, -0.5, -0.1, 0.0, 0.1, 0.5, 0.75, 1.0] {
        state.pursuit.as_mut().unwrap().busted = t;
        step(rt, binding, &state);
        for i in [0, 1, 3, 4] {
            let at = rt.position(bars[i]).unwrap().x;
            let width = rt.size(bars[i]).unwrap().x;
            let direction = if i < 2 { 1.0 } else { -1.0 };
            let actual = at + direction * width * 0.5;
            let expected = original[i].0 + direction * original[i].1 * 0.5;
            assert!((actual - expected).abs() < 1e-3, "segment {i}, signed input {t} retains its fixed edge");
        }
        assert_eq!(rt.script_of(bars[0]), Some(if t > 0.5 { 0x026D_ED57 } else { radar::INIT }));
        assert_eq!(rt.script_of(bars[4]), Some(if t < -0.5 { 0x026D_ED57 } else { radar::INIT }));
    }
    let meter = rt.find(package, 0x7B42_2BA3).unwrap();
    let cooldown = rt.find(package, 0x84A2_26EC).unwrap();
    assert_group_drawn(&rt.tree(package), meter.index, assets, true);
    assert_group_drawn(&rt.tree(package), cooldown.index, assets, false);
    state.pursuit.as_mut().unwrap().cooldown = true;
    state.pursuit.as_mut().unwrap().evade = 0.25;
    step(rt, binding, &state);
    step(rt, binding, &state);
    let tree = rt.tree(package);
    assert_group_drawn(&tree, meter.index, assets, false);
    assert_group_drawn(&tree, cooldown.index, assets, true);
    let fill = rt.find(package, 0xCF81_7638).unwrap();
    assert!((rt.size(fill).unwrap().x - 54.0).abs() < 1e-3);
    state.pursuit.as_mut().unwrap().cooldown = false;
    step(rt, binding, &state);
    step(rt, binding, &state);
    assert_group_drawn(&rt.tree(package), meter.index, assets, true);
    assert_group_drawn(&rt.tree(package), cooldown.index, assets, false);
}

fn assert_group_drawn(tree: &blackbox_feng::UiTree, group: usize, assets: &crate::ui::UiAssets, expected: bool) {
    use crate::gui::UiOutput;
    use crate::ui::present::{BlackboxPresenter, Screen};

    let under_group = |mut index: usize| {
        for _ in 0..tree.nodes.len() {
            if index == group {
                return true;
            }
            let Some(parent) = tree.nodes[index].parent else { return false };
            index = parent;
        }
        false
    };
    let mut subtree = tree.clone();
    subtree.draw_order.retain(|index| under_group(*index));
    let mut output = UiOutput::default();
    let screen = Screen { width: 3840.0, height: 2160.0, pixels_per_point: 1.0 };
    BlackboxPresenter::default().present(&subtree, assets, screen, &mut output);
    let layer = output.layer.unwrap();
    let drawn = layer.meshes.iter().any(|mesh| {
        mesh.indices.as_chunks::<3>().0.iter().any(|indices| {
            let vertices = [indices[0], indices[1], indices[2]].map(|i| &mesh.vertices[i as usize]);
            let [a, b, c] = vertices.map(|v| glam::Vec2::from_array(v.position));
            let area = (b - a).perp_dot(c - a).abs();
            area > 1e-3 && vertices.iter().any(|v| v.color_rgba.iter().any(|channel| *channel > 0))
        })
    });
    assert_eq!(drawn, expected, "installed group {:08x} presents visible triangle area", tree.nodes[group].name_hash);
}

/// The cool-down bar's "HIDING" mark (`docs/specs/pursuit-hiding.md` section 5): hidden at rest, faded in while a
/// hiding spot latches the player, faded out again when it lets go.
#[test]
#[ignore = "needs installed HUD assets"]
fn installed_hiding_mark_fades_in_under_the_cooldown_bar_and_out_again() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/InGameC.bun"]);
    let mut rt = Runtime::new();
    let package = rt.load(catalog.find("HUD_SingleRace.fng").expect("installed HUD").clone());
    let mut binding = bind::HudBinding::new(&mut rt, package);
    binding.advance_mirror_fade(1.0);
    let mut state = HudState {
        pursuit: Some(PursuitHud { secs: 30.0, evade: 0.4, cooldown: true, ..Default::default() }),
        ..Default::default()
    };
    let marks = [0x60B4_5523, 0x3807_4901];
    let alpha = |rt: &Runtime, hash: u32| rt.alpha(rt.find(package, hash).unwrap()).unwrap();
    for _ in 0..8 {
        step(&mut rt, &mut binding, &state);
    }
    for hash in marks {
        assert_eq!(alpha(&rt, hash), 0, "{hash:x} rests invisible while no spot hides the player");
    }
    state.pursuit.as_mut().unwrap().hiding = Some(1.0);
    for _ in 0..400 {
        step(&mut rt, &mut binding, &state);
    }
    for hash in marks {
        assert!(alpha(&rt, hash) > 100, "{hash:x} faded in: {}", alpha(&rt, hash));
    }
    let tree = rt.tree(package);
    for hash in marks {
        assert!(tree.nodes.iter().find(|n| n.name_hash == hash).unwrap().visible, "{hash:x} is drawn");
    }
    state.pursuit.as_mut().unwrap().hiding = None;
    for _ in 0..400 {
        step(&mut rt, &mut binding, &state);
    }
    for hash in marks {
        assert_eq!(alpha(&rt, hash), 0, "{hash:x} faded out when the spot let go");
    }
}
