//! Keyboard and mouse state, collected from window events between frames.

use std::collections::HashSet;

use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::KeyCode;

#[derive(Default)]
pub struct Input {
    keys: HashSet<KeyCode>,
    left: bool,
    right: bool,
    /// The cursor is captured for mouse look.
    captured: bool,
    /// Raw mouse motion since the last frame.
    mouse_delta: (f32, f32),
    /// Scroll since the last frame, in lines.
    scroll: f32,
}

impl Input {
    pub fn key(&self, key: KeyCode) -> bool {
        self.keys.contains(&key)
    }

    pub fn left_button(&self) -> bool {
        self.left
    }

    pub fn right_button(&self) -> bool {
        self.right
    }

    /// Whether mouse motion should turn the camera without a button held.
    pub fn mouse_captured(&self) -> bool {
        self.captured
    }

    pub fn mouse_delta(&self) -> (f32, f32) {
        self.mouse_delta
    }

    pub fn scroll(&self) -> f32 {
        self.scroll
    }

    pub(super) fn on_key(&mut self, key: KeyCode, state: ElementState) {
        match state {
            ElementState::Pressed => self.keys.insert(key),
            ElementState::Released => self.keys.remove(&key),
        };
    }

    pub(super) fn on_button(&mut self, button: MouseButton, state: ElementState) {
        let pressed = state == ElementState::Pressed;
        match button {
            MouseButton::Left => self.left = pressed,
            MouseButton::Right => self.right = pressed,
            _ => {}
        }
    }

    pub(super) fn set_captured(&mut self, captured: bool) {
        self.captured = captured;
    }

    pub(super) fn on_motion(&mut self, dx: f64, dy: f64) {
        self.mouse_delta.0 += dx as f32;
        self.mouse_delta.1 += dy as f32;
    }

    pub(super) fn on_scroll(&mut self, delta: MouseScrollDelta) {
        self.scroll += match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(p) => p.y as f32 / 60.0,
        };
    }

    /// Clear the per-frame deltas.
    pub(super) fn end_frame(&mut self) {
        self.mouse_delta = (0.0, 0.0);
        self.scroll = 0.0;
    }

    /// Forget held keys and buttons (e.g. when the window loses focus).
    pub(super) fn release_all(&mut self) {
        self.keys.clear();
        self.left = false;
        self.right = false;
    }
}
