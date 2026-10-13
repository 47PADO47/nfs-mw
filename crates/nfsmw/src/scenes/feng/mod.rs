//! The FEng viewer (`feng-viewer`): every screen of the install in a searchable list on the left, the chosen screen
//! on the menus' flat backdrop with the camera off. The scene draws the list and hands each pick to the front end,
//! which runs the screen (`frontend::Start::Browse`); the screens themselves are the front end's.

use anyhow::{Result, anyhow, bail};
use blackbox_gfx::{FrameParams, Instance, RenderBackend};
use game_install::GameDir;

use crate::frontend::{Browse, MenuScene};
use crate::input::ActionState;
use crate::scenes::sidebar;
use crate::ui::{Catalog, SCREEN_FILES};
use crate::viewer::Scene;

pub struct FengViewerScene {
    /// The file name of every screen, sorted.
    names: Vec<String>,
    /// Index into `names` of the screen on the backdrop.
    selected: usize,
    panel: sidebar::Panel,
    /// Whether Ctrl+B has hidden the list. The screen then takes the whole window.
    collapsed: bool,
    /// The hand-over with the front end: picks go out, and the list's pointer state comes back in.
    browse: Browse,
    /// The menus' flat backdrop: no car and no room, so nothing is turned or followed.
    backdrop: MenuScene,
}

impl FengViewerScene {
    /// Lists the screens of the install and starts on the one `start` names (in any case), or the first one.
    pub fn open(dir: &GameDir, start: Option<&str>) -> Result<Self> {
        let catalog = Catalog::load(dir, &SCREEN_FILES);
        let names: Vec<String> = catalog.entries().into_iter().map(|entry| entry.name.clone()).collect();
        if names.is_empty() {
            bail!("the install has no FEng screens (try list-screens)");
        }
        let selected = match start {
            Some(wanted) => names
                .iter()
                .position(|name| name.eq_ignore_ascii_case(wanted))
                .ok_or_else(|| anyhow!("no screen {wanted:?} in the install (try list-screens)"))?,
            None => 0,
        };
        Ok(Self {
            names,
            selected,
            panel: sidebar::Panel::default(),
            collapsed: false,
            browse: Browse::default(),
            backdrop: MenuScene::new(None),
        })
    }

    /// The screen the front end shows first.
    pub fn start_name(&self) -> &str {
        &self.names[self.selected]
    }

    /// The hand-over the front end reads the picks from.
    pub fn browse(&self) -> Browse {
        self.browse.clone()
    }
}

impl Scene for FengViewerScene {
    fn title(&self) -> String {
        self.backdrop.title()
    }

    fn init(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        self.backdrop.init(renderer)
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        self.backdrop.update(renderer, input, dt);
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        self.backdrop.frame(aspect)
    }

    fn hud_state(&self) -> Option<crate::hud::HudState> {
        self.backdrop.hud_state()
    }

    fn ui(&mut self, ctx: &egui::Context) -> f32 {
        if sidebar::toggle_pressed(ctx) {
            self.collapsed = !self.collapsed;
        }
        if self.collapsed {
            // No list to be over; the backdrop has no camera to recentre.
            self.browse.set_over_panel(false);
            return 0.0;
        }
        let clicked = sidebar::show(ctx, &mut self.panel, &self.names, self.selected, None, "screens");
        if let Some(index) = clicked {
            self.selected = index;
            self.browse.pick(&self.names[index]);
        }
        self.browse.set_over_panel(ctx.is_pointer_over_egui());
        sidebar::width(ctx)
    }
}
