use crate::{Block, Error, MAX_BLOCK, Node, Result, invalid, math};

pub(super) enum Kernel {
    Input,
    OnePole { previous: f32, alpha: f32, beta: f32 },
    HighPass { history: [f32; 8], coefficients: [f32; 5] },
    BandPass { history: [f32; 2], radius: f32, cosine: f32, feed: f32, bypass: bool },
    Gain(f32),
    AllPass { ring: Vec<f32>, position: usize, feedback: f32 },
    Sum,
    Cache { block: Box<Block>, ready: bool, consumed: bool },
}

impl Kernel {
    pub fn new(node: Node, rate: u32) -> Result<Self> {
        let [first, second, third] = node.parameters;
        Ok(match node.opcode {
            0 => Self::Input,
            1 => {
                if second < 0 {
                    return Err(invalid("negative filter cutoff"));
                }
                let fraction64 = 2.0 * f64::from(second) / f64::from(rate);
                let fraction = fraction64 as f32;
                let alpha = (1.0 - fraction64) as f32;
                let gain = f32::from(third) / 256.0;
                Self::OnePole { previous: 0.0, alpha: alpha * gain, beta: fraction * gain }
            }
            3 => {
                if first < 0 {
                    return Err(invalid("negative filter cutoff"));
                }
                Self::HighPass { history: [0.0; 8], coefficients: math::high_pass(rate, first) }
            }
            5 => {
                let raw_radius = 1.0 - f64::from(second) * f64::from(std::f32::consts::PI) / f64::from(rate);
                let radius = raw_radius as f32;
                let angle = (f64::from(first) * f64::from(std::f32::consts::TAU) / f64::from(rate)) as f32;
                let radius64 = f64::from(radius);
                let square = (raw_radius * raw_radius) as f32;
                let cosine = (2.0 * radius64 / (f64::from(square) + 1.0)) * math::cosine(angle);
                let root = math::square_root_one_plus(-(cosine * cosine) as f32);
                let feed = ((1.0 - f64::from(square)) * root * f64::from(third) / 256.0) as f32;
                let bypass = f64::from(second) >= f64::from(rate) * f64::from(0.63661975f32)
                    || first <= 0
                    || i32::from(first) >= (rate / 2) as i32;
                Self::BandPass { history: [0.0; 2], radius, cosine: cosine as f32, feed, bypass }
            }
            6 => Self::Gain(f32::from(first) / 256.0),
            7 => Self::AllPass {
                ring: vec![0.0; math::delay_length(rate, second)?],
                position: 0,
                feedback: f32::from(first >> 1) * (1.0f32 / 127.0),
            },
            8 => Self::Sum,
            9 => Self::Cache { block: Box::new([0.0; MAX_BLOCK]), ready: false, consumed: false },
            opcode => return Err(Error::Unsupported(opcode)),
        })
    }

    pub fn reset(&mut self) {
        match self {
            Self::OnePole { previous, .. } => *previous = 0.0,
            Self::HighPass { history, .. } => history.fill(0.0),
            Self::BandPass { history, .. } => history.fill(0.0),
            Self::AllPass { ring, position, .. } => {
                ring.fill(0.0);
                *position = 0;
            }
            Self::Cache { ready, consumed, block } => {
                *ready = false;
                *consumed = false;
                block.fill(0.0);
            }
            _ => {}
        }
    }

    pub fn cached(&mut self) -> Option<Block> {
        let Self::Cache { block, ready, consumed } = self else {
            return None;
        };
        if *consumed {
            *ready = false;
            *consumed = false;
        }
        if !*ready {
            return None;
        }
        *consumed = true;
        Some(**block)
    }

    pub fn render(&mut self, input: Block, second: Option<Block>, frames: usize) -> Block {
        let mut output = [0.0; MAX_BLOCK];
        match self {
            Self::Input => output = input,
            Self::OnePole { previous, alpha, beta } => {
                let mut state = f64::from(*previous);
                for i in 0..frames {
                    state = f64::from(*beta) * f64::from(input[i]) + state * f64::from(*alpha);
                    output[i] = state as f32;
                }
                *previous = state as f32;
            }
            Self::HighPass { history, coefficients } => {
                for i in 0..frames {
                    output[i] = (f64::from(history[3]) * f64::from(coefficients[4])
                        + (f64::from(history[0]) + f64::from(history[6])) * f64::from(coefficients[1])
                        + (f64::from(history[5]) + f64::from(history[1])) * f64::from(coefficients[2])
                        + (f64::from(history[2]) + f64::from(history[4])) * f64::from(coefficients[3])
                        + (f64::from(history[7]) + f64::from(input[i])) * f64::from(coefficients[0]))
                        as f32;
                    history.copy_within(0..7, 1);
                    history[0] = input[i];
                }
            }
            Self::BandPass { history, radius, cosine, feed, bypass } => {
                if *bypass {
                    return input;
                }
                let radius = f64::from(*radius);
                let cosine = f64::from(*cosine);
                for i in 0..frames {
                    output[i] = (2.0 * radius * cosine * f64::from(history[0])
                        + (f64::from(input[i]) + f64::from(1e-20f32)) * f64::from(*feed)
                        - radius * radius * f64::from(history[1])) as f32;
                    history[1] = history[0];
                    history[0] = output[i];
                }
            }
            Self::Gain(gain) => {
                for i in 0..frames {
                    output[i] = input[i] * *gain;
                }
            }
            Self::AllPass { ring, position, feedback } => {
                let gain = f64::from(*feedback);
                for i in 0..frames {
                    let delayed = f64::from(ring[*position]);
                    let next = gain * delayed + f64::from(input[i]) + f64::from(1e-20f32);
                    output[i] = (next * -gain + delayed) as f32;
                    ring[*position] = next as f32;
                    *position = (*position + 1) % ring.len();
                }
            }
            Self::Sum => {
                let second = second.unwrap_or([0.0; MAX_BLOCK]);
                for i in 0..frames {
                    output[i] = input[i] + second[i];
                }
            }
            Self::Cache { block, ready, .. } => {
                **block = input;
                *ready = true;
                output = input;
            }
        }
        output
    }
}
