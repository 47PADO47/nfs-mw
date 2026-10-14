use crate::{Connection, Layout, Node, Patch, Result, invalid};

const LENGTHS: [usize; 10] = [4, 8, 4, 4, 8, 8, 4, 8, 4, 4];

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(count).ok_or_else(|| invalid("size overflow"))?;
        let value = self.bytes.get(self.at..end).ok_or_else(|| invalid("truncated record"))?;
        self.at = end;
        Ok(value)
    }

    fn count(&mut self) -> Result<usize> {
        let raw: [u8; 4] = self.take(4)?.try_into().map_err(|_| invalid("short count"))?;
        usize::try_from(i32::from_be_bytes(raw)).map_err(|_| invalid("negative count"))
    }
}

pub(super) fn parse(bytes: &[u8]) -> Result<Patch> {
    let mut reader = Reader { bytes, at: 0 };
    if reader.take(4)? != b"FX\x0b\0" {
        return Err(invalid("signature or version"));
    }
    if reader.count()? != bytes.len() {
        return Err(invalid("declared size"));
    }
    let mut counts = [0; 12];
    for count in &mut counts {
        *count = reader.count()?;
    }
    let mut layouts: [Layout; 6] = std::array::from_fn(|_| Layout::default());
    for (index, layout) in layouts.iter_mut().enumerate() {
        if counts[index] > bytes.len() / 4 || counts[index + 6] > bytes.len() / 12 {
            return Err(invalid("counts exceed payload"));
        }
        for _ in 0..counts[index] {
            let prefix = reader.take(2)?;
            let opcode = prefix[0];
            let output_selector = prefix[1];
            let length = *LENGTHS.get(usize::from(opcode)).ok_or_else(|| invalid("unknown record length"))?;
            let mut parameters = [0; 3];
            for (i, word) in reader.take(length - 2)?.as_chunks::<2>().0.iter().enumerate() {
                parameters[i] = i16::from_be_bytes([word[0], word[1]]);
            }
            layout.nodes.push(Node { opcode, output_selector, parameters });
        }
        for _ in 0..counts[index + 6] {
            let source = reader.count()?;
            let destination = reader.count()?;
            let ports = reader.take(4)?;
            if source >= layout.nodes.len()
                || destination >= layout.nodes.len()
                || !matches!(ports[0], 1 | 2)
                || !matches!(ports[1], 1 | 2)
            {
                return Err(invalid("connection index or port"));
            }
            layout.connections.push(Connection {
                source,
                destination,
                source_port: ports[0],
                destination_port: ports[1],
            });
        }
    }
    if reader.at != bytes.len() {
        return Err(invalid("trailing bytes"));
    }
    Ok(Patch { layouts })
}
