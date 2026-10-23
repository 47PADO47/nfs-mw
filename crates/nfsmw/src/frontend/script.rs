//! A scripted pad for screenshots and tests: `--ui-script "wait 1;right;right;wait 0.5;accept"`.
//!
//! The script replaces the real input and the real clock: every frame takes one step of 1/60 s. A button name
//! presses it for 4 frames and releases it for 4 frames, `hold NAME SECONDS` keeps it down, `wait SECONDS`
//! does nothing. `console COMMAND` runs a console command (`console story race 16.2.1`), to drive a world from a script.

use std::collections::VecDeque;

use bevy_input::keyboard::KeyCode;
use blackbox_feng::runtime::pad;
use glam::Vec2;

/// The clock step of a scripted frame.
pub const STEP: f32 = 1.0 / 60.0;

#[derive(Debug, Clone, PartialEq)]
struct Step {
    mask: u32,
    frames: u32,
    /// A key press for the frame this step starts on (`key NAME`), for the remap screen.
    key: Option<KeyCode>,
    /// Where the pointer is from this step on (`pointer X Y`, FEng units).
    pointer: Option<Vec2>,
    /// Text typed on the frame this step starts on (`type TEXT`), for the alias name prompt.
    text: Option<String>,
    /// A console command run on the frame this step starts on (`console COMMAND`), for scripts that drive the world.
    console: Option<String>,
}

/// The keys a script can press: the letters, Space, Enter, Numpad Enter and Escape (Space, Enter and Numpad Enter press Start for
/// the split-screen Press Start screens: the keyboard's, and the arrow block's).
fn key(name: &str) -> Result<KeyCode, String> {
    const LETTERS: [KeyCode; 26] = [
        KeyCode::KeyA,
        KeyCode::KeyB,
        KeyCode::KeyC,
        KeyCode::KeyD,
        KeyCode::KeyE,
        KeyCode::KeyF,
        KeyCode::KeyG,
        KeyCode::KeyH,
        KeyCode::KeyI,
        KeyCode::KeyJ,
        KeyCode::KeyK,
        KeyCode::KeyL,
        KeyCode::KeyM,
        KeyCode::KeyN,
        KeyCode::KeyO,
        KeyCode::KeyP,
        KeyCode::KeyQ,
        KeyCode::KeyR,
        KeyCode::KeyS,
        KeyCode::KeyT,
        KeyCode::KeyU,
        KeyCode::KeyV,
        KeyCode::KeyW,
        KeyCode::KeyX,
        KeyCode::KeyY,
        KeyCode::KeyZ,
    ];
    if let [c] = name.as_bytes()
        && c.is_ascii_alphabetic()
    {
        return Ok(LETTERS[usize::from(c.to_ascii_lowercase() - b'a')]);
    }
    match name {
        "Space" => Ok(KeyCode::Space),
        "Enter" => Ok(KeyCode::Enter),
        "NumpadEnter" => Ok(KeyCode::NumpadEnter),
        "Escape" => Ok(KeyCode::Escape),
        other => Err(format!("unknown key {other:?} (a letter, Space, Enter, NumpadEnter or Escape)")),
    }
}

#[derive(Debug, Clone, Default)]
pub struct UiScript {
    steps: VecDeque<Step>,
    /// The key the last frame's step pressed, until the frontend takes it (`take_key`).
    key: Option<KeyCode>,
    /// Where the scripted pointer is, once a `pointer` step has run.
    pointer: Option<Vec2>,
    /// The text the last frame's step typed, until the front end takes it (`take_text`).
    text: Option<String>,
    /// The console command the last frame's step ran, until the front end takes it (`take_console`).
    console: Option<String>,
    /// Deterministic prompt device for screenshots, not a gameplay preference.
    pub device: Option<crate::input::InputDevice>,
}

fn button(name: &str) -> Result<u32, String> {
    Ok(match name {
        "up" => pad::UP,
        "down" => pad::DOWN,
        "left" => pad::LEFT,
        "right" => pad::RIGHT,
        "accept" => pad::ACCEPT,
        "back" => pad::BACK,
        "start" => pad::START,
        "button1" => (pad::BUTTON0 << 1) | (pad::BUTTON0 << 5),
        "button2" => pad::BUTTON0 << 4,
        "button3" => pad::BUTTON0,
        "ltrigger" => pad::LTRIGGER,
        "rtrigger" => pad::RTRIGGER,
        other => {
            return Err(format!(
                "unknown button {other:?} (up down left right accept back start button1 button2 button3 ltrigger rtrigger)"
            ));
        }
    })
}

fn number(text: &str) -> Result<f32, String> {
    text.parse().map_err(|_| format!("expected a number, got {text:?}"))
}

fn frames(seconds: &str) -> Result<u32, String> {
    let s: f32 = seconds.parse().map_err(|_| format!("expected seconds, got {seconds:?}"))?;
    Ok((s / STEP).round().max(0.0) as u32)
}

