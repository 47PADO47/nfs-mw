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
    /// Driving button: nitrous oxide.
    Nos,
    /// Driving button: put the car back on the road.
    ResetCar,
    /// Button: switch between the chase camera and the free camera.
    ToggleCamera,
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
    /// Menu hot key: quit from the main menu.
    MenuQuit,
    /// Button: a click, which skips a boot movie and continues from the title screen.
    Click,
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
            "nos",
            "reset_car",
            "toggle_camera",
            "menu_up",
            "menu_down",
            "menu_left",
            "menu_right",
            "menu_accept",
            "menu_back",
            "menu_start",
            "menu_quit",
            "click",
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
    pub const ALL: [Action; 29] = [
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
        Action::Nos,
        Action::ResetCar,
        Action::ToggleCamera,
        Action::MenuUp,
        Action::MenuDown,
        Action::MenuLeft,
        Action::MenuRight,
        Action::MenuAccept,
        Action::MenuBack,
        Action::MenuStart,
        Action::MenuQuit,
        Action::Click,
    ];

    /// Actions that still work while the UI has the keyboard.
    pub(super) fn works_in_ui(self) -> bool {
        matches!(self, Action::Cancel | Action::Console)
    }

    pub(super) fn index(self) -> usize {
        self as usize
    }

    /// Whether several devices adding up should still stay within -1..1.
    pub(super) fn is_bounded(self) -> bool {
        !matches!(self, Action::LookX | Action::LookY | Action::OrbitX | Action::OrbitY | Action::Zoom)
    }
}
