//! One packet of output: the heart of the synthesis (spec §4).

use super::GinsuSynth;
use crate::round;

/// A jump with no cycles, or a time that is not a number, is no jump: the whole packet plays.
fn checked(jump: Jump) -> Jump {
    if jump.cycles == 0 || !jump.time.is_finite() {
        return Jump { cycles: 0, time: 1.0 };
    }
    jump
}

/// Where the jump goes in the packet: how many cycles, and at which fraction of the packet.
#[derive(Debug, Clone, Copy, Default)]
struct Jump {
    cycles: i32,
    time: f32,
}

impl GinsuSynth {
    /// Appends the next packet to the (empty) queue.
    pub(super) fn next_packet(&mut self) {
        self.queue.clear();
        self.queue_pos = 0;
        let tables = self.data.tables();
        self.current_cycle = tables.sample_to_cycle(self.current_pos);
        let change = (self.target_pos - self.current_pos) as f32 / self.countdown as f32;

        let jump = self.plan_jump(change);
        let packet = self.packet_size;
        let before = round::round(jump.time * packet as f32).clamp(0, packet);
        self.push_samples(before);
        self.no_jump_remaining = (self.no_jump_remaining - before).max(0);

        if jump.cycles != 0 {
            self.jump_by(jump.cycles);
        }

        self.current_pos = round::round(self.current_pos as f32 + change);
        if self.countdown > 1 {
            self.countdown -= 1;
        }
    }

    /// Decides whether this packet jumps, by how many whole cycles and when.
    fn plan_jump(&self, change: f32) -> Jump {
        if self.no_jump_remaining >= self.packet_size {
            return Jump { cycles: 0, time: 1.0 };
        }
        let tables = self.data.tables();
        let packet = self.packet_size as f32;
        let playback_cycle = tables.sample_to_cycle(self.playback_pos);
        let playback_rate = packet / tables.cycle_period(playback_cycle);
        let current_rate = change / tables.cycle_period(self.current_cycle);
        let pos_diff = self.current_cycle - playback_cycle;
        let rate_diff = current_rate - playback_rate;
        let nojump_time = self.no_jump_remaining as f32 / packet;
        let nojump_dist = pos_diff + rate_diff * nojump_time;
        let packet_dist = pos_diff + rate_diff;

        if round::floor(nojump_dist) != round::floor(packet_dist) {
            let cycles = match nojump_dist < packet_dist {
                true => round::ceil(nojump_dist),
                false => round::floor(nojump_dist),
            };
            return checked(Jump { cycles, time: (cycles as f32 - pos_diff) / rate_diff });
        }
        checked(self.catch_up(nojump_dist, nojump_time, rate_diff))
    }

    /// The lead is already larger than the packet's own drift: jump as soon as jumps are allowed.
    fn catch_up(&self, nojump_dist: f32, nojump_time: f32, rate_diff: f32) -> Jump {
        let max_dist = round::ceil((rate_diff * 0.999_999_94).abs()) as f32;
        if nojump_dist.abs() <= max_dist {
            return Jump::default();
        }
        let cycles = match nojump_dist < 0.0 {
            true => round::ceil(nojump_dist),
            false => round::floor(nojump_dist),
        };
        Jump { cycles, time: nojump_time }
    }

    /// Reads `count` samples from the playback position onto the queue and advances it.
    fn push_samples(&mut self, count: i32) {
        let start = self.queue.len();
        self.queue.resize(start + count as usize, 0.0);
        self.data.read(self.playback_pos, &mut self.queue[start..]);
        self.playback_pos += count;
    }

    /// Cross-fades from the current position to `cycles` whole cycles away, then fills the packet.
    fn jump_by(&mut self, cycles: i32) {
        let overlap = self.overlap_size as usize;
        let mut old = std::mem::take(&mut self.scratch[0]);
        let mut new = std::mem::take(&mut self.scratch[1]);
        old.resize(overlap, 0.0);
        new.resize(overlap, 0.0);

        let tables = self.data.tables();
        self.data.read(self.playback_pos, &mut old);
        let cycle = tables.sample_to_cycle(self.playback_pos);
        self.playback_pos = tables.cycle_to_sample(cycle + cycles as f32);
        self.data.read(self.playback_pos, &mut new);

        let step = 1.0 / self.overlap_size as f32;
        let mut blend = 0.0f32;
        for (a, b) in old.iter().zip(&new) {
            self.queue.push(a + blend * (b - a));
            blend += step;
        }
        self.scratch = [old, new];

        self.playback_pos += self.overlap_size;
        self.no_jump_remaining = self.no_jump_size - self.overlap_size;
        let rest = self.packet_size - self.queue.len() as i32;
        if rest > 0 {
            self.push_samples(rest);
            self.no_jump_remaining -= rest;
        }
    }
}
