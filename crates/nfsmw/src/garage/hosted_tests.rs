//! The pages the vehicle panel hosts from the garage (tuning, saved setups, the appearance extras), driven with pointer events the
//! way the panel draws them: the draft starts as what is applied, and a changed draft is the request to apply.

use super::{Car, Model, Selection, hosted};
use crate::settings::{Partial, Settings};
use nfsmw_data::car::customization::{Catalog, Category as Part, Choice};
use nfsmw_data::car::physics::PerformanceCatalog;
use nfsmw_data::car::physics::performance::CategoryOptions;
use nfsmw_data::car::physics::tuning::TuningCategory;

#[derive(Clone, Copy)]
enum Page {
    Setups,
    Tuning,
    Appearance,
}

struct Ui {
    ctx: egui::Context,
    model: Model,
    settings: Settings,
    shapes: Vec<egui::epaint::ClippedShape>,
    page: Page,
}

fn fixture(page: Page) -> Ui {
    let categories = [CategoryOptions { max_level: 2, junkman: true, stock_present: true, ..Default::default() }; 7];
    let selection = Selection { car: "RACER".into(), ..Default::default() };
    let mut ui = Ui {
        ctx: egui::Context::default(),
        model: Model {
            cars: vec![Car {
                folder: "RACER".into(),
                name: "Racer".into(),
                paintable: true,
                performance: PerformanceCatalog { categories },
                ..Default::default()
            }],
            applied: selection.clone(),
            draft: selection,
            ..Default::default()
        },
        settings: Settings::from(Partial::default()),
        shapes: Vec::new(),
        page,
    };
    ui.frame(Vec::new());
    ui.frame(Vec::new());
    ui
}

impl Ui {
    fn frame(&mut self, events: Vec<egui::Event>) -> Option<Selection> {
        self.ctx.begin_pass(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 1200.0))),
            events,
            ..Default::default()
        });
        let before = hosted::begin(&mut self.model).expect("no shop");
        let (model, settings, page) = (&mut self.model, &mut self.settings, self.page);
        egui::Window::new("hosted").default_size(egui::vec2(700.0, 1100.0)).show(&self.ctx, |ui| match page {
            Page::Setups => hosted::setups(ui, model),
            Page::Tuning => hosted::tuning(ui, model),
            Page::Appearance => hosted::appearance(ui, model, settings),
        });
        let mut output = self.ctx.end_pass();
        output.textures_delta.clear();
        self.shapes = output.shapes;
        hosted::changed(&self.model, &before)
    }

    fn positions(&self, label: &str) -> Vec<egui::Pos2> {
        self.shapes
            .iter()
            .filter_map(|shape| {
                let egui::epaint::Shape::Text(text) = &shape.shape else { return None };
                if text.galley.text().lines().next() != Some(label) {
                    return None;
                }
                let position = text.pos + text.galley.rect.center().to_vec2();
                shape.clip_rect.contains(position).then_some(position)
            })
            .collect()
    }

    fn click_at(&mut self, pos: egui::Pos2) -> Option<Selection> {
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let pressed = self.frame(vec![egui::Event::PointerMoved(pos), button(true)]);
        let released = self.frame(vec![button(false)]);
        assert!(self.frame(Vec::new()).is_none(), "idle frames must not repeat a request");
        pressed.or(released)
    }

    fn click(&mut self, label: &str) -> Option<Selection> {
        let pos = *self.positions(label).last().unwrap_or_else(|| panic!("missing visible widget: {label}"));
        self.click_at(pos)
    }

    fn tune(&mut self, category: TuningCategory, positive: bool) -> Option<Selection> {
        let label = self.positions(category.label())[0];
        let rail = self
            .shapes
            .iter()
            .find_map(|shape| {
                let egui::epaint::Shape::Rect(rect) = &shape.shape else { return None };
                let rail = rect.rect;
                (rail.width() > 150.0
                    && rail.height() < 15.0
                    && rail.center().y > label.y
                    && rail.center().y < label.y + 35.0)
                    .then_some(rail)
            })
            .unwrap_or_else(|| panic!("missing slider rail for {category:?}"));
        let x = match positive {
            true => rail.max.x - 0.5,
            false => rail.min.x + 0.5,
        };
        self.click_at(egui::pos2(x, rail.center().y))
    }
}

