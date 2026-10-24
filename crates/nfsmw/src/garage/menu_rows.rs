//! One set of controls for pointer, keyboard and gamepad garage navigation.

use nfsmw_data::car::customization::{Category as Part, DecalZone};
use nfsmw_data::car::physics::{PerformanceCategory as Hardware, tuning::TuningCategory};

use super::Model;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Row {
    Car,
    Part(Part),
    Performance(Hardware),
    Junkman(Hardware),
    Profile,
    Tuning(TuningCategory),
    Gauge,
    Color(usize),
    Decal(usize, usize),
    Number(usize),
}

impl Row {
    pub fn label(self) -> String {
        match self {
            Self::Car => "Car".into(),
            Self::Part(category) => category.label().into(),
            Self::Performance(category) => category.label().into(),
            Self::Junkman(category) => format!("{} · Junkman", category.label()),
            Self::Profile => "Tuning profile".into(),
            Self::Tuning(category) => category.label().into(),
            Self::Gauge => "Gauge style".into(),
            Self::Color(index) => [
                "Body color",
                "Rim color",
                "Vinyl color 1",
                "Vinyl color 2",
                "Vinyl color 3",
                "Gauge backing",
                "Gauge needle",
                "Gauge digits",
            ][index]
                .into(),
            Self::Decal(zone, position) => format!("{} · position {}", DecalZone::ALL[zone].label(), position + 1),
            Self::Number(digit) => ["Door number · first digit", "Door number · second digit"][digit].into(),
        }
    }

    pub fn enabled(self, model: &Model) -> bool {
        let Some(car) = model.selected() else { return matches!(self, Self::Car) };
        match self {
            Self::Part(Part::RimPaint) | Self::Color(1) => car.parts.as_ref().is_some_and(|catalog| {
                model.draft.parts.parts[Part::Wheels as usize]
                    .is_some_and(|hash| catalog.paintable_wheels.contains(&hash))
            }),
            Self::Color(0) => car.paintable,
            Self::Junkman(category) => car.performance.junkman_available(category, model.draft.performance),
            Self::Tuning(category) => category.available(
                model.draft.performance,
                car.parts.as_ref().is_some_and(|catalog| catalog.aero_changed(&model.draft.parts)),
            ),
            Self::Number(digit) => car.parts.as_ref().is_some_and(|catalog| {
                let body = model.draft.parts.parts[Part::Body as usize].or(catalog.stock[Part::Body as usize]);
                [2, 3].into_iter().all(|zone| {
                    body.and_then(|body| catalog.decals[zone].body_number_positions.get(&body))
                        .is_some_and(|positions| positions[digit])
                })
            }),
            _ => true,
        }
    }

    pub fn options(self, model: &Model) -> (Vec<String>, usize) {
        let Some(car) = model.selected() else { return (Vec::new(), 0) };
        match self {
            Self::Car => {
                let cars: Vec<_> = model.cars.iter().collect();
                let selected = cars.iter().position(|car| car.folder == model.draft.car).unwrap_or(0);
                (
                    cars.into_iter()
                        .map(|car| match &car.unavailable {
                            Some(reason) => format!("{} · Locked: {reason}", car.name),
                            None => car.name.clone(),
                        })
                        .collect(),
                    selected,
                )
            }
            Self::Part(category) => {
                let Some(catalog) = &car.parts else { return (vec!["Stock".into()], 0) };
                let choices = &catalog.choices[category as usize];
                choices_options(choices, model.draft.parts.parts[category as usize])
            }
            Self::Performance(category) => {
                let max = car.performance.categories[category as usize].max_level;
                let mut choices = vec!["Original equipment".into()];
                choices.extend((1..=max).map(|level| format!("Stage {level}")));
                (choices, model.draft.performance.get(category) as usize)
            }
            Self::Junkman(category) => {
                (vec!["Off".into(), "Installed".into()], usize::from(model.draft.performance.has_junkman(category)))
            }
            Self::Profile => ((1..=3).map(|i| format!("Profile {i}")).collect(), model.draft.tuning.active as usize),
            Self::Tuning(category) => {
                let value = model.draft.tuning.profiles[model.draft.tuning.active.min(2) as usize][category as usize];
                ((-5..=5).map(|value| format!("{value:+}")).collect(), (value + 5) as usize)
            }
            Self::Gauge => {
                let skins = skins(model);
                let index = skins.iter().position(|skin| *skin == model.draft.gauges.skin).unwrap_or(0);
                (skins.into_iter().map(|skin| format!("Style {skin}")).collect(), index)
            }
            Self::Color(_) => {
                let value = self.color(model);
                let mut choices = vec!["Installed color".into()];
                choices.extend(PALETTE.iter().map(|(label, _)| (*label).into()));
                let position = value.and_then(|rgb| PALETTE.iter().position(|(_, color)| *color == rgb));
                if let Some([r, g, b]) = value.filter(|_| position.is_none()) {
                    choices.push(format!("Custom #{r:02X}{g:02X}{b:02X}"));
                    return (choices, PALETTE.len() + 1);
                }
                (choices, position.map_or(0, |i| i + 1))
            }
            Self::Decal(zone, position) => {
                choices_options(&decal_choices(model, zone, None), model.draft.parts.decals[zone][position])
            }
            Self::Number(digit) => {
                choices_options(&decal_choices(model, 3, Some(digit)), model.draft.parts.decals[3][6 + digit])
            }
        }
    }

