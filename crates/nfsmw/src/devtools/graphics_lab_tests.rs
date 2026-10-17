use super::*;

#[test]
fn traffic_tab_previews_range_and_density_independently_and_reset_restores_defaults() {
    let ctx = egui::Context::default();
    let mut lab = GraphicsLab { open: true, tab: profiles::Tab::Traffic, ..Default::default() };
    let mut settings = Settings::from(crate::settings::Partial::default());
    settings.traffic = 28;
    click(&ctx, &mut lab, &mut settings, "Extended");
    assert_eq!(settings.traffic_policy.spawn_min, 450);
    assert_eq!(settings.traffic, 28);
    click(&ctx, &mut lab, &mut settings, "30%");
    assert_eq!(settings.traffic_policy.density_percent, 30);
    click(&ctx, &mut lab, &mut settings, "Reference");
    assert_eq!(settings.traffic_policy.density_percent, 30);
    assert_eq!(settings.traffic, 28);
    click(&ctx, &mut lab, &mut settings, "Reset section");
    assert_eq!(settings.traffic, crate::settings::DEFAULT_TRAFFIC);
    assert_eq!(settings.traffic_policy, crate::settings::TrafficPolicy::default());
}

#[test]
fn panel_stays_interactive_when_renderer_requests_mouse_capture_after_startup() {
    let mut lab = GraphicsLab { open: true, ..Default::default() };
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

fn input_app() -> App {
    let mut app = App::new();
    app.insert_resource(GraphicsLab { open: false, ..Default::default() })
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::devtools::Console>()
        .init_resource::<crate::garage::Overlay>()
        .insert_resource(MouseCapture(true))
        .insert_resource(UiFocus(false))
        .add_systems(PreUpdate, toggle);
    app
}

#[test]
fn f8_takes_mouse_and_controls_without_alt_and_escape_restores_them() {
    let mut app = input_app();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::F8);
    app.update();
    assert!(app.world().resource::<GraphicsLab>().open);
    assert!(app.world().resource::<UiFocus>().0);
    assert!(!app.world().resource::<MouseCapture>().0);
    assert_eq!(throttle(&app), 0.0);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
    app.update();
    assert!(app.world().resource::<UiFocus>().0, "no modifier needs to remain held");
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Escape);
    app.update();
    assert!(!app.world().resource::<GraphicsLab>().open);
    assert!(!app.world().resource::<UiFocus>().0);
    assert!(app.world().resource::<MouseCapture>().0);
    assert!(throttle(&app) > 0.9);
    assert!(!app.world().resource::<ButtonInput<KeyCode>>().just_pressed(KeyCode::Escape));
}

#[test]
fn switching_from_garage_to_f8_transfers_and_restores_the_original_capture() {
    let mut app = input_app();
    let mut capture = MouseCapture(true);
    let mut focus = UiFocus(false);
    app.world_mut().resource_mut::<crate::garage::Overlay>().set_open(true, &mut capture, &mut focus);
    app.insert_resource(capture).insert_resource(focus);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::F8);
    app.update();
    assert!(!app.world().resource::<crate::garage::Overlay>().open);
    assert!(app.world().resource::<GraphicsLab>().open);
    assert!(app.world().resource::<UiFocus>().0);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::F8);
    app.update();
    assert!(!app.world().resource::<UiFocus>().0);
    assert!(app.world().resource::<MouseCapture>().0);
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

pub(super) fn click(ctx: &egui::Context, lab: &mut GraphicsLab, settings: &mut Settings, label: &str) {
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

        options: Default::default(),
        stats: ExperimentStats { supported: true, ..Default::default() },
        ..Default::default()
    };
    let mut settings = Settings::from(crate::settings::Partial::default());
    let startup = settings;
    click(&ctx, &mut lab, &mut settings, "Lighting");
    click(&ctx, &mut lab, &mut settings, "Sun");
    assert!(lab.options.sun_shadows && !lab.options.far_shadows);
    assert!(lab.options.world_ambient);
    click(&ctx, &mut lab, &mut settings, "Lighting");
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
    click(&ctx, &mut lab, &mut settings, "Restore startup · 0");
    assert_eq!(lab.options, crate::settings::Appearance::default().lighting);
    assert_eq!(settings.post_aa, startup.post_aa);
    assert_eq!(settings.hud, !startup.hud);
}

#[test]
fn panel_accepts_option_clicks_without_alt() {
    let ctx = egui::Context::default();
    let mut lab = GraphicsLab {
        open: true,
        options: Default::default(),
        stats: ExperimentStats { supported: true, ..Default::default() },
        ..Default::default()
    };
    let mut settings = Settings::from(crate::settings::Partial::default());
    click(&ctx, &mut lab, &mut settings, "Lighting");
    click(&ctx, &mut lab, &mut settings, "Sun");
    assert!(lab.options.sun_shadows);
}

