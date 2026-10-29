//! What the player can ask for, independent of the device that asks.

/// An abstract input. Axes carry a signed amount; buttons are 1 while held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Axis, -1..1: forward is positive.
    MoveForward,
    /// Axis, -1..1: right is positive.
    MoveRight,
    /// Axis, -1..1: up is positive.
    MoveUp,
    /// Free-look turn this frame, in mouse pixels: right is positive.
    LookX,
    /// Free-look turn this frame, in mouse pixels: down is positive.
    LookY,
    /// Orbit turn this frame, in mouse pixels: right is positive.
    OrbitX,
    /// Orbit turn this frame, in mouse pixels: down is positive.
    OrbitY,
    /// Zoom or speed change this frame, in scroll lines: away from the player is positive.
    Zoom,
    /// Button: go faster.
    Boost,
    /// Button: back out (close the console, release the mouse, then quit).
    Cancel,
    /// Button: open or close the developer console.
    Console,
    /// Driving, 0..1: accelerator.
    Throttle,
    /// Driving, 0..1: brake, which becomes reverse once the car stands still.
    Brake,
    /// Driving axis, -1..1: right is positive.
    Steer,
    /// Driving button: handbrake.
    Handbrake,
    /// Driving button: shift up one gear.
    ShiftUp,
    /// Driving button: shift down one gear.
    ShiftDown,
    /// Driving button: select reverse (a gear selector's reverse slot or a gear key).
    GearReverse,
    /// Driving button: select neutral.
    GearNeutral,
    /// Driving buttons: select that gear by number (an H-pattern shifter or gear keys).
    Gear1,
    Gear2,
    Gear3,
    Gear4,
    Gear5,
    Gear6,
    Gear7,
    /// Driving, 0..1: clutch pedal (read only with the `clutch` setting on).
    Clutch,
    /// Driving button: nitrous oxide.
    Nos,
    /// Button: the speed breaker, which slows the world while its meter lasts (`speedbreaker.rs`).
    Speedbreaker,
    /// Driving button: put the car back on the road.
    ResetCar,
    /// Button: switch between the chase camera and the free camera.
    ToggleCamera,
    /// Driving button: the next gameplay camera (the camera setting).
    CycleCamera,
    /// Driving button: the car's headlights on or off.
    ToggleHeadlights,
    /// Driving button: hold to look back at the car (the camera swings round).
    LookBack,
    /// In-world button: interact with what the car or player stands on (enter a race, safe house or shop).
    Interact,
    /// Menu button: the focus moves up.
    MenuUp,
    MenuDown,
    MenuLeft,
    MenuRight,
    /// Menu button: accept.
    MenuAccept,
    /// Menu button: back.
    MenuBack,
    /// Menu button: start (pauses the game while driving, resumes from the pause menu).
    MenuStart,
    /// Menu button: the second face button (`PAD_BUTTON1`): the post-race screen's Restart.
    MenuButton1,
    /// Menu button: the fifth face button (`PAD_BUTTON4`): the win photo's Race Stats.
    MenuButton2,
    /// Menu button: the first face button (`PAD_BUTTON0`): the custom race car select's Showcase.
    MenuButton3,
    /// Menu keys of the Showcase screen: zoom the camera in and out (keys 9 and 0).
    MenuZoomIn,
    MenuZoomOut,
    /// Menu buttons: the left and right shoulder (`PAD_LTRIGGER`, `PAD_RTRIGGER`): the post-race screen's racer arrows.
    MenuTriggerLeft,
    MenuTriggerRight,
    /// Menu hot key: quit from the main menu.
    MenuQuit,
    /// Button: a click, which skips a boot movie and continues from the title screen.
    Click,
    /// Radio button: pause the song, or resume it.
    RadioToggle,
    /// Radio button: the next song.
    RadioNext,
    /// Radio button: the previous song.
    RadioPrevious,
    /// Button: open or close the runtime garage overlay.
    Garage,
    /// Button: open or close the full-screen world map, from free roam (`frontend/world_map`).
    Map,
    /// Driving button: take the HUD's oldest notification off the screen (`hud/notify`).
    DismissNotification,
    /// HUD button: the Custom map's next mode: off, compact, centered, expanded, then off again
    /// (`ui/theme/custom/map/modes.rs`). Never a driving input.
    MapMode,
    /// HUD button: open the expanded map, or go back to the mode it was opened from.
    MapExpand,
    /// HUD buttons: zoom the Custom map in and out a step.
    MapZoomIn,
    MapZoomOut,
    /// HUD button: turn the map north-up or heading-up (the `minimap` setting).
    MapOrientation,
    /// HUD button: hide or show the whole HUD for the session (the `hud` setting, not saved).
    ToggleHud,
    /// EA Trax: grab/release the selected song for reordering (Y on the default controller layout).
    JukeboxGrab,
}