impl UiScript {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut steps = VecDeque::new();
        let mut device = None;
        for part in text.split([';', ',']).map(str::trim).filter(|p| !p.is_empty()) {
            let words: Vec<&str> = part.split_whitespace().collect();
            match words[..] {
                ["device", "xbox"] => device = Some(crate::input::InputDevice::Xbox),
                ["device", "keyboard"] => device = Some(crate::input::InputDevice::Keyboard),
                ["wait", s] => {
                    steps.push_back(Step { mask: 0, frames: frames(s)?, key: None, pointer: None, text: None, console: None })
                }
                ["hold", name, s] => steps.push_back(Step {
                    mask: button(name)?,
                    frames: frames(s)?,
                    key: None,
                    pointer: None,
                    text: None,
                    console: None,
                }),
                ["key", name] => {
                    steps.push_back(Step { mask: 0, frames: 1, key: Some(key(name)?), pointer: None, text: None, console: None })
                }
                ["pointer", x, y] => {
                    let at = Vec2::new(number(x)?, number(y)?);
                    steps.push_back(Step { mask: 0, frames: 1, key: None, pointer: Some(at), text: None, console: None });
                }
                ["console", ref command @ ..] if !command.is_empty() => steps.push_back(Step {
                    mask: 0,
                    frames: 1,
                    key: None,
                    pointer: None,
                    text: None,
                    console: Some(command.join(" ")),
                }),
                ["type", text] => {
                    steps.push_back(Step { mask: 0, frames: 1, key: None, pointer: None, text: Some(text.to_owned()), console: None });
                }
                [name] => {
                    steps.push_back(Step { mask: button(name)?, frames: 4, key: None, pointer: None, text: None, console: None });
                    steps.push_back(Step { mask: 0, frames: 4, key: None, pointer: None, text: None, console: None });
                }
                _ => return Err(format!("cannot read the script step {part:?}")),
            }
        }
        Ok(Self { steps, device, key: None, pointer: None, text: None, console: None })
    }

    /// The key the script pressed on the frame just stepped, if any. The frontend takes it once per frame.
    pub fn take_key(&mut self) -> Option<KeyCode> {
        self.key.take()
    }

    /// The text the script typed on the frame just stepped, if any. The front end takes it once per frame.
    pub fn take_text(&mut self) -> Option<String> {
        self.text.take()
    }

    /// The console command a `console` step ran on the frame just stepped, if any. The front end hands it to the console.
    pub fn take_console(&mut self) -> Option<String> {
        self.console.take()
    }

    /// Where a `pointer` step put the pointer, in FEng units; `None` until one has run.
    pub fn pointer(&self) -> Option<Vec2> {
        self.pointer
    }

    /// The pad mask of the next frame, or none when the script is over.
    pub fn next_mask(&mut self) -> Option<u32> {
        let step = self.steps.front_mut()?;
        let mask = step.mask;
        if let Some(key) = step.key {
            self.key = Some(key);
        }
        if step.pointer.is_some() {
            self.pointer = step.pointer;
        }
        if let Some(text) = step.text.take() {
            self.text = Some(text);
        }
        if let Some(command) = step.console.take() {
            self.console = Some(command);
        }
        step.frames = step.frames.saturating_sub(1);
        if step.frames == 0 {
            self.steps.pop_front();
        }
        Some(mask)
    }

    pub fn is_over(&self) -> bool {
        self.steps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_become_frames() {
        let mut s = UiScript::parse("wait 0.05; right; hold accept 0.1").unwrap();
        let masks: Vec<u32> = std::iter::from_fn(|| s.next_mask()).collect();
        assert_eq!(&masks[..3], &[0, 0, 0], "0.05 s is 3 frames");
        assert_eq!(&masks[3..7], &[pad::RIGHT; 4]);
        assert_eq!(&masks[7..11], &[0; 4]);
        assert_eq!(masks[11..].iter().filter(|&&m| m == pad::ACCEPT).count(), 6);
        assert!(s.is_over());
    }

    #[test]
    fn a_key_step_presses_once() {
        let mut s = UiScript::parse("key W; wait 0.05").unwrap();
        assert_eq!(s.next_mask(), Some(0));
        assert_eq!(s.take_key(), Some(KeyCode::KeyW));
        assert_eq!(s.take_key(), None, "taken once");
        assert_eq!(s.next_mask(), Some(0));
        assert_eq!(s.take_key(), None, "a wait presses nothing");
        assert!(UiScript::parse("key Tab").unwrap_err().contains("Tab"));
    }

    #[test]
    fn a_pointer_step_moves_the_scripted_pointer_and_it_stays() {
        let mut s = UiScript::parse("pointer -20 35.5; wait 0.05").unwrap();
        assert_eq!(s.pointer(), None, "nothing before the step runs");
        s.next_mask();
        assert_eq!(s.pointer(), Some(Vec2::new(-20.0, 35.5)));
        while s.next_mask().is_some() {}
        assert_eq!(s.pointer(), Some(Vec2::new(-20.0, 35.5)), "it stays where it was put");
        assert!(UiScript::parse("pointer left 3").is_err());
    }

    #[test]
    fn bad_steps_are_named() {
        assert!(UiScript::parse("jump").unwrap_err().contains("jump"));
        assert!(UiScript::parse("wait soon").is_err());
        assert!(UiScript::parse("").unwrap().is_over());
    }
}