#[test]
fn quality_shortcuts_only_apply_in_f8_and_restore_graphics_without_changing_driving() {
    use crate::settings::{CarShading, GraphicsPreset, Partial, Transmission};
    let mut app = input_app();
    let startup = Settings::from(Partial::default());
    app.insert_resource(startup).add_systems(PreUpdate, presets::hotkeys.after(toggle));
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Digit1);
    app.update();
    assert_eq!(app.world().resource::<Settings>().car_shading, startup.car_shading);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::F8);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Digit1);
    app.update();
    assert_eq!(app.world().resource::<Settings>().graphics_preset, GraphicsPreset::Custom);
    assert_eq!(app.world().resource::<Settings>().car_shading, CarShading::Glossy);
    assert_eq!(app.world().resource::<GraphicsLab>().options.shadow_resolution, 1024);
    app.world_mut().resource_mut::<Settings>().transmission = Transmission::Manual;
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Digit0);
    app.update();
    let result = app.world().resource::<Settings>();
    assert_eq!(result.car_shading, startup.car_shading);
    assert_eq!(result.render_scale, startup.render_scale);
    assert_eq!(result.transmission, Transmission::Manual);
}

#[test]
fn quality_buttons_change_cost_and_preserve_the_enhanced_art_treatment() {
    let ctx = egui::Context::default();
    let mut lab = GraphicsLab {
        open: true,
        stats: ExperimentStats { supported: true, ..Default::default() },
        ..Default::default()
    };
    let mut settings = Settings::from(crate::settings::Partial::default());
    click(&ctx, &mut lab, &mut settings, "Enhanced");
    click(&ctx, &mut lab, &mut settings, "High · 3");
    assert_eq!(lab.options.shadow_resolution, 4096);
    assert!(lab.options.contact_shadows && lab.options.clearcoat && lab.options.soft_shadows);
    assert_eq!(settings.ao, AoQuality::High);
    assert_eq!(settings.post_tonemap, PostTonemap::Aces);
    assert_eq!(settings.post_bloom, PostBloom::Off);
    click(&ctx, &mut lab, &mut settings, "Low · 1");
    assert_eq!(lab.options.shadow_resolution, 1024);
    assert!(!lab.options.contact_shadows && lab.options.clearcoat && !lab.options.soft_shadows);
    assert_eq!(settings.ao, AoQuality::Low);
    assert_eq!(settings.post_tonemap, PostTonemap::Aces);
}

#[test]
fn environment_buttons_select_time_rain_and_freeze_without_resetting_gameplay() {
    let ctx = egui::Context::default();
    let mut lab = GraphicsLab {
        open: true,
        environment: Some(crate::viewer::Environment { hour: 12.0, seconds_per_hour: 150.0, ..Default::default() }),
        ..Default::default()
    };
    let mut settings = Settings::from(crate::settings::Partial::default());
    settings.rain_amount.0 = 0;
    let camera = settings.camera_mode;
    click(&ctx, &mut lab, &mut settings, "Environment");
    click(&ctx, &mut lab, &mut settings, "Dusk");
    assert_eq!(lab.environment_change.unwrap().hour, 18.0);
    click(&ctx, &mut lab, &mut settings, "Freeze time");
    assert!(lab.options.freeze_sun);
    click(&ctx, &mut lab, &mut settings, "Rain");
    assert!(settings.always_rain);
    assert_eq!(settings.rain_amount.0, 100);
    click(&ctx, &mut lab, &mut settings, "No rain");
    assert!(!settings.always_rain);
    assert_eq!(settings.camera_mode, camera);
    assert_eq!(lab.environment.unwrap().hour, 18.0);
    click(&ctx, &mut lab, &mut settings, "High · 3");
    assert!(lab.options.freeze_sun, "quality must preserve a frozen environment");
    assert!(!settings.always_rain);
    assert_eq!(lab.environment.unwrap().hour, 18.0);
}