#[test]
fn tuning_sliders_apply_live_keep_three_profiles_and_obey_equipment_gates() {
    let mut ui = fixture(Page::Tuning);
    assert!(ui.tune(TuningCategory::Steering, true).is_none(), "stock suspension locks editing");
    ui.model.applied.performance.levels = [2; 7];
    ui.frame(Vec::new());
    let request = ui.tune(TuningCategory::Steering, true).unwrap();
    assert_eq!(request.tuning.profiles[0][0], 5);
    assert_eq!(request.performance, ui.model.applied.performance);
    ui.model.applied = request;
    let request = ui.click("Profile 2").unwrap();
    assert_eq!(request.tuning.active, 1);
    assert_eq!(request.tuning.profiles[1], [0; 7]);
    ui.model.applied = request;
    let request = ui.tune(TuningCategory::Handling, false).unwrap();
    assert_eq!(request.tuning.profiles[1][1], -5);
    ui.model.applied = request;
    assert!(ui.tune(TuningCategory::Aerodynamics, true).is_none(), "performance does not install aero parts");
    let request = ui.click("Reset profile").unwrap();
    assert_eq!(request.tuning.profiles[1], [0; 7]);
    assert_eq!(request.tuning.profiles[0][0], 5);
    ui.model.applied = request;
    assert!(ui.click("Reset profile").is_none());
    ui.model.applied.tuning.active = 0;
    ui.model.applied.performance = Default::default();
    ui.frame(Vec::new());
    assert!(ui.tune(TuningCategory::Steering, false).is_none());
    assert_eq!(ui.model.draft.tuning.profiles[0][0], 5, "original profile values survive removing equipment");
}

#[test]
fn a_body_kit_or_spoiler_unlocks_the_aero_slider_and_factory_parts_lock_it_again() {
    let mut ui = fixture(Page::Tuning);
    ui.model.applied.performance.levels = [2; 7];
    let mut catalog = Catalog::default();
    for category in Part::MODELS {
        let choice = Choice { hash: 100 + category as u32, label: "x".into(), language_hash: None, carbon: false };
        catalog.choices[category as usize].push(choice);
    }
    ui.model.cars[0].parts = Some(catalog.into());
    ui.model.applied.parts.parts[Part::Hood as usize] = Some(100 + Part::Hood as u32);
    ui.frame(Vec::new());
    assert!(ui.tune(TuningCategory::Aerodynamics, true).is_none(), "a hood is not aerodynamic");
    ui.model.applied.parts.parts[Part::Spoiler as usize] = Some(100 + Part::Spoiler as u32);
    ui.frame(Vec::new());
    let request = ui.tune(TuningCategory::Aerodynamics, true).expect("a spoiler unlocks aero");
    assert_eq!(request.tuning.profiles[0][TuningCategory::Aerodynamics as usize], 5);
    ui.model.applied = request;
    ui.model.applied.parts = Default::default();
    ui.frame(Vec::new());
    assert!(ui.tune(TuningCategory::Aerodynamics, false).is_none(), "factory parts relock the slider");
}

#[test]
fn restoring_the_appearance_keeps_the_model_parts() {
    let mut ui = fixture(Page::Appearance);
    ui.model.cars[0].parts = Some(Catalog::default().into());
    ui.model.applied.parts.parts[Part::Body as usize] = Some(42);
    ui.model.applied.parts.parts[Part::Paint as usize] = Some(7);
    ui.model.applied.parts.parts[Part::WindowTint as usize] = Some(8);
    ui.model.applied.color = Some([10, 20, 30]);
    ui.frame(Vec::new());
    let restored = ui.click("Restore stock appearance").unwrap();
    assert!(restored.color.is_none());
    assert!(restored.parts.parts[Part::Paint as usize].is_none());
    assert!(restored.parts.parts[Part::WindowTint as usize].is_none());
    assert_eq!(restored.parts.parts[Part::Body as usize], Some(42));
}

#[test]
fn named_setups_save_reopen_and_load_through_the_hosted_buttons() {
    let mut ui = fixture(Page::Setups);
    ui.model.cars.push(Car { folder: "OTHER".into(), ..Default::default() });
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("nfsmw-setups-{}-{stamp}.toml", std::process::id()));
    ui.model.preset_picker.path = Some(path.clone());
    ui.model.preset_picker.name = "Road setup".into();
    ui.model.applied.color = Some([20, 80, 210]);
    ui.model.applied.performance.levels = [2; 7];
    ui.model.applied.tuning.profiles[1] = [1, 2, 3, 4, 5, -1, -2];
    ui.model.applied.tuning.active = 1;
    let wanted = ui.model.applied.clone();
    ui.frame(Vec::new());
    // The setups are in a collapsing header: open it.
    assert!(ui.click("Saved setups").is_none());
    for _ in 0..20 {
        ui.frame(Vec::new());
    }
    assert!(ui.click("Save as / manage…").is_none());
    for _ in 0..20 {
        ui.frame(Vec::new());
    }
    assert!(ui.click("Save as").is_none());
    assert!(path.is_file());
    ui.model.applied = Selection { car: "OTHER".into(), ..Default::default() };
    ui.model.preset_picker = crate::preset_store::Picker::default();
    ui.model.preset_picker.path = Some(path.clone());
    ui.model.preset_picker.name = "Road setup".into();
    ui.frame(Vec::new());
    ui.frame(Vec::new());
    let request = ui.click("Road setup").expect("loading emits a normal garage transaction");
    assert_eq!(request, wanted);
    assert_eq!(ui.model.applied.car, "OTHER", "the scene owns committing a successful transaction");
    std::fs::remove_file(path).unwrap();
}