    pub fn set(self, model: &mut Model, index: usize) {
        if !self.enabled(model) {
            return;
        }
        match self {
            Self::Car => {
                if let Some(folder) = model.cars.get(index).map(|car| car.folder.clone()) {
                    model.select(folder);
                }
            }
            Self::Part(category) => {
                let hash = index.checked_sub(1).and_then(|i| {
                    model.selected()?.parts.as_ref()?.choices[category as usize].get(i).map(|choice| choice.hash)
                });
                model.draft.parts.parts[category as usize] = hash;
                if category == Part::Body {
                    clear_missing_decal_surfaces(model);
                }
                if category == Part::Wheels && !Self::Color(1).enabled(model) {
                    model.draft.parts.parts[Part::RimPaint as usize] = None;
                    model.draft.parts.rim_color = None;
                }
            }
            Self::Performance(category) => {
                let max = model.selected().map_or(0, |car| car.performance.categories[category as usize].max_level);
                model.draft.performance.levels[category as usize] = (index as u8).min(max);
                if !Self::Junkman(category).enabled(model) {
                    model.draft.performance.junkman &= !(1 << category as u8);
                }
            }
            Self::Junkman(category) => {
                model.draft.performance.junkman &= !(1 << category as u8);
                if index != 0 {
                    model.draft.performance.junkman |= 1 << category as u8;
                }
            }
            Self::Profile => model.draft.tuning.active = index.min(2) as u8,
            Self::Tuning(category) => {
                model.draft.tuning.profiles[model.draft.tuning.active.min(2) as usize][category as usize] =
                    index.min(10) as i8 - 5
            }
            Self::Gauge => model.draft.gauges.skin = skins(model).get(index).copied().unwrap_or(0),
            Self::Color(_) => {
                self.set_color(model, index.checked_sub(1).and_then(|i| PALETTE.get(i)).map(|(_, rgb)| *rgb))
            }
            Self::Decal(zone, position) => {
                let hash = index.checked_sub(1).and_then(|i| decal_choices(model, zone, None).get(i).map(|c| c.hash));
                model.draft.parts.decals[zone][position] = hash;
            }
            Self::Number(digit) => {
                let hash =
                    index.checked_sub(1).and_then(|i| decal_choices(model, 3, Some(digit)).get(i).map(|c| c.hash));
                if let Some(hash) = hash {
                    let other = 1 - digit;
                    let companion = model.draft.parts.decals[3][6 + other]
                        .or_else(|| decal_choices(model, 3, Some(other)).first().map(|choice| choice.hash));
                    for zone in [2, 3] {
                        model.draft.parts.decals[zone][6 + digit] = Some(hash);
                        model.draft.parts.decals[zone][6 + other] = companion;
                    }
                }
                if hash.is_none() {
                    for zone in [2, 3] {
                        model.draft.parts.decals[zone][6..8].fill(None);
                    }
                }
            }
        }
        model.shop_total = None;
        model.shop_error = None;
    }

    pub fn color(self, model: &Model) -> Option<[u8; 3]> {
        match self {
            Self::Color(0) => model.draft.color,
            Self::Color(1) => model.draft.parts.rim_color,
            Self::Color(i @ 2..=4) => model.draft.parts.vinyl_colors[i - 2],
            Self::Color(5) => model.draft.gauges.backing,
            Self::Color(6) => model.draft.gauges.needle,
            Self::Color(7) => model.draft.gauges.digits,
            _ => None,
        }
    }