#[test]
fn menu_and_f8_share_profile_requests_and_overrides_survive_preview() {
    use crate::frontend::graphics_options::GraphicsSetting;
    use crate::settings::{GraphicsQuality, Partial, VisualStyle};
    let ctx = egui::Context::default();
    let mut lab = GraphicsLab {
        open: true,
        stats: ExperimentStats { supported: true, ..Default::default() },
        ..Default::default()
    };
    let mut settings = Settings::from(Partial::default());
    let mut changed = Partial::default();
    assert!(GraphicsSetting::Style.step(&mut settings, &mut changed, true));
    assert_eq!(settings.visual_style(), VisualStyle::OriginalPc);
    pass(&ctx, &mut lab, &mut settings, vec![]);
    assert!(lab.options.original_shadows && lab.options.sun_shadows && lab.options.shadow_projection.is_some());
    click(&ctx, &mut lab, &mut settings, "Enhanced");
    assert_eq!(settings.visual_style(), VisualStyle::Enhanced);
    assert!(settings.appearance.lighting.sun_shadows);
    click(&ctx, &mut lab, &mut settings, "Low · 1");
    assert_eq!(settings.appearance.quality, GraphicsQuality::Low);
    assert_eq!(settings.visual_style(), VisualStyle::Enhanced);
    click(&ctx, &mut lab, &mut settings, "Lighting");
    click(&ctx, &mut lab, &mut settings, "Sun shadows");
    assert!(!settings.appearance.lighting.sun_shadows);
    assert_eq!(settings.visual_style(), VisualStyle::Custom);
    lab.open = false;
    pass(&ctx, &mut lab, &mut settings, vec![]);
    assert!(!settings.appearance.lighting.sun_shadows, "closed lab must not restore stale options");
    assert_eq!(settings.shadows, crate::settings::ShadowQuality::Off);
}

#[test]
fn restore_uses_canonical_startup_lighting_and_profile_load_only_owns_appearance() {
    let mut settings = Settings::from(crate::settings::Partial::default());
    settings.select_visual_style(crate::settings::VisualStyle::Enhanced);
    let startup = settings;
    let mut lab = GraphicsLab { options: Default::default(), ..Default::default() };
    lab.remember_startup(&settings);
    settings.select_visual_style(crate::settings::VisualStyle::OriginalPc);
    lab.restore_startup(&mut settings);
    assert_eq!(settings.appearance, startup.appearance);
    assert_eq!(settings.shadows, startup.shadows);
    assert_eq!(lab.options, startup.appearance.lighting);
    let mut saved = startup;
    saved.select_visual_style(crate::settings::VisualStyle::OriginalPc);
    settings.tire_smoke = !saved.tire_smoke;
    settings.always_rain = !saved.always_rain;
    presets::restore_appearance(&mut settings, &saved);
    assert_eq!(settings.appearance, saved.appearance);
    assert_eq!(settings.shadows, saved.shadows);
    assert_ne!(settings.tire_smoke, saved.tire_smoke);
    assert_ne!(settings.always_rain, saved.always_rain);
}

#[test]
fn section_resets_and_lighting_presets_keep_material_selection_independent() {
    let ctx = egui::Context::default();
    let mut lab = GraphicsLab {
        open: true,
        stats: ExperimentStats { supported: true, ..Default::default() },
        ..Default::default()
    };
    let mut s = Settings::from(crate::settings::Partial::default());
    s.select_visual_style(crate::settings::VisualStyle::Enhanced);
    click(&ctx, &mut lab, &mut s, "Materials");
    assert!(s.appearance.lighting.original_world_materials);
    click(&ctx, &mut lab, &mut s, "Lighting");
    click(&ctx, &mut lab, &mut s, "Sun");
    assert!(s.appearance.lighting.original_world_materials);
    click(&ctx, &mut lab, &mut s, "Reset section");
    assert!(s.appearance.lighting.original_world_materials);
    click(&ctx, &mut lab, &mut s, "Materials");
    click(&ctx, &mut lab, &mut s, "Reset section");
    assert!(s.appearance.lighting.original_world_materials);
}

#[test]
fn environment_comparison_presets_are_independent_and_session_local() {
    use crate::viewer::{Environment, SunMode};
    let ctx = egui::Context::default();
    let mut lab = GraphicsLab { open: true, environment: Some(Environment::default()), ..Default::default() };
    let mut settings = Settings::from(crate::settings::Partial::default());
    click(&ctx, &mut lab, &mut settings, "Environment");
    click(&ctx, &mut lab, &mut settings, "Overcast");
    assert_eq!(lab.environment_change.unwrap().cloud_override, Some(1.0));
    assert!(!settings.always_rain, "overcast is a cloud comparison, not a rain storm");
    click(&ctx, &mut lab, &mut settings, "Cloudy");
    assert_eq!(lab.environment_change.unwrap().cloud_override, Some(0.5));
    click(&ctx, &mut lab, &mut settings, "Sunny");
    assert_eq!(lab.environment_change.unwrap().cloud_override, Some(0.0));
    click(&ctx, &mut lab, &mut settings, "Location");
    assert_eq!(lab.environment_change.unwrap().cloud_override, None);
    click(&ctx, &mut lab, &mut settings, "Original cycle");
    assert_eq!(lab.environment_change.unwrap().sun_mode, SunMode::OriginalCycle);
    click(&ctx, &mut lab, &mut settings, "Fixed phase");
    assert_eq!(lab.environment_change.unwrap().sun_mode, SunMode::FixedPhase);
    click(&ctx, &mut lab, &mut settings, "Settle clouds to target");
    assert!(lab.environment_change.unwrap().settle_clouds);
}
