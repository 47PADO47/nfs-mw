//! Drive the actual widgets with pointer events, rather than setting the draft directly.

use super::{Car, Category, Model, Selection, show};
use crate::settings::{Partial, Settings};

struct Ui {
    ctx: egui::Context,
    model: Model,
    settings: Settings,
    shapes: Vec<egui::epaint::ClippedShape>,
}

fn fixture(cars: Vec<Car>, active: &str) -> Ui {
    let mut ui = Ui {
        ctx: egui::Context::default(),
        model: Model {
            cars,
            applied: Selection { car: active.into(), color: None, ..Default::default() },
            draft: Selection { car: active.into(), color: None, ..Default::default() },
            ..Model::default()
        },
        settings: Settings::from(Partial::default()),
        shapes: Vec::new(),
    };
    ui.frame(Vec::new());
    ui.frame(Vec::new());
    ui
}

#[test]
fn preview_hides_the_browser_without_changing_the_car_or_camera() {
    let mut ui = fixture(vec![Car { folder: "RACER".into(), name: "Racer".into(), ..Default::default() }], "RACER");
    let selection = ui.model.draft.clone();
    let camera = ui.settings.camera_mode;
    let full = ui.ctx.memory(|m| m.area_rect(egui::Id::new("garage_window"))).unwrap();
    assert!(ui.click("Preview car").is_none());
    ui.frame(Vec::new());
    ui.frame(Vec::new());
    let preview = ui.ctx.memory(|m| m.area_rect(egui::Id::new("garage_preview"))).unwrap();
    assert!(preview.width() < full.width() / 2.0, "preview leaves room to see the car");
    assert_eq!(ui.model.draft, selection);
    assert_eq!(ui.settings.camera_mode, camera);
    assert!(ui.click("Browse cars").is_none());
    assert!(!ui.model.preview);
}

#[test]
fn police_tab_and_manufacturer_search_filter_the_actual_cards() {
    let mut ui = fixture(
        vec![
            Car { folder: "RACER".into(), name: "Racer".into(), manufacturer: "Audi".into(), ..Car::default() },
            Car { folder: "PATROL".into(), name: "Patrol".into(), category: Category::Police, ..Car::default() },
        ],
        "RACER",
    );
    assert!(ui.click("Police (1)").is_none());
    assert_eq!(ui.model.category, Category::Police);
    assert_eq!(ui.click("Patrol").unwrap().car, "PATROL");
    ui.model.filter = "Audi".into();
    ui.frame(Vec::new());
    assert!(ui.click("Clear search").is_none());
    assert!(ui.model.filter.is_empty());
    assert!(ui.click("Racing cars (1)").is_none());
    assert_eq!(ui.click("Racer").unwrap().car, "RACER");
}