    pub fn set_color(self, model: &mut Model, color: Option<[u8; 3]>) {
        match self {
            Self::Color(index @ 2..=4) => model.draft.parts.vinyl_color_parts[index - 2] = None,
            Self::Color(index @ 5..=7) => model.draft.gauges.color_parts[index - 5] = None,
            _ => {}
        }
        if model.shop && color.is_some() {
            let category = match self {
                Self::Color(0) => Some(Part::Paint),
                Self::Color(1) => Some(Part::RimPaint),
                _ => None,
            };
            if let Some(category) = category
                && model.draft.parts.parts[category as usize].is_none()
                && let Some(catalog) = model.selected().and_then(|car| car.parts.as_ref())
            {
                let hash = match category {
                    Part::Paint => catalog.finishes.first().map(|(_, hash)| *hash),
                    _ => catalog.choices[category as usize].first().map(|choice| choice.hash),
                };
                model.draft.parts.parts[category as usize] = hash;
            }
        }
        match self {
            Self::Color(0) => model.draft.color = color,
            Self::Color(1) => model.draft.parts.rim_color = color,
            Self::Color(i @ 2..=4) => model.draft.parts.vinyl_colors[i - 2] = color,
            Self::Color(5) => model.draft.gauges.backing = color,
            Self::Color(6) => model.draft.gauges.needle = color,
            Self::Color(7) => model.draft.gauges.digits = color,
            _ => {}
        }
    }
}

pub(crate) fn clear_missing_decal_surfaces(model: &mut Model) {
    let Some(catalog) = model.selected().and_then(|car| car.parts.clone()) else { return };
    let body = model.draft.parts.parts[Part::Body as usize].or(catalog.stock[Part::Body as usize]);
    for (zone, decals) in catalog.decals.iter().enumerate() {
        for hash in &mut model.draft.parts.decals[zone] {
            if hash.is_some_and(|part| !decals.supports(body, part)) {
                *hash = None;
            }
        }
        let count = body.and_then(|body| decals.body_positions.get(&body)).copied().unwrap_or(0);
        let limit = match zone {
            2 | 3 => 6,
            _ => 8,
        };
        for position in count..limit {
            model.draft.parts.decals[zone][position] = None;
        }
        if !matches!(zone, 2 | 3) {
            continue;
        }
        let digits = body.and_then(|body| decals.body_number_positions.get(&body)).copied().unwrap_or([false; 2]);
        for (digit, available) in digits.into_iter().enumerate() {
            if !available {
                model.draft.parts.decals[zone][6 + digit] = None;
            }
        }
    }
}

fn decal_choices(model: &Model, zone: usize, digit: Option<usize>) -> Vec<nfsmw_data::car::customization::Choice> {
    let Some(catalog) = model.selected().and_then(|car| car.parts.as_ref()) else { return Vec::new() };
    let body = model.draft.parts.parts[Part::Body as usize].or(catalog.stock[Part::Body as usize]);
    let choices = match digit {
        Some(digit) => &catalog.number_choices[digit],
        None => &catalog.decals[zone].choices,
    };
    choices
        .iter()
        .filter(|choice| {
            catalog.decals[zone].supports(body, choice.hash)
                && (digit.is_none() || catalog.decals[2].supports(body, choice.hash))
        })
        .cloned()
        .collect()
}

fn choices_options(choices: &[nfsmw_data::car::customization::Choice], hash: Option<u32>) -> (Vec<String>, usize) {
    let selected = hash.and_then(|hash| choices.iter().position(|choice| choice.hash == hash)).map_or(0, |i| i + 1);
    let mut labels = vec!["Stock / remove".into()];
    labels.extend(choices.iter().map(|choice| choice.label.clone()));
    (labels, selected)
}

fn skins(model: &Model) -> Vec<u8> {
    let mut skins = vec![0];
    if let Some(catalog) = model.selected().and_then(|car| car.parts.as_ref()) {
        skins.extend(catalog.gauge_skins.iter().copied().filter(|skin| *skin != 0));
    }
    skins
}

const PALETTE: [(&str, [u8; 3]); 10] = [
    ("Black", [20, 20, 24]),
    ("White", [235, 235, 235]),
    ("Silver", [140, 148, 158]),
    ("Red", [190, 35, 35]),
    ("Blue", [30, 85, 210]),
    ("Yellow", [235, 190, 25]),
    ("Green", [35, 160, 60]),
    ("Orange", [240, 100, 20]),
    ("Purple", [130, 40, 190]),
    ("Pink", [230, 60, 150]),
];
