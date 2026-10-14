use super::*;

#[test]
fn panel_stays_interactive_when_renderer_requests_mouse_capture_after_startup() {
    let mut lab = GraphicsLab { open: true, interacting: true, ..Default::default() };
    let mut capture = MouseCapture(false);
    let mut focus = UiFocus(false);
    lab.sync_focus(&mut capture, &mut focus);
    capture.0 = true;
    lab.sync_focus(&mut capture, &mut focus);
    assert!(!capture.0 && focus.0);
    lab.open = false;
    lab.sync_focus(&mut capture, &mut focus);
    assert!(capture.0 && !focus.0);
}

#[test]
fn visible_overlay_preserves_driving_and_alt_only_captures_controls_while_held() {
    let mut app = App::new();
    app.insert_resource(GraphicsLab { open: false, ..Default::default() })
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::devtools::Console>()
        .insert_resource(MouseCapture(true))
        .insert_resource(UiFocus(false))
        .add_systems(PreUpdate, toggle);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::F8);
    app.update();
    assert!(app.world().resource::<GraphicsLab>().open);
    assert!(!app.world().resource::<UiFocus>().0);
    assert!(app.world().resource::<MouseCapture>().0);
    assert!(throttle(&app) > 0.9);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::AltLeft);
    app.update();
    assert!(app.world().resource::<UiFocus>().0);
    assert!(!app.world().resource::<MouseCapture>().0);
    assert_eq!(throttle(&app), 0.0);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
    app.update();
    assert!(app.world().resource::<GraphicsLab>().open);
    assert!(!app.world().resource::<UiFocus>().0);
    assert!(app.world().resource::<MouseCapture>().0);
    assert!(throttle(&app) > 0.9);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Escape);
    app.update();
    assert!(app.world().resource::<GraphicsLab>().open, "passive Escape belongs to the game");
    assert!(app.world().resource::<ButtonInput<KeyCode>>().just_pressed(KeyCode::Escape));
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::AltRight);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Escape);
    app.update();
    assert!(!app.world().resource::<GraphicsLab>().open);
    assert!(!app.world().resource::<ButtonInput<KeyCode>>().just_pressed(KeyCode::Escape));
}

fn throttle(app: &App) -> f32 {
    let mut actions = crate::input::ActionState::default();
    actions.update_for_test(
        &crate::input::Bindings::default(),
        &[KeyCode::KeyW],
        &[],
        app.world().resource::<UiFocus>().0,
    );
    actions.value(crate::input::Action::Throttle)
}

fn pass(
    ctx: &egui::Context,
    lab: &mut GraphicsLab,
    settings: &mut Settings,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.begin_pass(egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 900.0))),
        events,
        ..Default::default()
    });
    show(ctx, lab, settings, &Default::default());
    let mut output = ctx.end_pass();
    // This input-only harness has no painter; explicitly discard its font upload.
    output.textures_delta.clear();
    output
}

fn click(ctx: &egui::Context, lab: &mut GraphicsLab, settings: &mut Settings, label: &str) {
    // Let window auto-sizing settle, then hit the actual text drawn by the production panel.
    pass(ctx, lab, settings, vec![]);
    let output = pass(ctx, lab, settings, vec![]);
    let pos = output
        .shapes
        .iter()
        .find_map(|shape| {
            let egui::Shape::Text(text) = &shape.shape else { return None };
            (text.galley.job.text == label).then(|| text.pos + text.galley.rect.center().to_vec2())
        })
        .unwrap_or_else(|| panic!("no rendered button {label}"));
    pass(
        ctx,
        lab,
        settings,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
        ],
    );
    pass(
        ctx,
        lab,
        settings,
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }],
    );
}

#[test]
fn real_panel_buttons_switch_presets_swap_and_restore_without_touching_gameplay() {
    let ctx = egui::Context::default();
    let mut lab = GraphicsLab {
        open: true,
        interacting: true,
        options: Default::default(),
        stats: ExperimentStats { supported: true, ..Default::default() },
        ..Default::default()
    };
    let mut settings = Settings::from(crate::settings::Partial::default());
    let startup = settings;
    click(&ctx, &mut lab, &mut settings, "Sun");
    assert!(lab.options.sun_shadows && !lab.options.far_shadows);
    assert!(lab.options.world_ambient);
    click(&ctx, &mut lab, &mut settings, "Freeze sun");
    assert!(lab.options.freeze_sun);
    assert_eq!(lab.options.shadow_resolution, 2048);
    assert!(!lab.options.original_shadows);
    click(&ctx, &mut lab, &mut settings, "Original shadow overlays");
    assert!(lab.options.original_shadows);
    settings.post_aa = PostAa::Smaa;
    click(&ctx, &mut lab, &mut settings, "Store A");
    click(&ctx, &mut lab, &mut settings, "Coat");
    assert!(lab.options.contact_shadows && lab.options.clearcoat);
    settings.post_aa = PostAa::Off;
    click(&ctx, &mut lab, &mut settings, "Swap A/B");
    assert!(lab.options.sun_shadows && !lab.options.clearcoat);
    assert!(lab.options.freeze_sun, "A/B restores the diagnostic clock state");
    assert_eq!(settings.post_aa, PostAa::Smaa);
    click(&ctx, &mut lab, &mut settings, "Swap A/B");
    assert!(lab.options.clearcoat);
    assert_eq!(settings.post_aa, PostAa::Off);
    settings.hud = !startup.hud;
    click(&ctx, &mut lab, &mut settings, "Restore startup");
    assert_eq!(lab.options, RenderExperiments::default());
    assert_eq!(settings.post_aa, startup.post_aa);
    assert_eq!(settings.hud, !startup.hud);
}

#[test]
fn passive_panel_does_not_accept_option_clicks() {
    let ctx = egui::Context::default();
    let mut lab = GraphicsLab {
        open: true,
        options: Default::default(),
        stats: ExperimentStats { supported: true, ..Default::default() },
        ..Default::default()
    };
    let mut settings = Settings::from(crate::settings::Partial::default());
    click(&ctx, &mut lab, &mut settings, "Sun");
    assert!(!lab.options.sun_shadows);
}
