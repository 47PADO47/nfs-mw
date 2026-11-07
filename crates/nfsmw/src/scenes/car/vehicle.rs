//! The car on the floor as the developer tools' vehicle panel and the `vehicle` console command see it: its options, and a change
//! that dresses the car again at once (`garage::options`).

use blackbox_gfx::RenderBackend;
use nfsmw_data::car::customization::{Catalog, Selection};

use super::CarScene;
use crate::garage::CatalogSnapshot;
use crate::garage::options::{Change, Edit, Inputs, VehicleOptions};

/// What the car on the floor wears: the choices of its options, kept here because a viewer has no garage.
pub(super) struct Dressing {
    folder: String,
    /// The part catalogue of the car, read the first time its options are asked for.
    catalog: Option<CatalogSnapshot>,
    catalog_tried: bool,
    parts: Selection,
    color: Option<[u8; 3]>,
}

impl Dressing {
    pub(super) fn new(folder: &str, parts: Selection) -> Self {
        Self { folder: folder.to_owned(), catalog: None, catalog_tried: false, parts, color: None }
    }
}

impl CarScene {
    /// The viewer shows a car dressed in `parts` (`view-car --part`), so its options start from them.
    pub fn with_customization(mut self, parts: Selection) -> Self {
        self.dressing = Some(Dressing::new(&self.model.name, parts));
        self
    }

    /// The dressing of the car on the floor: what `--part` or an earlier change gave it, or stock for another car.
    fn dressing_of_car(&mut self) -> Option<&mut Dressing> {
        let source = self.source.as_ref()?;
        let folder = self.model.name.clone();
        if self.dressing.as_ref().is_none_or(|dressing| dressing.folder != folder) {
            self.dressing = Some(Dressing::new(&folder, Selection::default()));
        }
        let dressing = self.dressing.as_mut()?;
        if dressing.catalog.is_none() && !dressing.catalog_tried {
            dressing.catalog_tried = true;
            match Catalog::load(&source.dir, &folder) {
                Ok(mut catalog) => {
                    crate::garage::listing::arrange(&mut catalog, &crate::i18n::game);
                    dressing.catalog = Some(catalog.into());
                }
                Err(error) => log::warn!("{folder}: the options of its parts are not read: {error:#}"),
            }
        }
        Some(dressing)
    }

    /// The options of the car on the floor; no performance levels, since a viewer has no physics.
    pub(super) fn car_options(&mut self) -> Option<VehicleOptions> {
        let (paintable, original) = (self.model.paintable, self.model.paint.as_ref().map(|paint| paint.rgb));
        let folder = self.model.name.clone();
        let dressing = self.dressing_of_car()?;
        Some(VehicleOptions::build(&Inputs {
            folder: &folder,
            name: &folder,
            catalog: dressing.catalog.as_deref(),
            parts: &dressing.parts,
            color: dressing.color,
            paintable,
            original_color: original,
            performance: None,
        }))
    }

    /// Applies `change` and loads the car again with the new choices.
    pub(super) fn change_car(&mut self, renderer: &mut dyn RenderBackend, change: &Change) -> Result<String, String> {
        let options = self.car_options().ok_or("this viewer cannot load customized cars")?;
        let source = self.source.as_ref().ok_or("this viewer cannot load customized cars")?;
        let (dir, load) = (source.dir.clone(), source.options.clone());
        let dressing = self.dressing_of_car().ok_or("this viewer cannot load customized cars")?;
        let (mut parts, mut color, mut performance) = (dressing.parts.clone(), dressing.color, Default::default());
        let message = options.apply(
            dressing.catalog.as_deref(),
            &mut Edit { parts: &mut parts, color: &mut color, performance: &mut performance },
            change,
        )?;
        let mut model =
            nfsmw_data::car::load_customized(&dir, &options.folder, &load, &parts).map_err(|e| format!("{e:#}"))?;
        if let Some(rgb) = color {
            model.set_body_color(rgb).map_err(|e| format!("{e:#}"))?;
        }
        self.replace_model(renderer, model);
        if let Some(dressing) = self.dressing_of_car() {
            dressing.parts = parts;
            dressing.color = color;
        }
        Ok(message)
    }
}
