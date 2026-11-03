//! PC world-animation timing, independently implemented from the playback spec.
use nfsmw_data::world::mesh_animation::Instance;

#[derive(Clone, Copy, PartialEq, Debug)]
enum State {
    Stopped,
    Playing,
    Paused,
}

pub(super) struct Playback {
    pub frame: f32,
    begin: f32,
    end: f32,
    speed: f32,
    master_delay: f32,
    local_delay: f32,
    master_elapsed: f32,
    local_elapsed: f32,
    linear: bool,
    looping: bool,
    triggered: bool,
    reverse: bool,
    state: State,
}

impl Playback {
    pub fn new(instance: &Instance, frames: f32) -> Option<Self> {
        let speed = match instance.speed {
            0.0 => 1.0,
            value => value,
        };
        if !speed.is_finite()
            || speed <= 0.0
            || !frames.is_finite()
            || frames <= 0.0
            || !instance.master_delay.is_finite()
            || !instance.loop_delay.is_finite()
        {
            return None;
        }
        let mut flags = match instance.flags {
            0 => 0x28,
            value => value,
        };
        if instance.start_trigger != 0 || instance.stop_trigger != 0 {
            flags = (flags & !8) | 0x810;
        }
        let (begin, end) = match flags & 0x40 {
            0 => (0.0, frames),
            _ => (f32::from(instance.begin_range) * speed, f32::from(instance.end_range) * speed),
        };
        if !end.is_finite() || end <= begin {
            return None;
        }
        let linear = flags & 8 != 0 || flags & 0x10 == 0;
        let triggered = flags & 0x800 != 0;
        Some(Self {
            frame: 0.0,
            begin,
            end,
            speed,
            master_delay: instance.master_delay.max(0.0),
            local_delay: instance.loop_delay.max(0.0),
            master_elapsed: 0.0,
            local_elapsed: 0.0,
            linear,
            looping: flags & 0x20 != 0 && !triggered,
            triggered,
            reverse: linear && flags & 0x1000 != 0,
            state: match instance.start_trigger != 0 || instance.direction != 0 {
                true => State::Stopped,
                false => State::Playing,
            },
        })
    }

    pub fn visible(&self) -> bool {
        self.state != State::Stopped
    }

    pub fn advance(&mut self, dt: f32) {
        if self.state != State::Playing || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let step = dt * self.speed * 30.0;
        if !step.is_finite() {
            return;
        }
        if self.frame < self.end {
            if let Some(overflow) = delay(&mut self.master_elapsed, self.master_delay, step) {
                self.frame += overflow;
                return;
            }
            if let Some(overflow) = delay(&mut self.local_elapsed, self.local_delay, step) {
                self.frame += overflow;
                return;
            }
        }
        let next = self.frame
            + match self.reverse {
                true => -step,
                false => step,
            };
        if !next.is_finite() {
            return;
        }
        self.frame = next;
        if self.linear {
            let crossed = match self.reverse {
                true => self.frame < self.begin,
                false => self.frame > self.end,
            };
            if !crossed {
                return;
            }
            if !self.looping {
                self.frame = match self.reverse {
                    true => self.begin,
                    false => self.end,
                };
                return;
            }
            let range = self.end - self.begin;
            self.frame += match self.reverse {
                true => range,
                false => -range,
            };
            self.local_elapsed = 0.0;
            return;
        }
        if !self.reverse && self.frame > self.end {
            self.frame = self.end - (self.frame - self.end);
            self.reverse = true;
            if self.triggered {
                self.state = State::Paused;
            }
            return;
        }
        if self.reverse && self.frame < self.begin {
            self.frame = self.begin;
            self.reverse = false;
            self.local_elapsed = 0.0;
            if self.triggered {
                self.state = State::Paused;
            }
        }
    }
}

fn delay(elapsed: &mut f32, seconds: f32, step: f32) -> Option<f32> {
    let limit = seconds * 30.0;
    if *elapsed >= limit {
        return None;
    }
    *elapsed += step;
    Some(match *elapsed > limit {
        true => *elapsed % limit,
        false => 0.0,
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
