//! Pointer-driven performance controls, including package requests and stock eligibility.

use super::{Car, Model, Selection, show};
use crate::settings::{Partial, Settings};
use nfsmw_data::car::physics::performance::CategoryOptions;
use nfsmw_data::car::physics::{PerformanceCatalog, PerformanceCategory as Category, PerformancePackage};

struct Ui {
    ctx: egui::Context,
    model: Model,
    settings: Settings,
    shapes: Vec<egui::epaint::ClippedShape>,
}

fn fixture() -> Ui {
    let mut categories =
        [CategoryOptions { max_level: 2, junkman: true, stock_present: true, ..Default::default() }; 7];
    categories[Category::Engine as usize].max_level = 4;
    categories[Category::Nitrous as usize] =
        CategoryOptions { max_level: 3, junkman: true, stock_present: false, ..Default::default() };
    let selection = Selection { car: "RACER".into(), ..Default::default() };
    let mut ui = Ui {
        ctx: egui::Context::default(),
        model: Model {
            cars: vec![Car {
                folder: "RACER".into(),
                name: "Racer".into(),
                performance: PerformanceCatalog { categories },
                ..Default::default()
            }],
            applied: selection.clone(),
            draft: selection,
            ..Default::default()
        },
        settings: Settings::from(Partial::default()),
        shapes: Vec::new(),
    };
    ui.frame(Vec::new());
    ui.frame(Vec::new());
    // Grow the actual garage so all seven rows remain visible in these probes.
    let rect = ui.ctx.memory(|memory| memory.area_rect(egui::Id::new("garage_window"))).unwrap();
    let grip = rect.max - egui::vec2(4.0, 4.0);
    ui.frame(vec![egui::Event::PointerMoved(grip)]);
    ui.frame(vec![egui::Event::PointerButton {
        pos: grip,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    }]);
    let target = grip + egui::vec2(0.0, 380.0);
    ui.frame(vec![egui::Event::PointerMoved(target)]);
    ui.frame(vec![egui::Event::PointerButton {
        pos: target,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    ui.frame(Vec::new());
    ui
}

impl Ui {
    fn frame(&mut self, events: Vec<egui::Event>) -> Option<Selection> {
        self.ctx.begin_pass(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1300.0))),
            events,
            ..Default::default()
        });
        let request = show(&self.ctx, &mut true, &mut self.model, &mut self.settings);
        let mut output = self.ctx.end_pass();
        output.textures_delta.clear();
        self.shapes = output.shapes;
        request
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
        let pressed = self.frame(vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        let released = self.frame(vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        assert!(self.frame(Vec::new()).is_none(), "idle frames must not repeat a performance request");
        pressed.or(released)
    }

    fn click(&mut self, label: &str) -> Option<Selection> {
        let pos = *self.positions(label).last().unwrap_or_else(|| panic!("missing visible widget: {label}"));
        self.click_at(pos)
    }

    fn stage(&mut self, category: Category, current: &str, next: &str) -> Option<Selection> {
        let row = self.positions(category.label())[0];
        let combo = self
            .positions(current)
            .into_iter()
            .find(|position| position.y > row.y && position.y < row.y + 45.0)
            .unwrap_or_else(|| panic!("missing stage selector for {}", category.label()));
        assert!(self.click_at(combo).is_none(), "opening a selector must not apply a package");
        self.click(next)
    }

    fn junkman(&mut self, category: Category) -> Option<Selection> {
        let row = self.positions(category.label())[0];
        let checkbox = self
            .positions("Junkman")
            .into_iter()
            .find(|position| (position.y - row.y).abs() < 12.0)
            .unwrap_or_else(|| panic!("missing Junkman checkbox for {}", category.label()));
        self.click_at(checkbox)
    }

    fn applied(&mut self, request: Selection) -> PerformancePackage {
        assert_eq!(request, self.model.draft, "the emitted request must contain the complete package");
        assert_eq!(request.car, "RACER");
        self.model.applied = request.clone();
        request.performance
    }
}

#[test]
fn long_rim_catalog_can_be_searched_with_the_keyboard_before_selecting() {
    use nfsmw_data::car::customization::{Catalog, Category as Part, Choice};
    let mut ui = fixture();
    let mut catalog = Catalog::default();
    catalog.choices[Part::Wheels as usize] = (0..12)
        .map(|i| Choice { hash: 100 + i, label: format!("Rim {i:02}"), language_hash: None, carbon: false })
        .collect();
    ui.model.cars[0].parts = Some(catalog.into());
    assert!(ui.click("Parts").is_none());
    let row = ui.positions("Rims")[0];
    let combo = ui.positions("Stock").into_iter().find(|p| p.y > row.y && p.y < row.y + 40.0).unwrap();
    assert!(ui.click_at(combo).is_none());
    assert!(ui.click("Search parts").is_none());
    assert!(ui.frame(vec![egui::Event::Text("Rim 11".into())]).is_none());
    assert!(ui.positions("Rim 00").is_empty());
    assert_eq!(ui.click("Rim 11").unwrap().parts.parts[Part::Wheels as usize], Some(111));
}

#[test]
fn model_parts_apply_on_select_restore_stock_and_unlock_aero_only_for_body_or_spoiler() {
    use nfsmw_data::car::customization::{Catalog, Category as Part, Choice};
    use nfsmw_data::car::physics::tuning::TuningCategory;
    let mut ui = fixture();
    let mut catalog = Catalog::default();
    for category in Part::MODELS {
        catalog.choices[category as usize].push(Choice {
            hash: 100 + category as u32,
            label: format!("Custom {}", category.label()),
            language_hash: None,
            carbon: false,
        });
    }
    ui.model.cars[0].parts = Some(catalog.into());
    ui.model.draft.color = Some([30, 80, 130]);
    assert!(ui.click("Parts").is_none());
    for category in [Part::Wheels, Part::Hood, Part::Roof, Part::Body, Part::Spoiler] {
        let row = ui.positions(category.label())[0];
        let combo = ui.positions("Stock").into_iter().find(|p| p.y > row.y && p.y < row.y + 40.0).unwrap();
        assert!(ui.click_at(combo).is_none());
        let request = ui.click(&format!("Custom {}", category.label())).expect("live model request");
        assert_eq!(request.parts.parts[category as usize], Some(100 + category as u32));
        let aero = ui.model.cars[0].parts.as_ref().unwrap().aero_changed(&request.parts);
        assert_eq!(aero, matches!(category, Part::Body | Part::Spoiler));
    }
    assert!(ui.click("Tuning").is_none());
    let request = ui.tune(TuningCategory::Aerodynamics, true).expect("fitted body/spoiler unlocks aero slider");
    assert_eq!(request.tuning.profiles[0][TuningCategory::Aerodynamics as usize], 5);
    assert!(ui.click("Parts").is_none());
    let request = ui.click("Restore stock parts").unwrap();
    assert!(request.parts.is_stock());
    assert_eq!(request.color, Some([30, 80, 130]));
    assert_eq!(request.tuning.profiles[0][TuningCategory::Aerodynamics as usize], 5, "restoring parts retains tuning");
    assert!(ui.click("Tuning").is_none());
    assert!(ui.tune(TuningCategory::Aerodynamics, false).is_none(), "factory parts relock the slider");
}

#[test]
fn stock_and_maximum_use_each_cars_limits_and_apply_once() {
    let mut ui = fixture();
    assert!(ui.click("Stock").is_none());
    let request = ui.click("Maximum").unwrap();
    let maximum = ui.applied(request);
    assert_eq!(maximum.levels, [2, 2, 2, 2, 4, 2, 3]);
    assert_eq!(maximum.junkman, 0);
    assert!(ui.click("Maximum").is_none());
    let request = ui.click("Max + Junkman").unwrap();
    assert_eq!(ui.applied(request).junkman, 0x7f);
    assert!(ui.click("Max + Junkman").is_none());
    let request = ui.click("Stock").unwrap();
    assert_eq!(ui.applied(request), PerformancePackage::default());
    assert!(ui.click("Stock").is_none());
}

#[test]
fn individual_engine_stages_and_junkman_apply_without_resetting_other_categories() {
    let mut ui = fixture();
    let request = ui.stage(Category::Engine, "Original equipment", "Stage 2").unwrap();
    assert_eq!(ui.applied(request).levels, [0, 0, 0, 0, 2, 0, 0]);
    let request = ui.junkman(Category::Engine).unwrap();
    assert!(ui.applied(request).has_junkman(Category::Engine));
    let request = ui.stage(Category::Engine, "Stage 2", "Stage 4 · maximum").unwrap();
    let maximum = ui.applied(request);
    assert_eq!(maximum.get(Category::Engine), 4);
    assert!(maximum.has_junkman(Category::Engine));
    let request = ui.stage(Category::Engine, "Stage 4 · maximum", "Original equipment").unwrap();
    let stock_with_junkman = ui.applied(request);
    assert_eq!(stock_with_junkman.get(Category::Engine), 0);
    assert!(stock_with_junkman.has_junkman(Category::Engine), "stock engine remains eligible for Junkman");
    let request = ui.junkman(Category::Engine).unwrap();
    assert_eq!(ui.applied(request), PerformancePackage::default());
}

#[test]
fn nitrous_junkman_requires_a_kit_and_is_removed_when_returning_to_original_equipment() {
    let mut ui = fixture();
    assert!(ui.junkman(Category::Nitrous).is_none(), "stock car has no nitrous kit");
    assert_eq!(ui.model.draft.performance, PerformancePackage::default());
    let request = ui.stage(Category::Nitrous, "Original equipment", "Stage 1").unwrap();
    assert_eq!(ui.applied(request).get(Category::Nitrous), 1);
    let request = ui.junkman(Category::Nitrous).unwrap();
    assert!(ui.applied(request).has_junkman(Category::Nitrous));
    let request = ui.stage(Category::Nitrous, "Stage 1", "Original equipment").unwrap();
    assert_eq!(ui.applied(request), PerformancePackage::default());
    assert!(ui.junkman(Category::Nitrous).is_none());
}

#[test]
fn tuning_sliders_apply_live_keep_three_profiles_and_obey_equipment_gates() {
    use nfsmw_data::car::physics::tuning::TuningCategory;
    let mut ui = fixture();
    assert!(ui.click("Tuning").is_none());
    assert!(ui.tune(TuningCategory::Steering, true).is_none(), "stock suspension locks editing");
    assert!(ui.click("Performance").is_none());
    let request = ui.click("Maximum").unwrap();
    ui.applied(request);
    assert!(ui.click("Tuning").is_none());
    let request = ui.tune(TuningCategory::Steering, true).unwrap();
    assert_eq!(request.tuning.profiles[0][0], 5);
    assert_eq!(request.performance, ui.model.draft.performance);
    ui.model.applied = request;
    let request = ui.click("Profile 2").unwrap();
    assert_eq!(request.tuning.active, 1);
    assert_eq!(request.tuning.profiles[1], [0; 7]);
    ui.model.applied = request;
    let request = ui.tune(TuningCategory::Handling, false).unwrap();
    assert_eq!(request.tuning.profiles[1][1], -5);
    ui.model.applied = request;
    assert!(ui.tune(TuningCategory::Aerodynamics, true).is_none(), "paint and performance do not install aero parts");
    let request = ui.click("Reset profile").unwrap();
    assert_eq!(request.tuning.profiles[1], [0; 7]);
    assert_eq!(request.tuning.profiles[0][0], 5);
    ui.model.applied = request;
    assert!(ui.click("Reset profile").is_none());
    let request = ui.click("Profile 1").unwrap();
    ui.model.applied = request;
    assert!(ui.click("Performance").is_none());
    let request = ui.click("Stock").unwrap();
    assert_eq!(request.tuning.profiles[0][0], 5, "original profile values survive removing equipment");
    ui.model.applied = request;
    assert!(ui.click("Tuning").is_none());
    assert!(ui.tune(TuningCategory::Steering, false).is_none());
    assert_eq!(ui.model.draft.tuning.profiles[0][0], 5);
}

impl Ui {
    fn tune(
        &mut self,
        category: nfsmw_data::car::physics::tuning::TuningCategory,
        positive: bool,
    ) -> Option<Selection> {
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
fn appearance_selectors_apply_paint_and_tint_without_changing_model_parts() {
    use nfsmw_data::car::customization::{Catalog, Category as Part, Choice};
    let mut ui = fixture();
    let mut catalog = Catalog::default();
    for (category, label) in [(Part::Paint, "Metallic silver"), (Part::WindowTint, "Dark smoke")] {
        catalog.choices[category as usize].push(Choice {
            hash: 100 + category as u32,
            label: label.into(),
            language_hash: None,
            carbon: false,
        });
    }
    ui.model.cars[0].paintable = true;
    ui.model.cars[0].parts = Some(catalog.into());
    assert!(ui.click("Appearance").is_none());
    for (category, label) in [(Part::Paint, "Metallic silver"), (Part::WindowTint, "Dark smoke")] {
        let row = ui.positions(category.label())[0];
        let combo = ui.positions("Stock").into_iter().find(|p| p.y > row.y && p.y < row.y + 40.0).unwrap();
        assert!(ui.click_at(combo).is_none());
        let request = ui.click(label).unwrap();
        assert_eq!(request.parts.parts[category as usize], Some(100 + category as u32));
        assert!(request.parts.parts[..5].iter().all(Option::is_none));
    }
    ui.model.draft.parts.parts[Part::Body as usize] = Some(42);
    ui.model.draft.color = Some([10, 20, 30]);
    let restored = ui.click("Restore stock appearance").unwrap();
    assert!(restored.color.is_none());
    assert!(restored.parts.parts[Part::Paint as usize].is_none());
    assert!(restored.parts.parts[Part::WindowTint as usize].is_none());
    assert_eq!(restored.parts.parts[Part::Body as usize], Some(42));
    assert_eq!(ui.settings.car_shading, crate::settings::CarShading::Glossy);
}
