use crate::{Block, Layout, MAX_BLOCK, Patch, Result, invalid, kernel::Kernel};

/// Prepared graph, with no render-frame timing or allocation during processing.
pub struct Processor {
    kernels: Vec<Kernel>,
    inputs: Vec<[Option<usize>; 2]>,
    outputs: [Option<usize>; 2],
    channels: u8,
    source_cursor: usize,
}

impl Processor {
    pub fn new(patch: &Patch, sample_rate: u32, channels: u8) -> Result<Self> {
        if !matches!(channels, 1 | 2) || !(8000..=192000).contains(&sample_rate) {
            return Err(invalid("unsupported channel count or sample rate"));
        }
        let selected = (0..usize::from(channels))
            .rev()
            .find(|&i| !patch.layouts[i].nodes.is_empty())
            .ok_or_else(|| invalid("no compatible layout"))?;
        let layout = &patch.layouts[selected];
        if layout.nodes.len() > 128 {
            return Err(invalid("graph resource limit"));
        }
        let mut inputs = vec![[None; 2]; layout.nodes.len()];
        for connection in &layout.connections {
            if connection.source >= layout.nodes.len()
                || connection.destination >= layout.nodes.len()
                || !matches!(connection.source_port, 1 | 2)
                || !matches!(connection.destination_port, 1 | 2)
            {
                return Err(invalid("connection index or port"));
            }
            let destination = &mut inputs[connection.destination][usize::from(connection.destination_port - 1)];
            if destination.is_some() {
                return Err(invalid("duplicate node input"));
            }
            *destination = Some(connection.source);
        }
        validate(layout, &inputs)?;
        let mut outputs = [None; 2];
        for (index, node) in layout.nodes.iter().enumerate() {
            let selectors = match node.opcode {
                9 => [node.output_selector & 15, node.output_selector >> 4],
                _ => [node.output_selector, 0],
            };
            for selector in selectors {
                if selector == 0 {
                    continue;
                }
                if usize::from(selector) > selected + 1 {
                    return Err(invalid("output selector outside selected layout"));
                }
                outputs[usize::from(selector - 1)] = Some(index);
            }
        }
        if selected == 0 && channels == 2 {
            outputs[1] = outputs[0];
        }
        if outputs.iter().all(Option::is_none) {
            return Err(invalid("no selected output"));
        }
        let kernels = layout.nodes.iter().map(|&node| Kernel::new(node, sample_rate)).collect::<Result<_>>()?;
        Ok(Self { kernels, inputs, outputs, channels: (selected + 1) as u8, source_cursor: 0 })
    }

    pub fn layout_channels(&self) -> u8 {
        self.channels
    }

    pub fn reset(&mut self) {
        self.source_cursor = 0;
        for kernel in &mut self.kernels {
            kernel.reset();
        }
    }

    pub fn process_mono(&mut self, input: &[f32], left: &mut [f32], right: &mut [f32]) -> Result<()> {
        let frames = input.len();
        if frames == 0
            || frames > MAX_BLOCK
            || !frames.is_multiple_of(16)
            || left.len() != frames
            || right.len() != frames
        {
            return Err(invalid("block size or output lengths"));
        }
        if input.iter().any(|sample| !sample.is_finite()) {
            return Err(invalid("nonfinite input"));
        }
        self.source_cursor = 0;
        left.fill(0.0);
        right.fill(0.0);
        if let Some(index) = self.outputs[0] {
            let block = self.evaluate(index, input, frames)?;
            left.copy_from_slice(&block[..frames]);
        }
        if self.channels == 1 {
            right.copy_from_slice(left);
            return Ok(());
        }
        if let Some(index) = self.outputs[1] {
            let block = self.evaluate(index, input, frames)?;
            right.copy_from_slice(&block[..frames]);
        }
        if left.iter().chain(right.iter()).any(|sample| !sample.is_finite()) {
            return Err(invalid("nonfinite output"));
        }
        Ok(())
    }

    fn evaluate(&mut self, index: usize, source: &[f32], frames: usize) -> Result<Block> {
        if matches!(self.kernels[index], Kernel::Input) {
            let end = self.source_cursor + frames;
            let input = source.get(self.source_cursor..end).ok_or_else(|| invalid("uncached repeated source read"))?;
            let mut block = [0.0; MAX_BLOCK];
            block[..frames].copy_from_slice(input);
            self.source_cursor = end;
            return Ok(block);
        }
        if let Some(cached) = self.kernels[index].cached() {
            return Ok(cached);
        }
        let [first, second] = self.inputs[index];
        let first = first.ok_or_else(|| invalid("missing upstream input"))?;
        let block = self.evaluate(first, source, frames)?;
        let second = match self.kernels[index] {
            Kernel::Sum => {
                Some(self.evaluate(second.ok_or_else(|| invalid("missing second sum input"))?, source, frames)?)
            }
            _ => None,
        };
        Ok(self.kernels[index].render(block, second, frames))
    }
}

fn validate(layout: &Layout, inputs: &[[Option<usize>; 2]]) -> Result<()> {
    for (index, node) in layout.nodes.iter().enumerate() {
        if node.opcode != 0 && inputs[index][0].is_none() {
            return Err(invalid("missing primary upstream input"));
        }
        if node.opcode == 8 && inputs[index][1].is_none() {
            return Err(invalid("missing second sum input"));
        }
    }
    let mut visited = vec![0u8; inputs.len()];
    for start in 0..inputs.len() {
        let mut pending = vec![(start, false)];
        while let Some((index, leaving)) = pending.pop() {
            if leaving {
                visited[index] = 2;
                continue;
            }
            if visited[index] == 1 {
                return Err(invalid("cyclic processor graph"));
            }
            if visited[index] == 2 {
                continue;
            }
            visited[index] = 1;
            pending.push((index, true));
            pending.extend(inputs[index].iter().flatten().map(|&upstream| (upstream, false)));
        }
    }
    Ok(())
}