impl Action {
    pub fn name(self) -> &'static str {
        const NAMES: [&str; Action::ALL.len()] = [
            "move_forward",
            "move_right",
            "move_up",
            "look_x",
            "look_y",
            "orbit_x",
            "orbit_y",
            "zoom",
            "boost",
            "cancel",
            "console",
            "throttle",
            "brake",
            "steer",
            "handbrake",
            "shift_up",
            "shift_down",
            "gear_reverse",
            "gear_neutral",
            "gear_1",
            "gear_2",
            "gear_3",
            "gear_4",
            "gear_5",
            "gear_6",
            "gear_7",
            "clutch",
            "nos",
            "speedbreaker",
            "reset_car",
            "toggle_camera",
            "cycle_camera",
            "toggle_headlights",
            "look_back",
            "interact",
            "menu_up",
            "menu_down",
            "menu_left",
            "menu_right",
            "menu_accept",
            "menu_back",
            "menu_start",
            "menu_button_1",
            "menu_button_2",
            "menu_button_3",
            "menu_zoom_in",
            "menu_zoom_out",
            "menu_trigger_left",
            "menu_trigger_right",
            "menu_quit",
            "click",
            "radio_toggle",
            "radio_next",
            "radio_previous",
            "garage",
            "map",
            "dismiss_notification",
            "map_mode",
            "map_expand",
            "map_zoom_in",
            "map_zoom_out",
            "map_orientation",
            "toggle_hud",
            "jukebox_grab",
        ];
        NAMES[self.index()]
    }

    pub fn parse(name: &str) -> Result<Self, String> {
        let name = name.to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|a| a.name() == name)
            .ok_or_else(|| format!("unknown action {name:?}; use keys to list actions"))
    }
    pub const ALL: [Action; 64] = [
        Action::MoveForward,
        Action::MoveRight,
        Action::MoveUp,
        Action::LookX,
        Action::LookY,
        Action::OrbitX,
        Action::OrbitY,
        Action::Zoom,
        Action::Boost,
        Action::Cancel,
        Action::Console,
        Action::Throttle,
        Action::Brake,
        Action::Steer,
        Action::Handbrake,
        Action::ShiftUp,
        Action::ShiftDown,
        Action::GearReverse,
        Action::GearNeutral,
        Action::Gear1,
        Action::Gear2,
        Action::Gear3,
        Action::Gear4,
        Action::Gear5,
        Action::Gear6,
        Action::Gear7,
        Action::Clutch,
        Action::Nos,
        Action::Speedbreaker,
        Action::ResetCar,
        Action::ToggleCamera,
        Action::CycleCamera,
        Action::ToggleHeadlights,
        Action::LookBack,
        Action::Interact,
        Action::MenuUp,
        Action::MenuDown,
        Action::MenuLeft,
        Action::MenuRight,
        Action::MenuAccept,
        Action::MenuBack,
        Action::MenuStart,
        Action::MenuButton1,
        Action::MenuButton2,
        Action::MenuButton3,
        Action::MenuZoomIn,
        Action::MenuZoomOut,
        Action::MenuTriggerLeft,
        Action::MenuTriggerRight,
        Action::MenuQuit,
        Action::Click,
        Action::RadioToggle,
        Action::RadioNext,
        Action::RadioPrevious,
        Action::Garage,
        Action::Map,
        Action::DismissNotification,
        Action::MapMode,
        Action::MapExpand,
        Action::MapZoomIn,
        Action::MapZoomOut,
        Action::MapOrientation,
        Action::ToggleHud,
        Action::JukeboxGrab,
    ];

    /// Actions that still work while the UI has the keyboard.
    pub(super) fn works_in_ui(self) -> bool {
        matches!(self, Action::Cancel | Action::Console | Action::Garage)
    }

    pub(super) fn index(self) -> usize {
        self as usize
    }

    /// Whether several devices adding up should still stay within -1..1.
    pub(super) fn is_bounded(self) -> bool {
        !matches!(self, Action::LookX | Action::LookY | Action::OrbitX | Action::OrbitY | Action::Zoom)
    }
}
