//! A scripted pad for screenshots and tests: `--ui-script "wait 1;right;right;wait 0.5;accept"`.
//!
//! The script replaces the real input and the real clock: every frame takes one step of 1/60 s. A button name
//! presses it for 4 frames and releases it for 4 frames, `hold NAME SECONDS` keeps it down, `wait SECONDS`
//! does nothing.

use std::collections::VecDeque;

use blackbox_feng::runtime::pad;

/// The clock step of a scripted frame.
pub const STEP: f32 = 1.0 / 60.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Step {
    mask: u32,
    frames: u32,
}

#[derive(Debug, Clone, Default)]
pub struct UiScript {
    steps: VecDeque<Step>,
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
        other => return Err(format!("unknown button {other:?} (up down left right accept back start)")),
    })
}

fn frames(seconds: &str) -> Result<u32, String> {
    let s: f32 = seconds.parse().map_err(|_| format!("expected seconds, got {seconds:?}"))?;
    Ok((s / STEP).round().max(0.0) as u32)
}

impl UiScript {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut steps = VecDeque::new();
        for part in text.split([';', ',']).map(str::trim).filter(|p| !p.is_empty()) {
            let words: Vec<&str> = part.split_whitespace().collect();
            match words[..] {
                ["wait", s] => steps.push_back(Step { mask: 0, frames: frames(s)? }),
                ["hold", name, s] => steps.push_back(Step { mask: button(name)?, frames: frames(s)? }),
                [name] => {
                    steps.push_back(Step { mask: button(name)?, frames: 4 });
                    steps.push_back(Step { mask: 0, frames: 4 });
                }
                _ => return Err(format!("cannot read the script step {part:?}")),
            }
        }
        Ok(Self { steps })
    }

    /// The pad mask of the next frame, or none when the script is over.
    pub fn next_mask(&mut self) -> Option<u32> {
        let step = self.steps.front_mut()?;
        let mask = step.mask;
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
    fn bad_steps_are_named() {
        assert!(UiScript::parse("jump").unwrap_err().contains("jump"));
        assert!(UiScript::parse("wait soon").is_err());
        assert!(UiScript::parse("").unwrap().is_over());
    }
}