#[test]
fn enlarging_the_window_displays_more_car_cards() {
    let cars = (0..50)
        .map(|index| {
            let name = format!("RACE_{index:02}");
            Car { folder: name.clone(), name, manufacturer: "Maker".into(), ..Car::default() }
        })
        .collect();
    let mut ui = fixture(cars, "RACE_00");
    fn visible(ui: &Ui) -> usize {
        ui.shapes
            .iter()
            .filter(|shape| {
                let egui::epaint::Shape::Text(text) = &shape.shape else { return false };
                text.galley.text().starts_with("RACE_")
                    && shape.clip_rect.contains(text.pos + text.galley.rect.center().to_vec2())
            })
            .count()
    }
    let before = visible(&ui);
    let rect = ui.ctx.memory(|memory| memory.area_rect(egui::Id::new("garage_window"))).unwrap();
    let grip = rect.max - egui::vec2(4.0, 4.0);
    ui.frame(vec![egui::Event::PointerMoved(grip)]);
    ui.frame(vec![
        egui::Event::PointerMoved(grip),
        egui::Event::PointerButton {
            pos: grip,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
    ]);
    let target = grip + egui::vec2(140.0, 150.0);
    ui.frame(vec![egui::Event::PointerMoved(target)]);
    ui.frame(vec![egui::Event::PointerButton {
        pos: target,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    ui.frame(Vec::new());
    let enlarged = ui.ctx.memory(|memory| memory.area_rect(egui::Id::new("garage_window"))).unwrap();
    assert!(enlarged.width() > rect.width() && enlarged.height() > rect.height());
    assert!(visible(&ui) > before, "resizing must reveal more cars, not just enlarge a blank window");
}

#[test]
fn named_presets_save_reopen_and_load_through_the_real_garage_buttons() {
    let mut car = Car { folder: "RACER".into(), name: "Racer".into(), paintable: true, ..Default::default() };
    car.performance.categories =
        [nfsmw_data::car::physics::performance::CategoryOptions { max_level: 4, ..Default::default() }; 7];
    let mut ui = fixture(vec![car, Car { folder: "OTHER".into(), ..Default::default() }], "RACER");
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("nfsmw-f6-{}-{stamp}.toml", std::process::id()));
    ui.model.preset_picker.path = Some(path.clone());
    ui.model.preset_picker.name = "Road setup".into();
    ui.model.draft.color = Some([20, 80, 210]);
    ui.model.draft.performance.levels = [3; 7];
    ui.model.draft.tuning.profiles[1] = [1, 2, 3, 4, 5, -1, -2];
    ui.model.draft.tuning.active = 1;
    let wanted = ui.model.draft.clone();
    ui.frame(Vec::new());
    assert!(ui.click("Saved presets").is_none());
    for _ in 0..20 {
        ui.frame(Vec::new());
    }
    assert!(ui.click("Save preset").is_none());
    assert!(path.is_file());
    ui.model.select("OTHER".into());
    ui.model.preset_picker = crate::preset_store::Picker::default();
    ui.model.preset_picker.path = Some(path.clone());
    ui.model.preset_picker.name = "Road setup".into();
    ui.frame(Vec::new());
    ui.frame(Vec::new());
    let request = ui.click("Load preset").expect("loading emits a normal garage transaction");
    assert_eq!(request, wanted);
    assert_eq!(ui.model.applied.car, "RACER", "the scene owns committing a successful transaction");
    std::fs::remove_file(path).unwrap();
}

impl Ui {
    fn frame(&mut self, events: Vec<egui::Event>) -> Option<Selection> {
        self.ctx.begin_pass(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 900.0))),
            events,
            ..Default::default()
        });
        let request = show(&self.ctx, &mut true, &mut self.model, &mut self.settings);
        let mut output = self.ctx.end_pass();
        output.textures_delta.clear();
        self.shapes = output.shapes;
        request
    }

    fn click(&mut self, label: &str) -> Option<Selection> {
        let position = self
            .shapes
            .iter()
            .find_map(|shape| {
                let egui::epaint::Shape::Text(text) = &shape.shape else { return None };
                if text.galley.text().lines().next() != Some(label) {
                    return None;
                }
                let position = text.pos + text.galley.rect.center().to_vec2();
                assert!(shape.clip_rect.contains(position), "{label} must be visible without scrolling");
                Some(position)
            })
            .unwrap_or_else(|| panic!("no visible widget labeled {label}"));
        self.frame(vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        let request = self.frame(vec![egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        self.frame(Vec::new());
        request
    }
}

#[test]
fn pointer_selection_and_paint_apply_immediately_without_repeated_requests() {
    let mut ui = Ui {
        ctx: egui::Context::default(),
        model: Model {
            cars: ["CAR_A", "CAR_B"]
                .map(|name| Car { folder: name.into(), name: name.into(), paintable: true, ..Car::default() })
                .into(),
            applied: Selection { car: "CAR_A".into(), color: None, ..Default::default() },
            draft: Selection { car: "CAR_A".into(), color: None, ..Default::default() },
            ..Model::default()
        },
        settings: Settings::from(Partial::default()),
        shapes: Vec::new(),
    };
    ui.frame(Vec::new());
    ui.frame(Vec::new());
    let request = ui.click("CAR_B").unwrap();
    assert_eq!(request.car, "CAR_B");
    assert_eq!(ui.model.draft.car, "CAR_B");
    assert_eq!(ui.model.applied.car, "CAR_A");
    assert!(ui.frame(Vec::new()).is_none(), "unchanged frames must not reload a car");
    ui.model.applied = request;
    assert!(ui.click("Appearance").is_none(), "switching option tabs must not apply a car change");
    let request = ui.click("Custom paint").unwrap();
    assert!(ui.model.draft.color.is_some());
    assert_eq!(request, ui.model.draft);
    assert_eq!(ui.model.applied.color, None, "the view waits for successful application");
    assert!(ui.frame(Vec::new()).is_none());
    assert!(ui.click("CAR_B").is_none(), "clicking the selected car must preserve custom paint");
    assert!(ui.model.draft.color.is_some());
    assert_eq!(ui.click("Original paint").unwrap().color, None);
}
