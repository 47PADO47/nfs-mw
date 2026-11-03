use super::*;
use crate::frontend::logic::{Cx, ScreenLogic};
use crate::input::{Bindings, InputDevice};

struct Cover;
impl ScreenLogic for Cover {
    fn covers_screens(&self) -> bool {
        true
    }
    fn message(&mut self, _: &mut Cx, _: u32) {}
}

fn background(h: &Harness) -> blackbox_feng::UiTree {
    let screens = h.screens.presented(&Bindings::default(), InputDevice::Xbox);
    let backgrounds: Vec<_> = screens.iter().filter(|screen| screen.name == "InGameBackground.fng").collect();
    assert_eq!(backgrounds.len(), 1, "one shared native pause background must be drawn");
    (*backgrounds[0].tree).clone()
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn batch12_pause_tuning_and_options_keep_one_authored_background_beneath_their_pages() {
    let mut h = Harness::open(screen::PAUSE_MENU, Args { pause: true, ..Default::default() }).expect("installed menus");
    h.wait(1.0);
    let tree = background(&h);
    let shade = tree.nodes.iter().find(|node| node.name_hash == 0x2716_CDBF).expect("authored backdrop group");
    assert!(shade.visible && shade.world_colour[3] > 0);
    let centre = tree.nodes.iter().find(|node| node.name_hash == 0xC6AF_DD7E).unwrap();
    assert!(centre.visible && centre.size.x.abs() >= 500.0 && centre.size.y.abs() >= 480.0);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    h.wait(1.0);
    background(&h);
    h.press(pad::ACCEPT);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::PAUSE_OPTIONS));
    background(&h);
    let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
    h.screens.open_with("Pause_Performance_Tuning.fng", Box::new(Cover), false, &mut env);
    h.wait(1.0);
    let pages = h.screens.presented(&Bindings::default(), InputDevice::Xbox);
    assert_eq!(pages.len(), 2, "the covering tuning page hides the pause menu, while retaining the background");
    background(&h);
    h.screens.clear();
    let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
    h.screens.open(screen::MAIN_MENU, Args::default(), true, &mut env);
    h.wait(1.0);
    assert!(
        h.screens
            .presented(&Bindings::default(), InputDevice::Xbox)
            .iter()
            .all(|screen| screen.name != "InGameBackground.fng")
    );
}

#[test]
#[ignore = "requires installed pause artwork"]
fn pass_a_pause_side_gradients_emit_dark_fading_vertices() {
    use crate::ui::present::{BlackboxPresenter, Screen};
    let mut h = Harness::open(screen::PAUSE_MENU, Args { pause: true, ..Default::default() }).unwrap();
    h.wait(1.0);
    let mut tree = background(&h);
    let side = tree.nodes.iter().position(|n| n.name_hash == 0xa8de_2158).unwrap();
    tree.draw_order = vec![side];
    let mut out = crate::gui::UiOutput::default();
    BlackboxPresenter::default().present(
        &tree,
        h.screens.assets(),
        Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 },
        &mut out,
    );
    let vertices = &out.layer.unwrap().meshes[0].vertices;
    assert_eq!(vertices.len(), 4);
    assert!(
        vertices.iter().all(|v| v.color_rgba[..3] == [0; 3]),
        "side gradient must darken, not whiten: {vertices:?}"
    );
    assert_eq!(vertices.iter().map(|v| v.color_rgba[3]).min(), Some(0));
    assert!(vertices.iter().any(|v| v.color_rgba[3] > 0), "inner edge darkens");
}

#[test]
#[ignore = "requires installed pause artwork and Vulkan GPU"]
fn pass_a_gpu_pause_background_darkens_continuously_across_aspects() {
    use crate::frontend::tests::rendered_playtest::render;
    let mut h = Harness::open(screen::PAUSE_MENU, Args { pause: true, ..Default::default() }).unwrap();
    h.wait(1.0);
    for (width, height) in [(1440, 1080), (1920, 1080), (2560, 1080)] {
        h.screens.set_aspect(width as f32 / height as f32);
        let image = render(&background(&h), h.screens.assets(), (width, height), "pause-background");
        for x in [0, width / 4, width / 2, width * 3 / 4, width - 1] {
            let pixel = image.pixel(x, height / 10).unwrap();
            assert!(
                pixel[0] <= 53 && pixel[1] <= 104 && pixel[2] <= 155,
                "pause must not whiten the scene: {x} {pixel:?}"
            );
        }
        let centre = image.pixel(width / 2, height / 10).unwrap();
        assert!((12..=18).contains(&centre[0]), "authored centre alpha: {centre:?}");
        for x in [0, width - 1] {
            assert!(image.pixel(x, height / 10).unwrap()[0] > centre[0], "side edge must fade out");
        }
        let shown = h.screens.presented_trees(&Bindings::default(), InputDevice::Xbox);
        render(shown.last().unwrap(), h.screens.assets(), (width, height), "pause-controls");
    }
}
